use std::time::{Duration, SystemTime};

use ai::api_keys::{
    ApiKeyManager, GEAP_REFRESH_LEAD_TIME, GeapCredentials, GeapCredentialsState, GeapFederation,
    GeapMintBinding, GeapRefreshOutcome, LoadGeapCredentialsError,
};
use futures::channel::oneshot;
use warp_errors::report_error;
use warpui::r#async::Timer;
use warpui::{AppContext, ModelContext, SingletonEntity};

use crate::auth::AuthStateProvider;
use crate::settings::{AISettings, AISettingsChangedEvent};
use crate::workspaces::user_workspaces::{
    GeminiEnterpriseBackgroundHost, TeamScope, UserWorkspaces, UserWorkspacesEvent,
};

/// Floor on the proactive refresh timer delay so a near-expired store
/// cannot spin mint -> store -> re-mint as a hot loop;
const GEAP_MIN_TIMER_DELAY: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GeapPolicy {
    Disabled,
    Unconfigured,
    /// Two or more of the user's teams enable Gemini Enterprise against different Google
    /// Cloud projects, named here.
    Conflicting(Vec<String>),
    Mintable(GeapMintBinding),
}

impl GeapPolicy {
    pub(crate) fn mint_binding(self) -> Option<GeapMintBinding> {
        match self {
            GeapPolicy::Mintable(binding) => Some(binding),
            GeapPolicy::Disabled | GeapPolicy::Unconfigured | GeapPolicy::Conflicting(_) => None,
        }
    }
}

fn geap_mint_binding_from_parts(
    user_uid: String,
    gcp_audience: Option<&str>,
    gcp_sa_email: Option<&str>,
) -> Option<GeapMintBinding> {
    let audience = gcp_audience.map(str::trim).unwrap_or_default();
    if audience.is_empty() {
        return None;
    }
    let federation = match gcp_sa_email
        .map(str::trim)
        .filter(|sa_email| !sa_email.is_empty())
    {
        Some(email) => GeapFederation::ServiceAccount {
            email: email.to_string(),
        },
        None => GeapFederation::DirectWif,
    };
    Some(GeapMintBinding {
        user_uid,
        audience: audience.to_string(),
        federation,
    })
}

fn geap_policy_from_host_settings(
    settings: Option<&crate::workspaces::workspace::LlmHostSettings>,
    app: &AppContext,
) -> GeapPolicy {
    let Some(settings) = settings else {
        return GeapPolicy::Unconfigured;
    };
    let Some(user_id) = AuthStateProvider::as_ref(app).get().user_id() else {
        return GeapPolicy::Disabled;
    };
    match geap_mint_binding_from_parts(
        user_id.as_string(),
        settings.gcp_audience.as_deref(),
        settings.gcp_sa_email.as_deref(),
    ) {
        Some(binding) => GeapPolicy::Mintable(binding),
        None => GeapPolicy::Unconfigured,
    }
}

/// The GEAP policy for a request made under `scope`. Use this whenever a scope is available --
/// i.e. whenever the work is rooted in a window.
pub(crate) fn current_geap_policy<S: TeamScope + ?Sized>(
    scope: &S,
    app: &AppContext,
) -> GeapPolicy {
    let user_workspaces = UserWorkspaces::as_ref(app);
    if !user_workspaces.is_gemini_enterprise_credentials_enabled(scope, app) {
        return GeapPolicy::Disabled;
    }
    geap_policy_from_host_settings(user_workspaces.gemini_enterprise_host_settings(scope), app)
}

