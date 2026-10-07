#[cfg(not(target_family = "wasm"))]
pub mod persistence;

use cloud_objects::cloud_object::{
    GenericCloudObject, GenericServerObject, ObjectType, ServerObjectModel,
};
use cloud_objects::ids::ServerId;
use serde::{Deserialize, Serialize};
pub use workflow_types::{Argument, ArgumentType, Workflow};

/// The model for a `CloudWorkflow`.
#[derive(Clone, Debug, PartialEq)]
pub struct CloudWorkflowModel {
    pub data: Workflow,
}

impl CloudWorkflowModel {
    pub fn new(workflow: Workflow) -> Self {
        Self { data: workflow }
    }
}

impl ServerObjectModel for CloudWorkflowModel {
    fn object_type(&self) -> ObjectType {
        ObjectType::Workflow
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct WorkflowId(ServerId);
cloud_objects::server_id_traits! { WorkflowId, "Workflow" }

/// `CloudWorkflow` is a workflow retrieved from the server.
pub type CloudWorkflow = GenericCloudObject<WorkflowId, CloudWorkflowModel>;
pub type ServerWorkflow = GenericServerObject<WorkflowId, CloudWorkflowModel>;
