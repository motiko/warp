//! Rich-text editing and markdown rendering shared by file panes, the code editor and comments.

pub(crate) mod context_menu;
pub mod editor;
pub mod file;
pub mod link;
pub(crate) mod styles;
pub mod telemetry;

use warpui::AppContext;

/// Initialize rich-text keybindings.
pub fn init(app: &mut AppContext) {
    file::init(app);
    editor::view::init(app);
}
