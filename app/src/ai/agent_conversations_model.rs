pub mod entry;
mod query;

use std::collections::{HashMap, HashSet};

pub use entry::{
    AgentConversationEntry, AgentConversationEntryId, AgentConversationNavigationSubject,
};
use fuzzy_match::FuzzyMatchResult;
use itertools::Itertools;
pub use query::query_conversation_entries;
use warp_core::execution_mode::AppExecutionMode;
use warp_core::features::FeatureFlag;
use warp_core::ui::theme::WarpTheme;
use warp_core::ui::theme::color::internal_colors;
use warp_errors::report_error;
use warpui::color::ColorU;
use warpui::{AppContext, Entity, ModelContext, ModelHandle, RequestState, SingletonEntity};

use crate::ai::active_agent_views_model::ActiveAgentViewsModel;
use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::{AIConversationId, ConversationStatus};
use crate::ai::blocklist::{BlocklistAIHistoryEvent, BlocklistAIHistoryModel};
use crate::ai::conversation_navigation::ConversationNavigationData;
use crate::auth::AuthStateProvider;
use crate::auth::auth_manager::{AuthManager, AuthManagerEvent};
use crate::server::retry_strategies::OUT_OF_BAND_REQUEST_RETRY_STRATEGY;
use crate::server::server_api::ServerApiProvider;
use crate::settings::AISettings;
use crate::ui_components::icons::Icon;
use crate::workspace::{RestoreConversationLayout, WorkspaceAction};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitialConversationLoadState {
    LoadingLocal,
    WaitingForCloud,
    LoadingCloud,
    Loaded,
    CloudFailed,
}

impl InitialConversationLoadState {
    fn can_start_cloud_load(self) -> bool {
        match self {
            InitialConversationLoadState::WaitingForCloud => true,
            InitialConversationLoadState::LoadingLocal
            | InitialConversationLoadState::LoadingCloud
            | InitialConversationLoadState::Loaded
            | InitialConversationLoadState::CloudFailed => false,
        }
    }
}

/// Frontend-specific classification of a normalized conversation-list entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentConversationListEntryState {
    Selected,
    OpenElsewhere,
    Available,
    Unavailable,
}

/// Per-frontend policy for classifying normalized conversation-list entries.
pub trait AgentConversationListPolicy: 'static {
    /// Classifies `entry` as selected, open elsewhere, available, or unavailable.
    fn classify_entry(
        &self,
        entry: &AgentConversationEntry,
        app: &AppContext,
    ) -> AgentConversationListEntryState;
}

/// A normalized conversation entry paired with optional title-match metadata.
pub struct AgentConversationQueryResult {
    pub entry: AgentConversationEntry,
    pub title_match: Option<FuzzyMatchResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentRunDisplayStatus {
    InProgress,
    Succeeded,
    Error,
    Blocked { blocked_action: String },
    Cancelled,
}

impl AgentRunDisplayStatus {
    pub fn from_conversation_status(status: &ConversationStatus) -> Self {
        match status {
            ConversationStatus::InProgress => Self::InProgress,
            // A recovery is in flight; the run is still working.
            ConversationStatus::TransientError => Self::InProgress,
            ConversationStatus::Success => Self::Succeeded,
            ConversationStatus::Error => Self::Error,
            ConversationStatus::Cancelled => Self::Cancelled,
            ConversationStatus::Blocked { blocked_action } => Self::Blocked {
                blocked_action: blocked_action.clone(),
            },
            // Treat a yielded conversation as still in progress for the
            // agent-run list display so it stays in the working bucket.
            ConversationStatus::WaitingForEvents => Self::InProgress,
        }
    }

    pub fn to_conversation_status(&self) -> ConversationStatus {
        match self {
            AgentRunDisplayStatus::InProgress => ConversationStatus::InProgress,
            AgentRunDisplayStatus::Succeeded => ConversationStatus::Success,
            AgentRunDisplayStatus::Error => ConversationStatus::Error,
            AgentRunDisplayStatus::Blocked { blocked_action } => ConversationStatus::Blocked {
                blocked_action: blocked_action.clone(),
            },
            AgentRunDisplayStatus::Cancelled => ConversationStatus::Cancelled,
        }
    }

    pub fn is_working(&self) -> bool {
        matches!(self, AgentRunDisplayStatus::InProgress)
    }

