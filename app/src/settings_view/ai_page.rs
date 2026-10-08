//! The "AI (optional)" settings page: the master AI switch and the user's own custom inference
//! endpoints.

use ai_config::api_keys::{ApiKeyManager, CustomEndpointParams};
use settings::ToggleableSetting as _;
use warp_core::ui::theme::color::internal_colors;
use warpui::elements::{
    Border, ChildView, Container, CornerRadius, CrossAxisAlignment, Element, Empty, Flex,
    MainAxisAlignment, MainAxisSize, ParentElement, Radius, Shrinkable, Text,
};
use warpui::fonts::{Properties, Weight};
use warpui::keymap::{ContextPredicate, Keystroke};
use warpui::ui_components::components::{Coords, UiComponent, UiComponentStyles};
use warpui::ui_components::switch::SwitchStateHandle;
use warpui::{
    Action, AppContext, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

use super::custom_inference_modal::{
    CustomEndpointModal, CustomEndpointModalEvent, CustomEndpointModalViewState,
};
use super::remove_custom_endpoint_confirmation_dialog::{
    RemoveCustomEndpointConfirmationDialog, RemoveCustomEndpointConfirmationDialogEvent,
};
use super::settings_page::{
    CONTENT_FONT_SIZE, MatchData, PageTitle, PageType, SettingsPageMeta, SettingsPageViewHandle,
    SettingsWidget, TOGGLE_BUTTON_RIGHT_PADDING,
};
use super::{SettingsAction, SettingsSection, ToggleSettingActionPair, flags};
use crate::ToastStack;
use crate::appearance::Appearance;
use crate::modal::{Modal, ModalEvent, ModalViewState};
use crate::settings::{AISettings, AISettingsChangedEvent};
use crate::ui_components::icons::Icon;
use crate::util::bindings;
use crate::view_components::DismissibleToast;
use crate::view_components::action_button::{ActionButton, ButtonSize, SecondaryTheme};

const PAGE_TITLE: &str = "AI (optional)";
const AI_DESCRIPTION: &str = "AI features stay off unless you turn them on. Add your own model \
     endpoint below; it must support OpenAI Chat Completions, OpenAI Responses, or Anthropic \
     Messages. API keys are stored only on this device.";
const CUSTOM_ENDPOINT_MODAL_MAX_HEIGHT_PERCENTAGE: f32 = 0.8;

pub fn init_actions_from_parent_view<T: Action + Clone>(
    app: &mut AppContext,
    context: &ContextPredicate,
    builder: fn(SettingsAction) -> T,
) {
    ToggleSettingActionPair::add_toggle_setting_action_pairs_as_bindings(
        vec![
            ToggleSettingActionPair::new(
                "AI",
                builder(SettingsAction::AI(AIPageAction::ToggleGlobalAI)),
                context,
                flags::IS_ANY_AI_ENABLED,
            )
            .with_group(bindings::BindingGroup::WarpAi),
        ],
        app,
    );
}

pub struct AIPageView {
    page: PageType<Self>,
    custom_endpoint_modal_state: CustomEndpointModalViewState,
    remove_custom_endpoint_confirmation_dialog: ViewHandle<RemoveCustomEndpointConfirmationDialog>,
    add_custom_endpoint_button: ViewHandle<ActionButton>,
    custom_endpoint_edit_buttons: Vec<ViewHandle<ActionButton>>,
}

pub enum AIPageEvent {
    ShowModal,
    HideModal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AIPageAction {
    ToggleGlobalAI,
    OpenAddCustomEndpointModal,
    OpenEditCustomEndpointModal(usize),
}

impl AIPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let controls_enabled = Self::custom_endpoint_controls_enabled(ctx);

        ctx.subscribe_to_model(&ApiKeyManager::handle(ctx), |me, _, _, ctx| {
            me.sync_custom_endpoint_buttons(ctx);
            ctx.notify();
        });
        ctx.subscribe_to_model(&AISettings::handle(ctx), |me, _, event, ctx| {
            if let AISettingsChangedEvent::IsAnyAIEnabled { .. } = event {
                me.sync_custom_endpoint_buttons(ctx);
            }
            ctx.notify();
        });

        let add_custom_endpoint_button = ctx.add_typed_action_view(|_| {
            ActionButton::new("+ Add custom endpoint", SecondaryTheme)
                .with_size(ButtonSize::Small)
                .on_click(|ctx| {
                    ctx.dispatch_typed_action(AIPageAction::OpenAddCustomEndpointModal);
                })
        });
        add_custom_endpoint_button.update(ctx, |button, ctx| {
            button.set_disabled(!controls_enabled, ctx);
        });

        let custom_endpoint_modal_body =
            ctx.add_typed_action_view(|ctx| CustomEndpointModal::new(None, None, ctx));
        ctx.subscribe_to_view(&custom_endpoint_modal_body, |me, _, event, ctx| {
            me.handle_custom_endpoint_modal_event(event, ctx);
        });
        let custom_endpoint_modal_view = ctx.add_typed_action_view(|ctx| {
            Modal::new(
                Some("Add custom endpoint".to_string()),
                custom_endpoint_modal_body.clone(),
                ctx,
            )
            .with_modal_style(UiComponentStyles {
                width: Some(560.),
                ..Default::default()
            })
            .with_header_style(UiComponentStyles {
                padding: Some(Coords {
                    top: 24.,
                    bottom: 0.,
                    left: 24.,
                    right: 24.,
                }),
                font_size: Some(16.),
                font_weight: Some(Weight::Bold),
                ..Default::default()
            })
            .with_body_style(UiComponentStyles {
                padding: Some(Coords {
                    top: 0.,
                    bottom: 24.,
                    left: 24.,
                    right: 0.,
                }),
                ..Default::default()
            })
            .with_background_opacity(100)
            .with_max_height_percentage(CUSTOM_ENDPOINT_MODAL_MAX_HEIGHT_PERCENTAGE)
            .with_dismiss_on_click()
            .with_dismiss_keystroke(Keystroke::parse("escape").unwrap())
        });
        ctx.subscribe_to_view(
            &custom_endpoint_modal_view,
            |me, _, event, ctx| match event {
                ModalEvent::Close => me.hide_custom_endpoint_modal(ctx),
            },
        );
        let custom_endpoint_modal_state =
            CustomEndpointModalViewState::new(ModalViewState::new(custom_endpoint_modal_view));

        let remove_custom_endpoint_confirmation_dialog =
            ctx.add_typed_action_view(RemoveCustomEndpointConfirmationDialog::new);
        ctx.subscribe_to_view(
            &remove_custom_endpoint_confirmation_dialog,
            |me, _, event, ctx| {
                me.handle_remove_custom_endpoint_confirmation_dialog_event(event, ctx);
            },
        );

        let custom_endpoint_edit_buttons = Self::create_custom_endpoint_edit_buttons(
            ApiKeyManager::as_ref(ctx).custom_endpoints().len(),
            controls_enabled,
            ctx,
        );

        Self {
            page: Self::build_page(),
            custom_endpoint_modal_state,
            remove_custom_endpoint_confirmation_dialog,
            add_custom_endpoint_button,
            custom_endpoint_edit_buttons,
        }
    }

    fn build_page() -> PageType<Self> {
        let global_ai_switch_state = SwitchStateHandle::default();
        PageType::new_uncategorized(
            vec![Box::new(CustomEndpointsWidget)],
            Some(
                PageTitle::new(PAGE_TITLE).with_trailing_element(move |_, appearance, app| {
                    Container::new(
                        appearance
                            .ui_builder()
                            .switch(global_ai_switch_state.clone())
                            .check(AISettings::as_ref(app).is_any_ai_enabled())
                            .build()
                            .on_click(move |ctx, _, _| {
                                ctx.dispatch_typed_action(AIPageAction::ToggleGlobalAI);
                            })
                            .finish(),
                    )
                    .with_padding_right(TOGGLE_BUTTON_RIGHT_PADDING)
                    .finish()
                }),
            ),
        )
    }

    pub fn get_modal_content(&self, app: &AppContext) -> Option<Box<dyn Element>> {
        if self.custom_endpoint_modal_state.is_open() {
            Some(self.custom_endpoint_modal_state.render())
        } else if self
            .remove_custom_endpoint_confirmation_dialog
            .as_ref(app)
            .is_visible()
        {
            Some(ChildView::new(&self.remove_custom_endpoint_confirmation_dialog).finish())
        } else {
            None
        }
    }

    fn custom_endpoint_controls_enabled(app: &AppContext) -> bool {
        AISettings::as_ref(app).is_any_ai_enabled()
    }

    fn sync_custom_endpoint_buttons(&mut self, ctx: &mut ViewContext<Self>) {
        let enabled = Self::custom_endpoint_controls_enabled(ctx);
        self.add_custom_endpoint_button.update(ctx, |button, ctx| {
            button.set_disabled(!enabled, ctx);
        });

        let endpoint_count = ApiKeyManager::as_ref(ctx).custom_endpoints().len();
        if self.custom_endpoint_edit_buttons.len() != endpoint_count {
            self.custom_endpoint_edit_buttons =
                Self::create_custom_endpoint_edit_buttons(endpoint_count, enabled, ctx);
        } else {
            for button in &self.custom_endpoint_edit_buttons {
                button.update(ctx, |button, ctx| {
                    button.set_disabled(!enabled, ctx);
                });
            }
        }
    }

    fn create_custom_endpoint_edit_buttons(
        count: usize,
        enabled: bool,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<ViewHandle<ActionButton>> {
        (0..count)
            .map(|index| {
                let button = ctx.add_typed_action_view(move |_| {
                    ActionButton::new("Edit", SecondaryTheme)
                        .with_icon(Icon::Pencil)
                        .with_size(ButtonSize::Small)
                        .on_click(move |ctx| {
                            ctx.dispatch_typed_action(AIPageAction::OpenEditCustomEndpointModal(
                                index,
                            ));
                        })
                });
                button.update(ctx, |button, ctx| {
                    button.set_disabled(!enabled, ctx);
                });
                button
            })
            .collect()
    }

    fn show_custom_endpoint_modal(&mut self, index: Option<usize>, ctx: &mut ViewContext<Self>) {
        if !Self::custom_endpoint_controls_enabled(ctx) {
            return;
        }
        let endpoint = match index {
            Some(index) => {
                let Some(endpoint) = ApiKeyManager::as_ref(ctx)
                    .custom_endpoints()
                    .get(index)
                    .cloned()
                else {
                    return;
                };
                Some(endpoint)
            }
            None => None,
        };

        self.remove_custom_endpoint_confirmation_dialog
            .update(ctx, |dialog, ctx| dialog.hide(ctx));

        let title = if index.is_some() {
            "Edit custom endpoint"
        } else {
            "Add custom endpoint"
        };
        self.custom_endpoint_modal_state
            .set_title(Some(title.to_string()), ctx);
        self.custom_endpoint_modal_state
            .prefill(endpoint.as_ref(), index, ctx);
        self.custom_endpoint_modal_state.open(ctx);
        ctx.emit(AIPageEvent::ShowModal);
        ctx.notify();
    }

    fn hide_custom_endpoint_modal(&mut self, ctx: &mut ViewContext<Self>) {
        self.custom_endpoint_modal_state.close(ctx);
        ctx.emit(AIPageEvent::HideModal);
        ctx.notify();
    }

    fn show_toast(message: &str, ctx: &mut ViewContext<Self>) {
        let window_id = ctx.window_id();
        let toast = DismissibleToast::success(message.to_string());
        ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
            toast_stack.add_ephemeral_toast(toast, window_id, ctx);
        });
    }

    fn handle_custom_endpoint_modal_event(
        &mut self,
        event: &CustomEndpointModalEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        if !Self::custom_endpoint_controls_enabled(ctx) {
            self.hide_custom_endpoint_modal(ctx);
            return;
        }
        match event {
            CustomEndpointModalEvent::Close => self.hide_custom_endpoint_modal(ctx),
            CustomEndpointModalEvent::AddEndpoint {
                name,
                url,
                api_key,
                schema,
                models,
            } => {
                let params = CustomEndpointParams {
                    name: name.clone(),
                    url: url.clone(),
                    api_key: api_key.clone(),
                    models: models.clone(),
                    schema: *schema,
                };
                if let Err(error) = crate::ai::custom_endpoints::add(params, ctx) {
                    log::warn!("Could not add custom endpoint: {error:#}");
                    return;
                }
                self.hide_custom_endpoint_modal(ctx);
                Self::show_toast("Endpoint added", ctx);
            }
            CustomEndpointModalEvent::SaveEndpoint {
                index,
                name,
                url,
                api_key,
                schema,
                models,
            } => {
                let params = CustomEndpointParams {
                    name: name.clone(),
                    url: url.clone(),
                    api_key: api_key.clone(),
                    models: models.clone(),
                    schema: *schema,
                };
                if let Err(error) = crate::ai::custom_endpoints::save(*index, params, ctx) {
                    log::warn!("Could not save custom endpoint: {error:#}");
                    return;
                }
                self.hide_custom_endpoint_modal(ctx);
                Self::show_toast("Endpoint saved", ctx);
            }
            CustomEndpointModalEvent::RemoveEndpoint { index } => {
                self.hide_custom_endpoint_modal(ctx);
                self.show_remove_custom_endpoint_confirmation_dialog(*index, ctx);
            }
        }
    }

    fn show_remove_custom_endpoint_confirmation_dialog(
        &mut self,
        index: usize,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(endpoint) = ApiKeyManager::as_ref(ctx)
            .custom_endpoints()
            .get(index)
            .cloned()
        else {
            return;
        };
        let model_labels = endpoint
            .models
            .iter()
            .map(|model| model.alias.clone().unwrap_or_else(|| model.name.clone()))
            .filter(|label| !label.trim().is_empty())
            .collect();
        self.remove_custom_endpoint_confirmation_dialog
            .update(ctx, |dialog, ctx| {
                dialog.show(index, endpoint.name.clone(), model_labels, ctx);
            });
        ctx.notify();
    }

    fn handle_remove_custom_endpoint_confirmation_dialog_event(
        &mut self,
        event: &RemoveCustomEndpointConfirmationDialogEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        self.remove_custom_endpoint_confirmation_dialog
            .update(ctx, |dialog, ctx| dialog.hide(ctx));
        match event {
            RemoveCustomEndpointConfirmationDialogEvent::Cancel => {}
            RemoveCustomEndpointConfirmationDialogEvent::Confirm(index) => {
                if !Self::custom_endpoint_controls_enabled(ctx) {
                    ctx.notify();
                    return;
                }
                if let Err(error) = crate::ai::custom_endpoints::remove(*index, ctx) {
                    log::warn!("Could not remove custom endpoint: {error:#}");
                    return;
                }
                self.sync_custom_endpoint_buttons(ctx);
                Self::show_toast("Endpoint removed", ctx);
            }
        }
        ctx.notify();
    }
}

