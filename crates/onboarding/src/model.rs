use warp_core::send_telemetry_from_ctx;
use warpui_core::{Entity, ModelContext};

use crate::telemetry::OnboardingEvent;

/// UI customization settings chosen during the "Customize your UI" onboarding slide.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UICustomizationSettings {
    pub use_vertical_tabs: bool,
    pub show_project_explorer: bool,
    pub show_global_search: bool,
    pub show_code_review_button: bool,
}

impl UICustomizationSettings {
    /// Returns true if any tools-panel sub-setting is enabled.
    pub fn tools_panel_enabled(&self) -> bool {
        self.show_project_explorer || self.show_global_search
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingStep {
    Intro,
    Customize,
    ThemePicker,
}

impl OnboardingStep {
    fn slide_name(self) -> &'static str {
        match self {
            OnboardingStep::Intro => "intro",
            OnboardingStep::Customize => "customize",
            OnboardingStep::ThemePicker => "theme_picker",
        }
    }
}

#[derive(Clone, Debug)]
pub enum OnboardingStateEvent {
    SelectedSlideChanged,
    Completed,
}

/// State shared by the onboarding slides.
#[derive(Clone, Debug)]
pub struct OnboardingStateModel {
    step: OnboardingStep,
    ui_customization: UICustomizationSettings,
}

impl Default for OnboardingStateModel {
    fn default() -> Self {
        Self::new()
    }
}

impl OnboardingStateModel {
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Intro,
            ui_customization: UICustomizationSettings::default(),
        }
    }

    pub fn step(&self) -> OnboardingStep {
        self.step
    }

    pub fn ui_customization(&self) -> &UICustomizationSettings {
        &self.ui_customization
    }

    pub fn set_use_vertical_tabs(&mut self, value: bool, ctx: &mut ModelContext<Self>) {
        if self.ui_customization.use_vertical_tabs == value {
            return;
        }
        send_telemetry_from_ctx!(
            OnboardingEvent::SettingChanged {
                setting: "tab_styling".to_string(),
                value: if value { "vertical" } else { "horizontal" }.to_string(),
            },
            ctx
        );
        self.ui_customization.use_vertical_tabs = value;
        ctx.notify();
    }

    pub fn set_tools_panel_enabled(&mut self, enabled: bool, ctx: &mut ModelContext<Self>) {
        send_telemetry_from_ctx!(
            OnboardingEvent::SettingChanged {
                setting: "tools_panel".to_string(),
                value: if enabled { "enabled" } else { "disabled" }.to_string(),
            },
            ctx
        );
        self.ui_customization.show_project_explorer = enabled;
        self.ui_customization.show_global_search = enabled;
        ctx.notify();
    }

    pub fn set_show_project_explorer(&mut self, value: bool, ctx: &mut ModelContext<Self>) {
        if self.ui_customization.show_project_explorer == value {
            return;
        }
        send_telemetry_from_ctx!(
            OnboardingEvent::SettingChanged {
                setting: "project_explorer".to_string(),
                value: value.to_string(),
            },
            ctx
        );
        self.ui_customization.show_project_explorer = value;
        ctx.notify();
    }

    pub fn set_show_global_search(&mut self, value: bool, ctx: &mut ModelContext<Self>) {
        if self.ui_customization.show_global_search == value {
            return;
        }
        send_telemetry_from_ctx!(
            OnboardingEvent::SettingChanged {
                setting: "global_search".to_string(),
                value: value.to_string(),
            },
            ctx
        );
        self.ui_customization.show_global_search = value;
        ctx.notify();
    }

    pub fn set_show_code_review_button(&mut self, value: bool, ctx: &mut ModelContext<Self>) {
        if self.ui_customization.show_code_review_button == value {
            return;
        }
        send_telemetry_from_ctx!(
            OnboardingEvent::SettingChanged {
                setting: "code_review".to_string(),
                value: if value { "enabled" } else { "disabled" }.to_string(),
            },
            ctx
        );
        self.ui_customization.show_code_review_button = value;
        ctx.notify();
    }

    pub fn complete(&mut self, ctx: &mut ModelContext<Self>) {
        send_telemetry_from_ctx!(OnboardingEvent::OnboardingSlidesCompleted, ctx);
        ctx.emit(OnboardingStateEvent::Completed);
        ctx.notify();
    }

    pub fn back(&mut self, ctx: &mut ModelContext<Self>) {
        let prev = match self.step {
            OnboardingStep::Intro => None,
            OnboardingStep::Customize => Some(OnboardingStep::Intro),
            OnboardingStep::ThemePicker => Some(OnboardingStep::Customize),
        };

        if let Some(prev) = prev {
            send_telemetry_from_ctx!(OnboardingEvent::SlideNavigatedBack, ctx);
            self.set_step(prev, ctx);
        }
    }

    pub fn next(&mut self, ctx: &mut ModelContext<Self>) {
        let next = match self.step {
            OnboardingStep::Intro => Some(OnboardingStep::Customize),
            OnboardingStep::Customize => Some(OnboardingStep::ThemePicker),
            OnboardingStep::ThemePicker => None,
        };

        if let Some(next) = next {
            send_telemetry_from_ctx!(OnboardingEvent::SlideNavigatedNext, ctx);
            self.set_step(next, ctx);
        }
    }

    pub fn set_step(&mut self, step: OnboardingStep, ctx: &mut ModelContext<Self>) {
        if self.step == step {
            return;
        }

        self.step = step;
        send_telemetry_from_ctx!(
            OnboardingEvent::SlideViewed {
                slide_name: step.slide_name().to_string(),
            },
            ctx
        );

        ctx.emit(OnboardingStateEvent::SelectedSlideChanged);
        ctx.notify();
    }

    /// The `(step_index, step_count)` shown by the bottom-nav progress dots for the current step.
    pub fn progress(&self) -> (usize, usize) {
        let step_index = match self.step {
            OnboardingStep::Intro => 0,
            OnboardingStep::Customize => 1,
            OnboardingStep::ThemePicker => 2,
        };
        (step_index, 3)
    }
}

impl Entity for OnboardingStateModel {
    type Event = OnboardingStateEvent;
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