    pub fn status_icon_and_color(&self, theme: &WarpTheme) -> (Icon, ColorU) {
        match self {
            AgentRunDisplayStatus::InProgress => (Icon::ClockLoader, theme.ansi_fg_magenta()),
            AgentRunDisplayStatus::Succeeded => (Icon::Check, theme.ansi_fg_green()),
            AgentRunDisplayStatus::Error => (Icon::Triangle, theme.ansi_fg_red()),
            AgentRunDisplayStatus::Blocked { .. } => (Icon::StopFilled, theme.ansi_fg_yellow()),
            AgentRunDisplayStatus::Cancelled => {
                (Icon::StopFilled, internal_colors::neutral_5(theme))
            }
        }
    }
}

impl std::fmt::Display for AgentRunDisplayStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRunDisplayStatus::InProgress => write!(f, "In progress"),
            AgentRunDisplayStatus::Succeeded => write!(f, "Done"),
            AgentRunDisplayStatus::Error => write!(f, "Error"),
            AgentRunDisplayStatus::Blocked { .. } => write!(f, "Blocked"),
            AgentRunDisplayStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Stores conversation metadata needed for display in conversation views.
pub struct ConversationMetadata {
    pub nav_data: ConversationNavigationData,
}

/// A unified interface for reading agent conversations from open panes, recently closed panes,
/// local history, and cloud-synced conversation metadata. Backs the conversation list surfaces.
pub struct AgentConversationsModel {
    /// A map of conversation IDs to live and recently closed conversations.
    conversations: HashMap<AIConversationId, ConversationMetadata>,
    initial_load_state: InitialConversationLoadState,
}

pub enum AgentConversationsModelEvent {
    /// Conversation data was loaded or refreshed.
    ConversationsLoaded,
    /// A conversation's status, title, artifacts, or metadata changed.
    ConversationUpdated,
}

impl Entity for AgentConversationsModel {
    type Event = AgentConversationsModelEvent;
}

impl SingletonEntity for AgentConversationsModel {}

impl AgentConversationsModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let auth_manager = AuthManager::handle(ctx);
        ctx.subscribe_to_model(&auth_manager, Self::handle_auth_manager_event);

        let history_model = BlocklistAIHistoryModel::handle(ctx);
        ctx.subscribe_to_model(&history_model, move |me, _, event, ctx| {
            me.handle_history_event(event, ctx);
        });

        let active_views_model = ActiveAgentViewsModel::handle(ctx);
        ctx.subscribe_to_model(&active_views_model, |me, _, _event, ctx| {
            me.sync_conversations(ctx);
        });

        let mut model = Self {
            conversations: HashMap::new(),
            initial_load_state: InitialConversationLoadState::LoadingLocal,
        };

