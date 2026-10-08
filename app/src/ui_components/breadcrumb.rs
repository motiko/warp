use std::fmt::Debug;

use itertools::{Itertools, Position};
use warpui::Element;
use warpui::elements::{CrossAxisAlignment, Flex, MainAxisSize, ParentElement, Shrinkable};
use warpui::ui_components::components::{UiComponent, UiComponentStyles};

use crate::appearance::Appearance;

/// A value which may be rendered as a breadcrumb.
pub trait Breadcrumb: Debug + 'static {
    /// The label to display for this breadcrumb.
    fn label(&self) -> String;
}

impl Breadcrumb for String {
    fn label(&self) -> String {
        self.clone()
    }
}

/// This implementation is for cases where a breadcrumb type is required but unused, such as panes
/// that do not have any breadcrumbs.
impl Breadcrumb for () {
    fn label(&self) -> String {
        String::new()
    }
}

/// State for a breadcrumb component.
#[derive(Clone)]
pub struct BreadcrumbState<T: Breadcrumb> {
    breadcrumb: T,
}

impl<T: Breadcrumb> BreadcrumbState<T> {
    pub fn new(breadcrumb: T) -> Self {
        Self { breadcrumb }
    }
}

fn render_breadcrumb<T: Breadcrumb>(
    state: &BreadcrumbState<T>,
    is_last_item: bool,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let suffix = if is_last_item { "" } else { " / " };
    let name = state.breadcrumb.label() + suffix;
    let font_color = appearance
        .theme()
        .sub_text_color(appearance.theme().background());

    appearance
        .ui_builder()
        .span(name)
        .with_style(UiComponentStyles {
            font_color: Some(font_color.into()),
            ..Default::default()
        })
        .build()
        .finish()
}

/// Render a row of breadcrumbs.
pub fn render_breadcrumbs<T, I>(breadcrumbs: I, appearance: &Appearance) -> Box<dyn Element>
where
    T: Breadcrumb,
    I: IntoIterator<Item = BreadcrumbState<T>>,
{
    let children = breadcrumbs
        .into_iter()
        .with_position()
        .map(|(position, breadcrumb)| {
            // Each breadcrumb is expanded so that it inherits the parent `Flex`'s size constraint.
            Shrinkable::new(
                1.,
                render_breadcrumb(
                    &breadcrumb,
                    matches!(position, Position::Last | Position::Only),
                    appearance,
                ),
            )
            .finish()
        });

    Flex::row()
        .with_children(children)
        .with_main_axis_size(MainAxisSize::Min)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .finish()
}
