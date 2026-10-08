use std::sync::Arc;

use remote_server::auth::RemoteServerAuthContext;
use warpui::r#async::BoxFuture;

use crate::auth::auth_state::AuthState;

/// Builds the app-wide auth context used by remote-server connections. The app has no account,
/// so connections never carry an auth token and are keyed by the local anonymous id.
pub fn local_auth_context(
    auth_state: Arc<AuthState>,
    crash_reporting_enabled: bool,
) -> RemoteServerAuthContext {
    let anonymous_id = auth_state.anonymous_id();
    RemoteServerAuthContext::new(
        || -> BoxFuture<'static, Option<String>> { Box::pin(async { None }) },
        move || anonymous_id.clone(),
        String::new(),
        String::new(),
        crash_reporting_enabled,
    )
}
