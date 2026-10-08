use std::collections::HashMap;

use cloud_objects::cloud_object::{
    GenericCloudObject, GenericServerObject, GenericStringModel, JsonObjectType,
};
use cloud_objects::ids::GenericStringObjectId;
use serde::{Deserialize, Serialize};

use crate::cloud_environment::SourceRepo;
use crate::{JsonModel, JsonSerializer};

/// Runtime configuration snapshot for agent execution.
///
/// This is the merged/resolved config used when spawning or running an agent.
/// It combines settings from config files and CLI args.
/// Unlike `AgentConfig` (the cloud model), field names here use the runtime format
/// (e.g. `model_id` instead of `base_model_id`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AgentConfigSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Map<String, serde_json::Value>>,
    /// Config name for searchability/traceability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
    /// Runner ID (JsonRunner GSO) used to override the environment's compute
    /// config (docker image, instance shape, setup commands).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_prompt: Option<String>,
    /// MCP server configuration map (unwrapped; no `mcpServers` wrapper).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<serde_json::Map<String, serde_json::Value>>,
    /// Profile ID for local agent runs. This configures the terminal session
    /// with the specified execution profile. Only used for local runs, not cloud runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    /// Self-hosted worker ID that should execute this task.
    /// If None or Some("warp"), the task will be dispatched to Warp-hosted (Namespace) workers.
    /// Otherwise, the task will only be assigned to a connected self-hosted worker with matching ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_host: Option<String>,
    /// Skill spec to use as the base prompt for the agent.
    /// Format: "skill_name", "repo:skill_name", or "org/repo:skill_name".
    /// The skill is resolved at runtime in the agent environment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_spec: Option<String>,
    /// Whether computer use is enabled for this agent run.
    /// If None, the default behavior is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer_use_enabled: Option<bool>,
    /// Extra repositories the worker should clone in addition to the environment's repos.
    /// This is server-populated for per-task sources such as a GitHub webhook's origin repo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_source_repos: Option<Vec<SourceRepo>>,
}

impl AgentConfigSnapshot {
    /// Returns true if this config is empty (no options are set).
    pub fn is_empty(&self) -> bool {
        let Self {
            experimental,
            name,
            environment_id,
            runner_id,
            model_id,
            base_prompt,
            mcp_servers,
            profile_id,
            worker_host,
            skill_spec,
            computer_use_enabled,
            additional_source_repos,
        } = self;

        experimental.is_none()
            && name.is_none()
            && environment_id.is_none()
            && runner_id.is_none()
            && model_id.is_none()
            && base_prompt.is_none()
            && mcp_servers.is_none()
            && profile_id.is_none()
            && worker_host.is_none()
            && skill_spec.is_none()
            && computer_use_enabled.is_none()
            && additional_source_repos.is_none()
    }
}

/// A ScheduledAmbientAgent represents configuration for ambient agents that run on a cron schedule.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ScheduledAmbientAgent {
    /// Agent name
    #[serde(default)]
    pub name: String,
    /// Cron schedule expression
    #[serde(default)]
    pub cron_schedule: String,
    /// Whether the scheduled agent is enabled
    #[serde(default)]
    pub enabled: bool,
    /// The prompt to use for the scheduled agent
    #[serde(default)]
    pub prompt: String,
    /// The latest failure to execute this scheduled agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_spawn_error: Option<String>,
    /// Configuration for how the ambient agent should run.
    #[serde(default, skip_serializing_if = "AgentConfigSnapshot::is_empty")]
    pub agent_config: AgentConfigSnapshot,
}

impl ScheduledAmbientAgent {
    pub fn new(name: String, cron_schedule: String, enabled: bool, prompt: String) -> Self {
        Self {
            name,
            cron_schedule,
            enabled,
            prompt,
            last_spawn_error: None,
            agent_config: Default::default(),
        }
    }
}

impl JsonModel for ScheduledAmbientAgent {
    fn json_object_type() -> JsonObjectType {
        JsonObjectType::ScheduledAmbientAgent
    }
}

pub type CloudScheduledAmbientAgent =
    GenericCloudObject<GenericStringObjectId, CloudScheduledAmbientAgentModel>;
pub type CloudScheduledAmbientAgentModel =
    GenericStringModel<ScheduledAmbientAgent, JsonSerializer>;
pub type ServerScheduledAmbientAgent =
    GenericServerObject<GenericStringObjectId, CloudScheduledAmbientAgentModel>;

pub type AgentConfigMap = HashMap<String, serde_json::Value>;

#[cfg(test)]
#[path = "scheduled_ambient_agent_tests.rs"]
mod tests;
