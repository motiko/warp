//! Shared utilities for file-attachment handling (size limits, download, and resolving
//! shared-session prompt attachments).

use session_sharing_protocol::common::AgentAttachment;

use crate::terminal::model::BlockId;

/// Splits a shared-session prompt's `AgentAttachment`s into the three shapes callers need to
/// stage a follow-up: block references (for the context model), plain-text selections (also for
/// the context model, joined with `\n` by the caller), and file references (attachment id,
/// display name).
pub(crate) fn resolve_agent_attachments(
    attachments: Vec<AgentAttachment>,
) -> (Vec<BlockId>, Vec<String>, Vec<(String, String)>) {
    let mut block_ids = Vec::new();
    let mut selected_text_parts = Vec::new();
    let mut file_downloads = Vec::new();
    for attachment in attachments {
        match attachment {
            AgentAttachment::BlockReference { block_id } => {
                block_ids.push(BlockId::from(block_id.to_string()));
            }
            AgentAttachment::PlainText { content } => {
                selected_text_parts.push(content);
            }
            AgentAttachment::FileReference {
                attachment_id,
                file_name,
            } => {
                file_downloads.push((attachment_id, file_name));
            }
        }
    }
    (block_ids, selected_text_parts, file_downloads)
}
