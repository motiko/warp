pub mod changelog_section;
pub use changelog_section::ChangelogSectionView;
use warpui::ViewContext;
use warpui::elements::{
    Align, Border, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Hoverable, Icon,
    MouseStateHandle, ParentElement, ScrollbarWidth, Shrinkable,
};
use warpui::platform::Cursor;
use warpui::ui_components::components::{UiComponent, UiComponentStyles};

use crate::appearance::Appearance;

pub const HEADER_FONT_SIZE: f32 = 16.;
pub const SECTION_HEADER_FONT_SIZE: f32 = 16.;
pub const DESCRIPTION_FONT_SIZE: f32 = 14.;

pub const KEYBOARD_ICON_SIZE: f32 = 30.;
pub const CHEVRON_ICON_SIZE: f32 = 20.;
pub const FOOTER_ICON_SIZE: f32 = 15.;
pub const ICON_PADDING: f32 = 3.;

pub const DROPDOWN_ICON_OPACITY: u8 = 75;

// TODO: update scrollbar behaviour to not take up space when non-active
// Spacing to offset scrollbar width (which makes things off-centered)
pub const SCROLLBAR_OFFSET: f32 = 7.;
pub const SCROLLBAR_WIDTH: ScrollbarWidth = ScrollbarWidth::Auto;

pub const SECTION_SPACING: f32 = 12.;
pub const BUTTON_PADDING: f32 = 10.;
pub const ITEM_PADDING_BOTTOM: f32 = 6.;

pub const CHEVRON_DOWN_SKINNY_SVG_PATH: &str = "bundled/svg/chevron-down-skinny.svg";
pub const CHEVRON_RIGHT_SKINNY_SVG_PATH: &str = "bundled/svg/chevron-right-skinny.svg";

#[derive(Debug)]
pub enum SectionAction {
    OpenUrl(String),
    ToggleExpanded,
}

pub trait SectionView {
    fn is_expanded(&self) -> bool;

    fn toggle_expanded(&mut self, ctx: &mut ViewContext<Self>);

    fn section_link(&self, appearance: &Appearance) -> Option<Box<dyn Element>>;

    fn render_section_header(
        &self,
        section_name: &'static str,
        appearance: &Appearance,
        top_bar_mouse_state: MouseStateHandle,
    ) -> Box<dyn Element> {
        Hoverable::new(top_bar_mouse_state, |state| {
            let mut section_header = Flex::row();

            let section_title = Shrinkable::new(
                1.0,
                Align::new(
                    appearance
                        .ui_builder()
                        .wrappable_text(section_name, false)
                        .with_style(UiComponentStyles {
                            font_family_id: Some(appearance.ui_font_family()),
                            font_size: Some(SECTION_HEADER_FONT_SIZE),
                            ..Default::default()
                        })
                        .build()
                        .finish(),
                )
                .left()
                .finish(),
            )
            .finish();

            let icon_path = if self.is_expanded() {
                CHEVRON_DOWN_SKINNY_SVG_PATH
            } else {
                CHEVRON_RIGHT_SKINNY_SVG_PATH
            };

            let icon_color = if state.is_hovered() {
                appearance.theme().active_ui_detail()
            } else {
                appearance
                    .theme()
                    .active_ui_detail()
                    .with_opacity(DROPDOWN_ICON_OPACITY)
            };

            let dropdown_icon =
                ConstrainedBox::new(Icon::new(icon_path, icon_color.into_solid()).finish())
                    .with_height(CHEVRON_ICON_SIZE)
                    .with_width(CHEVRON_ICON_SIZE)
                    .finish();

            section_header.add_child(dropdown_icon);
            section_header.add_child(section_title);

            if let Some(link) = self.section_link(appearance) {
                section_header.add_child(link)
            }

            Container::new(
                section_header
                    .with_cross_axis_alignment(CrossAxisAlignment::Center)
                    .finish(),
            )
            .with_uniform_padding(SECTION_SPACING)
            .with_background(appearance.theme().surface_2())
            .with_border(
                Border::top(1.)
                    .with_border_color(appearance.theme().split_pane_border_color().into()),
            )
            .finish()
        })
        .on_click(move |ctx, _, _| ctx.dispatch_typed_action(SectionAction::ToggleExpanded))
        .with_cursor(Cursor::PointingHand)
        .finish()
    }
}
