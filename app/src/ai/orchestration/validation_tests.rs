use ai::agent::action::RunAgentsExecutionMode;

use super::{accept_disabled_reason_with_auth, empty_env_recommendation_message};
use crate::ai::orchestration::config_state::{AuthSecretSelection, OrchestrationConfigState};

fn state(
    harness: &str,
    mode: RunAgentsExecutionMode,
    auth: AuthSecretSelection,
) -> OrchestrationConfigState {
    let mut state =
        OrchestrationConfigState::from_run_agents_fields(Some("auto"), Some(harness), &mode);
    state.auth_secret_selection = auth;
    state
}

fn cloud() -> RunAgentsExecutionMode {
    RunAgentsExecutionMode::Remote {
        environment_id: "env-1".to_string(),
        worker_host: "warp".to_string(),
        computer_use_enabled: false,
        runner_id: String::new(),
    }
}

#[test]
fn accept_allowed_for_oz_local_and_cloud() {
    for mode in [RunAgentsExecutionMode::Local, cloud()] {
        let state = state("oz", mode, AuthSecretSelection::Unset);
        assert_eq!(accept_disabled_reason_with_auth(&state), None);
    }
}

#[test]
fn empty_environment_recommendation_uses_visible_environment_availability() {
    let mode = RunAgentsExecutionMode::Remote {
        environment_id: String::new(),
        worker_host: "warp".to_string(),
        computer_use_enabled: false,
        runner_id: String::new(),
    };

    assert_eq!(
        empty_env_recommendation_message(&mode, true).as_deref(),
        Some("We recommend selecting an environment for cloud agents.")
    );
    assert_eq!(
        empty_env_recommendation_message(&mode, false).as_deref(),
        Some("We recommend creating an environment for cloud agents.")
    );
}

#[test]
fn accept_blocked_for_product_disabled_local_codex() {
    let state = state(
        "codex",
        RunAgentsExecutionMode::Local,
        AuthSecretSelection::Unset,
    );
    assert_eq!(
        accept_disabled_reason_with_auth(&state),
        Some("Local Codex child agents are temporarily disabled.".to_string())
    );
}

#[test]
fn accept_blocked_for_opencode_cloud() {
    let state = state("opencode", cloud(), AuthSecretSelection::Unset);
    let reason =
        accept_disabled_reason_with_auth(&state).expect("OpenCode + Cloud should block Accept");
    assert!(reason.contains("OpenCode"));
}

#[test]
fn accept_allowed_for_cloud_harness_regardless_of_auth_secret_selection() {
    for harness in ["claude", "codex"] {
        for auth in [
            AuthSecretSelection::Unset,
            AuthSecretSelection::Named("my-key".to_string()),
            AuthSecretSelection::Inherit,
        ] {
            let state = state(harness, cloud(), auth);
            assert_eq!(accept_disabled_reason_with_auth(&state), None);
        }
    }
}
