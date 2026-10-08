//! Source-facing helpers that centralize the derivation of the agent-icon shape
//! ([`IconWithStatusVariant`]) from the underlying state models, so a single logical agent run
//! renders with the same glyph on every surface (vertical tabs, pane header).
//!
//! Surfaces feed the resulting variant into [`render_icon_with_status`]. The pure inner
//! functions in this module are exercised directly by the tests in `agent_icon_tests.rs`.
use warpui::AppContext;

use crate::ai::agent::conversation::ConversationStatus;
use crate::terminal::view::TerminalView;
use crate::ui_components::icon_with_status::IconWithStatusVariant;

/// Returns the agent-icon variant for a live [`TerminalView`], or `None` when the terminal is
/// not an agent surface (plain terminal / shell / empty conversation).
pub(crate) fn terminal_view_agent_icon_variant(
    terminal_view: &TerminalView,
    app: &AppContext,
) -> Option<IconWithStatusVariant> {
    let inputs = TerminalIconInputs {
        selected_conversation_status: terminal_view.selected_conversation_status_for_display(app),
        has_selected_conversation: terminal_view
            .selected_conversation_display_title(app)
            .is_some(),
    };
    agent_icon_variant_from_terminal_inputs(&inputs)
}

/// Primitive inputs gathered once from the live [`TerminalView`] / [`AppContext`].
struct TerminalIconInputs {
    /// The conversation status that the terminal view would surface in its status-icon slot.
    selected_conversation_status: Option<ConversationStatus>,
    /// Whether the terminal view currently has a selected conversation.
    has_selected_conversation: bool,
}

/// Maps primitive inputs to an [`IconWithStatusVariant`]: a selected conversation renders the
/// Oz agent variant, and everything else returns `None` so the caller renders a plain-terminal
/// indicator.
fn agent_icon_variant_from_terminal_inputs(
    inputs: &TerminalIconInputs,
) -> Option<IconWithStatusVariant> {
    inputs
        .has_selected_conversation
        .then(|| IconWithStatusVariant::OzAgent {
            status: inputs.selected_conversation_status.clone(),
        })
}

#[cfg(test)]
#[path = "agent_icon_tests.rs"]
mod tests;
