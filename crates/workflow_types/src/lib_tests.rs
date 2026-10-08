use super::{Argument, ArgumentType, Workflow};

fn assert_workflow_roundtrips(workflow: &Workflow) {
    let serialized = serde_json::to_string(workflow).expect("Serialized workflow.");
    let deserialized =
        serde_json::from_str::<Workflow>(&serialized).expect("Deserialized workflow.");
    assert_eq!(&deserialized, workflow);
}

#[test]
fn drive_enum_arguments_deserialize_as_text() {
    let serialized = r#"{"name":"name","command":"command","tags":[],"description":null,"arguments":[{"name":"text","arg_type":"Text","description":null,"default_value":"default"},{"name":"server id enum","arg_type":"Enum","enum_id":"test_uid00000000000123","description":"description","default_value":null}],"source_url":null,"author":null,"author_url":null,"shells":[],"environment_variables":null}"#;

    let deserialized: Workflow = serde_json::from_str(serialized).expect("failed to deserialize");

    let arg_types: Vec<_> = deserialized
        .arguments()
        .iter()
        .map(|arg| arg.arg_type().clone())
        .collect();
    assert_eq!(arg_types, vec![ArgumentType::Text, ArgumentType::Text]);
}

#[test]
fn test_agent_mode_workflow_serialization() {
    let workflow = Workflow::AgentMode {
        name: "name".to_string(),
        query: "query {{text}}".to_string(),
        arguments: vec![Argument {
            name: "text".to_string(),
            arg_type: ArgumentType::Text,
            description: None,
            default_value: Some("default".to_string()),
        }],
        description: None,
    };

    let serialized = serde_json::to_string(&workflow).expect("failed to serialize");
    let correct_serialized = r#"{"type":"agent_mode","name":"name","query":"query {{text}}","arguments":[{"name":"text","arg_type":"Text","description":null,"default_value":"default"}]}"#;

    assert_eq!(
        serialized, correct_serialized,
        "Workflow should serialize correctly"
    );

    let deserialized: Workflow =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialized");

    assert_eq!(deserialized, workflow);
}

#[test]
fn test_workflow_roundtrips() {
    let sample_workflow = Workflow::new("Test name", "Command name");
    assert_workflow_roundtrips(&sample_workflow);

    let arguments = vec![Argument {
        name: "Argument".to_string(),
        description: Some("no".to_string()),
        default_value: None,
        arg_type: Default::default(),
    }];
    let arguments_workflow = sample_workflow.clone().with_arguments(arguments);
    assert_workflow_roundtrips(&arguments_workflow);

    let description_workflow = sample_workflow.with_description("cool description".to_string());
    assert_workflow_roundtrips(&description_workflow);

    let workflow_with_additional_fields = Workflow::Command {
        name: "Test".to_string(),
        command: "Command".to_string(),
        tags: vec![],
        description: None,
        arguments: vec![],
        source_url: Some("url".to_string()),
        author: Some("author_name".to_string()),
        author_url: None,
        shells: vec![],
    };
    assert_workflow_roundtrips(&workflow_with_additional_fields);
}
