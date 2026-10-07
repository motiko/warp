//! Rich-text editing and markdown rendering shared by file panes, the code editor and comments.

pub(crate) mod context_menu;
pub mod editor;
pub mod file;
pub mod link;
pub(crate) mod styles;
pub mod telemetry;

use serde::{Deserialize, Serialize};
use warpui::AppContext;

/// A notebook location. Mainly, this lets us distinguish between cloud and file-based notebooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum NotebookLocation {
    /// A cloud notebook in the user's personal space.
    PersonalCloud,
    /// A cloud notebook in a team space.
    Team,
    /// A notebook backed by a local file.
    LocalFile,
    /// A notebook backed by a remote file.
    RemoteFile,
}

/// Initialize rich-text keybindings.
pub fn init(app: &mut AppContext) {
    file::init(app);
    editor::view::init(app);
}
