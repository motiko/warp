use std::ffi::OsString;

use clap::Parser;

use super::*;

fn set_env_var(name: &str, value: &str) -> Option<OsString> {
    let previous = std::env::var_os(name);
    // Safety: tests that mutate process environment are marked `serial` so we
    // do not race with other environment readers/writers in this crate.
    unsafe { std::env::set_var(name, value) };
    previous
}

fn restore_env_var(name: &str, previous: Option<OsString>) {
    match previous {
        // Safety: tests that mutate process environment are marked `serial` so
        // we do not race with other environment readers/writers in this crate.
        Some(value) => unsafe { std::env::set_var(name, value) },
        // Safety: tests that mutate process environment are marked `serial` so
        // we do not race with other environment readers/writers in this crate.
        None => unsafe { std::env::remove_var(name) },
    }
}

#[test]
fn worker_subcommand_helpers_name_existing_subcommands() {
    #[cfg(unix)]
    assert_eq!(terminal_server_subcommand(), "terminal-server");
    #[cfg(not(target_family = "wasm"))]
    assert_eq!(ripgrep_search_subcommand(), "ripgrep-search");
    assert_eq!(finish_update_flag(), "--finish-update");
    assert_eq!(dump_debug_info_flag(), "--dump-debug-info");
}

#[test]
#[serial_test::serial]
fn help_hides_api_key_env_value() {
    const API_KEY: &str = "warp-cli-test-api-key-NOT-REAL";

    let previous_api_key = set_env_var("WARP_API_KEY", API_KEY);

    let help = <Args as clap::CommandFactory>::command()
        .render_long_help()
        .to_string();
    let args = Args::try_parse_from(["warp"]).expect("API key env var should parse");

    restore_env_var("WARP_API_KEY", previous_api_key);

    assert!(
        help.contains("WARP_API_KEY"),
        "help should identify the API key environment variable:\n{help}"
    );
    assert!(
        !help.contains(API_KEY),
        "help should not reveal the API key environment value:\n{help}"
    );
    assert_eq!(args.api_key().map(String::as_str), Some(API_KEY));
}

#[test]
#[serial_test::serial]
fn hidden_server_overrides_parse_from_env() {
    let previous_server_root = set_env_var(SERVER_ROOT_URL_OVERRIDE_ENV, "http://localhost:8080");
    let previous_ws = set_env_var(WS_SERVER_URL_OVERRIDE_ENV, "ws://localhost:8082/graphql/v2");

    let args = Args::try_parse_from(["warp"]).unwrap();

    restore_env_var(SERVER_ROOT_URL_OVERRIDE_ENV, previous_server_root);
    restore_env_var(WS_SERVER_URL_OVERRIDE_ENV, previous_ws);

    assert_eq!(args.server_root_url(), Some("http://localhost:8080"));
    assert_eq!(args.ws_server_url(), Some("ws://localhost:8082/graphql/v2"));
}
