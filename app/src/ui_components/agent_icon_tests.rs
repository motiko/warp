use super::{TerminalIconInputs, agent_icon_variant_from_terminal_inputs};
use crate::ai::agent::conversation::ConversationStatus;
use crate::ui_components::icon_with_status::IconWithStatusVariant;

/// Returns the Oz agent status, or `None` when the variant is not an Oz agent icon.
/// [`IconWithStatusVariant`] can't derive `PartialEq` because `NeutralElement` carries a
/// `Box<dyn Element>`.
fn oz_status(variant: Option<IconWithStatusVariant>) -> Option<Option<ConversationStatus>> {
    match variant? {
        IconWithStatusVariant::OzAgent { status } => Some(status),
        IconWithStatusVariant::Neutral { .. } | IconWithStatusVariant::NeutralElement { .. } => {
            None
        }
    }
}

#[test]
fn plain_terminal_has_no_agent_icon() {
    let inputs = TerminalIconInputs {
        selected_conversation_status: None,
        has_selected_conversation: false,
    };
    assert_eq!(
        oz_status(agent_icon_variant_from_terminal_inputs(&inputs)),
        None
    );
}

#[test]
fn selected_conversation_renders_oz_agent_with_status() {
    let inputs = TerminalIconInputs {
        selected_conversation_status: Some(ConversationStatus::InProgress),
        has_selected_conversation: true,
    };
    assert_eq!(
        oz_status(agent_icon_variant_from_terminal_inputs(&inputs)),
        Some(Some(ConversationStatus::InProgress))
    );
}
