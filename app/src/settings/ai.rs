//! Settings for optional AI features.

use ai_config::api_keys::CustomEndpointDefinitions;
use settings::{RespectUserSyncSetting, SupportedPlatforms, SyncToCloud, define_settings_group};

define_settings_group!(AISettings, settings: [
    // If `false`, all AI features are disabled.
    is_any_ai_enabled: IsAnyAIEnabled {
        type: bool,
        default: false,
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::No),
        surface: settings::SettingSurfaces::GUI,
        private: false,
        toml_path: "agents.warp_agent.is_any_ai_enabled",
        description: "Controls whether all AI features are enabled.",
    },
    // Non-secret custom inference endpoint definitions. Credentials remain in secure storage.
    custom_endpoints: CustomEndpoints {
        type: CustomEndpointDefinitions,
        default: CustomEndpointDefinitions::default(),
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        surface: settings::SettingSurfaces::ALL,
        private: false,
        toml_path: "agents.custom_endpoints",
        max_table_depth: 2,
        description: "Custom inference endpoint definitions.",
    }
]);

impl AISettings {
    pub fn is_any_ai_enabled(&self) -> bool {
        *self.is_any_ai_enabled
    }
}
