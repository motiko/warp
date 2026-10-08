//! Footer offering to warpify a detected subshell, rendered at the bottom of the active block or
//! the alt screen.

use std::sync::Arc;

use parking_lot::FairMutex;
use pathfinder_color::ColorU;
use warp_core::send_telemetry_from_ctx;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::color::contrast::{
    MinimumAllowedContrast, high_enough_contrast, pick_best_foreground_color,
};
use warp_core::ui::theme::Fill as ThemeFill;
use warp_core::ui::theme::color::internal_colors;
use warpui::elements::{
    ChildView, Container, CrossAxisAlignment, Expanded, Flex, MainAxisSize, ParentElement,
};
use warpui::prelude::Empty;
use warpui::{
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::{PADDING_LEFT, RichContentInsertionPosition, TerminalAction, TerminalView};
use crate::server::telemetry::TelemetryEvent;
use crate::settings::InputModeSettings;
use crate::terminal::TerminalModel;
use crate::ui_components::blended_colors;
use crate::ui_components::icons::Icon;
use crate::view_components::action_button::{
    ActionButton, ActionButtonTheme, ButtonSize, KeystrokeSource, TooltipAlignment,
};

impl TerminalView {
    pub(super) fn register_subscriptions_for_warpify_footer(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.subscribe_to_view(&self.warpify_footer, |me, _, event, ctx| {
            me.handle_warpify_footer_event(event, ctx);
        });

        let input_mode_settings = InputModeSettings::handle(ctx);
        let mut was_pinned_to_top = input_mode_settings
            .as_ref(ctx)
            .input_mode
            .is_pinned_to_top();
        ctx.subscribe_to_model(&input_mode_settings, move |me, settings_handle, _, ctx| {
            let is_pinned_to_top = settings_handle.as_ref(ctx).is_pinned_to_top();
            if was_pinned_to_top != is_pinned_to_top {
                was_pinned_to_top = is_pinned_to_top;
                me.maybe_show_warpify_footer_in_blocklist(ctx);
            }
        });
    }

    fn handle_warpify_footer_event(
        &mut self,
        event: &WarpifyFooterViewEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            WarpifyFooterViewEvent::Dismiss => {
                self.hide_warpify_footer_in_blocklist(ctx);
                ctx.notify();
            }
            WarpifyFooterViewEvent::Warpify => {
                self.hide_warpify_footer_in_blocklist(ctx);
                self.handle_action(&TerminalAction::TriggerSubshellBootstrap, ctx);
                send_telemetry_from_ctx!(
                    TelemetryEvent::WarpifyFooterAcceptedWarpify { is_ssh: false },
                    ctx
                );
            }
        }
    }

    pub(super) fn should_render_warpify_footer(&self, app: &AppContext) -> bool {
        self.warpify_footer.as_ref(app).is_active()
    }

    pub(super) fn maybe_show_warpify_footer_in_blocklist(&mut self, ctx: &mut ViewContext<Self>) {
        // This is a bit of a hack- but it ensures we never show more than one footer in the
        // blocklist.
        self.hide_warpify_footer_in_blocklist(ctx);
        let is_alt_screen_active = self.model.lock().is_alt_screen_active();
        if is_alt_screen_active || !self.should_render_warpify_footer(ctx) {
            return;
        }

        let should_insert_after_block = !InputModeSettings::as_ref(ctx).is_pinned_to_top();

        self.insert_rich_content(
            None,
            self.warpify_footer.clone(),
            None,
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: should_insert_after_block,
            },
            ctx,
        );
    }

    pub(super) fn hide_warpify_footer_in_blocklist(&mut self, ctx: &mut ViewContext<Self>) {
        let mut model = self.model.lock();
        let block_list = model.block_list_mut();
        block_list.remove_rich_content(self.warpify_footer.id());
        ctx.notify();
    }
}

/// Footer rendered at the bottom of the active block or alt screen when a subshell command is
/// detected, offering to warpify it.
pub struct WarpifyFooterView {
    terminal_model: Arc<FairMutex<TerminalModel>>,
    warpify_button: ViewHandle<ActionButton>,
    dismiss_button: ViewHandle<ActionButton>,
    /// Whether the footer is currently offering subshell warpification.
    is_active: bool,
}

impl WarpifyFooterView {
    pub fn new(terminal_model: Arc<FairMutex<TerminalModel>>, ctx: &mut ViewContext<Self>) -> Self {
        let button_size = ButtonSize::XSmall;

        let warpify_button = ctx.add_typed_action_view(|_ctx| {
            ActionButton::new("Warpify subshell", FooterButtonTheme::new(None))
                .with_icon(Icon::Warp)
                .with_size(button_size)
                .with_tooltip("Enable Warp shell integration in this session")
                .with_tooltip_alignment(TooltipAlignment::Left)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(WarpifyFooterViewAction::Warpify);
                })
        });

        let dismiss_button = ctx.add_typed_action_view(|_ctx| {
            ActionButton::new("Dismiss", FooterButtonTheme::new(None))
                .with_size(button_size)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(WarpifyFooterViewAction::Dismiss);
                })
        });

        Self {
            terminal_model,
            warpify_button,
            dismiss_button,
            is_active: false,
        }
    }

    /// Activates the footer so it offers subshell warpification.
    pub fn show(&mut self, ctx: &mut ViewContext<Self>) {
        self.warpify_button.update(ctx, |button, ctx| {
            button.set_keybinding(
                Some(KeystrokeSource::Binding("terminal:warpify_subshell")),
                ctx,
            );
        });
        self.is_active = true;
        ctx.notify();
    }

    /// Returns whether the footer is currently offering subshell warpification.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Deactivates the footer.
    pub fn clear(&mut self, ctx: &mut ViewContext<Self>) {
        self.is_active = false;
        self.warpify_button.update(ctx, |button, ctx| {
            button.set_keybinding(None, ctx);
        });
        ctx.notify();
    }
}

