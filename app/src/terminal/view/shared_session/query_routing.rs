use crate::terminal::TerminalModel;

/// How a follow-up prompt for this pane should be routed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AIQueryRouting {
    /// Connected as a live shared-session viewer; the follow-up is forwarded to the sharer.
    /// `is_executor` gates whether this viewer may submit.
    LiveRemoteVm { is_executor: bool },
    /// Continues on the local machine.
    Local,
}

impl AIQueryRouting {
    pub(crate) fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }
}

/// Resolves the [`AIQueryRouting`] for a pane. `terminal_model` must already be locked by the
/// caller; this function does not lock it.
pub(crate) fn resolve_ai_query_routing(terminal_model: &TerminalModel) -> AIQueryRouting {
    let status = terminal_model.shared_session_status();
    if status.is_active_viewer() {
        AIQueryRouting::LiveRemoteVm {
            is_executor: status.is_executor(),
        }
    } else {
        AIQueryRouting::Local
    }
}
