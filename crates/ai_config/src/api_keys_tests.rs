use super::*;

fn make_manager(keys: ApiKeys) -> ApiKeyManager {
    let custom_endpoints = keys.custom_endpoints.clone();
    ApiKeyManager {
        keys,
        custom_endpoints: CustomEndpointState {
            definitions: None,
            settings_valid: true,
            keys: HashMap::new(),
            resolved: custom_endpoints,
        },
    }
}

fn endpoint(
    name: &str,
    url: &str,
    api_key: &str,
    models: &[(&str, Option<&str>)],
) -> CustomEndpoint {
    endpoint_with_keys(
        name,
        url,
        api_key,
        &models
            .iter()
            .enumerate()
            .map(|(i, (n, a))| (*n, *a, format!("cfg-{i}")))
            .collect::<Vec<_>>()
            .iter()
            .map(|(n, a, k)| (*n, *a, k.as_str()))
            .collect::<Vec<_>>(),
    )
}

fn endpoint_with_keys(
    name: &str,
    url: &str,
    api_key: &str,
    models: &[(&str, Option<&str>, &str)],
) -> CustomEndpoint {
    CustomEndpoint {
        name: name.into(),
        url: url.into(),
        api_key: api_key.into(),
        schema: CustomEndpointSchema::default(),
        models: models
            .iter()
            .map(|(n, a, cfg)| CustomEndpointModel {
                name: (*n).into(),
                alias: a.map(|s| s.into()),
                config_key: (*cfg).into(),
            })
            .collect(),
    }
}

#[test]
fn custom_endpoint_definitions_round_trip_without_secrets() {
    let legacy = vec![endpoint_with_keys(
        "OpenRouter",
        "https://openrouter.ai/api/v1",
        "secret",
        &[("openai/gpt-5", Some("GPT-5"), "config-key")],
    )];
    let (definitions, keys) = CustomEndpointDefinitions::from_legacy(&legacy).unwrap();
    let json = serde_json::to_string(&definitions).unwrap();
    let decoded: CustomEndpointDefinitions = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, definitions);
    assert!(!json.contains("secret"));
    assert_eq!(keys.values().next().map(String::as_str), Some("secret"));
}

#[test]
fn custom_endpoint_definitions_reject_duplicate_model_config_keys() {
    let legacy = vec![
        endpoint_with_keys(
            "One",
            "https://one.example.com",
            "one",
            &[("model-one", None, "duplicate")],
        ),
        endpoint_with_keys(
            "Two",
            "https://two.example.com",
            "two",
            &[("model-two", None, "duplicate")],
        ),
    ];

    assert!(CustomEndpointDefinitions::from_legacy(&legacy).is_err());
}

#[test]
fn custom_endpoint_url_requires_public_https() {
    for valid in [
        "https://api.example.com/v1",
        "https://openrouter.ai/api/v1",
        "https://8.8.8.8/v1",
    ] {
        assert_eq!(validate_custom_endpoint_url(valid), Ok(()));
    }
    for invalid in [
        "http://api.example.com/v1",
        "https://localhost:8080",
        "https://127.0.0.1/v1",
        "https://10.0.0.1/v1",
        "https://[::1]/v1",
        "not a url",
    ] {
        assert!(
            validate_custom_endpoint_url(invalid).is_err(),
            "{invalid} should be rejected"
        );
    }
}

#[test]
fn legacy_endpoint_ids_are_deterministic_and_preserve_config_keys() {
    let legacy = vec![endpoint_with_keys(
        "Endpoint",
        "https://api.example.com/v1",
        "secret",
        &[("model", None, "existing-config-key")],
    )];
    let (first, first_keys) = CustomEndpointDefinitions::from_legacy(&legacy).unwrap();
    let (second, second_keys) = CustomEndpointDefinitions::from_legacy(&legacy).unwrap();

    assert_eq!(first, second);
    assert_eq!(first_keys, second_keys);
    let (id, definition) = first.definitions().next().unwrap();
    assert!(id.as_str().starts_with(LEGACY_ENDPOINT_PREFIX));
    assert_eq!(definition.models[0].config_key, "existing-config-key");
}

