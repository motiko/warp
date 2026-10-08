use serde_json::json;
use strum::IntoEnumIterator;
use warp_core::telemetry::{TelemetryEvent, TelemetryEventDesc};

use super::{OnboardingEvent, OnboardingEventDiscriminant};

#[test]
fn slide_and_setting_payloads() {
    assert_eq!(
        OnboardingEvent::SlideViewed {
            slide_name: "customize".to_string(),
        }
        .payload(),
        Some(json!({
            "slide_name": "customize",
        }))
    );
    assert_eq!(
        OnboardingEvent::SettingChanged {
            setting: "theme".to_string(),
            value: "Dark".to_string(),
        }
        .payload(),
        Some(json!({
            "setting": "theme",
            "value": "Dark",
        }))
    );
}

#[test]
fn navigation_events_have_no_payload() {
    assert_eq!(OnboardingEvent::OnboardingSlidesCompleted.payload(), None);
    assert_eq!(OnboardingEvent::GetStartedClicked.payload(), None);
    assert_eq!(OnboardingEvent::SlideNavigatedNext.payload(), None);
    assert_eq!(OnboardingEvent::SlideNavigatedBack.payload(), None);
}

#[test]
fn event_names_match_discriminant_names() {
    assert_eq!(
        OnboardingEvent::SlideNavigatedNext.name(),
        "onboarding_slide_navigated_next"
    );
    for discriminant in OnboardingEventDiscriminant::iter() {
        assert!(discriminant.name().starts_with("onboarding_"));
    }
}
