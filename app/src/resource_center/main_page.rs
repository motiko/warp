use warpui::elements::{
    Align, ClippedScrollStateHandle, ClippedScrollable, Container, Element, Empty, Fill, Flex,
    MainAxisSize, MouseStateHandle, ParentElement, Shrinkable,
};
use warpui::presenter::ChildView;
use warpui::ui_components::components::{UiComponent, UiComponentStyles};
use warpui::{AppContext, Entity, ModelHandle, SingletonEntity, View, ViewContext, ViewHandle};

use super::ChangelogSectionView;
use super::section_views::{BUTTON_PADDING, FOOTER_ICON_SIZE, SCROLLBAR_WIDTH, SECTION_SPACING};
use crate::appearance::Appearance;
use crate::changelog_model::ChangelogModel;
use crate::channel::ChannelState;
use crate::features::FeatureFlag;
use crate::settings::Settings;
use crate::workspace::WorkspaceAction;

pub struct ResourceCenterMainView {
    copy_version_mouse_state: MouseStateHandle,
    clipped_scroll_state: ClippedScrollStateHandle,
    changelog_view: ViewHandle<ChangelogSectionView>,
}

impl ResourceCenterMainView {
    pub fn new(
        ctx: &mut ViewContext<Self>,
        changelog_model_handle: ModelHandle<ChangelogModel>,
    ) -> Self {
        let showing_new_changelog = match ChannelState::app_version() {
            Some(version) => !Settings::has_changelog_been_shown(version, ctx),
            None => false,
        };
        let changelog_view = ctx.add_typed_action_view(|ctx: &mut ViewContext<_>| {
            ChangelogSectionView::new(changelog_model_handle, showing_new_changelog, ctx)
        });
        Self {
            copy_version_mouse_state: Default::default(),
            clipped_scroll_state: Default::default(),
            changelog_view,
        }
    }

    fn render_body(&self, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        ClippedScrollable::vertical(
            self.clipped_scroll_state.clone(),
            ChildView::new(&self.changelog_view).finish(),
            SCROLLBAR_WIDTH,
            theme.disabled_text_color(theme.background()).into(),
            theme.main_text_color(theme.background()).into(),
            Fill::None,
        )
        .finish()
    }

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
        let mut main_page = Flex::column()
            .with_child(Shrinkable::new(20., self.render_body(appearance)).finish())
            // Placeholder so the pane extends to the bottom of the window.
            .with_child(Shrinkable::new(0.1, Empty::new().finish()).finish());

        if FeatureFlag::Autoupdate.is_enabled() && ChannelState::show_autoupdate_menu_items() {
            main_page.add_child(self.render_current_version(appearance));
        }

        main_page.finish()
    }
}
