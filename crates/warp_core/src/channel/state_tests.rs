use super::ChannelState;

#[test]
fn default_channel_state_is_offline() {
    assert!(ChannelState::is_offline());
    assert!(ChannelState::server_root_url().is_empty());
    assert!(!ChannelState::server_root_domain().is_tuple());
}
