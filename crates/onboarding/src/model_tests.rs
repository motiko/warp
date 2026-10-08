use warp_core::telemetry::testing::MockTelemetryContextProvider;
use warpui_core::{App, Entity, ModelHandle};

use crate::model::{
    OnboardingStateEvent, OnboardingStateModel, OnboardingStep, UICustomizationSettings,
};

fn add_test_model(app: &mut App) -> ModelHandle<OnboardingStateModel> {
    app.update(MockTelemetryContextProvider::register);
    app.add_model(|_| OnboardingStateModel::new())
}

fn step(app: &App, model: &ModelHandle<OnboardingStateModel>) -> OnboardingStep {
    model.read(app, |model, _| model.step())
}

fn progress(app: &App, model: &ModelHandle<OnboardingStateModel>) -> (usize, usize) {
    model.read(app, |model, _| model.progress())
}

fn ui_settings(app: &App, model: &ModelHandle<OnboardingStateModel>) -> UICustomizationSettings {
    model.read(app, |model, _| model.ui_customization().clone())
}

/// Completion is an event rather than a state change, so it can't be read back off the model.
#[derive(Default)]
struct CompletionObserver {
    completions: usize,
}

impl Entity for CompletionObserver {
    type Event = ();
}

fn observe_completions(
    app: &mut App,
    model: &ModelHandle<OnboardingStateModel>,
) -> ModelHandle<CompletionObserver> {
    let model = model.clone();
    app.add_model(move |ctx| {
        ctx.subscribe_to_model(
            &model,
            |observer: &mut CompletionObserver, _, event, _| match event {
                OnboardingStateEvent::Completed => observer.completions += 1,
                OnboardingStateEvent::SelectedSlideChanged => {}
            },
        );
        CompletionObserver::default()
    })
}

#[test]
fn next_advances_through_intro_customize_and_theme_picker() {
    App::test((), |mut app| async move {
        let model = add_test_model(&mut app);
        assert_eq!(step(&app, &model), OnboardingStep::Intro);
        assert_eq!(progress(&app, &model), (0, 3));

        model.update(&mut app, |model, ctx| model.next(ctx));
        assert_eq!(step(&app, &model), OnboardingStep::Customize);
        assert_eq!(progress(&app, &model), (1, 3));

        model.update(&mut app, |model, ctx| model.next(ctx));
        assert_eq!(step(&app, &model), OnboardingStep::ThemePicker);
        assert_eq!(progress(&app, &model), (2, 3));

        model.update(&mut app, |model, ctx| model.next(ctx));
        assert_eq!(step(&app, &model), OnboardingStep::ThemePicker);
    });
}

#[test]
fn back_returns_to_previous_step_and_stops_at_intro() {
    App::test((), |mut app| async move {
        let model = add_test_model(&mut app);
        model.update(&mut app, |model, ctx| {
            model.set_step(OnboardingStep::ThemePicker, ctx)
        });

        model.update(&mut app, |model, ctx| model.back(ctx));
        assert_eq!(step(&app, &model), OnboardingStep::Customize);

        model.update(&mut app, |model, ctx| model.back(ctx));
        assert_eq!(step(&app, &model), OnboardingStep::Intro);

        model.update(&mut app, |model, ctx| model.back(ctx));
        assert_eq!(step(&app, &model), OnboardingStep::Intro);
    });
}

#[test]
fn complete_emits_completed_event() {
    App::test((), |mut app| async move {
        let model = add_test_model(&mut app);
        let observer = observe_completions(&mut app, &model);

        model.update(&mut app, |model, ctx| model.complete(ctx));
        assert_eq!(observer.read(&app, |observer, _| observer.completions), 1);
    });
}

#[test]
fn ui_customization_defaults_to_terminal_settings() {
    App::test((), |mut app| async move {
        let model = add_test_model(&mut app);
        let ui = ui_settings(&app, &model);
        assert_eq!(ui, UICustomizationSettings::default());
        assert!(!ui.use_vertical_tabs);
        assert!(!ui.tools_panel_enabled());
        assert!(!ui.show_code_review_button);
    });
}

#[test]
fn ui_customization_setters_update_settings() {
    App::test((), |mut app| async move {
        let model = add_test_model(&mut app);

        model.update(&mut app, |model, ctx| {
            model.set_use_vertical_tabs(true, ctx);
            model.set_show_code_review_button(true, ctx);
            model.set_show_global_search(true, ctx);
        });
        let ui = ui_settings(&app, &model);
        assert!(ui.use_vertical_tabs);
        assert!(ui.show_code_review_button);
        assert!(ui.show_global_search);
        assert!(!ui.show_project_explorer);
        assert!(ui.tools_panel_enabled());

        model.update(&mut app, |model, ctx| {
            model.set_show_project_explorer(true, ctx)
        });
        assert!(ui_settings(&app, &model).show_project_explorer);
    });
}

#[test]
fn tools_panel_toggle_sets_all_sub_settings() {
    App::test((), |mut app| async move {
        let model = add_test_model(&mut app);

        model.update(&mut app, |model, ctx| {
            model.set_tools_panel_enabled(true, ctx)
        });
        let ui_enabled = ui_settings(&app, &model);
        assert!(ui_enabled.show_project_explorer);
        assert!(ui_enabled.show_global_search);

        model.update(&mut app, |model, ctx| {
            model.set_tools_panel_enabled(false, ctx)
        });
        let ui_disabled = ui_settings(&app, &model);
        assert!(!ui_disabled.show_project_explorer);
        assert!(!ui_disabled.show_global_search);
        assert!(!ui_disabled.tools_panel_enabled());
    });
}