#[test]
fn endpoint_definitions_join_keys_fail_closed_and_recover() {
    warpui_core::App::test((), |mut app| async move {
        app.update(|ctx| {
            warpui_extras::secure_storage::register_noop("test", ctx);
        });
        let manager = app.add_singleton_model(ApiKeyManager::new);
        let legacy = vec![endpoint_with_keys(
            "Endpoint",
            "https://api.example.com/v1",
            "secret",
            &[("model", None, "config-key")],
        )];
        let (definitions, keys) = CustomEndpointDefinitions::from_legacy(&legacy).unwrap();
        let endpoint_id = definitions.id_at(0).unwrap().clone();

        manager
            .update(&mut app, |manager, ctx| {
                manager.set_custom_endpoint_definitions(definitions.clone(), ctx);
                assert_eq!(manager.custom_endpoints()[0].api_key, "");
                manager.persist_custom_endpoint_keys(keys, ctx)
            })
            .unwrap();
        manager.read(&app, |manager, _| {
            assert_eq!(manager.custom_endpoint_key(&endpoint_id), Some("secret"));
            assert_eq!(manager.custom_endpoints()[0].api_key, "secret");
        });

        manager.update(&mut app, |manager, ctx| {
            manager.invalidate_custom_endpoint_definitions(ctx);
        });
        manager.read(&app, |manager, _| {
            assert!(!manager.custom_endpoint_settings_valid());
            assert!(manager.custom_endpoints().is_empty());
            assert_eq!(manager.custom_endpoint_key(&endpoint_id), Some("secret"));
        });

        manager.update(&mut app, |manager, ctx| {
            manager.set_custom_endpoint_definitions(definitions, ctx);
        });
        manager.read(&app, |manager, _| {
            assert!(manager.custom_endpoint_settings_valid());
            assert_eq!(manager.custom_endpoints()[0].api_key, "secret");
        });

        manager
            .update(&mut app, |manager, ctx| {
                manager.persist_custom_endpoint_key(endpoint_id.clone(), None, ctx)
            })
            .unwrap();
        manager.read(&app, |manager, _| {
            assert_eq!(manager.custom_endpoint_key(&endpoint_id), None);
            assert_eq!(manager.custom_endpoints()[0].api_key, "");
        });
    });
}

#[test]
fn empty_active_definitions_disable_the_legacy_fallback() {
    warpui_core::App::test((), |mut app| async move {
        let manager = app.add_singleton_model(|_| {
            make_manager(ApiKeys {
                custom_endpoints: vec![endpoint_with_keys(
                    "Legacy",
                    "https://legacy.example.com",
                    "secret",
                    &[("model", None, "config-key")],
                )],
            })
        });
        manager.read(&app, |manager, _| {
            assert_eq!(manager.custom_endpoints().len(), 1);
        });

        manager.update(&mut app, |manager, ctx| {
            manager.set_custom_endpoint_definitions(CustomEndpointDefinitions::default(), ctx);
        });
        manager.read(&app, |manager, _| {
            assert!(manager.custom_endpoints().is_empty());
        });
    });
}
#[test]
fn serde_round_trip_with_custom_endpoints() {
    let keys = ApiKeys {
        custom_endpoints: vec![
            endpoint("ep1", "https://a.io/v1", "key1", &[("gpt-4", Some("fast"))]),
            endpoint(
                "ep2",
                "https://b.io/v1",
                "key2",
                &[("llama-70b", None), ("mixtral", Some("mix"))],
            ),
        ],
    };
    let json = serde_json::to_string(&keys).unwrap();
    let deser: ApiKeys = serde_json::from_str(&json).unwrap();
    assert_eq!(keys, deser);
}

#[test]
fn serde_ignores_legacy_provider_keys() {
    let json = r#"{"openai":"sk-x","unknown_field":"value","custom_endpoints":[]}"#;
    let keys: ApiKeys = serde_json::from_str(json).unwrap();
    assert!(keys.custom_endpoints.is_empty());
}

#[test]
fn serde_legacy_endpoint_defaults_to_chat_completions() {
    let endpoint: CustomEndpoint = serde_json::from_str(
        r#"{"name":"legacy","url":"https://example.com","api_key":"key","models":[]}"#,
    )
    .unwrap();
    assert_eq!(endpoint.schema, CustomEndpointSchema::OpenaiChatCompletions);
}

#[test]
fn display_label_uses_alias_when_present() {
    let m = CustomEndpointModel {
        name: "raw-name".into(),
        alias: Some("My Alias".into()),
        config_key: "k".into(),
    };
    assert_eq!(m.display_label(), "My Alias");
}

#[test]
fn display_label_falls_back_to_name_when_alias_missing() {
    let m = CustomEndpointModel {
        name: "raw-name".into(),
        alias: None,
        config_key: "k".into(),
    };
    assert_eq!(m.display_label(), "raw-name");
}

#[test]
fn display_label_falls_back_to_name_when_alias_is_whitespace() {
    let m = CustomEndpointModel {
        name: "raw-name".into(),
        alias: Some("   ".into()),
        config_key: "k".into(),
    };
    assert_eq!(m.display_label(), "raw-name");
}
