use super::{Availability, commands};

#[test]
fn argument_hint_uses_shared_command_prefix_and_text() {
    let hint = commands::RENAME_TAB
        .argument_hint()
        .expect("rename-tab has an argument hint");

    assert_eq!(hint.input_prefix, "/rename-tab ");
    assert_eq!(hint.text, "<tab name>");
    assert_eq!(commands::OPEN_CODE_REVIEW.argument_hint(), None);
}

#[test]
fn always_available_in_any_context() {
    assert!(Availability::ALWAYS.contains(Availability::ALWAYS));
    assert!(Availability::LOCAL.contains(Availability::ALWAYS));
    assert!((Availability::LOCAL | Availability::REPOSITORY).contains(Availability::ALWAYS));
}

#[test]
fn repository_requirement_needs_repository() {
    let command = Availability::REPOSITORY;
    assert!((Availability::LOCAL | Availability::REPOSITORY).contains(command));
    assert!(!Availability::LOCAL.contains(command));
}

#[test]
fn local_requirement_needs_local_session() {
    let command = Availability::LOCAL;
    assert!((Availability::LOCAL | Availability::REPOSITORY).contains(command));
    assert!(!Availability::REPOSITORY.contains(command));
}

#[test]
fn combined_requirements_need_every_flag() {
    let command = Availability::LOCAL | Availability::REPOSITORY;
    assert!((Availability::LOCAL | Availability::REPOSITORY).contains(command));
    assert!(!Availability::LOCAL.contains(command));
    assert!(!Availability::REPOSITORY.contains(command));
}
