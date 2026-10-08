use std::collections::HashMap;
use std::sync::Arc;

use warp_core::settings::ChangeEventReason;
use warpui::{
    Entity, ModelContext, SingletonEntity, Tracked,
    WindowId,
};

use super::team::{DiscoverableTeam, DiscoveryOptions};
use super::workspace::{
    AdminEnablementSetting, EnterpriseSecretRegex, UgcCollectionEnablementSetting, Workspace,
    WorkspaceUid,
};
use crate::server::experiments::ServerExperiment;
use crate::server::ids::ServerId;
use crate::server::server_api::team::TeamClient;
use crate::server::server_api::workspace::WorkspaceClient;
#[cfg(test)]
use crate::server::server_api::{team::MockTeamClient, workspace::MockWorkspaceClient};
use crate::settings::PrivacySettings;
use crate::workspaces::workspace::PurchaseAddOnCreditsPolicy;

#[derive(Debug)]
pub enum UserWorkspacesEvent {
    AddDomainRestrictionsSuccess,
    AddDomainRestrictionsRejected(anyhow::Error),
    DeleteDomainRestrictionSuccess,
    DeleteDomainRestrictionRejected(anyhow::Error),
    EmailInviteSent,
    EmailInviteRejected(anyhow::Error),
    ToggleInviteLinksSuccess,
    ToggleInviteLinksRejected(anyhow::Error),
    ResetInviteLinks,
    ResetInviteLinksRejected(anyhow::Error),
    DeleteTeamInvite,
    DeleteTeamInviteRejected(anyhow::Error),
    GenerateUpgradeLink(String),
    GenerateUpgradeLinkRejected(anyhow::Error),
    GenerateStripeBillingPortalLink(String),
    GenerateStripeBillingPortalLinkRejected(anyhow::Error),
    ToggleTeamDiscoverabilitySuccess,
    ToggleTeamDiscoverabilityRejected(anyhow::Error),
    JoinTeamWithTeamDiscoverySuccess,
    JoinTeamWithTeamDiscoveryRejected(anyhow::Error),
    JoinTeamInWorkspaceSuccess {
        team_uid: ServerId,
    },
    JoinTeamInWorkspaceRejected(anyhow::Error),
    JoinWorkspaceFromDiscoverySuccess,
    JoinWorkspaceFromDiscoveryRejected(anyhow::Error),
    FetchDiscoverableTeamsSuccess(Vec<DiscoverableTeam>),
    FetchDiscoveryOptionsSuccess(DiscoveryOptions),
    FetchDiscoveryOptionsRejected(anyhow::Error),
    TransferTeamOwnershipSuccess,
    TransferTeamOwnershipRejected(anyhow::Error),
    SetTeamMemberRoleSuccess,
    SetTeamMemberRoleRejected(anyhow::Error),
    RemoveUserFromTeamSuccess,
    RemoveUserFromTeamRejected(anyhow::Error),
    RemoveUserFromWorkspaceSuccess,
    RemoveUserFromWorkspaceRejected(anyhow::Error),
    UpdateWorkspaceSettingsSuccess,
    UpdateWorkspaceSettingsRejected(anyhow::Error),
    AiOveragesUpdated,
    PurchaseAddonCreditsSuccess,
    /// The purchase requires the user to complete checkout in the browser
    /// (no saved payment method). Credits arrive via webhook + polling after
    /// checkout completes.
    PurchaseAddonCreditsCheckoutRequired {
        checkout_url: String,
    },
    PurchaseAddonCreditsRejected(anyhow::Error),
    /// Fired whenever the set of teams the user is on changes.
    TeamsChanged,
    /// Fired when the selected workspace actually changes to a different one.
    CurrentWorkspaceChanged,
    /// Fired when a single window's team assignment changes. Windows are independent, so
    /// subscribers that hold per-window state must only react to their own window.
    WindowTeamChanged {
        window_id: WindowId,
    },
    /// Fired when a service agreement's sunsetted_to_build_ts field is updated.
    SunsettedToBuildDataUpdated,
}