/// The GEAP gate for the single, app-wide credential store.
///
/// Every trigger for a background mint -- a settings poll, a token nearing expiry, a
/// request-time safety net with no scope threaded to it yet -- has no window behind it, so this
/// deliberately takes no team scope and instead reads across all of the user's teams:
/// background GEAP work succeeds if any one of them enables it. See
/// [`UserWorkspaces::gemini_enterprise_host_for_any_enabling_team`].
pub(crate) fn current_geap_policy_for_any_team(app: &AppContext) -> GeapPolicy {
    match UserWorkspaces::as_ref(app).gemini_enterprise_host_for_any_enabling_team(app) {
        GeminiEnterpriseBackgroundHost::NoneEnabled => GeapPolicy::Disabled,
        // Nothing can be minted, but the user's org does use GEAP and an admin has to align
        // the disagreeing teams on one project. `Conflicting` is the state that says so, named
        // by the teams involved, and offers the same admin recovery action `Unconfigured`
        // does; `Disabled` would tell them the feature is simply not theirs.
        GeminiEnterpriseBackgroundHost::Conflicting(team_names) => {
            log::warn!(
                "GEAP: teams ({}) enable Gemini Enterprise against different Google Cloud \
                 projects; background minting has no window to choose between them",
                team_names.join(", ")
            );
            GeapPolicy::Conflicting(team_names.into_iter().map(str::to_string).collect())
        }
        GeminiEnterpriseBackgroundHost::Enabled(settings) => {
            geap_policy_from_host_settings(Some(settings), app)
        }
    }
}

pub trait GeapCredentialRefresher {
    fn subscribe_to_geap_settings_changes(&mut self, ctx: &mut ModelContext<Self>)
    where
        Self: Sized;
}

impl GeapCredentialRefresher for ApiKeyManager {
    fn subscribe_to_geap_settings_changes(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.subscribe_to_model(&UserWorkspaces::handle(ctx), |manager, _, event, ctx| {
            if matches!(
                event,
                UserWorkspacesEvent::UpdateWorkspaceSettingsSuccess
                    | UserWorkspacesEvent::TeamsChanged
            ) {
                refresh_geap_credentials(manager, ctx);
            }
        });

        ctx.subscribe_to_model(&AISettings::handle(ctx), |manager, _, event, ctx| {
            if matches!(
                event,
                AISettingsChangedEvent::GeminiEnterpriseCredentialsEnabled { .. }
            ) {
                refresh_geap_credentials(manager, ctx);
            }
        });
    }
}

/// Standard (non-forced) refresh: the skip-if-valid guard decides whether a
/// mint is actually needed.
pub(crate) fn refresh_geap_credentials(
    manager: &mut ApiKeyManager,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    refresh_geap_credentials_with_options(manager, false, None, ctx);
}

pub(crate) fn force_refresh_geap_credentials(
    manager: &mut ApiKeyManager,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    refresh_geap_credentials_with_options(manager, true, None, ctx);
}

/// Mint kickoff for a request blocked on an expired credential.
pub(crate) fn start_geap_refresh_for_waiter(
    manager: &mut ApiKeyManager,
    waiter: oneshot::Sender<GeapRefreshOutcome>,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    refresh_geap_credentials_with_options(manager, false, Some(waiter), ctx);
}

/// Request-time safety net. The triggering request is never delayed —
/// it carries the currently stored token.
pub(crate) fn refresh_geap_credentials_if_needed(
    manager: &mut ApiKeyManager,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    let binding = match current_geap_policy_for_any_team(ctx) {
        GeapPolicy::Disabled | GeapPolicy::Unconfigured | GeapPolicy::Conflicting(_) => return,
        GeapPolicy::Mintable(binding) => binding,
    };
    let needs_mint = match manager.geap_credentials_state() {
        GeapCredentialsState::Refreshing { .. } => false,
        GeapCredentialsState::Loaded {
            credentials,
            minted_for,
            ..
        } => *minted_for != binding || credentials.needs_refresh(),
        GeapCredentialsState::Missing
        | GeapCredentialsState::Unconfigured
        | GeapCredentialsState::ConflictingAcrossTeams { .. }
        | GeapCredentialsState::Disabled
        | GeapCredentialsState::Failed { .. } => true,
    };
    if needs_mint {
        log::info!("GEAP: request-time safety net arming a credential refresh");
        refresh_geap_credentials(manager, ctx);
    }
}

