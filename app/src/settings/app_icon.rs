use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use warp_core::settings::macros::define_settings_group;
use warp_core::settings::{SupportedPlatforms, SyncToCloud};

/// The app icon to use (mac-only).
///
/// IMPORTANT NOTE: If you add a new icon, you will need to update the logic in
/// AlefbetDockTilePlugin.m to read the new icon and also add the icon to
/// app/DockTilePlugin/Resources.
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    Sequence,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "The app icon displayed in the dock.",
    rename_all = "snake_case"
)]
pub enum AppIcon {
    /// Blue alef outline on the dark Alefbet background.
    #[default]
    #[schemars(description = "Default")]
    Default,
    #[schemars(description = "Mono")]
    Mono,
    #[schemars(description = "Emerald")]
    Emerald,
    #[schemars(description = "Amber")]
    Amber,
    #[schemars(description = "Rose")]
    Rose,
    /// Dark alef outline on a light background.
    #[schemars(description = "Light")]
    Light,
}

impl std::fmt::Display for AppIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match &self {
            AppIcon::Default => "Default",
            AppIcon::Mono => "Mono",
            AppIcon::Emerald => "Emerald",
            AppIcon::Amber => "Amber",
            AppIcon::Rose => "Rose",
            AppIcon::Light => "Light",
        };
        write!(f, "{value}")
    }
}

impl AppIconSettings {
    pub fn get_base_icon_file_name(icon: AppIcon) -> &'static str {
        match icon {
            AppIcon::Default => "alefbet",
            AppIcon::Mono => "mono",
            AppIcon::Emerald => "emerald",
            AppIcon::Amber => "amber",
            AppIcon::Rose => "rose",
            AppIcon::Light => "light",
        }
    }
}

define_settings_group!(AppIconSettings, settings: [
    app_icon: AppIconState {
        type: AppIcon,
        default: AppIcon::Default,
        supported_platforms: SupportedPlatforms::MAC,
        sync_to_cloud: SyncToCloud::Never,
        surface: settings::SettingSurfaces::GUI,
        private: false,
        storage_key: "AppIcon",
        toml_path: "appearance.icon.app_icon",
        description: "The app icon displayed in the dock.",
    },
    show_dock_icon: ShowDockIconState {
        type: bool,
        default: true,
        supported_platforms: SupportedPlatforms::MAC,
        sync_to_cloud: SyncToCloud::Never,
        surface: settings::SettingSurfaces::GUI,
        private: false,
        storage_key: "ShowDockIcon",
        toml_path: "appearance.icon.show_dock_icon",
        description: "Whether Alefbet is shown in the macOS Dock and Cmd-Tab switcher.",
    },
]);
