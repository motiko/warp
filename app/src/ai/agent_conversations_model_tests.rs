use std::collections::HashMap;

use chrono::Utc;
use persistence::model::ConversationUsageMetadata;
use warp_core::features::FeatureFlag;
use warpui::{App, SingletonEntity};

use super::entry::{
    AgentConversationEntryId, AgentConversationNavigationSubject, AgentConversationProvenance,
};
use super::query::{DEFAULT_RESULT_COUNT, MAX_SEARCH_RESULTS};
use super::{
    AgentConversationsModel, ConversationMetadata, InitialConversationLoadState,
    query_conversation_entries,
};
use crate::ai::active_agent_views_model::ActiveAgentViewsModel;
use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::{
    AIAgentHarness, AIConversationId, ServerAIConversationMetadata,
};
use crate::ai::blocklist::history_model::BlocklistAIHistoryModel;
use crate::ai::conversation_navigation::ConversationNavigationData;
use crate::auth::AuthStateProvider;
use crate::cloud_object::{Owner, Revision, ServerMetadata, ServerPermissions};
use crate::server::ids::ServerId;
use crate::workspace::{WorkspaceAction, WorkspaceRegistry};

fn create_test_model() -> AgentConversationsModel {
    AgentConversationsModel {
        conversations: HashMap::new(),
        initial_load_state: InitialConversationLoadState::LoadingLocal,
    }
}

fn model_with_titles(
    titles: &[(String, chrono::DateTime<chrono::Local>)],
) -> AgentConversationsModel {
    let mut model = create_test_model();
    for (title, last_updated) in titles {
        let conversation_id = AIConversationId::new();
        let mut metadata = create_test_conversation_metadata(conversation_id, title);
        metadata.nav_data.last_updated = *last_updated;
        model.conversations.insert(conversation_id, metadata);
    }
    model
}

#[test]
fn conversation_query_caps_recent_entries_and_places_newest_last() {
    App::test((), |mut app| async move {
        add_entry_projection_test_models(&mut app);
        let now = chrono::Local::now();
        let titles = (0..55)
            .map(|index| {
                (
                    format!("Conversation {index}"),
                    now - chrono::Duration::seconds(index as i64),
                )
            })
            .collect::<Vec<_>>();
        let model = model_with_titles(&titles);

        app.update(|ctx| {
            let results = query_conversation_entries(model.get_entries(ctx), "");

            assert_eq!(results.len(), DEFAULT_RESULT_COUNT);
            assert_eq!(
                results
                    .first()
                    .map(|result| result.entry.display.title.as_str()),
                Some("Conversation 49")
            );
            assert_eq!(
                results
                    .last()
                    .map(|result| result.entry.display.title.as_str()),
                Some("Conversation 0")
            );
        });
    });
}

#[test]
fn conversation_query_filters_titles_and_caps_best_fuzzy_results() {
    App::test((), |mut app| async move {
        add_entry_projection_test_models(&mut app);
        let now = chrono::Local::now();
        let titles = (0..=MAX_SEARCH_RESULTS + 2)
            .map(|index| {
                let title = if index == 1 {
                    "Fix unit tests".to_owned()
                } else {
                    format!("Deploy service {index}")
                };
                (title, now - chrono::Duration::seconds(index as i64))
            })
            .collect::<Vec<_>>();
        let model = model_with_titles(&titles);

        app.update(|ctx| {
            let results = query_conversation_entries(model.get_entries(ctx), "deploy");

            assert_eq!(results.len(), MAX_SEARCH_RESULTS);
            assert!(
                results
                    .iter()
                    .all(|result| result.entry.display.title.contains("Deploy"))
            );
            assert!(results.windows(2).all(|window| {
                window[0].title_match.as_ref().unwrap().score
                    <= window[1].title_match.as_ref().unwrap().score
            }));
        });
    });
}