        // Cloud conversation metadata is fetched on AuthComplete instead of here to avoid
        // duplicate requests at startup.
        if AppExecutionMode::as_ref(ctx).can_fetch_agent_runs_for_management() {
            model.sync_conversations(ctx);
        } else {
            model.initial_load_state = InitialConversationLoadState::Loaded;
        }
        model
    }

    fn handle_auth_manager_event(
        &mut self,
        _: ModelHandle<AuthManager>,
        event: &AuthManagerEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        if matches!(event, AuthManagerEvent::AuthComplete)
            && self.initial_load_state.can_start_cloud_load()
            && AppExecutionMode::as_ref(ctx).can_fetch_agent_runs_for_management()
        {
            self.fetch_cloud_conversation_metadata(ctx);
        }
    }

    /// Sync all conversations to the AgentConversationsModel.
    ///
    /// This function will loop through all active panes, recently closed panes, and historical
    /// conversations to construct a complete snapshot of conversations.
    pub fn sync_conversations(&mut self, ctx: &mut ModelContext<Self>) {
        if !FeatureFlag::InteractiveConversationManagementView.is_enabled() {
            return;
        }

        let nav_data_list = ConversationNavigationData::all_conversations(ctx);

        self.conversations.clear();
        for nav_data in nav_data_list {
            let conversation_id = nav_data.id;
            let metadata = ConversationMetadata { nav_data };
            self.conversations.insert(conversation_id, metadata);
        }
        if self.initial_load_state == InitialConversationLoadState::LoadingLocal {
            self.initial_load_state = InitialConversationLoadState::WaitingForCloud;
        }

        ctx.emit(AgentConversationsModelEvent::ConversationsLoaded);
    }

    /// Fetches cloud conversation metadata and merges it with the metadata stored in the local
    /// db in the BlocklistAIHistoryModel.
    fn fetch_cloud_conversation_metadata(&mut self, ctx: &mut ModelContext<Self>) {
        if AuthStateProvider::as_ref(ctx).get().user_id().is_none() {
            return;
        }
        if !AISettings::as_ref(ctx).is_any_ai_enabled(ctx) {
            return;
        }
        self.initial_load_state = InitialConversationLoadState::LoadingCloud;

        let ai_client = ServerApiProvider::as_ref(ctx).get_ai_client();
        ctx.spawn_with_retry_on_error(
            move || {
                let ai_client = ai_client.clone();
                async move { ai_client.list_ai_conversation_metadata(None).await }
            },
            OUT_OF_BAND_REQUEST_RETRY_STRATEGY,
            |model, result, ctx| match result {
                RequestState::RequestSucceeded(conversation_metadata) => {
                    model.initial_load_state = InitialConversationLoadState::Loaded;
                    if !conversation_metadata.is_empty() {
                        BlocklistAIHistoryModel::handle(ctx).update(ctx, |history_model, _| {
                            history_model.merge_cloud_conversation_metadata(conversation_metadata);
                        });
                    }
                    model.sync_conversations(ctx);
                    ctx.emit(AgentConversationsModelEvent::ConversationsLoaded);
                }
                RequestState::RequestFailed(e) => {
                    model.initial_load_state = InitialConversationLoadState::CloudFailed;
                    report_error!(e);
                }
                RequestState::RequestFailedRetryPending(_) => {}
            },
        );
    }

    /// Returns normalized, owned entries for conversation list and navigation surfaces, most
    /// recently updated first.
    pub fn get_entries(&self, app: &AppContext) -> Vec<AgentConversationEntry> {
        let history_model = BlocklistAIHistoryModel::as_ref(app);
        let mut entries = Vec::new();
        let mut emitted_conversation_ids = HashSet::new();

        for metadata in self.conversations.values() {
            emitted_conversation_ids.insert(metadata.nav_data.id);
            entries.push(entry::entry_for_conversation(metadata, history_model));
        }

        for metadata in history_model.get_local_conversations_metadata() {
            if emitted_conversation_ids.contains(&metadata.id) {
                continue;
            }
            let nav_data =
                ConversationNavigationData::from_historical_conversation_metadata(metadata);
            entries.push(entry::entry_for_historical_metadata(
                metadata,
                nav_data,
                history_model,
            ));
        }

        entries
            .into_iter()
            .sorted_by(|a, b| b.display.last_updated.cmp(&a.display.last_updated))
            .collect()
    }

    pub fn get_entry_by_id(
        &self,
        id: &AgentConversationEntryId,
        app: &AppContext,
    ) -> Option<AgentConversationEntry> {
        let history_model = BlocklistAIHistoryModel::as_ref(app);
        match id {
            AgentConversationEntryId::Conversation(conversation_id) => self
                .conversations
                .get(conversation_id)
                .map(|metadata| entry::entry_for_conversation(metadata, history_model))
                .or_else(|| {
                    history_model
                        .get_conversation_metadata(conversation_id)
                        .map(|metadata| {
                            let nav_data =
                                ConversationNavigationData::from_historical_conversation_metadata(
                                    metadata,
                                );
                            entry::entry_for_historical_metadata(metadata, nav_data, history_model)
                        })
                }),
        }
    }

    pub fn resolve_open_action(
        subject: AgentConversationNavigationSubject,
        restore_layout: Option<RestoreConversationLayout>,
        app: &AppContext,
    ) -> Option<WorkspaceAction> {
        let model = Self::as_ref(app);
        match subject {
            AgentConversationNavigationSubject::Entry(id) => model
                .get_entry_by_id(&id, app)
                .and_then(|entry| model.resolve_entry_open_action(&entry, restore_layout, app)),
            AgentConversationNavigationSubject::ServerToken(server_token) => model
                .entry_for_server_token(&server_token, app)
                .and_then(|entry| model.resolve_entry_open_action(&entry, restore_layout, app))
                .or(Some(WorkspaceAction::OpenConversationTranscriptViewer {
                    conversation_id: server_token,
                })),
        }
    }

    fn resolve_entry_open_action(
        &self,
        entry: &AgentConversationEntry,
        restore_layout: Option<RestoreConversationLayout>,
        app: &AppContext,
    ) -> Option<WorkspaceAction> {
        let active_views_model = ActiveAgentViewsModel::as_ref(app);

        if let Some(conversation_id) = entry.identity.local_conversation_id
            && active_views_model.is_conversation_open(conversation_id, app)
        {
            if let Some(nav_data) = self
                .conversations
                .get(&conversation_id)
                .map(|metadata| &metadata.nav_data)
            {
                return Some(WorkspaceAction::RestoreOrNavigateToConversation {
                    conversation_id,
                    window_id: nav_data.window_id,
                    pane_view_locator: nav_data.pane_view_locator,
                    terminal_view_id: nav_data.terminal_view_id,
                    restore_layout,
                });
            }

            if let Some(terminal_view_id) =
                active_views_model.get_terminal_view_id_for_conversation(conversation_id, app)
            {
                return Some(WorkspaceAction::FocusTerminalViewInWorkspace { terminal_view_id });
            }
        }

        if let Some(conversation_id) = entry.identity.local_conversation_id {
            let nav_data = self
                .conversations
                .get(&conversation_id)
                .map(|metadata| &metadata.nav_data);
            if !entry.backing.has_cloud_data
                || entry.backing.has_local_persisted_data
                || entry.backing.has_loaded_conversation
                || nav_data.is_some()
            {
                return Some(WorkspaceAction::RestoreOrNavigateToConversation {
                    conversation_id,
                    window_id: nav_data.and_then(|nav_data| nav_data.window_id),
                    pane_view_locator: None,
                    terminal_view_id: nav_data.and_then(|nav_data| nav_data.terminal_view_id),
                    restore_layout,
                });
            }
        }

        entry
            .identity
            .server_conversation_token
            .as_ref()
            .map(|token| WorkspaceAction::OpenConversationTranscriptViewer {
                conversation_id: token.clone(),
            })
    }

    fn entry_for_server_token(
        &self,
        server_token: &ServerConversationToken,
        app: &AppContext,
    ) -> Option<AgentConversationEntry> {
        let conversation_id = BlocklistAIHistoryModel::as_ref(app)
            .find_conversation_id_by_server_token(server_token)?;
        self.get_entry_by_id(
            &AgentConversationEntryId::Conversation(conversation_id),
            app,
        )
    }

    fn handle_history_event(
        &mut self,
        event: &BlocklistAIHistoryEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        if !FeatureFlag::InteractiveConversationManagementView.is_enabled() {
            return;
        }
        match event {
            // Events that affect conversation navigation data - need full sync
            BlocklistAIHistoryEvent::StartedNewConversation { .. }
            | BlocklistAIHistoryEvent::SetActiveConversation { .. }
            | BlocklistAIHistoryEvent::AppendedExchange { .. }
            | BlocklistAIHistoryEvent::SplitConversation { .. }
            | BlocklistAIHistoryEvent::RestoredConversations { .. }
            | BlocklistAIHistoryEvent::RemoveConversation { .. }
            | BlocklistAIHistoryEvent::DeletedConversation { .. }
            | BlocklistAIHistoryEvent::ClearedConversationsForTerminalSurface { .. }
            | BlocklistAIHistoryEvent::ClearedActiveConversation { .. } => {
                self.sync_conversations(ctx);
            }

            // Status, title, artifacts and token are looked up at render time, so only notify.
            BlocklistAIHistoryEvent::UpdatedConversationStatus { .. }
            | BlocklistAIHistoryEvent::UpdatedConversationTitle { .. }
            | BlocklistAIHistoryEvent::UpdatedConversationArtifacts { .. }
            | BlocklistAIHistoryEvent::ConversationServerTokenAssigned { .. } => {
                ctx.emit(AgentConversationsModelEvent::ConversationUpdated);
            }

            // Task/exchange-level changes that don't affect conversation navigation.
            BlocklistAIHistoryEvent::CreatedSubtask { .. }
            | BlocklistAIHistoryEvent::UpgradedTask { .. }
            | BlocklistAIHistoryEvent::ReassignedExchange { .. }
            | BlocklistAIHistoryEvent::UpdatedTodoList { .. }
            | BlocklistAIHistoryEvent::UpdatedAutoexecuteOverride { .. }
            // UpdatedStreamingExchange covers streaming and other exchange-level updates but
            // doesn't change any ConversationNavigationData fields (title comes from
            // UpdateTaskDescription, last_updated uses exchange.start_time which is set at append time).
            | BlocklistAIHistoryEvent::UpdatedStreamingExchange { .. }
            | BlocklistAIHistoryEvent::ConversationTransferredBetweenTerminalSurfaces { .. }
            | BlocklistAIHistoryEvent::ConversationUsageMetadataUpdated { .. }
            | BlocklistAIHistoryEvent::LocalSharedSessionEstablished { .. }
            | BlocklistAIHistoryEvent::UpdatedConversationMetadata { .. } => {}
        }
    }

    /// Clears all stored conversation data in memory.
    /// This is used when logging out to ensure no conversation history persists across users.
    pub(crate) fn reset(&mut self) {
        self.conversations.clear();
        self.initial_load_state = InitialConversationLoadState::WaitingForCloud;
    }
}

#[cfg(test)]
#[path = "agent_conversations_model_tests.rs"]
mod tests;
