use pathfinder_color::ColorU;
use warp_core::context_flag::ContextFlag;
use warpui::assets::asset_cache::AssetSource;
use warpui::elements::{
    Align, CacheOption, ConstrainedBox, Container, CrossAxisAlignment, Element, Flex, Image,
    MainAxisAlignment, MouseStateHandle, ParentElement, Text, Wrap,
};
use warpui::ui_components::components::UiComponent;
use warpui::{
    AppContext, Entity, ModelHandle, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use super::SettingsSection;
use super::settings_page::{
    MatchData, PageType, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget,
};
use crate::appearance::Appearance;
use crate::autoupdate::{self, AutoupdateStage, AutoupdateState};
use crate::channel::ChannelState;
use crate::themes::theme::ColorScheme;
use crate::workspace::WorkspaceAction;

pub struct AboutPageView {
    page: PageType<Self>,
}

impl AboutPageView {
    pub fn new(ctx: &mut ViewContext<AboutPageView>) -> Self {
        ctx.observe(
            &AutoupdateState::handle(ctx),
            |_, _: ModelHandle<AutoupdateState>, ctx| ctx.notify(),
        );
        AboutPageView {
            page: PageType::new_monolith(AboutPageWidget::default(), None, false),
        }
    }
}

pub enum AboutPageEvent {
    CheckForUpdate,
}

#[derive(Debug, Clone)]
pub enum AboutPageAction {
    Relaunch,
    DownloadUpdate,
    CheckForUpdate,
}

impl Entity for AboutPageView {
    type Event = AboutPageEvent;
}

impl TypedActionView for AboutPageView {
    type Action = AboutPageAction;

    fn handle_action(&mut self, action: &AboutPageAction, ctx: &mut ViewContext<Self>) {
        match action {
            AboutPageAction::Relaunch => autoupdate::initiate_relaunch_for_update(ctx),
            AboutPageAction::DownloadUpdate => autoupdate::manually_download_new_version(ctx),
            AboutPageAction::CheckForUpdate => ctx.emit(AboutPageEvent::CheckForUpdate),
        }
        ctx.notify();
    }
}

struct UpdateStatus {
    text: &'static str,
    is_warning: bool,
    call_to_action: Option<(&'static str, AboutPageAction)>,
}

fn update_status(app: &AppContext) -> UpdateStatus {
    let status = |text, is_warning, call_to_action| UpdateStatus {
        text,
        is_warning,
        call_to_action,
    };
    match autoupdate::get_update_state(app) {
        AutoupdateStage::NoUpdateAvailable => status(
            "Up to date",
            false,
            Some(("Check for updates", AboutPageAction::CheckForUpdate)),
        ),
        AutoupdateStage::CheckingForUpdate => status("Checking for update...", false, None),
        AutoupdateStage::DownloadingUpdate => status("Downloading update...", false, None),
        AutoupdateStage::UpdateReady { .. } => status(
            "Update available",
            true,
            Some(("Relaunch", AboutPageAction::Relaunch)),
        ),
        AutoupdateStage::Updating { .. } => status("Updating...", false, None),
        AutoupdateStage::UpdatedPendingRestart { .. } => status(
            "Installed update",
            false,
            Some(("Relaunch", AboutPageAction::Relaunch)),
        ),
        AutoupdateStage::UnableToUpdateToNewVersion { .. } => status(
            "A new version is available but can't be installed",
            true,
            Some(("Update manually", AboutPageAction::DownloadUpdate)),
        ),
        AutoupdateStage::UnableToLaunchNewVersion { .. } => status(
            "A new version is installed but can't be launched",
            true,
            Some(("Update manually", AboutPageAction::DownloadUpdate)),
        ),
    }
}

impl View for AboutPageView {
    fn ui_name() -> &'static str {
        "AboutPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

#[derive(Default)]
struct AboutPageWidget {
    copy_version_button_mouse_state: MouseStateHandle,
    update_call_to_action_mouse_state: MouseStateHandle,
}

impl AboutPageWidget {
    fn render_update_status(&self, appearance: &Appearance, app: &AppContext) -> Box<dyn Element> {
        let theme = appearance.theme();
        let UpdateStatus {
            text,
            is_warning,
            call_to_action,
        } = update_status(app);
        let color: ColorU = if is_warning {
            theme.terminal_colors().bright.red.into()
        } else {
            theme.active_ui_text_color().with_opacity(60).into()
        };
        let mut row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(
                Text::new_inline(text, appearance.ui_font_family(), appearance.ui_font_size())
                    .with_color(color)
                    .finish(),
            );
        if let Some((label, action)) = call_to_action {
            row.add_child(
                appearance
                    .ui_builder()
                    .link(
                        label.into(),
                        None,
                        Some(Box::new(move |ctx| {
                            ctx.dispatch_typed_action(action.clone());
                        })),
                        self.update_call_to_action_mouse_state.clone(),
                    )
                    .soft_wrap(false)
                    .build()
                    .finish(),
            );
        }
        Container::new(row.finish()).with_margin_top(8.).finish()
    }
}

impl SettingsWidget for AboutPageWidget {
    type View = AboutPageView;

    fn search_terms(&self) -> &str {
        "about alefbet version"
    }

    fn render(
        &self,
        _view: &AboutPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let ui_builder = appearance.ui_builder();

        let image_path = if theme.inferred_color_scheme() == ColorScheme::LightOnDark {
            "bundled/svg/alefbet-logo-with-light-title.svg"
        } else {
            "bundled/svg/alefbet-logo-with-dark-title.svg"
        };

        let version = ChannelState::app_version().unwrap_or("v#.##.###");

        let version_text = ui_builder
            .span(version.to_string())
            .with_soft_wrap()
            .build()
            .with_margin_top(16.)
            .finish();

        let copy_version_icon = appearance
            .ui_builder()
            .copy_button(16., self.copy_version_button_mouse_state.clone())
            .build()
            .on_click(move |ctx, _, _| {
                ctx.dispatch_typed_action(WorkspaceAction::CopyVersion(version));
            })
            .finish();

        let version_row = Wrap::row()
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_children([
                version_text,
                Container::new(copy_version_icon)
                    .with_margin_top(16.)
                    .with_padding_left(6.)
                    .finish(),
            ]);

        let mut column = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                ConstrainedBox::new(
                    Image::new(
                        AssetSource::Bundled { path: image_path },
                        CacheOption::BySize,
                    )
                    .finish(),
                )
                .with_max_height(100.)
                .with_max_width(350.)
                .finish(),
            )
            .with_child(version_row.finish());
        if ContextFlag::PromptForVersionUpdates.is_enabled() {
            column.add_child(self.render_update_status(appearance, app));
        }
        column.add_child(
            ui_builder
                .span("Copyright 2026 Warp")
                .build()
                .with_margin_top(16.)
                .finish(),
        );

        Align::new(column.finish()).finish()
    }
}

impl SettingsPageMeta for AboutPageView {
    fn section() -> SettingsSection {
        SettingsSection::About
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<AboutPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AboutPageView>) -> Self {
        SettingsPageViewHandle::About(view_handle)
    }
}
