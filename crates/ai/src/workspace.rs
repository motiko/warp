pub use persistence::workspace_metadata::WorkspaceMetadata;

/// An event to update the workspace metadata.
#[derive(Debug, Clone, Copy)]
pub enum WorkspaceMetadataEvent {
    Queried,
    Modified,
    Created,
}