/// UserWorkspaces is a singleton model that holds workspace metadata (name, members, etc).
/// It should be used for getting information about the workspaces, teams, current teams,
/// and all other things related to operating on workspace and team data.
/// TODO: move other server_api calls to update_manager to correctly update sqlite.
pub struct UserWorkspaces {
    current_workspace_uid: Tracked<Option<WorkspaceUid>>,
    workspaces: Tracked<Vec<Workspace>>,
    window_team_uids: HashMap<WindowId, Option<ServerId>>,
    joinable_teams: Vec<DiscoverableTeam>,
    /// The user-level add-on credits purchase policy from the latest
    /// workspaces-metadata response. Teamless (fresh free) users have no
    /// team and their only workspace is the server's placeholder, which is
    /// filtered out of `workspaces` — this is the only place their purchase
    /// policy survives.
    user_purchase_policy: Option<PurchaseAddOnCreditsPolicy>,
    team_client: Arc<dyn TeamClient>,
    workspace_client: Arc<dyn WorkspaceClient>,
}

/// Represents the workspaces a user potentially has access to.
#[derive(Clone)]
pub struct WorkspacesMetadataResponse {
    /// The list of workspaces the user is currently on.
    pub workspaces: Vec<Workspace>,
    /// The list of discoverable teams that the user can join.
    pub joinable_teams: Vec<DiscoverableTeam>,
    /// The list of experiments applicable to the user.
    pub experiments: Option<Vec<ServerExperiment>>,
    /// The user-level add-on credits purchase policy; the teamless-purchase
    /// fallback (see [`UserWorkspaces::purchase_policy`]).
    pub user_purchase_policy: Option<PurchaseAddOnCreditsPolicy>,
}

// A representation of all data we fetch at a single time via our 10 minute poll.
// Prefer adding to this struct if you need relatively fresh data vs making
// independent queries.
pub struct WorkspacesMetadataWithPricing {
    pub metadata: WorkspacesMetadataResponse,
    pub pricing_info: Option<warp_graphql::billing::PricingInfo>,
}

impl UserWorkspaces {
    #[cfg(test)]
    pub fn mock(
        team_client: Arc<dyn TeamClient>,
        workspace_client: Arc<dyn WorkspaceClient>,
        cached_workspaces: Vec<Workspace>,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        // In tests, avoid subscribing to [`ServerExperiments`] because it
        // requires us to register that singleton along with _its_ dependencies
        // for all tests that use [`UserWorkspaces`] (a lot of them do).
        Self {
            current_workspace_uid: cached_workspaces.first().map(|w| w.uid).into(),
            workspaces: cached_workspaces.into(),
            window_team_uids: Default::default(),
            joinable_teams: Default::default(),
            user_purchase_policy: None,
            team_client,
            workspace_client,
        }
    }

