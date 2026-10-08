use warpui::{App, EntityId, WindowId};

use super::*;

fn setup_model(app: &mut App) -> ModelHandle<ActiveAgentViewsModel> {
    app.add_singleton_model(|_| ActiveAgentViewsModel::new())
}

#[test]
fn conversation_switch_updates_last_focused_terminal_state() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window = WindowId::new();
        let terminal = EntityId::new();
        let conversation_1 = AIConversationId::new();
        let conversation_2 = AIConversationId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window, Some(terminal), ctx);
            model.update_focused_conversation_for_terminal(terminal, Some(conversation_1));
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_last_focused_terminal_id(), Some(terminal));
            assert_eq!(model.get_last_focused_conversation(), Some(conversation_1));
        });

        model.update(&mut app, |model, _| {
            model.update_focused_conversation_for_terminal(terminal, Some(conversation_2));
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_last_focused_terminal_id(), Some(terminal));
            assert_eq!(model.get_last_focused_conversation(), Some(conversation_2));
        });
    });
}

#[test]
fn per_window_focused_state_is_independent() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window_a = WindowId::new();
        let window_b = WindowId::new();
        let terminal_a = EntityId::new();
        let terminal_b = EntityId::new();
        let conversation_a = AIConversationId::new();
        let conversation_b = AIConversationId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, Some(terminal_a), ctx);
            model.update_focused_conversation_for_terminal(terminal_a, Some(conversation_a));
            model.handle_pane_focus_change(window_b, Some(terminal_b), ctx);
            model.update_focused_conversation_for_terminal(terminal_b, Some(conversation_b));
        });

        model.read(&app, |model, _| {
            assert_eq!(
                model.get_focused_conversation(window_a),
                Some(conversation_a)
            );
            assert_eq!(
                model.get_focused_conversation(window_b),
                Some(conversation_b)
            );
        });
    });
}

#[test]
fn clearing_one_window_does_not_affect_other() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window_a = WindowId::new();
        let window_b = WindowId::new();
        let terminal_a = EntityId::new();
        let terminal_b = EntityId::new();
        let conversation_a = AIConversationId::new();
        let conversation_b = AIConversationId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, Some(terminal_a), ctx);
            model.update_focused_conversation_for_terminal(terminal_a, Some(conversation_a));
            model.handle_pane_focus_change(window_b, Some(terminal_b), ctx);
            model.update_focused_conversation_for_terminal(terminal_b, Some(conversation_b));
        });

        // Clear window A's focus by passing None for terminal_view_id.
        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, None, ctx);
        });

        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(window_a), None);
            assert_eq!(
                model.get_focused_conversation(window_b),
                Some(conversation_b)
            );
        });
    });
}

#[test]
fn last_focused_terminal_tracks_most_recent_globally() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window_a = WindowId::new();
        let window_b = WindowId::new();
        let terminal_a = EntityId::new();
        let terminal_b = EntityId::new();
        let conversation_a = AIConversationId::new();
        let conversation_b = AIConversationId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, Some(terminal_a), ctx);
            model.update_focused_conversation_for_terminal(terminal_a, Some(conversation_a));
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_last_focused_terminal_id(), Some(terminal_a));
            assert_eq!(model.get_last_focused_conversation(), Some(conversation_a));
        });

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_b, Some(terminal_b), ctx);
            model.update_focused_conversation_for_terminal(terminal_b, Some(conversation_b));
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_last_focused_terminal_id(), Some(terminal_b));
            assert_eq!(model.get_last_focused_conversation(), Some(conversation_b));
        });

        // Clearing window B's focus should NOT clear last_focused (it persists).
        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_b, None, ctx);
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_last_focused_terminal_id(), Some(terminal_b));
            assert_eq!(model.get_last_focused_conversation(), Some(conversation_b));
        });
    });
}

#[test]
fn unknown_window_returns_none() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window_a = WindowId::new();
        let unknown_window = WindowId::new();
        let terminal = EntityId::new();
        let conversation = AIConversationId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, Some(terminal), ctx);
            model.update_focused_conversation_for_terminal(terminal, Some(conversation));
        });

        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(unknown_window), None);
        });
    });
}

#[test]
fn focus_change_without_agent_view_has_no_conversation() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window = WindowId::new();
        let terminal = EntityId::new();

        // No agent view handles registered → active_conversation_id should be None.
        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window, Some(terminal), ctx);
        });

        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(window), None);
        });
    });
}

#[test]
fn remove_focused_state_for_window_cleans_up() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window_a = WindowId::new();
        let window_b = WindowId::new();
        let terminal_a = EntityId::new();
        let terminal_b = EntityId::new();
        let conversation_a = AIConversationId::new();
        let conversation_b = AIConversationId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, Some(terminal_a), ctx);
            model.update_focused_conversation_for_terminal(terminal_a, Some(conversation_a));
            model.handle_pane_focus_change(window_b, Some(terminal_b), ctx);
            model.update_focused_conversation_for_terminal(terminal_b, Some(conversation_b));
        });

        // Remove window A's state (simulating undo-close expiry).
        model.update(&mut app, |model, ctx| {
            model.remove_focused_state_for_window(window_a, ctx);
        });

        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(window_a), None);
            assert_eq!(
                model.get_focused_conversation(window_b),
                Some(conversation_b)
            );
        });

        // Removing again is a no-op.
        model.update(&mut app, |model, ctx| {
            model.remove_focused_state_for_window(window_a, ctx);
        });
    });
}

#[test]
fn overwriting_same_window_updates_state() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window = WindowId::new();
        let terminal_1 = EntityId::new();
        let terminal_2 = EntityId::new();
        let conversation_1 = AIConversationId::new();
        let conversation_2 = AIConversationId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window, Some(terminal_1), ctx);
            model.update_focused_conversation_for_terminal(terminal_1, Some(conversation_1));
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(window), Some(conversation_1));
        });

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window, Some(terminal_2), ctx);
            model.update_focused_conversation_for_terminal(terminal_2, Some(conversation_2));
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(window), Some(conversation_2));
        });
    });
}
