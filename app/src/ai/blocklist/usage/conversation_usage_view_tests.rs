//! Tests for [`ConversationUsageView`].

use std::collections::HashMap;

use super::*;
use crate::persistence::model::{ModelTokenUsage, PRIMARY_AGENT_CATEGORY};

fn placeholder_usage_info() -> ConversationUsageInfo {
    ConversationUsageInfo {
        credits_spent: 0.0,
        platform_credits_spent: 0.0,
        credits_spent_for_last_block: None,
        tool_calls: 0,
        models: Vec::new(),
        context_window_usage: 0.0,
        context_window_segments: Vec::new(),
        files_changed: 0,
        lines_added: 0,
        lines_removed: 0,
        commands_executed: 0,
        total_tokens: None,
        total_cost_in_cents: None,
        tokens_for_last_block: None,
        cost_in_cents_for_last_block: None,
    }
}

#[test]
fn custom_endpoint_models_use_the_external_key_icon_bucket() {
    let view = ConversationUsageView::new(
        ConversationUsageInfo {
            models: vec![ModelTokenUsage {
                model_id: "Friendly alias".to_string(),
                custom_endpoint_tokens: 6,
                custom_endpoint_token_usage_by_category: HashMap::from([(
                    PRIMARY_AGENT_CATEGORY.to_string(),
                    6,
                )]),
                ..Default::default()
            }],
            ..placeholder_usage_info()
        },
        None,
        MouseStateHandle::default(),
    );

    assert_eq!(
        view.collect_models_by_category()
            .get(PRIMARY_AGENT_CATEGORY),
        Some(&vec![("Friendly alias".to_string(), true)])
    );
}
