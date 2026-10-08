use remote_server::auth::RemoteServerAuthContext;
use warpui::AppContext;
use warpui::r#async::BoxFuture;

use crate::anonymous_id::get_or_create_anonymous_id;

/// Builds the app-wide auth context used by remote-server connections. The app has no account,
/// so connections never carry an auth token and are keyed by the local anonymous id.
pub fn local_auth_context(
    crash_reporting_enabled: bool,
    ctx: &AppContext,
) -> RemoteServerAuthContext {
    let anonymous_id = get_or_create_anonymous_id(ctx).to_string();
    RemoteServerAuthContext::new(
        || -> BoxFuture<'static, Option<String>> { Box::pin(async { None }) },
        move || anonymous_id.clone(),
        String::new(),
        String::new(),
        crash_reporting_enabled,
    )
}
