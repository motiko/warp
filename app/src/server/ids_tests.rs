use super::ServerId;

#[test]
pub fn test_server_id_serialization() {
    let id = ServerId::try_from("Ymgrzu0nh2HwDNeYEtXF1x").expect("valid server id");
    let serialized = serde_json::to_string(&id).expect("failed to serialize");
    assert_eq!(serialized, "\"Ymgrzu0nh2HwDNeYEtXF1x\"");
    let deserialized: ServerId =
        serde_json::from_str(serialized.as_str()).expect("failed to deserialize");
    assert_eq!(id, deserialized);
}
