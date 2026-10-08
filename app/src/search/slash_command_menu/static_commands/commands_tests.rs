use std::collections::HashSet;

use super::*;

#[test]
fn command_names_and_kinds_are_unique() {
    let mut names = HashSet::new();
    let mut kinds = HashSet::new();
    for command in all_commands() {
        assert!(
            names.insert(command.name),
            "duplicate slash command name: {}",
            command.name
        );
        assert!(
            kinds.insert(command.kind),
            "duplicate slash command kind: {:?}",
            command.kind
        );
    }
}

#[test]
fn command_registry_keeps_icon_metadata() {
    assert_eq!(OPEN_CODE_REVIEW.kind, SlashCommandKind::OpenCodeReview);
    assert_eq!(OPEN_CODE_REVIEW.icon_path, "bundled/svg/diff.svg");

    let registry = Registry::new();
    assert_eq!(
        registry
            .get_command_with_name(OPEN_CODE_REVIEW.name)
            .map(|command| command.icon_path),
        Some("bundled/svg/diff.svg")
    );
}

#[test]
fn version_command_is_not_registered() {
    assert!(
        all_commands()
            .iter()
            .all(|command| command.name != "/version")
    );
}

#[test]
fn rename_tab_command_requires_argument() {
    let command = COMMAND_REGISTRY
        .get_command_with_name(RENAME_TAB.name)
        .expect("expected /rename-tab to be registered");
    let argument = command
        .argument
        .as_ref()
        .expect("expected /rename-tab to require an argument");

    assert!(!argument.is_optional);
    assert!(!argument.should_execute_on_selection);
    assert_eq!(argument.hint_text, Some("<tab name>"));
}

#[test]
fn set_tab_color_command_requires_argument() {
    let command = COMMAND_REGISTRY
        .get_command_with_name(SET_TAB_COLOR.name)
        .expect("expected /set-tab-color to be registered");
    let argument = command
        .argument
        .as_ref()
        .expect("expected /set-tab-color to require an argument");

    assert!(!argument.is_optional);
    assert!(!argument.should_execute_on_selection);

    let hint = argument
        .hint_text
        .expect("/set-tab-color hint text is set dynamically");
    for color in color_dot::TAB_COLOR_OPTIONS {
        let lower = color.to_string().to_ascii_lowercase();
        assert!(hint.contains(&lower), "hint should mention `{lower}`");
    }
    assert!(hint.contains("none"), "hint should mention `none`");
}
