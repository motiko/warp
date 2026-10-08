// Onboarding library crate

pub mod components;
mod model;
pub mod slides;
pub mod telemetry;

pub use model::{
    OnboardingStateEvent, OnboardingStateModel, OnboardingStep, UICustomizationSettings,
};
pub use telemetry::OnboardingEvent;
