use std::default::Default;
use std::sync::Arc;

use parking_lot::FairMutex;
use style::{MODAL_HEIGHT, MODAL_WIDTH};
use warpui::keymap::FixedBinding;
use warpui::presenter::ChildView;
use warpui::ui_components::components::UiComponentStyles;
use warpui::{
    AppContext, Element, Entity, EntityId, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::modal::Modal;
use crate::pane_group::TerminalPaneId;
use crate::terminal::TerminalModel;

mod body;
mod style;

use body::Body;

use self::body::BodyEvent;
use super::{SharedSessionActionSource, SharedSessionScrollbackType};

const MODAL_HEADER: &str = "Share session";

pub struct ShareSessionModal {
    modal: ViewHandle<Modal<Body>>,
    terminal_pane_id: Option<TerminalPaneId>,
    /// Where we opened the modal from.
    open_source: SharedSessionActionSource,
}

#[derive(Debug)]
pub enum ShareSessionModalAction {
    Cancel,
}

pub enum ShareSessionModalEvent {
    Close,
    StartSharing {
        terminal_pane_id: TerminalPaneId,
        scrollback_type: SharedSessionScrollbackType,
        source: SharedSessionActionSource,
    },
}

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        ShareSessionModalAction::Cancel,
        id!("ShareSessionModal"),
    )]);
}

impl ShareSessionModal {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let body = ctx.add_typed_action_view(Body::new);
        ctx.subscribe_to_view(&body, move |me, _, event, ctx| {
            me.handle_body_event(event, ctx);
        });

        let modal = ctx.add_typed_action_view(|ctx| {
            Modal::new(Some(MODAL_HEADER.to_string()), body, ctx)
                .with_modal_style(UiComponentStyles {
                    width: Some(MODAL_WIDTH),
                    height: Some(MODAL_HEIGHT),
                    ..Default::default()
                })
                .with_header_style(style::modal_header_styles())
                .with_body_style(style::modal_body_styles())
                .with_background_opacity(100)
                .with_dismiss_on_click()
                .close_modal_button_disabled()
        });

        Self {
            modal,
            terminal_pane_id: None,
            // This should get overwritten when the modal is actually opened.
            open_source: SharedSessionActionSource::Tab,
        }
    }

    /// Closes the share session modal. If `focus_pane` is `true` we will return focus to the most
    /// recently focused pane.
    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        if self.terminal_pane_id.is_none() {
            log::warn!("Tried to close share modal when no terminal pane ID was present");
            return;
        };
        ctx.emit(ShareSessionModalEvent::Close);
        ctx.notify();
    }

    fn start_sharing(
        &mut self,
        scrollback_type: SharedSessionScrollbackType,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(terminal_pane_id) = self.terminal_pane_id else {
            return;
        };
        ctx.emit(ShareSessionModalEvent::StartSharing {
            terminal_pane_id,
            scrollback_type,
            source: self.open_source,
        });
    }

    pub fn open(
        &mut self,
        terminal_pane_id: TerminalPaneId,
        open_source: SharedSessionActionSource,
        model: Arc<FairMutex<TerminalModel>>,
        terminal_view_id: EntityId,
        ctx: &mut ViewContext<Self>,
    ) {
        self.terminal_pane_id = Some(terminal_pane_id);
        self.open_source = open_source;
        self.modal.update(ctx, |modal, ctx| {
            modal.body().update(ctx, |modal, ctx| {
                modal.open(open_source, model, terminal_view_id, ctx);
            });
        });
        ctx.notify();
    }

    fn handle_body_event(&mut self, event: &BodyEvent, ctx: &mut ViewContext<Self>) {
        match event {
            BodyEvent::Close => self.close(ctx),
            BodyEvent::StartSharing { scrollback_type } => {
                self.start_sharing(*scrollback_type, ctx)
            }
        }
    }

    #[cfg(test)]
    pub fn terminal_pane_id(&self) -> Option<TerminalPaneId> {
        self.terminal_pane_id
    }
}

impl Entity for ShareSessionModal {
    type Event = ShareSessionModalEvent;
}

impl View for ShareSessionModal {
    fn ui_name() -> &'static str {
        "ShareSessionModal"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.modal).finish()
    }
}

impl TypedActionView for ShareSessionModal {
    type Action = ShareSessionModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            ShareSessionModalAction::Cancel => self.close(ctx),
        }
    }
}
