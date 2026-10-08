use chrono::{DateTime, Utc};
use warpui::AppContext;

use super::{AgentRunDisplayStatus, ConversationMetadata};
use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::blocklist::history_model::{AIConversationMetadata, BlocklistAIHistoryModel};
use crate::ai::conversation_navigation::ConversationNavigationData;
use crate::workspace::RestoreConversationLayout;

/// Stable projection identity used by list and navigation surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentConversationEntryId {
    Conversation(AIConversationId),
}

impl AgentConversationEntryId {
    pub fn as_key(&self) -> String {
        match self {
            AgentConversationEntryId::Conversation(id) => format!("conv_{id}"),
        }
    }
}

/// Navigation request input for resolving an entry or server-token handle at action time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentConversationNavigationSubject {
    Entry(AgentConversationEntryId),
    ServerToken(ServerConversationToken),
}

/// Normalized row data for agent conversation list and navigation surfaces.
///
/// The entry keeps local conversation identity, cloud token identity, display fields, and source
/// availability together so callers do not recompute navigation policy from stale partial sources.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentConversationEntry {
    pub id: AgentConversationEntryId,
    pub identity: AgentConversationIdentity,
    pub provenance: AgentConversationProvenance,
    pub display: AgentConversationDisplayData,
    pub backing: AgentConversationBackingData,
}

/// Cross-system identifiers that may refer to the same underlying conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentConversationIdentity {
    pub local_conversation_id: Option<AIConversationId>,
    pub server_conversation_token: Option<ServerConversationToken>,
}

/// Display-only fields for rendering a conversation entry without consulting source models.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentConversationDisplayData {
    pub title: String,
    pub last_updated: DateTime<Utc>,
    pub status: AgentRunDisplayStatus,
    pub working_directory: Option<String>,
}

/// Source category that explains why an entry exists and which backing systems can refresh it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentConversationProvenance {
    LocalInteractive,
    CloudSyncedConversation,
}

/// Availability flags for the source data that contributed to an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentConversationBackingData {
    pub has_loaded_conversation: bool,
    pub has_local_persisted_data: bool,
    pub has_cloud_data: bool,
}

impl AgentConversationEntry {
    pub fn has_open_action(
        &self,
        restore_layout: Option<RestoreConversationLayout>,
        app: &AppContext,
    ) -> bool {
        super::AgentConversationsModel::resolve_open_action(
            AgentConversationNavigationSubject::Entry(self.id),
            restore_layout,
            app,
        )
        .is_some()
    }
}

fn conversation_title(
    metadata: &ConversationMetadata,
    history_model: &BlocklistAIHistoryModel,
) -> String {
    history_model
        .conversation(&metadata.nav_data.id)
        .and_then(|conversation| conversation.title().clone())
        .unwrap_or(metadata.nav_data.title.clone())
}

fn conversation_display_status(
    metadata: &ConversationMetadata,
    history_model: &BlocklistAIHistoryModel,
) -> AgentRunDisplayStatus {
    history_model
        .conversation(&metadata.nav_data.id)
        .map(|conversation| AgentRunDisplayStatus::from_conversation_status(conversation.status()))
        .unwrap_or(AgentRunDisplayStatus::Succeeded)
}

pub(super) fn entry_for_conversation(
    metadata: &ConversationMetadata,
    history_model: &BlocklistAIHistoryModel,
) -> AgentConversationEntry {
    let conversation_metadata = history_model.get_conversation_metadata(&metadata.nav_data.id);
    entry_for_conversation_parts(
        metadata.nav_data.clone(),
        conversation_metadata,
        history_model,
    )
}

pub(super) fn entry_for_historical_metadata(
    metadata: &AIConversationMetadata,
    nav_data: ConversationNavigationData,
    history_model: &BlocklistAIHistoryModel,
) -> AgentConversationEntry {
    entry_for_conversation_parts(nav_data, Some(metadata), history_model)
}

fn entry_for_conversation_parts(
    nav_data: ConversationNavigationData,
    conversation_metadata: Option<&AIConversationMetadata>,
    history_model: &BlocklistAIHistoryModel,
) -> AgentConversationEntry {
    let metadata = ConversationMetadata { nav_data };
    let conversation_id = metadata.nav_data.id;
    let server_conversation_token = server_conversation_token_for_conversation(
        conversation_id,
        Some(&metadata.nav_data),
        history_model,
    );
    let has_loaded_conversation = history_model.conversation(&conversation_id).is_some();
    let has_local_persisted_data = conversation_metadata
        .is_some_and(|metadata| metadata.has_local_data)
        || has_loaded_conversation;
    let has_cloud_data = conversation_metadata.is_some_and(|metadata| metadata.has_cloud_data)
        || server_conversation_token.is_some();
    let provenance = if has_cloud_data {
        AgentConversationProvenance::CloudSyncedConversation
    } else {
        AgentConversationProvenance::LocalInteractive
    };

    AgentConversationEntry {
        id: AgentConversationEntryId::Conversation(conversation_id),
        identity: AgentConversationIdentity {
            local_conversation_id: Some(conversation_id),
            server_conversation_token,
        },
        provenance,
        display: AgentConversationDisplayData {
            title: conversation_title(&metadata, history_model),
            last_updated: metadata.nav_data.last_updated.into(),
            status: conversation_display_status(&metadata, history_model),
            working_directory: metadata
                .nav_data
                .latest_working_directory
                .clone()
                .or_else(|| metadata.nav_data.initial_working_directory.clone()),
        },
        backing: AgentConversationBackingData {
            has_loaded_conversation,
            has_local_persisted_data,
            has_cloud_data,
        },
    }
}

fn server_conversation_token_for_conversation(
    conversation_id: AIConversationId,
    nav_data: Option<&ConversationNavigationData>,
    history_model: &BlocklistAIHistoryModel,
) -> Option<ServerConversationToken> {
    history_model
        .conversation(&conversation_id)
        .and_then(|conversation| conversation.server_conversation_token())
        .cloned()
        .or_else(|| {
            history_model
                .get_conversation_metadata(&conversation_id)
                .and_then(|metadata| metadata.server_conversation_token.clone())
        })
        .or_else(|| nav_data.and_then(|nav_data| nav_data.server_conversation_token.clone()))
}
