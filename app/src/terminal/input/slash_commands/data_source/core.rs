use std::collections::HashMap;
use std::path::PathBuf;

use fuzzy_match::FuzzyMatchResult;
use ordered_float::OrderedFloat;
#[cfg(not(target_family = "wasm"))]
use repo_metadata::repositories::DetectedRepositories;
use warp_core::ui::appearance::Appearance;
#[cfg(not(target_family = "wasm"))]
use warp_util::local_or_remote_path::LocalOrRemotePath;
use warpui::fonts::FamilyId;
use warpui::{AppContext, Entity, EntityId, ModelContext, ModelHandle, SingletonEntity};

use crate::search::slash_command_menu::fuzzy_match::SlashCommandFuzzyMatchResult;
use crate::search::slash_command_menu::static_commands::{Availability, commands};
use crate::search::slash_command_menu::{SlashCommandId, StaticCommand};
use crate::terminal::input::slash_command_model::{
    DetectedCommand, ParsedSlashCommandInput, slash_command_composition_filter,
};
use crate::terminal::input::slash_commands::AcceptSlashCommand;
use crate::terminal::model::session::SessionType;
use crate::terminal::model::session::active_session::{ActiveSession, ActiveSessionEvent};
use crate::workspaces::user_workspaces::{TeamContext, TeamContextResolver};

/// Event emitted when the set of active slash commands changes.
#[derive(Debug, Clone, Copy)]
pub struct UpdatedActiveCommands;

/// Multiplier to ensure static commands always appear at the top of the match results.
const SCORE_MULTIPLIER: OrderedFloat<f64> = OrderedFloat(1000.0);

fn split_command_and_argument(buffer: &str) -> (&str, Option<&str>) {
    buffer
        .split_once(' ')
        .map_or((buffer, None), |(command, argument)| {
            (command, Some(argument))
        })
}

/// Subscribe a concrete data source to dependencies that affect command availability. The
/// callback remains concrete, so this helper does not require a surface trait.
pub(super) fn subscribe_to_shared_dependencies<T>(
    active_session: &ModelHandle<ActiveSession>,
    recompute_active_commands: fn(&mut T, &mut ModelContext<T>),
    ctx: &mut ModelContext<T>,
) where
    T: Entity<Event = UpdatedActiveCommands>,
{
    ctx.subscribe_to_model(active_session, move |me, _, event, ctx| match event {
        ActiveSessionEvent::UpdatedPwd | ActiveSessionEvent::Bootstrapped => {
            recompute_active_commands(me, ctx);
        }
    });
}

/// State shared by slash command data sources.
///
/// Surface-neutral behavior is provided by [`SlashCommandDataSource`]. Surface-specific behavior
/// such as agent view, compact rendering, recomputation, and event emission lives on the wrapping
/// data source types.
pub struct SlashCommandDataSourceState {
    active_session: ModelHandle<ActiveSession>,
    terminal_view_id: EntityId,
    active_commands_by_id: HashMap<SlashCommandId, StaticCommand>,
    active_repo_root: Option<PathBuf>,
    /// Resolves the team context of the window this data source's terminal surface belongs to,
    /// minted by that surface at construction. See [`SlashCommandDataSource::team_context`].
    team_context_resolver: TeamContextResolver,
}
impl SlashCommandDataSourceState {
    pub(super) fn new(
        active_session: ModelHandle<ActiveSession>,
        terminal_view_id: EntityId,
        team_context_resolver: TeamContextResolver,
    ) -> Self {
        Self {
            active_session,
            terminal_view_id,
            active_commands_by_id: HashMap::new(),
            active_repo_root: None,
            team_context_resolver,
        }
    }
}

/// Surface-neutral slash command behavior shared by slash command data sources.
///
/// Implementors provide access to their shared state. Default methods own the behavior whose
/// meaning is identical across surfaces, while each concrete surface retains lifecycle wiring,
/// availability policy, active-command recomputation, event emission, and query presentation.
pub trait SlashCommandDataSource {
    fn state(&self) -> &SlashCommandDataSourceState;

    fn state_mut(&mut self) -> &mut SlashCommandDataSourceState;

    fn active_session(&self) -> &ModelHandle<ActiveSession> {
        &self.state().active_session
    }

    fn terminal_view_id(&self) -> EntityId {
        self.state().terminal_view_id
    }

