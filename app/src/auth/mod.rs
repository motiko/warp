pub mod auth_manager;
pub mod auth_redirect;
mod user_properties;
pub use auth_manager::AuthManager;
pub use auth_state::AuthStateProvider;
use itertools::Itertools;
pub use user_uid::UserUid;
use warp_core::user_preferences::GetUserPreferences as _;
use warp_errors::report_error;
pub use warp_server_auth::{auth_state, credentials, user, user_uid};
use warpui::{AppContext, SingletonEntity};

use crate::root_view::RootView;
use crate::server::telemetry::TelemetryEvent;
use crate::settings::{
    CRASH_REPORTING_ENABLED_DEFAULTS_KEY, PrivacySettings, TELEMETRY_ENABLED_DEFAULTS_KEY,
};
use crate::workspaces::update_manager::TeamUpdateManager;
use crate::{GlobalResourceHandlesProvider, persistence, send_telemetry_sync_from_app_ctx};

// Log out the user, clears workspace state, stops running processes, and deletes database.
pub fn log_out(app: &mut AppContext) {
    send_telemetry_sync_from_app_ctx!(TelemetryEvent::LogOut, app);

    let global_resource_handles = GlobalResourceHandlesProvider::as_ref(app).get();

    // As part of Logout v0, we remove sqlite3 so sessions and cloud objects don't persist between accounts.
    // TODO: Implement per-user scoping of sqlite3.
    persistence::remove(&global_resource_handles.model_event_sender);

    AuthManager::handle(app).update(app, |auth_manager, ctx| {
        auth_manager.log_out(ctx);
    });
    // Stop the workspace metadata polling loop that was started on login.
    TeamUpdateManager::handle(app).update(app, |manager, _| {
        manager.stop_polling_for_workspace_metadata_updates();
    });
    remove_cloud_persisted_settings(app);

    // Dispatch the GUI root-view action on every open GUI window so its state can be updated
    // correctly. Other front-ends, such as the TUI, manage their logout transition separately.
    let window_ids = app.window_ids().collect_vec();
    for window_id in window_ids {
        if let Some(root_views) = app.views_of_type::<RootView>(window_id) {
            let Some(root_view) = root_views.first() else {
                continue;
            };
            app.dispatch_action(
                window_id,
                &[root_view.id()],
                "root_view:log_out",
                &(),
                log::Level::Info,
            );
        }
    }

    #[cfg(target_family = "wasm")]
    crate::platform::wasm::emit_event(crate::platform::wasm::WarpEvent::LoggedOut);
}

// Remove the cloud persisted settings from user defaults.
// When a user signs out, we remove cloud persisted settings of their account.
// This is so they do not experience the old settings when they log in with a different account.
// Partial deletion of user defaults is a stopgap for Logout v0. The correct solution is:
fn remove_cloud_persisted_settings(app: &mut AppContext) {
    if let Err(e) = app
        .private_user_preferences()
        .remove_value(TELEMETRY_ENABLED_DEFAULTS_KEY)
    {
        report_error!(
            anyhow::Error::new(e)
                .context("Failed to remove Telemetry Enabled Defaults Key from user defaults")
        );
    }

    if let Err(e) = app
        .private_user_preferences()
        .remove_value(CRASH_REPORTING_ENABLED_DEFAULTS_KEY)
    {
        report_error!(
            anyhow::Error::new(e).context(
                "Failed to remove Crash Reporting Enabled Defaults Key from user defaults"
            )
        );
    }

    // Reset the Privacy Settings in the login screen to default values.
    PrivacySettings::handle(app).update(app, |privacy_settings, _| {
        privacy_settings.refresh_to_default();
    });
}
