//! Seam for translating a natural-language query into shell commands from `#` command search.
//!
//! No provider is registered by default, so the search reports that AI must be set up first.

use std::sync::Arc;

use itertools::Itertools;
use warpui::{Entity, SingletonEntity};

use crate::search::mixer::BoxFuture;
use crate::terminal::model::session::Session;
use crate::workflows::workflow::{Argument, Workflow};

/// Describes the shell environment the generated commands will run in.
#[derive(Clone, Debug, Default)]
pub struct NaturalLanguageCommandContext {
    pub os_category: Option<String>,
    pub linux_distribution: Option<String>,
    pub shell_name: String,
    pub shell_version: Option<String>,
}

impl NaturalLanguageCommandContext {
    pub fn for_session(session: &Session) -> Self {
        Self {
            os_category: session.host_info().os_category.clone(),
            linux_distribution: session.host_info().linux_distribution.clone(),
            shell_name: session.shell().shell_type().name().to_owned(),
            shell_version: session.shell().version().clone(),
        }
    }
}

/// A shell command suggested for a natural-language query.
#[derive(Clone, Debug)]
pub struct GeneratedCommand {
    pub command: String,
    /// Shown as the title of the resulting workflow.
    pub description: String,
    pub parameters: Vec<GeneratedCommandParameter>,
}

#[derive(Clone, Debug)]
pub struct GeneratedCommandParameter {
    pub id: String,
    pub description: String,
}

impl From<GeneratedCommand> for Workflow {
    fn from(command: GeneratedCommand) -> Self {
        Workflow::new(command.description, command.command).with_arguments(
            command
                .parameters
                .into_iter()
                .map(|parameter| Argument {
                    name: parameter.id,
                    description: Some(parameter.description),
                    default_value: None,
                    arg_type: Default::default(),
                })
                .collect_vec(),
        )
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub enum GenerateCommandsError {
    /// AI is turned off or no model has been configured.
    NoProvider,
    BadPrompt,
    ProviderError,
}

pub trait NaturalLanguageCommandProvider: 'static + Send + Sync {
    fn generate_commands(
        &self,
        query: String,
        context: Option<NaturalLanguageCommandContext>,
    ) -> BoxFuture<'static, Result<Vec<GeneratedCommand>, GenerateCommandsError>>;
}

/// Holds the provider used by `#` command search, if one has been configured.
#[derive(Default)]
pub struct NaturalLanguageCommandRegistry {
    provider: Option<Arc<dyn NaturalLanguageCommandProvider>>,
}

impl NaturalLanguageCommandRegistry {
    pub fn provider(&self) -> Option<Arc<dyn NaturalLanguageCommandProvider>> {
        self.provider.clone()
    }

    pub fn set_provider(&mut self, provider: Option<Arc<dyn NaturalLanguageCommandProvider>>) {
        self.provider = provider;
    }
}

impl Entity for NaturalLanguageCommandRegistry {
    type Event = ();
}

impl SingletonEntity for NaturalLanguageCommandRegistry {}