#[test]
fn conversation_query_orders_equal_fuzzy_scores_by_recency() {
    App::test((), |mut app| async move {
        add_entry_projection_test_models(&mut app);
        let now = chrono::Local::now();
        let titles = [0, 2, 1]
            .into_iter()
            .map(|index| {
                (
                    "Deploy service".to_owned(),
                    now - chrono::Duration::seconds(index),
                )
            })
            .collect::<Vec<_>>();
        let model = model_with_titles(&titles);

        app.update(|ctx| {
            let results = query_conversation_entries(model.get_entries(ctx), "deploy");

            assert!(results.windows(2).all(|window| {
                window[0].entry.display.last_updated <= window[1].entry.display.last_updated
            }));
        });
    });
}

fn create_test_conversation_metadata(
    conversation_id: AIConversationId,
    title: &str,
) -> ConversationMetadata {
    ConversationMetadata {
        nav_data: ConversationNavigationData {
            id: conversation_id,
            title: title.to_string(),
            initial_query: None,
            last_updated: chrono::Local::now(),
            terminal_view_id: None,
            window_id: None,
            pane_view_locator: None,
            initial_working_directory: None,
            latest_working_directory: None,
            is_selected: false,
            is_in_active_pane: false,
            is_closed: false,
            server_conversation_token: None,
        },
    }
}

fn add_entry_projection_test_models(app: &mut App) {
    app.add_singleton_model(|_| AuthStateProvider::new_for_test());
    app.add_singleton_model(|_| BlocklistAIHistoryModel::new(vec![], vec![], &[]));
    app.add_singleton_model(|_| ActiveAgentViewsModel::new());
    app.add_singleton_model(|_| WorkspaceRegistry::new());
}

fn mock_server_metadata() -> ServerMetadata {
    ServerMetadata {
        uid: ServerId::default(),
        revision: Revision::now(),
        metadata_last_updated_ts: Utc::now().into(),
        trashed_ts: None,
        folder_id: None,
        is_welcome_object: false,
        creator_uid: None,
        last_editor_uid: None,
        current_editor_uid: None,
    }
}

fn mock_server_permissions() -> ServerPermissions {
    ServerPermissions {
        space: Owner::mock_current_user(),
        guests: Vec::new(),
        anyone_link_sharing: None,
        permissions_last_updated_ts: Utc::now().into(),
    }
}

fn create_server_conversation_metadata(
    title: &str,
    server_token: &str,
) -> ServerAIConversationMetadata {
    ServerAIConversationMetadata {
        title: title.to_string(),
        working_directory: None,
        harness: AIAgentHarness::Oz,
        usage: ConversationUsageMetadata {
            was_summarized: false,
            context_window_usage: 0.0,
            credits_spent: 0.0,
            platform_credits_spent: 0.0,
            total_provider_cost_in_cents: None,
            credits_spent_for_last_block: None,
            charged_usage_for_last_block: None,
            total_charged_usage: None,
            token_usage: vec![],
            tool_usage_metadata: Default::default(),
            context_window_segments: Vec::new(),
        },
        metadata: mock_server_metadata(),
        creator: None,
        permissions: mock_server_permissions(),
        ambient_agent_task_id: None,
        server_conversation_token: ServerConversationToken::new(server_token.to_string()),
        artifacts: Vec::new(),
    }
}

#[test]
fn test_get_entries_includes_local_only_entry() {
    App::test((), |mut app| async move {
        add_entry_projection_test_models(&mut app);

        let conversation_id = AIConversationId::new();
        let mut model = create_test_model();
        model.conversations.insert(
            conversation_id,
            create_test_conversation_metadata(conversation_id, "Local conversation"),
        );

        app.update(|ctx| {
            let entries = model.get_entries(ctx);

            assert_eq!(entries.len(), 1);
            let entry = &entries[0];
            assert_eq!(
                entry.id,
                AgentConversationEntryId::Conversation(conversation_id)
            );
            assert_eq!(entry.identity.local_conversation_id, Some(conversation_id));
            assert_eq!(
                entry.provenance,
                AgentConversationProvenance::LocalInteractive
            );
            assert_eq!(entry.display.title, "Local conversation");
        });
    });
}

