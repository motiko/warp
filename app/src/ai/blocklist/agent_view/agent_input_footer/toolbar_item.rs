use serde::{Deserialize, Serialize};
use warpui::SingletonEntity;

use crate::context_chips::{ContextChipKind, agent_footer_available_chips};
use crate::features::FeatureFlag;
use crate::settings::CodeSettings;
use crate::terminal::shared_session::SharedSessionStatus;
use crate::ui_components::icons::Icon;

/// A configurable item
///
/// This unifies context-chip data displays with interactive control buttons so
/// they can all be arranged through the same drag-and-drop editor.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "An item that can appear in the agent toolbar.",
    rename_all = "snake_case"
)]
pub enum AgentToolbarItemKind {
    #[schemars(description = "A prompt context chip.")]
    ContextChip(ContextChipKind),
    // Agent view only
    ModelSelector,
    NLDToggle,
    ContextWindowUsage,
    /// Trigger for the "Conversation" usage popover. Gated on
    /// [`FeatureFlag::PricingTransparency`], since the popover's content is
    /// entirely usage figures.
    UsageSummary,
    FileExplorer,
    // Renamed from ImageAttach; alias preserves existing user toolbar configs.
    #[serde(alias = "ImageAttach")]
    FileAttach,
    /// Shows the fast-forward (auto-approve) toggle in the footer.
    FastForwardToggle,
}

impl AgentToolbarItemKind {
    /// Whether this item should be visible to session viewers.
    /// Items that control host settings or initiate actions on the host's
    /// behalf are hidden from viewers.
    pub fn available_to_session_viewer(&self, status: &SharedSessionStatus) -> bool {
        match self {
            Self::FileExplorer | Self::FileAttach => !status.is_viewer(),
            Self::FastForwardToggle => !status.is_viewer() || status.is_executor(),
            Self::ContextChip(_)
            | Self::ModelSelector
            | Self::NLDToggle
            | Self::ContextWindowUsage
            | Self::UsageSummary => true,
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::ContextChip(_) => "Context Chip",
            Self::ModelSelector => "Model Selector",
            Self::NLDToggle => "Autodetection",
            Self::FileAttach => "Attach File",
            Self::ContextWindowUsage => "Context Usage",
            Self::UsageSummary => "Conversation Usage",
            Self::FileExplorer => "File Explorer",
            Self::FastForwardToggle => "Fast Forward",
        }
    }

    pub fn icon(&self) -> Option<Icon> {
        match self {
            Self::ContextChip(kind) => kind.udi_icon(),
            Self::ModelSelector => Some(Icon::Agent),
            Self::NLDToggle => Some(Icon::NLD),
            Self::FileAttach => Some(Icon::Plus),
            Self::ContextWindowUsage => Some(Icon::ContextRemaining100),
            Self::UsageSummary => Some(Icon::PieChart),
            Self::FileExplorer => Some(Icon::FileCopy),
            Self::FastForwardToggle => Some(Icon::FastForward),
        }
    }

    /// Whether this item should be included in the toolbar given the current app state.
    /// Feature-flag checks live in `all_available()` / `default_*()`. This method
    /// handles runtime conditions that depend on user settings or workspace state.
    pub fn is_available(&self, app: &warpui::AppContext) -> bool {
        match self {
            // Drops the item from the toolbar editor once the flag goes off. The render
            // path does not consult this method, so it repeats the check itself.
            Self::UsageSummary => FeatureFlag::PricingTransparency.is_enabled(),
            // Matches the gating on every other project explorer entry point, so the chip
            // cannot open a tool view the rest of the app hides. See
            // `Workspace::compute_left_panel_views` and the `SHOW_PROJECT_EXPLORER`
            // keybinding predicate.
            Self::FileExplorer => {
                cfg!(feature = "local_fs") && *CodeSettings::as_ref(app).show_project_explorer
            }
            _ => true,
        }
    }

    pub fn is_context_chip(&self) -> bool {
        matches!(self, Self::ContextChip(_))
    }

    pub fn context_chip_kind(&self) -> Option<&ContextChipKind> {
        match self {
            Self::ContextChip(kind) => Some(kind),
            _ => None,
        }
    }

    /// Default left-side items for the agent view footer.
    pub fn default_left() -> Vec<Self> {
        let mut items = vec![
            Self::ContextChip(ContextChipKind::Ssh),
            Self::ContextChip(ContextChipKind::WorkingDirectory),
            Self::ContextChip(ContextChipKind::ShellGitBranch),
            Self::ContextChip(ContextChipKind::GitDiffStats),
        ];
        if FeatureFlag::GithubPrPromptChip.is_enabled() {
            items.push(Self::ContextChip(ContextChipKind::GithubPullRequest));
        }
        items.push(Self::NLDToggle);
        items
    }

    /// Default right-side items for the agent view footer.
    pub fn default_right() -> Vec<Self> {
        let mut items = vec![
            Self::ContextChip(ContextChipKind::AgentPlanAndTodoList),
            Self::ContextWindowUsage,
        ];
        if FeatureFlag::PricingTransparency.is_enabled() {
            items.push(Self::UsageSummary);
        }
        items.push(Self::ModelSelector);
        items.push(Self::FileAttach);
        items
    }

    /// All items available for the agent view footer configurator.
    pub fn all_available() -> Vec<Self> {
        let mut items: Vec<Self> = agent_footer_available_chips()
            .into_iter()
            .map(Self::ContextChip)
            .collect();
        items.extend([
            Self::ModelSelector,
            Self::NLDToggle,
            Self::FileAttach,
            Self::ContextWindowUsage,
            // Opt-in only: deliberately absent from `default_left`/`default_right`.
            Self::FileExplorer,
        ]);
        if FeatureFlag::PricingTransparency.is_enabled() {
            items.push(Self::UsageSummary);
        }
        if FeatureFlag::FastForwardAutoexecuteButton.is_enabled() {
            items.push(Self::FastForwardToggle);
        }
        items
    }
}

impl From<ContextChipKind> for AgentToolbarItemKind {
    fn from(kind: ContextChipKind) -> Self {
        Self::ContextChip(kind)
    }
}

#[cfg(test)]
#[path = "toolbar_item_tests.rs"]
mod tests;
