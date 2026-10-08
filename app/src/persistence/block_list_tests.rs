use chrono::{Duration, Local};
use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;
use diesel::{Connection, RunQueryDsl};
use diesel_migrations::MigrationHarness;

use super::{create_block, get_all_restored_blocks, save_block};
use crate::app_state::PaneUuid;
use crate::persistence::schema;
use crate::terminal::model::block::SerializedBlock;

const PANE_UUID: &[u8] = &[1, 2, 3, 4];

/// Builds an in-memory SQLite database with all migrations applied and a single terminal pane.
fn test_connection() -> SqliteConnection {
    let mut conn =
        SqliteConnection::establish(":memory:").expect("in-memory sqlite connection should open");
    conn.run_pending_migrations(::persistence::MIGRATIONS)
        .expect("migrations should run");
    conn.batch_execute(
        "INSERT INTO windows (id, active_tab_index, quake_mode, fullscreen_state)
             VALUES (1, 0, 0, 0);
         INSERT INTO tabs (id, window_id, pinned) VALUES (1, 1, 0);
         INSERT INTO pane_nodes (id, tab_id, is_leaf) VALUES (1, 1, 1);
         INSERT INTO pane_leaves (pane_node_id, kind, is_focused) VALUES (1, 'terminal', 1);
         INSERT INTO terminal_panes (id, kind, uuid, is_active)
             VALUES (1, 'terminal', x'01020304', 1);",
    )
    .expect("terminal pane rows should insert");
    conn
}

fn completed_block(command: &str, minutes_ago: i64) -> SerializedBlock {
    let mut block = SerializedBlock::new_for_test(command.as_bytes().to_vec(), Vec::new());
    let ts = Local::now() - Duration::minutes(minutes_ago);
    block.start_ts = Some(ts);
    block.completed_ts = Some(ts);
    block.id = command.to_owned().into();
    block
}

/// Inserts a block row as a build that still had agent features would have written it.
fn insert_legacy_block(
    conn: &mut SqliteConnection,
    block: &SerializedBlock,
    ai_metadata: Option<&String>,
    agent_view_visibility: Option<&str>,
) {
    let mut row = create_block(PANE_UUID.to_vec(), block, true);
    row.ai_metadata = ai_metadata;
    row.agent_view_visibility = agent_view_visibility.map(str::to_owned);
    diesel::insert_into(schema::blocks::dsl::blocks)
        .values(row)
        .execute(conn)
        .expect("legacy block should insert");
}

#[test]
fn restore_skips_blocks_from_agent_conversations() {
    let mut conn = test_connection();

    save_block(
        &mut conn,
        PANE_UUID.to_vec(),
        &completed_block("ls", 4),
        true,
    )
    .expect("block should save");
    let ai_metadata = r#"{"conversation_id":"c","has_agent_written_to_block":false}"#.to_owned();
    insert_legacy_block(
        &mut conn,
        &completed_block("agent-ran", 3),
        Some(&ai_metadata),
        None,
    );
    insert_legacy_block(
        &mut conn,
        &completed_block("in-agent-view", 2),
        None,
        Some(r#"{"Agent":{"origin_conversation_id":"c"}}"#),
    );
    insert_legacy_block(
        &mut conn,
        &completed_block("attached-as-context", 1),
        None,
        Some(r#"{"Terminal":{"pending_conversation_ids":[],"conversation_ids":["c"]}}"#),
    );

    let restored = get_all_restored_blocks(&mut conn).expect("blocks should load");
    let commands: Vec<_> = restored[&PaneUuid(PANE_UUID.to_vec())]
        .iter()
        .map(|block| String::from_utf8_lossy(&block.stylized_command).into_owned())
        .collect();
    assert_eq!(commands, vec!["ls", "attached-as-context"]);
}
