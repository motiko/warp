use std::collections::{HashMap, HashSet};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;
use warp_errors::report_error;
use warpui_core::{Entity, ModelContext, SingletonEntity};
use warpui_extras::secure_storage::{self, AppContextExt};

/// Legacy secure-storage blob that held custom endpoints (with their keys) before definitions
/// moved to `settings.toml`. Only read, to migrate.
const SECURE_STORAGE_KEY: &str = "AiApiKeys";
const CUSTOM_ENDPOINT_KEYS_SECURE_STORAGE_KEY: &str = "AiCustomEndpointKeys";
const GENERATED_ENDPOINT_PREFIX: &str = "endpoint-";
const LEGACY_ENDPOINT_PREFIX: &str = "legacy-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyManagerEvent {
    /// Custom endpoint definitions or keys were updated in-memory.
    KeysUpdated,
}

/// The legacy secure-storage payload; only its custom endpoints are still read.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiKeys {
    pub custom_endpoints: Vec<CustomEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomEndpoint {
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub models: Vec<CustomEndpointModel>,
    pub schema: CustomEndpointSchema,
}

/// The request/response protocol used by a custom inference endpoint.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum CustomEndpointSchema {
    /// OpenAI Chat Completions, retained as the legacy/default protocol.
    #[default]
    OpenaiChatCompletions,
    /// OpenAI Responses.
    OpenaiResponses,
    /// Anthropic Messages.
    AnthropicMessages,
}

impl CustomEndpointSchema {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::OpenaiChatCompletions => "OpenAI Chat Completions",
            Self::OpenaiResponses => "OpenAI Responses",
            Self::AnthropicMessages => "Anthropic Messages",
        }
    }

    pub fn from_display_name(name: &str) -> Option<Self> {
        match name {
            "OpenAI Chat Completions" => Some(Self::OpenaiChatCompletions),
            "OpenAI Responses" => Some(Self::OpenaiResponses),
            "Anthropic Messages" => Some(Self::AnthropicMessages),
            _ => None,
        }
    }
}

/// Stable file-safe identity for a custom endpoint definition.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct CustomEndpointId(String);

impl CustomEndpointId {
    pub fn generated() -> Self {
        Self(format!("{GENERATED_ENDPOINT_PREFIX}{}", Uuid::new_v4()))
    }

    pub fn from_legacy(endpoint: &CustomEndpoint, index: usize) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(index.to_le_bytes());
        hasher.update(endpoint.name.as_bytes());
        hasher.update([0]);
        hasher.update(endpoint.url.as_bytes());
        for model in &endpoint.models {
            hasher.update([0]);
            hasher.update(model.config_key.as_bytes());
        }
        let digest = hex::encode(hasher.finalize());
        Self(format!("{LEGACY_ENDPOINT_PREFIX}{}", &digest[..24]))
    }

    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (!value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
        .then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for CustomEndpointId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).ok_or_else(|| serde::de::Error::custom("invalid custom endpoint key"))
    }
}