/// The refresh guard + mint kickoff that all triggers funnel through.
fn refresh_geap_credentials_with_options(
    manager: &mut ApiKeyManager,
    force: bool,
    waiter: Option<oneshot::Sender<GeapRefreshOutcome>>,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    let minted_for = match current_geap_policy_for_any_team(ctx) {
        GeapPolicy::Disabled => {
            manager.set_geap_credentials_state(GeapCredentialsState::Disabled, ctx);
            return;
        }
        GeapPolicy::Unconfigured => {
            manager.set_geap_credentials_state(GeapCredentialsState::Unconfigured, ctx);
            return;
        }
        GeapPolicy::Conflicting(team_names) => {
            manager.set_geap_credentials_state(
                GeapCredentialsState::ConflictingAcrossTeams { team_names },
                ctx,
            );
            return;
        }
        GeapPolicy::Mintable(binding) => binding,
    };
    if matches!(
        manager.geap_credentials_state(),
        GeapCredentialsState::Refreshing { .. }
    ) {
        return;
    }
    if !force
        && let GeapCredentialsState::Loaded {
            credentials,
            minted_for: current_binding,
            ..
        } = manager.geap_credentials_state()
        && *current_binding == minted_for
        && !credentials.needs_refresh()
    {
        return;
    }
    let previous = match manager.geap_credentials_state() {
        GeapCredentialsState::Loaded {
            credentials,
            minted_for: current_binding,
            ..
        } if *current_binding == minted_for => Some((credentials.clone(), current_binding.clone())),
        _ => None,
    };
    log::info!(
        "GEAP: minting credentials (audience={}, force={force})",
        minted_for.audience
    );
    manager.install_geap_refresh_waiter(waiter);
    manager.set_geap_credentials_state(GeapCredentialsState::Refreshing { previous }, ctx);

    // Minting requires a Warp-issued OIDC identity token, which this build cannot obtain.
    let result = Err(LoadGeapCredentialsError::MintIdentityToken {
        detail: "Warp-issued identity tokens are not available".to_owned(),
    });
    apply_geap_mint_result(manager, result, minted_for, force, ctx);
}

fn apply_geap_mint_result(
    manager: &mut ApiKeyManager,
    result: Result<GeapCredentials, LoadGeapCredentialsError>,
    minted_for: GeapMintBinding,
    force: bool,
    ctx: &mut ModelContext<ApiKeyManager>,
) {
    let waiters = manager.take_geap_refresh_waiters();
    let outcome = apply_geap_mint_result_inner(manager, result, minted_for, force, ctx);
    for waiter in waiters {
        let _ = waiter.send(outcome);
    }
}