    /// The team context of the window this data source's terminal surface belongs to. Resolved
    /// on demand so it follows the surface if it is ever moved between windows.
    fn team_context<'a>(&self, app: &'a AppContext) -> TeamContext<'a> {
        (self.state().team_context_resolver)(app)
    }

    fn active_commands(&self) -> impl Iterator<Item = (&SlashCommandId, &StaticCommand)> {
        self.state().active_commands_by_id.iter()
    }

    /// Classifies slash command input.
    fn parse_input(&self, buffer: &str) -> ParsedSlashCommandInput {
        if !buffer.starts_with('/') {
            return ParsedSlashCommandInput::None;
        }
        if let Some(detected) = self.parse_slash_command(buffer) {
            return ParsedSlashCommandInput::SlashCommand(detected);
        }
        match slash_command_composition_filter(buffer) {
            Some(filter) => ParsedSlashCommandInput::Composing {
                filter: filter.to_owned(),
            },
            None => ParsedSlashCommandInput::None,
        }
    }

    /// Matches `buffer` against active slash commands, returning the detected command and
    /// space-delimited argument, if provided.
    fn parse_slash_command(&self, buffer: &str) -> Option<DetectedCommand> {
        let (possible_command, possible_argument) = split_command_and_argument(buffer);

        let is_matching_command = |command: &StaticCommand| {
            if command.name != possible_command {
                return false;
            }

            if let Some(argument) = command.argument.as_ref() {
                argument.is_optional || possible_argument.is_some()
            } else {
                possible_argument.is_none_or(|argument| argument.trim().is_empty())
            }
        };
        let matched_command = self
            .active_commands()
            .find_map(|(_, command)| is_matching_command(command).then(|| command.clone()))?;

        Some(DetectedCommand {
            command: matched_command,
            argument: possible_argument.map(str::to_owned),
        })
    }

    /// Update the active repository root for this terminal. Returns whether the value changed,
    /// so the caller can decide whether to recompute active commands.
    fn update_active_repo_root(&mut self, repo_root: Option<PathBuf>) -> bool {
        if self.state().active_repo_root != repo_root {
            self.state_mut().active_repo_root = repo_root;
            true
        } else {
            false
        }
    }

    /// Replace the active command set. Returns whether the active commands changed.
    fn replace_active_commands(
        &mut self,
        commands: HashMap<SlashCommandId, StaticCommand>,
    ) -> bool {
        if self.state().active_commands_by_id == commands {
            false
        } else {
            self.state_mut().active_commands_by_id = commands;
            true
        }
    }

    /// Availability bits for the current session.
    fn base_availability(&self, ctx: &AppContext) -> Availability {
        let mut availability = Availability::empty();

        let is_local = self
            .active_session()
            .as_ref(ctx)
            .session_type(ctx)
            .is_some_and(|st| st == SessionType::Local);
        if is_local {
            availability |= Availability::LOCAL;
        }

        // Derive REPOSITORY from the *live* working directory rather than the
        // cached `active_repo_root`. The cache is only refreshed after async git
        // detection resolves, but the pwd-changed recompute runs immediately on
        // `cd`; keying off the cache would leave repo-gated commands (e.g.
        // `/open-code-review`) available in the stale window after leaving a repo.
        // Repo roots are only tracked for local sessions, so this is gated on
        // `is_local`. `active_repo_root` is retained solely as the recompute
        // trigger that re-runs this once detection caches a newly-entered
        // repo's root.
        if is_local && self.cwd_is_in_repository(ctx) {
            availability |= Availability::REPOSITORY;
        }

        availability
    }

    /// Whether the active session's current working directory is inside a
    /// detected git repository. Uses the live cwd (not the cached
    /// `active_repo_root`) so REPOSITORY-gated commands update immediately on
    /// `cd`, without waiting for async repo detection to resolve. Delegates path
    /// membership to `DetectedRepositories`, reusing its centralized
    /// canonicalization + ancestor walk.
    #[cfg(not(target_family = "wasm"))]
    fn cwd_is_in_repository(&self, ctx: &AppContext) -> bool {
        let active_session = self.active_session().as_ref(ctx);
        let Some(cwd) = active_session.current_working_directory() else {
            return false;
        };

        // Repo detection converts the shell-native CWD (e.g. Git Bash/MSYS2/WSL
        // "/c/Users/...") to an OS-native path via `ShellLaunchData` before
        // caching the repo root (see the `detect_possible_git_repo` call site in
        // `terminal/view.rs`). The live CWD must go through the same conversion
        // so it can match those cached roots; otherwise repo-gated commands
        // would be hidden inside a repo on Windows shell variants. Fall back to
        // the raw path when no session/launch-data conversion applies (the
        // common native-shell case, where the conversion is already a no-op).
        let path = active_session
            .session(ctx)
            .and_then(|session| {
                session
                    .launch_data()
                    .and_then(|data| data.maybe_convert_absolute_path(cwd))
            })
            .unwrap_or_else(|| PathBuf::from(cwd));

        DetectedRepositories::as_ref(ctx)
            .get_root_for_path(&LocalOrRemotePath::Local(path))
            .is_some()
    }

    /// Repo detection is not wired up on wasm, so no directory is ever in a repo.
    #[cfg(target_family = "wasm")]
    fn cwd_is_in_repository(&self, _ctx: &AppContext) -> bool {
        false
    }

    /// Fuzzy-match the active commands against `query_text`. Returns scored [`InlineItem`]s with
    /// compact layout left unset; the caller applies any surface-specific presentation.
    fn match_active_commands(&self, query_text: &str, app: &AppContext) -> Vec<InlineItem> {
        let mut results = Vec::new();
        for (id, command) in &self.state().active_commands_by_id {
            let Some(fuzzy_result) = SlashCommandFuzzyMatchResult::try_match(
                query_text,
                command.name,
                None, // Don't match on description for slash commands.
            ) else {
                continue;
            };
            let score = fuzzy_result.score();
            // Only include results with score > 25 once the user has started typing a query and is past the first character
            if query_text.len() > 1 && score <= 25.0 {
                continue;
            }
            // Boost prefix matches so that closer matches rank above longer fuzzy matches.
            let prefix_boost = prefix_match_bonus(query_text, command.name);
            results.push(
                InlineItem::from_slash_command(id, command, app)
                    .with_name_match_result(fuzzy_result.name_match_result)
                    .with_description_match_result(fuzzy_result.description_match_result)
                    .with_score(
                        OrderedFloat(score) * SCORE_MULTIPLIER
                            + OrderedFloat(prefix_boost) * SCORE_MULTIPLIER
                            // Boost commands with shorter names, if match result is otherwise
                            // equal.
                            + OrderedFloat(1. / command.name.len() as f64),
                    ),
            );
        }
        results
    }

    /// Active commands ordered for the zero-state (empty query) menu.
    ///
    /// DataSource implementations must return highest priority items last (results sorted in
    /// ascending order of priority). This orders all active commands alphabetically, except for
    /// the explicitly prioritized commands, which are appended after them in the listed order.
    fn ordered_zero_state_commands(&self, app: &AppContext) -> Vec<InlineItem> {
        use itertools::Itertools;

        let prioritized_commands = vec![&*commands::EDIT];

        let mut active_prioritized_commands = vec![];
        let mut results = vec![];

        for (active_command_id, active_command) in self
            .active_commands()
            .sorted_by_key(|(_, command)| std::cmp::Reverse(&command.name))
        {
            if prioritized_commands
                .iter()
                .any(|prioritized_command| prioritized_command.name == active_command.name)
            {
                active_prioritized_commands.push((active_command_id, active_command));
            } else {
                results.push(InlineItem::from_slash_command(
                    active_command_id,
                    active_command,
                    app,
                ));
            }
        }

        for prioritized_command in prioritized_commands {
            if let Some((id, command)) = active_prioritized_commands
                .iter()
                .find(|(_, active_command)| active_command.name == prioritized_command.name)
            {
                results.push(InlineItem::from_slash_command(id, command, app));
            }
        }

        results
    }
}

