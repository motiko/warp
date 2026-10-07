use super::{ChannelState, derive_http_origin_from_ws_url};

#[test]
fn wss_becomes_https_and_strips_path() {
    let got = derive_http_origin_from_ws_url("wss://rtc.app.warp.dev/graphql/v2");
    assert_eq!(got.as_deref(), Some("https://rtc.app.warp.dev"));
}

#[test]
fn ws_becomes_http_and_preserves_port() {
    let got = derive_http_origin_from_ws_url("ws://localhost:8080/graphql/v2");
    assert_eq!(got.as_deref(), Some("http://localhost:8080"));
}

#[test]
fn unparseable_input_returns_none() {
    assert!(derive_http_origin_from_ws_url("not a url").is_none());
    assert!(derive_http_origin_from_ws_url("https://app.warp.dev").is_none());
}

#[test]
fn default_channel_state_is_offline() {
    assert!(ChannelState::is_offline());
    assert!(ChannelState::server_root_url().is_empty());
    assert!(ChannelState::ws_server_url().is_empty());
    assert!(ChannelState::session_sharing_server_url().is_none());
    assert!(!ChannelState::server_root_domain().is_tuple());
}
