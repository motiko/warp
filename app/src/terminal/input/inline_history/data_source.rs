//! Data source for the inline history menu.
//!
//! Ordering semantics match the legacy up-arrow history menu:
//! - Items from different sessions appear before items from the current session
//! - Within each group, items are sorted by timestamp (oldest first)
//! - Commands are deduplicated, keeping the most recent occurrence
//! - The result is that current session items appear at the bottom (closer to input)

use chrono::Local;
use ordered_float::OrderedFloat;
use warpui::{AppContext, Entity, ModelHandle, SingletonEntity};

use crate::search::SyncDataSource;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::terminal::history::{History, LinkedWorkflowData};
use crate::terminal::input::inline_history::search_item::InlineHistoryItem;
use crate::terminal::input::inline_menu::{
    InlineMenuAction, InlineMenuClickBehavior, InlineMenuType,
};
use crate::terminal::model::session::active_session::ActiveSession;

#[derive(Clone, Debug)]
pub struct AcceptHistoryItem {
    pub command: String,
    pub linked_workflow_data: Option<LinkedWorkflowData>,
}

impl InlineMenuAction for AcceptHistoryItem {
    const MENU_TYPE: InlineMenuType = InlineMenuType::InlineHistoryMenu;

    fn click_behavior(&self) -> InlineMenuClickBehavior {
        InlineMenuClickBehavior::SelectOnClick
    }
}

/// Data source that provides the command history of the active session.
pub struct InlineHistoryMenuDataSource {
    active_session: ModelHandle<ActiveSession>,
}

impl InlineHistoryMenuDataSource {
    pub fn new(active_session: ModelHandle<ActiveSession>) -> Self {
        Self { active_session }
    }
}

impl SyncDataSource for InlineHistoryMenuDataSource {
    type Action = AcceptHistoryItem;

    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        let trimmed_query = query.text.trim();
        let prefix_match_len = trimmed_query.len();
        let session_id = self.active_session.as_ref(app).session(app).map(|s| s.id());

        let mut results: Vec<QueryResult<AcceptHistoryItem>> = Vec::new();
        for suggestion in History::handle(app)
            .as_ref(app)
            .up_arrow_suggestions(session_id, app)
        {
            let command = suggestion.normalized_text();
            if !trimmed_query.is_empty() && !command.starts_with(trimmed_query) {
                continue;
            }

            let search_item = InlineHistoryItem::new(
                command.to_owned(),
                suggestion.entry.linked_workflow_data(),
                suggestion.entry.start_ts.unwrap_or_else(Local::now),
            )
            .with_prefix_match_len(prefix_match_len)
            .with_score(OrderedFloat(results.len() as f64));
            results.push(QueryResult::from(search_item));
        }

        Ok(results)
    }
}

impl Entity for InlineHistoryMenuDataSource {
    type Event = ();
}