impl Entity for AIPageView {
    type Event = AIPageEvent;
}

impl View for AIPageView {
    fn ui_name() -> &'static str {
        "AIPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl TypedActionView for AIPageView {
    type Action = AIPageAction;

    fn handle_action(&mut self, action: &AIPageAction, ctx: &mut ViewContext<Self>) {
        match action {
            AIPageAction::ToggleGlobalAI => {
                if let Err(e) = AISettings::handle(ctx).update(ctx, |settings, ctx| {
                    settings.is_any_ai_enabled.toggle_and_save_value(ctx)
                }) {
                    log::warn!("Failed to set value for Global AI setting: {e:?}");
                }
                ctx.notify();
            }
            AIPageAction::OpenAddCustomEndpointModal => self.show_custom_endpoint_modal(None, ctx),
            AIPageAction::OpenEditCustomEndpointModal(index) => {
                self.show_custom_endpoint_modal(Some(*index), ctx)
            }
        }
    }
}

impl SettingsPageMeta for AIPageView {
    fn section() -> SettingsSection {
        SettingsSection::AI
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

impl From<ViewHandle<AIPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<AIPageView>) -> Self {
        SettingsPageViewHandle::AI(view_handle)
    }
}

struct CustomEndpointsWidget;

impl CustomEndpointsWidget {
    fn render_endpoint_row(
        name: &str,
        model_labels: Vec<String>,
        edit_button: Box<dyn Element>,
        enabled: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let text_color = if enabled {
            theme.main_text_color(theme.surface_2())
        } else {
            theme.disabled_ui_text_color()
        };
        let endpoint_name = Text::new_inline(
            name.to_string(),
            appearance.ui_font_family(),
            appearance.ui_font_size(),
        )
        .with_style(Properties::default().weight(Weight::Semibold))
        .with_color(text_color.into())
        .finish();
        let left = Flex::column()
            .with_spacing(8.)
            .with_child(endpoint_name)
            .with_child(super::render_model_chips(
                model_labels,
                appearance,
                text_color,
            ))
            .finish();
        let row = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Shrinkable::new(1., left).finish())
            .with_child(edit_button)
            .finish();
        Container::new(row)
            .with_uniform_padding(12.)
            .with_background(internal_colors::fg_overlay_1(theme))
            .with_border(Border::all(1.).with_border_fill(internal_colors::fg_overlay_3(theme)))
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .finish()
    }
}

