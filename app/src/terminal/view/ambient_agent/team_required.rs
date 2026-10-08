use warpui::{AppContext, SingletonEntity};

use crate::auth::AuthStateProvider;
use crate::workspaces::user_workspaces::UserWorkspaces;

pub(crate) const TOAST_ADMIN: &str =
    "Cloud agents need a team. Join or create a team to start cloud runs.";
pub(crate) const TOAST_NON_ADMIN: &str =
    "Cloud agents need a team. Join a team to start cloud runs.";

fn is_current_user_workspace_admin(app: &AppContext) -> bool {
    let Some(email) = AuthStateProvider::as_ref(app).get().user_email() else {
        return false;
    };
    UserWorkspaces::as_ref(app)
        .current_workspace()
        .is_some_and(|workspace| workspace.is_workspace_admin(&email))
}

/// Toast copy for submissions blocked by the team requirement, split by whether the user can
/// create teams.
pub(crate) fn toast_message(app: &AppContext) -> &'static str {
    if is_current_user_workspace_admin(app) {
        TOAST_ADMIN
    } else {
        TOAST_NON_ADMIN
    }
}