#[test]
fn test_get_entries_includes_cloud_metadata_only_entry() {
    App::test((), |mut app| async move {
        let token = "cloud-token-only";
        add_entry_projection_test_models(&mut app);
        BlocklistAIHistoryModel::handle(&app).update(&mut app, |model, _| {
            model.merge_cloud_conversation_metadata(vec![create_server_conversation_metadata(
                "Cloud conversation",
                token,
            )]);
        });

        let model = create_test_model();

        app.update(|ctx| {
            let entries = model.get_entries(ctx);

            assert_eq!(entries.len(), 1);
            let entry = &entries[0];
            assert_eq!(
                entry
                    .identity
                    .server_conversation_token
                    .as_ref()
                    .map(|t| t.as_str()),
                Some(token)
            );
            assert_eq!(
                entry.provenance,
                AgentConversationProvenance::CloudSyncedConversation
            );
            assert!(entry.backing.has_cloud_data);
            assert!(!entry.backing.has_loaded_conversation);
            assert!(!entry.backing.has_local_persisted_data);
        });
    });
}

#[test]
fn test_resolve_open_action_handles_server_token_subject_without_entry() {
    App::test((), |mut app| async move {
        add_entry_projection_test_models(&mut app);
        app.add_singleton_model(|_| create_test_model());

        let server_token = ServerConversationToken::new("server-token-subject".to_string());
        app.update(|ctx| {
            let action = AgentConversationsModel::resolve_open_action(
                AgentConversationNavigationSubject::ServerToken(server_token.clone()),
                None,
                ctx,
            );

            assert!(matches!(
                action,
                Some(WorkspaceAction::OpenConversationTranscriptViewer { conversation_id }) if conversation_id == server_token
            ));
        });
    });
}

#[test]
fn test_resolve_open_action_opens_metadata_only_cloud_conversation_by_server_token() {
    App::test((), |mut app| async move {
        let token = "metadata-only-token";
        add_entry_projection_test_models(&mut app);
        BlocklistAIHistoryModel::handle(&app).update(&mut app, |model, _| {
            model.merge_cloud_conversation_metadata(vec![create_server_conversation_metadata(
                "Cloud conversation",
                token,
            )]);
        });
        app.add_singleton_model(|_| create_test_model());

        app.update(|ctx| {
            let entries = AgentConversationsModel::as_ref(ctx).get_entries(ctx);
            let entry = entries
                .iter()
                .find(|entry| {
                    entry
                        .identity
                        .server_conversation_token
                        .as_ref()
                        .is_some_and(|server_token| server_token.as_str() == token)
                })
                .expect("metadata-only cloud entry should exist");

            assert!(entry.backing.has_cloud_data);
            assert!(!entry.backing.has_loaded_conversation);
            assert!(!entry.backing.has_local_persisted_data);

            let action = AgentConversationsModel::resolve_open_action(
                AgentConversationNavigationSubject::Entry(entry.id),
                None,
                ctx,
            );

            assert!(matches!(
                action,
                Some(WorkspaceAction::OpenConversationTranscriptViewer { conversation_id }) if conversation_id.as_str() == token
            ));
        });
    });
}

#[test]
fn local_conversation_sync_finishes_initial_load_without_starting_cloud_load() {
    App::test((), |mut app| async move {
        let _interactive_management_guard =
            FeatureFlag::InteractiveConversationManagementView.override_enabled(true);
        add_entry_projection_test_models(&mut app);
        let model = app.add_singleton_model(|_| create_test_model());

        model.update(&mut app, |model, ctx| model.sync_conversations(ctx));

        model.read(&app, |model, _| {
            assert_eq!(
                model.initial_load_state,
                InitialConversationLoadState::WaitingForCloud
            );
        });
    });
}
