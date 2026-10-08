pub mod cloud_object_styling;
pub mod drive_helpers;
pub mod export;
pub mod folders;
pub mod import;
pub(crate) mod index;
pub mod items;
pub mod settings;
pub mod sharing;
pub mod workflows;

use std::fmt;

pub use cloud_objects::drive::CloudObjectTypeAndId;
pub use index::DriveIndexVariant;
use serde::{Deserialize, Serialize};
use warp_core::user_preferences::GetUserPreferences as _;
use warpui::AppContext;

use crate::server::ids::ServerId;
use crate::ui_components::icons::Icon;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct OpenWarpDriveObjectSettings {
    /// The folder that should be focused in the Warp Drive when the object is opened.
    pub focused_folder_id: Option<ServerId>,
    /// The email of the user to invite to the object, if the object is being opened via the request access flow.
    pub invitee_email: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DriveObjectType {
    Workflow,
    Notebook,
    Folder,
    EnvVarCollection,
}

impl From<DriveObjectType> for Icon {
    fn from(cloud_object_type: DriveObjectType) -> Icon {
        match cloud_object_type {
            DriveObjectType::Workflow => Icon::Workflow,
            DriveObjectType::Notebook => Icon::Notebook,
            DriveObjectType::Folder => Icon::Folder,
            DriveObjectType::EnvVarCollection => Icon::EnvVarCollection,
        }
    }
}

impl fmt::Display for DriveObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriveObjectType::Notebook => write!(f, "notebook"),
            DriveObjectType::Workflow => write!(f, "workflow"),
            DriveObjectType::Folder => write!(f, "folder"),
            DriveObjectType::EnvVarCollection => write!(f, "env var collection"),
        }
    }
}

pub fn should_auto_open_welcome_folder(app: &mut AppContext) -> bool {
    app.private_user_preferences()
        .read_value(settings::HAS_AUTO_OPENED_WELCOME_FOLDER)
        .unwrap_or_default()
        .and_then(|s| serde_json::from_str(&s).ok())
        .map(|has_opened: bool| !has_opened)
        .unwrap_or(true)
}

pub fn write_has_auto_opened_welcome_folder_to_user_defaults(app: &mut AppContext) {
    let _ = app
        .private_user_preferences()
        .write_value(settings::HAS_AUTO_OPENED_WELCOME_FOLDER, true.to_string());
}

/// Enum used for sorting elements in the Warp Drive Index (and potentially other places).
/// In the future it can be used to add other options (like, by name or by author), and exposed to
/// users in the index.
#[derive(
    Default,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Sort order for Warp Drive items.",
    rename_all = "snake_case"
)]
pub enum DriveSortOrder {
    /// Sort by newest revision first in main index, most recently trashed in trash index
    #[default]
    ByTimestamp,
    /// A => Z
    AlphabeticalDescending,
    /// Z => A
    AlphabeticalAscending,
    /// Sort by object type, with folders first
    ByObjectType,
}