    #[cfg(test)]
    pub fn default_mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::mock(
            Arc::new(MockTeamClient::new()),
            Arc::new(MockWorkspaceClient::new()),
            vec![],
            ctx,
        )
    }

    pub fn new(
        team_client: Arc<dyn TeamClient>,
        workspace_client: Arc<dyn WorkspaceClient>,
        cached_workspaces: Vec<Workspace>,
        current_workspace_uid: Option<WorkspaceUid>,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            current_workspace_uid: current_workspace_uid.into(),
            workspaces: cached_workspaces.into(),
            window_team_uids: Default::default(),
            joinable_teams: Default::default(),
            user_purchase_policy: None,
            team_client,
            workspace_client,
        }
    }

    pub fn register_window(
        &mut self,
        window_id: WindowId,
        team_uid: Option<ServerId>,
        ctx: &mut ModelContext<Self>,
    ) {
        let previous_team_uid = self.team_uid_for_window(window_id);
        self.window_team_uids.entry(window_id).or_insert(team_uid);
        if self.team_uid_for_window(window_id) != previous_team_uid {
            ctx.emit(UserWorkspacesEvent::WindowTeamChanged { window_id });
        }
        ctx.notify();
    }

    pub fn inherited_or_default_team_uid(
        &self,
        source_window_id: Option<WindowId>,
    ) -> Option<ServerId> {
        source_window_id
            .and_then(|source_window_id| self.team_uid_for_window(source_window_id))
            .or_else(|| {
                self.current_workspace()
                    .and_then(|workspace| workspace.teams.first())
                    .map(|team| team.uid)
            })
    }

    pub fn team_uid_for_window(&self, window_id: WindowId) -> Option<ServerId> {
        self.window_team_uids.get(&window_id).copied().flatten()
    }

    /// Returns the windows whose team assignment changed.
    #[must_use]
    fn reconcile_window_team_assignments(&mut self) -> Vec<WindowId> {
        let team_uids = self
            .current_workspace()
            .map(|workspace| {
                workspace
                    .teams
                    .iter()
                    .map(|team| team.uid)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let fallback_team_uid = team_uids.first().copied();

        let mut reassigned_windows = Vec::new();
        for (window_id, window_team_uid) in self.window_team_uids.iter_mut() {
            if window_team_uid.is_none_or(|team_uid| !team_uids.contains(&team_uid))
                && *window_team_uid != fallback_team_uid
            {
                *window_team_uid = fallback_team_uid;
                reassigned_windows.push(*window_id);
            }
        }
        reassigned_windows
    }

    fn emit_window_team_changed(windows: Vec<WindowId>, ctx: &mut ModelContext<Self>) {
        for window_id in windows {
            ctx.emit(UserWorkspacesEvent::WindowTeamChanged { window_id });
        }
    }

    pub fn workspace_from_uid(&self, workspace_uid: WorkspaceUid) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.uid == workspace_uid)
    }

    /// Note that the workspace is populated with dummy data until the initial fetch
    /// completes (only workspace name/ID and workspace team's name/ID are cached in
    /// sqlite locally).
    /// Consider whether you need to wait for the results of the fetch before checking the
    /// values of other fields.
    pub fn current_workspace(&self) -> Option<&Workspace> {
        self.current_workspace_uid
            .and_then(|workspace_uid| self.workspace_from_uid(workspace_uid))
    }

    /// Updates the user-level add-on credits purchase policy captured from a
    /// workspaces-metadata response. Must be called on every path that
    /// applies such a response so the teamless fallback can't go stale.
    pub fn set_user_purchase_policy(&mut self, policy: Option<PurchaseAddOnCreditsPolicy>) {
        self.user_purchase_policy = policy;
    }

    pub fn set_current_workspace_uid(
        &mut self,
        workspace_uid: WorkspaceUid,
        ctx: &mut ModelContext<Self>,
    ) {
        let changed = *self.current_workspace_uid != Some(workspace_uid);
        *self.current_workspace_uid = Some(workspace_uid);
        let reassigned_windows = self.reconcile_window_team_assignments();
        self.notify_and_emit_teams_changed(ctx);
        Self::emit_window_team_changed(reassigned_windows, ctx);
        if changed {
            ctx.emit(UserWorkspacesEvent::CurrentWorkspaceChanged);
        }
    }

    pub fn update_workspaces(&mut self, workspaces: Vec<Workspace>, ctx: &mut ModelContext<Self>) {
        // Check if sunsetted_to_build_ts changed for any workspace
        let sunsetted_to_build_changed = self.has_sunsetted_to_build_data_changed(&workspaces);

        *self.workspaces = workspaces;
        let reassigned_windows = self.reconcile_window_team_assignments();
        self.notify_and_emit_teams_changed(ctx);
        Self::emit_window_team_changed(reassigned_windows, ctx);

        if sunsetted_to_build_changed {
            ctx.emit(UserWorkspacesEvent::SunsettedToBuildDataUpdated);
        }
    }

    /// Checks if any workspace's service agreement sunsetted_to_build_ts field has changed.
    fn has_sunsetted_to_build_data_changed(&self, new_workspaces: &[Workspace]) -> bool {
        for new_workspace in new_workspaces {
            // Find the corresponding old workspace
            let old_workspace = self.workspaces.iter().find(|w| w.uid == new_workspace.uid);

            if let Some(old_workspace) = old_workspace {
                // Check if any team's service agreement sunsetted_to_build_ts changed
                for new_team in &new_workspace.teams {
                    let old_team = old_workspace.teams.iter().find(|t| t.uid == new_team.uid);

                    if let Some(old_team) = old_team {
                        let old_sunsetted = old_team
                            .billing_metadata
                            .service_agreements
                            .first()
                            .and_then(|sa| sa.sunsetted_to_build_ts);

                        let new_sunsetted = new_team
                            .billing_metadata
                            .service_agreements
                            .first()
                            .and_then(|sa| sa.sunsetted_to_build_ts);

                        // Detect if it changed from None to Some or changed value
                        if old_sunsetted != new_sunsetted {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn notify_and_emit_teams_changed(&self, ctx: &mut ModelContext<Self>) {
        // PrivacySettings can't observe UserWorkspaces for updates, as it's initialized too early in
        // the app initialization flow. So, we update it manually whenever teams data changes.
        PrivacySettings::handle(ctx).update(ctx, |settings, ctx| {
            settings.set_is_telemetry_force_enabled(self.is_telemetry_force_enabled());
            settings.set_enterprise_secret_redaction_settings(
                self.is_enterprise_secret_redaction_enabled(),
                self.get_enterprise_secret_redaction_regex_list(),
                ChangeEventReason::CloudSync,
                ctx,
            );
        });

        ctx.emit(UserWorkspacesEvent::TeamsChanged);
        ctx.notify();
    }

    pub fn update_joinable_teams(
        &mut self,
        joinable_teams: Vec<DiscoverableTeam>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.joinable_teams.clone_from(&joinable_teams);
        ctx.emit(UserWorkspacesEvent::FetchDiscoverableTeamsSuccess(
            joinable_teams,
        ));
        ctx.notify();
    }

    // TODO follow up with moving other modifying calls out of UserWorkspaces to TeamUpdateManager

    pub fn is_telemetry_force_enabled(&self) -> bool {
        self.current_workspace()
            .map(|workspace| workspace.settings.telemetry_settings.force_enabled)
            .unwrap_or(false)
    }

    pub fn is_enterprise_secret_redaction_enabled(&self) -> bool {
        self.current_workspace()
            .map(|workspace| workspace.settings.secret_redaction_settings.enabled)
            .unwrap_or(false)
    }

    pub fn get_enterprise_secret_redaction_regex_list(&self) -> Vec<EnterpriseSecretRegex> {
        self.current_workspace()
            .map(|workspace| workspace.settings.secret_redaction_settings.regexes.clone())
            .unwrap_or_default()
    }

    pub fn get_ugc_collection_enablement_setting(&self) -> UgcCollectionEnablementSetting {
        self.current_workspace()
            .map(|workspace| workspace.settings.ugc_collection_settings.setting.clone())
            .unwrap_or_default()
    }

    pub fn get_cloud_conversation_storage_enablement_setting(&self) -> AdminEnablementSetting {
        self.current_workspace()
            .map(|workspace| {
                workspace
                    .settings
                    .cloud_conversation_storage_settings
                    .setting
                    .clone()
            })
            .unwrap_or_default()
    }

}

impl Entity for UserWorkspaces {
    type Event = UserWorkspacesEvent;
}

/// Mark UserWorkspaces as global application state.
impl SingletonEntity for UserWorkspaces {}
