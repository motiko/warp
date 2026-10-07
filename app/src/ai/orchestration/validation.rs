//! Frontend-neutral validation predicates for orchestration edit flows.

use ai::agent::action::RunAgentsExecutionMode;
use warp_cli::agent::Harness;

use super::config_state::OrchestrationConfigState;
use crate::ai::local_harness_setup::{LocalHarnessSetupState, local_harness_setup_state};
use crate::ai::orchestration::providers::ORCHESTRATION_WARP_WORKER_HOST;

/// Returns `true` when the auth secret picker should be visible. No harness has managed
/// auth-secret types once managed secrets are gone, so the picker never shows; the picker
/// itself is removed with the cloud-agent orchestration layer.
pub fn should_show_auth_secret_picker() -> bool {
    false
}

/// [`OrchestrationConfigState::accept_disabled_reason`] plus the local harness
/// setup gate. Card views should prefer this.
pub fn accept_disabled_reason_with_auth(state: &OrchestrationConfigState) -> Option<String> {
    if let Some(reason) = state.accept_disabled_reason() {
        return Some(reason.to_string());
    }
    if matches!(state.execution_mode, RunAgentsExecutionMode::Local)
        && let Some(harness) = Harness::parse_local_child_harness(&state.harness_type)
    {
        match local_harness_setup_state(harness) {
            LocalHarnessSetupState::MissingHarness { tooltip } => {
                return Some(tooltip.to_string());
            }
            LocalHarnessSetupState::ProductDisabled { message } => {
                return Some(message.to_string());
            }
            LocalHarnessSetupState::Ready => {}
        }
    }
    None
}

/// Soft recommendation copy shown when a Warp-hosted Cloud run has no
/// environment selected. `None` when not applicable.
pub fn empty_env_recommendation_message(
    execution_mode: &RunAgentsExecutionMode,
    has_visible_environment: bool,
) -> Option<String> {
    let RunAgentsExecutionMode::Remote {
        environment_id,
        worker_host,
        ..
    } = execution_mode
    else {
        return None;
    };
    if !environment_id.trim().is_empty() {
        return None;
    }
    if !worker_host.eq_ignore_ascii_case(ORCHESTRATION_WARP_WORKER_HOST) {
        return None;
    }
    Some(if has_visible_environment {
        "We recommend selecting an environment for cloud agents.".to_string()
    } else {
        "We recommend creating an environment for cloud agents.".to_string()
    })
}

#[cfg(test)]
#[path = "validation_tests.rs"]
mod tests;
