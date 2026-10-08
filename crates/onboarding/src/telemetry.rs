use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use strum_macros::{EnumDiscriminants, EnumIter};
use warp_core::telemetry::{EnablementState, TelemetryEvent, TelemetryEventDesc};

/// Telemetry events for the onboarding flow.
#[derive(Clone, Debug, Serialize, Deserialize, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIter))]
#[strum_discriminants(name(OnboardingEventDiscriminant))]
pub enum OnboardingEvent {
    /// A specific slide was viewed.
    SlideViewed { slide_name: String },
    /// A setting was changed during onboarding.
    SettingChanged { setting: String, value: String },
    /// The onboarding slides were completed.
    OnboardingSlidesCompleted,
    /// The user clicked the "Get Started" button.
    GetStartedClicked,
    /// The user navigated to the next slide.
    SlideNavigatedNext,
    /// The user navigated to the previous slide.
    SlideNavigatedBack,
}

impl TelemetryEvent for OnboardingEvent {
    fn name(&self) -> &'static str {
        OnboardingEventDiscriminant::from(self).name()
    }

    fn payload(&self) -> Option<Value> {
        match self {
            OnboardingEvent::SlideViewed { slide_name } => Some(json!({
                "slide_name": slide_name,
            })),
            OnboardingEvent::SettingChanged { setting, value } => Some(json!({
                "setting": setting,
                "value": value,
            })),
            OnboardingEvent::OnboardingSlidesCompleted
            | OnboardingEvent::GetStartedClicked
            | OnboardingEvent::SlideNavigatedNext
            | OnboardingEvent::SlideNavigatedBack => None,
        }
    }

    fn description(&self) -> &'static str {
        OnboardingEventDiscriminant::from(self).description()
    }

    fn enablement_state(&self) -> EnablementState {
        EnablementState::Always
    }

    fn contains_ugc(&self) -> bool {
        false
    }

    fn event_descs() -> impl Iterator<Item = Box<dyn TelemetryEventDesc>> {
        warp_core::telemetry::enum_events::<Self>()
    }
}

impl TelemetryEventDesc for OnboardingEventDiscriminant {
    fn name(&self) -> &'static str {
        match self {
            OnboardingEventDiscriminant::SlideViewed => "onboarding_slide_viewed",
            OnboardingEventDiscriminant::SettingChanged => "onboarding_setting_changed",
            OnboardingEventDiscriminant::OnboardingSlidesCompleted => "onboarding_slides_completed",
            OnboardingEventDiscriminant::GetStartedClicked => "onboarding_get_started_clicked",
            OnboardingEventDiscriminant::SlideNavigatedNext => "onboarding_slide_navigated_next",
            OnboardingEventDiscriminant::SlideNavigatedBack => "onboarding_slide_navigated_back",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            OnboardingEventDiscriminant::SlideViewed => {
                "User viewed a slide in the onboarding flow"
            }
            OnboardingEventDiscriminant::SettingChanged => {
                "User changed a setting during onboarding"
            }
            OnboardingEventDiscriminant::OnboardingSlidesCompleted => {
                "User completed the onboarding slides"
            }
            OnboardingEventDiscriminant::GetStartedClicked => "User clicked the Get Started button",
            OnboardingEventDiscriminant::SlideNavigatedNext => "User navigated to the next slide",
            OnboardingEventDiscriminant::SlideNavigatedBack => {
                "User navigated to the previous slide"
            }
        }
    }

    fn enablement_state(&self) -> EnablementState {
        EnablementState::Always
    }
}

warp_core::register_telemetry_event!(OnboardingEvent);

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod tests;
