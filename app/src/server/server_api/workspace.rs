use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use super::ServerApi;

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait WorkspaceClient: 'static + Send + Sync {}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl WorkspaceClient for ServerApi {}