impl SettingsWidget for CustomEndpointsWidget {
    type View = AIPageView;

    fn search_terms(&self) -> &str {
        "ai custom inference endpoint model api key openai anthropic llm"
    }

    fn render(
        &self,
        view: &AIPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let enabled = AIPageView::custom_endpoint_controls_enabled(app);

        let mut column = Flex::column().with_spacing(12.);
        column.add_child(
            appearance
                .ui_builder()
                .paragraph(AI_DESCRIPTION)
                .with_style(UiComponentStyles {
                    font_color: Some(theme.sub_text_color(theme.surface_1()).into()),
                    ..Default::default()
                })
                .build()
                .finish(),
        );

        let header = Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Text::new_inline(
                    "Custom endpoints",
                    appearance.ui_font_family(),
                    CONTENT_FONT_SIZE,
                )
                .with_style(Properties::default().weight(Weight::Semibold))
                .with_color(theme.active_ui_text_color().into())
                .finish(),
            )
            .with_child(view.add_custom_endpoint_button.as_ref(app).render(app))
            .finish();
        column.add_child(header);

        for (index, endpoint) in ApiKeyManager::as_ref(app)
            .custom_endpoints()
            .iter()
            .enumerate()
        {
            let model_labels = endpoint
                .models
                .iter()
                .map(|model| model.alias.clone().unwrap_or_else(|| model.name.clone()))
                .filter(|label| !label.trim().is_empty())
                .collect();
            let edit_button = view
                .custom_endpoint_edit_buttons
                .get(index)
                .map(|button| button.as_ref(app).render(app))
                .unwrap_or_else(|| Empty::new().finish());
            column.add_child(Self::render_endpoint_row(
                &endpoint.name,
                model_labels,
                edit_button,
                enabled,
                appearance,
            ));
        }

        Container::new(column.finish())
            .with_margin_bottom(16.)
            .finish()
    }
}