/// Computes a bonus score for slash command matches where the query is a prefix
/// of the command name. This ensures closer matches rank above longer fuzzy matches.
///
/// Returns a value in `[0.0, 100.0]` based on the query's coverage of the name.
/// An exact match yields the maximum bonus of 100; partial prefix matches yield
/// a proportionally smaller bonus.
fn prefix_match_bonus(query: &str, name: &str) -> f64 {
    let name_lower = name.to_lowercase();
    let name_stripped = name_lower.strip_prefix('/').unwrap_or(&name_lower);
    if name_stripped.starts_with(query) {
        // coverage = 1.0 for exact match, smaller for partial prefix match.
        let coverage = query.len() as f64 / name_stripped.len() as f64;
        coverage * 100.0
    } else {
        0.0
    }
}

#[derive(Debug, Clone)]
pub struct InlineItem {
    pub action: AcceptSlashCommand,
    pub icon_path: Option<&'static str>,
    pub name: String,
    pub description: Option<String>,
    pub font_family: FamilyId,
    pub name_match_result: Option<FuzzyMatchResult>,
    pub description_match_result: Option<FuzzyMatchResult>,
    pub score: OrderedFloat<f64>,
    pub compact_layout: bool,
}

impl InlineItem {
    pub(super) fn from_slash_command(
        command_id: &SlashCommandId,
        command: &StaticCommand,
        app: &AppContext,
    ) -> Self {
        let appearance = Appearance::as_ref(app);
        Self {
            action: AcceptSlashCommand { id: *command_id },
            icon_path: Some(command.icon_path),
            name: command.name.to_owned(),
            description: Some(command.description.to_owned()),
            font_family: appearance.monospace_font_family(),
            name_match_result: None,
            description_match_result: None,
            score: OrderedFloat(f64::MIN),
            compact_layout: false,
        }
    }

    fn with_name_match_result(mut self, result: Option<FuzzyMatchResult>) -> Self {
        self.name_match_result = result;
        self
    }

    fn with_description_match_result(mut self, result: Option<FuzzyMatchResult>) -> Self {
        self.description_match_result = result;
        self
    }

    fn with_score(mut self, score: OrderedFloat<f64>) -> Self {
        self.score = score;
        self
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