#[derive(Debug, Clone)]
pub enum WarpifyFooterViewAction {
    Warpify,
    Dismiss,
}

pub enum WarpifyFooterViewEvent {
    Warpify,
    Dismiss,
}

impl Entity for WarpifyFooterView {
    type Event = WarpifyFooterViewEvent;
}

impl View for WarpifyFooterView {
    fn ui_name() -> &'static str {
        "WarpifyFooterView"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        let terminal_model = self.terminal_model.lock();

        let button_row = Flex::row()
            .with_spacing(4.)
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(ChildView::new(&self.warpify_button).finish())
            .with_child(Expanded::new(1., Empty::new().finish()).finish())
            .with_child(ChildView::new(&self.dismiss_button).finish());

        let mut container = Container::new(button_row.finish())
            .with_horizontal_padding(*PADDING_LEFT)
            .with_vertical_padding(4.);

        if terminal_model.is_alt_screen_active()
            && let Some(bg_color) = terminal_model.alt_screen().inferred_bg_color()
        {
            container = container.with_background(bg_color);
        }

        container.finish()
    }
}

impl TypedActionView for WarpifyFooterView {
    type Action = WarpifyFooterViewAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            WarpifyFooterViewAction::Warpify => {
                if self.is_active {
                    self.clear(ctx);
                    ctx.emit(WarpifyFooterViewEvent::Warpify);
                }
            }
            WarpifyFooterViewAction::Dismiss => {
                self.clear(ctx);
                ctx.emit(WarpifyFooterViewEvent::Dismiss);
            }
        }
    }
}

#[derive(Clone)]
struct FooterButtonTheme {
    /// When set, enables alt-screen contrast adjustment for text and border.
    terminal_model: Option<Arc<FairMutex<TerminalModel>>>,
}

impl FooterButtonTheme {
    pub fn new(terminal_model: Option<Arc<FairMutex<TerminalModel>>>) -> Self {
        Self { terminal_model }
    }

    /// Returns the inferred background colour of the alt screen, if active.
    fn inferred_alt_screen_bg(&self) -> Option<ColorU> {
        let terminal_model = self.terminal_model.as_ref()?;
        let terminal_model = terminal_model.lock();
        terminal_model
            .is_alt_screen_active()
            .then(|| terminal_model.alt_screen().inferred_bg_color())
            .flatten()
    }

    /// Picks a colour that contrasts well against `bg`, choosing between two
    /// neutral candidates.
    fn contrast_adjusted_color(
        bg: ColorU,
        default: ColorU,
        contrast: MinimumAllowedContrast,
        appearance: &Appearance,
    ) -> ColorU {
        if high_enough_contrast(default, bg, contrast) {
            default
        } else {
            pick_best_foreground_color(
                bg,
                blended_colors::neutral_2(appearance.theme()),
                blended_colors::neutral_6(appearance.theme()),
                contrast,
            )
        }
    }
}

impl ActionButtonTheme for FooterButtonTheme {
    fn background(&self, hovered: bool, appearance: &Appearance) -> Option<ThemeFill> {
        if hovered {
            Some(internal_colors::fg_overlay_2(appearance.theme()))
        } else {
            None
        }
    }

    fn border(&self, appearance: &Appearance) -> Option<ColorU> {
        let color = appearance.theme().outline().into_solid();
        if let Some(bg) = self.inferred_alt_screen_bg() {
            return Some(Self::contrast_adjusted_color(
                bg,
                color,
                MinimumAllowedContrast::NonText,
                appearance,
            ));
        }
        Some(color)
    }

    fn text_color(
        &self,
        _hovered: bool,
        _background: Option<ThemeFill>,
        appearance: &Appearance,
    ) -> ColorU {
        let color = appearance
            .theme()
            .sub_text_color(appearance.theme().surface_1())
            .into_solid();

        // If rendered in the alt screen, the footer is rendered with the inferred background color
        // of the alt screen output grid (if there is one). In such cases, we have to ensure that
        // the text within the footer is high-contrast enough to be legible, since the background
        // color can essentially be anything.
        if let Some(bg) = self.inferred_alt_screen_bg() {
            return Self::contrast_adjusted_color(
                bg,
                color,
                MinimumAllowedContrast::Text,
                appearance,
            );
        }
        color
    }
}
