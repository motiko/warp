use super::{ClientId, ServerId, SyncId};

#[test]
pub fn test_client_sync_id_serialization() {
    let id: SyncId = SyncId::ClientId(ClientId::new());
    let serialized = serde_json::to_string(&id).expect("failed to serialize");
    assert_eq!(serialized, format!("\"{}\"", id.uid()));
    let deserialized: SyncId =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialize");
    assert_eq!(id, deserialized);
}

#[test]
pub fn test_server_sync_id_serialization() {
    let id = SyncId::ServerId(ServerId::from(123));
    let serialized = serde_json::to_string(&id).expect("failed to serialize");
    assert_eq!(serialized, format!("\"{}\"", ServerId::from(123)));
    let deserialized: SyncId =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialize");
    assert_eq!(id, deserialized);
}

#[test]
pub fn test_server_sync_id_uid_serialization() {
    let id = SyncId::ServerId(
        ServerId::try_from("Ymgrzu0nh2HwDNeYEtXF1x").expect("valid server id"),
    );
    let serialized = serde_json::to_string(&id).expect("failed to serialize");
    assert_eq!(
        serialized,
        format!("\"{}\"", String::from("Ymgrzu0nh2HwDNeYEtXF1x"))
    );
    let deserialized: SyncId =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialize");
    assert_eq!(id, deserialized);
}