fn apply_geap_mint_result_inner(
    manager: &mut ApiKeyManager,
    result: Result<GeapCredentials, LoadGeapCredentialsError>,
    minted_for: GeapMintBinding,
    force: bool,
    ctx: &mut ModelContext<ApiKeyManager>,
) -> GeapRefreshOutcome {
    let current_binding = match current_geap_policy_for_any_team(ctx) {
        GeapPolicy::Disabled => {
            log::info!("GEAP: gate flipped off mid-mint; discarding the mint result");
            manager.set_geap_credentials_state(GeapCredentialsState::Disabled, ctx);
            return GeapRefreshOutcome::Failed;
        }
        GeapPolicy::Unconfigured => {
            log::info!("GEAP: gate unconfigured mid-mint; discarding the mint result");
            manager.set_geap_credentials_state(GeapCredentialsState::Unconfigured, ctx);
            return GeapRefreshOutcome::Failed;
        }
        GeapPolicy::Conflicting(team_names) => {
            log::info!(
                "GEAP: teams began disagreeing on the project mid-mint; discarding the mint result"
            );
            manager.set_geap_credentials_state(
                GeapCredentialsState::ConflictingAcrossTeams { team_names },
                ctx,
            );
            return GeapRefreshOutcome::Failed;
        }
        GeapPolicy::Mintable(binding) => binding,
    };
    let previous = match manager.geap_credentials_state() {
        GeapCredentialsState::Refreshing {
            previous: Some((credentials, binding)),
        } if *binding == current_binding => Some((credentials.clone(), binding.clone())),
        _ => None,
    };

    // The user/account or federation config changed while the mint was in
    // flight. Discard it and immediately re-mint under the current binding.
    if minted_for != current_binding {
        log::info!("GEAP: binding changed mid-mint; discarding the result and re-minting");
        match previous {
            Some((credentials, minted_for)) => {
                manager.set_geap_credentials_state(
                    GeapCredentialsState::Loaded {
                        credentials,
                        loaded_at: SystemTime::now(),
                        minted_for,
                    },
                    ctx,
                );
                schedule_geap_token_refresh(manager, ctx);
            }
            None => {
                manager.set_geap_credentials_state(GeapCredentialsState::Missing, ctx);
            }
        }
        refresh_geap_credentials(manager, ctx);
        return GeapRefreshOutcome::Failed;
    }

    match result {
        Ok(credentials) => {
            log::info!(
                "GEAP: credentials minted (audience={}, expires_at={:?})",
                minted_for.audience,
                credentials.expires_at()
            );
            manager.set_geap_credentials_state(
                GeapCredentialsState::Loaded {
                    credentials,
                    loaded_at: SystemTime::now(),
                    minted_for,
                },
                ctx,
            );
            // Arm the next one-shot proactive refresh — this is what makes
            // the ~hourly loop self-sustaining.
            schedule_geap_token_refresh(manager, ctx);
            manager.clear_geap_mint_failure();
            GeapRefreshOutcome::Refreshed
        }
        Err(err) => {
            report_error!(anyhow::Error::new(err.clone()).context("GEAP: credential mint failed"));
            match previous {
                // A failed background re-mint keeps the previous token — even
                // near/past expiry (Google remains the authority on validity;
                // sending it can only yield a visible, recoverable 401, never
                // a silent downgrade) — and parks the chain. No reschedule:
                // the next agent request's safety net re-arms it, so a
                // hard-down network cannot cause unbounded STS traffic.
                Some((credentials, minted_for)) if !force => {
                    manager.set_geap_credentials_state(
                        GeapCredentialsState::Loaded {
                            credentials,
                            loaded_at: SystemTime::now(),
                            minted_for,
                        },
                        ctx,
                    );
                }
                // First mint (nothing servable to keep), or a forced refresh
                // where the user explicitly asked and needs visible feedback.
                _ => {
                    manager.set_geap_credentials_state(
                        GeapCredentialsState::Failed { error: err },
                        ctx,
                    );
                }
            }
            // Start the cooldown.
            manager.record_geap_mint_failure();
            GeapRefreshOutcome::Failed
        }
    }
}

/// A one-shot timer that re-mints [`GEAP_REFRESH_LEAD_TIME`] before the
/// loaded token's expiry. The timer is armed once per token — no periodic
/// polling; the process wakes exactly once per token lifetime.
fn schedule_geap_token_refresh(manager: &mut ApiKeyManager, ctx: &mut ModelContext<ApiKeyManager>) {
    let GeapCredentialsState::Loaded { credentials, .. } = manager.geap_credentials_state() else {
        return;
    };
    let Some(expires_at) = credentials.expires_at() else {
        return;
    };
    let delay = geap_refresh_timer_delay(expires_at, SystemTime::now());
    let _ = ctx.spawn(
        async move {
            Timer::after(delay).await;
        },
        |manager, _output, ctx| {
            refresh_geap_credentials(manager, ctx);
        },
    );
}

fn geap_refresh_timer_delay(expires_at: SystemTime, now: SystemTime) -> Duration {
    let fire_at = expires_at
        .checked_sub(GEAP_REFRESH_LEAD_TIME)
        .unwrap_or(now);
    fire_at
        .duration_since(now)
        .unwrap_or(Duration::ZERO)
        .max(GEAP_MIN_TIMER_DELAY)
}

#[cfg(test)]
#[path = "geap_credentials_tests.rs"]
mod tests;
