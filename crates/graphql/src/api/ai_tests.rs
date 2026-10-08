#[test]
fn list_ai_conversations_query_selects_token_usage_fields() {
    use cynic::QueryBuilder;

    use crate::queries::list_ai_conversations::{
        ListAIConversations, ListAIConversationsInput, ListAIConversationsVariables,
    };
    use crate::request_context::{ClientContext, OsContext, RequestContext};

    let operation = ListAIConversations::build(ListAIConversationsVariables {
        input: ListAIConversationsInput {
            conversation_ids: None,
        },
        request_context: RequestContext {
            client_context: ClientContext { version: None },
            os_context: OsContext {
                category: None,
                linux_kernel_version: None,
                name: None,
                version: None,
            },
        },
    });

    for field in [
        "warpTokenUsage",
        "byokTokenUsage",
        "toolUsageMetadata",
        "tokenUsageByCategory",
    ] {
        assert!(
            operation.query.contains(field),
            "restore query no longer selects `{field}`:\n{}",
            operation.query
        );
    }
}