impl fmt::Display for CustomEndpointId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Non-secret custom endpoint configuration stored in `settings.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomEndpointDefinition {
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub schema: CustomEndpointSchema,
    pub models: Vec<CustomEndpointModel>,
}

impl CustomEndpointDefinition {
    pub fn from_legacy(endpoint: &CustomEndpoint) -> Self {
        Self {
            name: endpoint.name.clone(),
            base_url: endpoint.url.clone(),
            schema: endpoint.schema,
            models: endpoint.models.clone(),
        }
    }

    pub fn into_endpoint(self, api_key: String) -> CustomEndpoint {
        CustomEndpoint {
            name: self.name,
            url: self.base_url,
            api_key,
            models: self.models,
            schema: self.schema,
        }
    }

    fn is_valid(&self) -> bool {
        !self.name.trim().is_empty()
            && validate_custom_endpoint_url(&self.base_url).is_ok()
            && !self.models.is_empty()
            && self.models.iter().all(|model| {
                !model.name.trim().is_empty()
                    && !model.config_key.trim().is_empty()
                    && model
                        .alias
                        .as_deref()
                        .is_none_or(|alias| !alias.trim().is_empty())
            })
    }
}

/// Complete ordered collection of custom endpoint definitions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct CustomEndpointDefinitions(IndexMap<CustomEndpointId, CustomEndpointDefinition>);

impl CustomEndpointDefinitions {
    pub fn definitions(
        &self,
    ) -> impl Iterator<Item = (&CustomEndpointId, &CustomEndpointDefinition)> {
        self.0.iter()
    }

    pub fn get(&self, id: &CustomEndpointId) -> Option<&CustomEndpointDefinition> {
        self.0.get(id)
    }

    pub fn id_at(&self, index: usize) -> Option<&CustomEndpointId> {
        self.0.get_index(index).map(|(id, _)| id)
    }

    pub fn insert(
        &mut self,
        id: CustomEndpointId,
        definition: CustomEndpointDefinition,
    ) -> anyhow::Result<Option<CustomEndpointDefinition>> {
        let mut definitions = self.0.clone();
        let previous = definitions.insert(id, definition);
        *self = Self::validated(definitions)
            .ok_or_else(|| anyhow::anyhow!("invalid custom endpoint definition"))?;
        Ok(previous)
    }

    pub fn remove(&mut self, id: &CustomEndpointId) -> Option<CustomEndpointDefinition> {
        self.0.shift_remove(id)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn from_legacy(
        endpoints: &[CustomEndpoint],
    ) -> anyhow::Result<(Self, HashMap<CustomEndpointId, String>)> {
        let mut definitions = IndexMap::new();
        let mut keys = HashMap::new();
        for (index, endpoint) in endpoints.iter().enumerate() {
            let id = CustomEndpointId::from_legacy(endpoint, index);
            definitions.insert(id.clone(), CustomEndpointDefinition::from_legacy(endpoint));
            if !endpoint.api_key.trim().is_empty() {
                keys.insert(id, endpoint.api_key.clone());
            }
        }
        let definitions = Self::validated(definitions)
            .ok_or_else(|| anyhow::anyhow!("legacy custom endpoints are invalid"))?;
        Ok((definitions, keys))
    }

    fn validated(
        definitions: IndexMap<CustomEndpointId, CustomEndpointDefinition>,
    ) -> Option<Self> {
        let mut config_keys = HashSet::new();
        if definitions.values().all(CustomEndpointDefinition::is_valid)
            && definitions
                .values()
                .flat_map(|definition| &definition.models)
                .all(|model| config_keys.insert(model.config_key.clone()))
        {
            Some(Self(definitions))
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for CustomEndpointDefinitions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let definitions =
            IndexMap::<CustomEndpointId, CustomEndpointDefinition>::deserialize(deserializer)?;
        Self::validated(definitions)
            .ok_or_else(|| serde::de::Error::custom("invalid custom endpoint definitions"))
    }
}

impl settings_value::SettingsValue for CustomEndpointDefinitions {
    fn from_file_value(value: &serde_json::Value) -> Option<Self> {
        let definitions = serde_json::from_value::<
            IndexMap<CustomEndpointId, CustomEndpointDefinition>,
        >(value.clone())
        .ok()?;
        Self::validated(definitions)
    }

    fn file_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema
    where
        Self: schemars::JsonSchema,
    {
        generator.subschema_for::<HashMap<String, CustomEndpointDefinition>>()
    }
}

impl schemars::JsonSchema for CustomEndpointDefinitions {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("CustomEndpointDefinitions")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        generator.subschema_for::<HashMap<String, CustomEndpointDefinition>>()
    }
}

pub fn validate_custom_endpoint_url(value: &str) -> Result<(), &'static str> {
    let parsed = Url::parse(value).map_err(|_| "Invalid URL")?;
    if parsed.scheme() != "https" {
        return Err("URL must use HTTPS");
    }
    let Some(host) = parsed.host_str().filter(|host| !host.is_empty()) else {
        return Err("URL must include a host");
    };
    if is_restricted_host(host) {
        return Err("URL must not use a local or private host");
    }
    Ok(())
}

fn is_restricted_host(host: &str) -> bool {
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok_and(is_restricted_ip)
}

fn is_restricted_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_restricted_ipv4(ip),
        IpAddr::V6(ip) => is_restricted_ipv6(ip),
    }
}

fn is_restricted_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_unspecified() || ip.is_private() || ip.is_link_local()
}

fn is_restricted_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.segments()[0] & 0xfe00 == 0xfc00
        || ip.segments()[0] & 0xffc0 == 0xfe80
        || ip.to_ipv4_mapped().is_some_and(is_restricted_ipv4)
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct CustomEndpointModel {
    pub name: String,
    pub alias: Option<String>,
    /// Stable identifier used as `ModelConfig.{base,coding,cli_agent,computer_use_agent}` and
    /// as the `CustomModelProviders.providers[*].models[*].config_key` on the request wire.
    /// Generated as a UUIDv4 at model creation.
    pub config_key: String,
}

impl CustomEndpointModel {
    /// Picker label: prefer the user-provided alias; fall back to the raw model name
    /// so a row is never blank.
    pub fn display_label(&self) -> &str {
        match self.alias.as_deref() {
            Some(alias) if !alias.trim().is_empty() => alias,
            _ => &self.name,
        }
    }
}

struct CustomEndpointState {
    definitions: Option<CustomEndpointDefinitions>,
    settings_valid: bool,
    keys: HashMap<CustomEndpointId, String>,
    resolved: Vec<CustomEndpoint>,
}

/// Holds the user's custom inference endpoints and their keys, which live only in secure storage.
pub struct ApiKeyManager {
    keys: ApiKeys,
    custom_endpoints: CustomEndpointState,
}

#[derive(Clone)]
pub struct CustomEndpointParams {
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub models: Vec<(String, Option<String>, Option<String>)>,
    pub schema: CustomEndpointSchema,
}

impl ApiKeyManager {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let keys = Self::load_keys_from_secure_storage(ctx);
        let custom_endpoint_keys = Self::load_custom_endpoint_keys_from_secure_storage(ctx);
        let resolved_custom_endpoints = keys.custom_endpoints.clone();
        Self {
            keys,
            custom_endpoints: CustomEndpointState {
                definitions: None,
                settings_valid: true,
                keys: custom_endpoint_keys,
                resolved: resolved_custom_endpoints,
            },
        }
    }

    pub fn keys(&self) -> &ApiKeys {
        &self.keys
    }

    pub fn custom_endpoints(&self) -> &[CustomEndpoint] {
        &self.custom_endpoints.resolved
    }

    pub fn custom_endpoint_definitions(&self) -> Option<&CustomEndpointDefinitions> {
        self.custom_endpoints.definitions.as_ref()
    }

    pub fn custom_endpoint_settings_valid(&self) -> bool {
        self.custom_endpoints.settings_valid
    }

    pub fn custom_endpoint_key(&self, id: &CustomEndpointId) -> Option<&str> {
        self.custom_endpoints
            .keys
            .get(id)
            .map(String::as_str)
            .filter(|key| !key.trim().is_empty())
    }

    pub fn set_custom_endpoint_definitions(
        &mut self,
        definitions: CustomEndpointDefinitions,
        ctx: &mut ModelContext<Self>,
    ) {
        if self.custom_endpoints.settings_valid
            && self.custom_endpoints.definitions.as_ref() == Some(&definitions)
        {
            return;
        }
        self.custom_endpoints.settings_valid = true;
        self.custom_endpoints.definitions = Some(definitions);
        self.rebuild_custom_endpoints();
        ctx.emit(ApiKeyManagerEvent::KeysUpdated);
    }

    pub fn invalidate_custom_endpoint_definitions(&mut self, ctx: &mut ModelContext<Self>) {
        if !self.custom_endpoints.settings_valid {
            return;
        }
        self.custom_endpoints.settings_valid = false;
        self.custom_endpoints.definitions = Some(CustomEndpointDefinitions::default());
        self.custom_endpoints.resolved.clear();
        ctx.emit(ApiKeyManagerEvent::KeysUpdated);
    }

    pub fn persist_custom_endpoint_key(
        &mut self,
        id: CustomEndpointId,
        key: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let mut keys = self.custom_endpoints.keys.clone();
        match key.filter(|key| !key.trim().is_empty()) {
            Some(key) => {
                keys.insert(id, key);
            }
            None => {
                keys.remove(&id);
            }
        }
        self.persist_custom_endpoint_keys(keys, ctx)
    }

    pub fn persist_custom_endpoint_keys(
        &mut self,
        keys: HashMap<CustomEndpointId, String>,
        ctx: &mut ModelContext<Self>,
    ) -> anyhow::Result<()> {
        let json = serde_json::to_string(&keys).map_err(|error| {
            anyhow::Error::new(error).context("Failed to serialize custom endpoint keys")
        })?;
        ctx.secure_storage()
            .write_value(CUSTOM_ENDPOINT_KEYS_SECURE_STORAGE_KEY, &json)
            .map_err(|error| {
                anyhow::Error::new(error)
                    .context("Failed to write custom endpoint keys to secure storage")
            })?;
        if self.custom_endpoints.keys != keys {
            self.custom_endpoints.keys = keys;
            self.rebuild_custom_endpoints();
            ctx.emit(ApiKeyManagerEvent::KeysUpdated);
        }
        Ok(())
    }

    fn load_keys_from_secure_storage(ctx: &mut ModelContext<Self>) -> ApiKeys {
        let key_json = match ctx.secure_storage().read_value(SECURE_STORAGE_KEY) {
            Ok(json) => json,
            Err(e) => {
                if !matches!(e, secure_storage::Error::NotFound) {
                    report_error!(
                        anyhow::Error::new(e)
                            .context("Failed to read API keys from secure storage")
                    );
                }
                return ApiKeys::default();
            }
        };

        match serde_json::from_str(&key_json) {
            Ok(keys) => keys,
            Err(e) => {
                report_error!(anyhow::Error::new(e).context("Failed to deserialize API keys"));
                ApiKeys::default()
            }
        }
    }

    fn load_custom_endpoint_keys_from_secure_storage(
        ctx: &mut ModelContext<Self>,
    ) -> HashMap<CustomEndpointId, String> {
        let json = match ctx
            .secure_storage()
            .read_value(CUSTOM_ENDPOINT_KEYS_SECURE_STORAGE_KEY)
        {
            Ok(json) => json,
            Err(error) => {
                if !matches!(error, secure_storage::Error::NotFound) {
                    report_error!(
                        anyhow::Error::new(error)
                            .context("Failed to read custom endpoint keys from secure storage")
                    );
                }
                return HashMap::new();
            }
        };
        match serde_json::from_str(&json) {
            Ok(keys) => keys,
            Err(error) => {
                report_error!(
                    anyhow::Error::new(error).context("Failed to deserialize custom endpoint keys")
                );
                HashMap::new()
            }
        }
    }

    fn rebuild_custom_endpoints(&mut self) {
        let Some(definitions) = &self.custom_endpoints.definitions else {
            self.custom_endpoints.resolved = self.keys.custom_endpoints.clone();
            return;
        };
        self.custom_endpoints.resolved = definitions
            .definitions()
            .map(|(id, definition)| {
                definition.clone().into_endpoint(
                    self.custom_endpoints
                        .keys
                        .get(id)
                        .cloned()
                        .unwrap_or_default(),
                )
            })
            .collect();
    }
}

impl Entity for ApiKeyManager {
    type Event = ApiKeyManagerEvent;
}

impl SingletonEntity for ApiKeyManager {}

#[cfg(test)]
#[path = "api_keys_tests.rs"]
mod tests;
