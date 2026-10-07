//! Builds the [`ChannelConfig`] for Warp's channel binaries.
//!
//! Alefbet ships without Warp's private channel configuration, so every channel resolves to an
//! offline config: no server, no telemetry, no autoupdate and no crash reporting.
use std::borrow::Cow;

use warp_core::AppId;
use warp_core::channel::{ChannelConfig, OzConfig, WarpServerConfig};

/// Builds the offline [`ChannelConfig`] for the given channel name.
#[macro_export]
macro_rules! load_config {
    ($channel:expr) => {
        $crate::offline_config($channel)
    };
}

/// Returns an offline [`ChannelConfig`] whose log file is namespaced by `channel`.
pub fn offline_config(channel: &str) -> ChannelConfig {
    ChannelConfig {
        app_id: AppId::new("dev", "alefbet", "Alefbet"),
        logfile_name: Cow::Owned(format!("alefbet-{channel}.log")),
        server_config: WarpServerConfig::offline(),
        oz_config: OzConfig::offline(),
        telemetry_config: None,
        autoupdate_config: None,
        crash_reporting_config: None,
        mcp_static_config: None,
    }
}
