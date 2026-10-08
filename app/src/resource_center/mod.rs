mod main_page;
pub mod utils;
pub use main_page::ResourceCenterMainView;
mod keybindings_page;
pub use keybindings_page::KeybindingsView;
mod section_views;
mod view;
pub use view::{ResourceCenterAction, ResourceCenterEvent, ResourceCenterPage, ResourceCenterView};
