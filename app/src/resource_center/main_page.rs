use warpui::elements::{
    Align, Container, Element, Empty, Flex, MainAxisSize, MouseStateHandle, ParentElement,
    Shrinkable,
};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{AppContext, Entity, SingletonEntity, View};

use super::section_views::{BUTTON_PADDING, FOOTER_ICON_SIZE, SECTION_SPACING};
use crate::appearance::Appearance;
use crate::channel::ChannelState;
use crate::workspace::WorkspaceAction;

#[derive(Default)]
pub struct ResourceCenterMainView {
    copy_version_mouse_state: MouseStateHandle,
}

impl ResourceCenterMainView {
    fn render_current_version(&self, appearance: &Appearance) -> Box<dyn Element> {
        // Use a dummy string for git release tag which is not available on local env
        let version = ChannelState::app_version().unwrap_or("v0.local.testing.string_00");

        let style = UiComponentStyles {
            font_color: Some(appearance.theme().nonactive_ui_text_color().into()),
            ..Default::default()
        };

        let text = appearance
            .ui_builder()
            .wrappable_text(version, true)
            .with_style(style)
            .build()
            .finish();

        let copy_icon = appearance
            .ui_builder()
            .copy_button(FOOTER_ICON_SIZE, self.copy_version_mouse_state.clone())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::CopyVersion(version))
            })
            .finish();

        Container::new(
            Flex::row()
                .with_child(Shrinkable::new(1., Align::new(text).left().finish()).finish())
                .with_child(Shrinkable::new(0.2, Align::new(copy_icon).finish()).finish())
                .with_main_axis_size(MainAxisSize::Max)
                .finish(),
        )
        .with_margin_left(SECTION_SPACING)
        .with_margin_bottom(BUTTON_PADDING)
        .with_uniform_padding(BUTTON_PADDING)
        .finish()
    }
}

impl Entity for ResourceCenterMainView {
    type Event = ();
}

impl View for ResourceCenterMainView {
    fn ui_name() -> &'static str {
        "ResourceCenterMain"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        Flex::column()
            // Placeholder so the pane extends to the bottom of the window.
            .with_child(Shrinkable::new(1., Empty::new().finish()).finish())
            .with_child(self.render_current_version(appearance))
            .finish()
    }
}
