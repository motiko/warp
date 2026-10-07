# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

`AGENTS.md` above is the primary engineering guide (build/run/test commands, validation order, coding
style, comment rules, feature flags, terminal-model locking, test-file conventions). Everything below is
supplementary and must not contradict it.

## Commands not covered in AGENTS.md

- Run a single test: `cargo nextest run -p <package> <test_name_substring>` or with a filter
  expression, e.g. `cargo nextest run -p warp -E 'test(prefix_tests)'`. `-p warp` is the GUI app crate
  (`app/`).
- `cargo check -p <package>` is the fastest feedback loop while editing. Note `cargo clippy -p warp`
  lints the GUI configuration; `cargo clippy --workspace` additionally unifies the `tui` feature
  through `warp_tui`, so both are run by presubmit.
- `./script/format --check` verifies formatting. The formatter runs `cargo fmt` with
  `imports_granularity=Module` and `group_imports=StdExternalCrate` under `RUSTC_BOOTSTRAP=1`, so
  plain `cargo fmt` will not produce the same output.
- `./script/check_no_inline_test_modules` fails on any inline `mod tests {` block (enforces the
  `_tests.rs` convention from AGENTS.md).
- Toolchain is pinned in `rust-toolchain.toml` (currently 1.92.0).
- Nextest config lives in `.config/nextest.toml`: tests are marked slow at 30s and killed at 60s;
  integration tests count double against parallelism.

## Channel binaries

`app/` defines one binary per release channel in `app/src/bin/` (`stable`, `preview`, `dev`, `local`,
`oss`, `integration`). Each `main` sets `ChannelState` from `warp_channel_config::load_config!` and calls
`warp::run()`. `./script/run` builds `warp` (local channel) when the internal `warp-channel-config`
generator is on `PATH`, otherwise `alefbet` (the OSS channel, built from `src/bin/oss.rs`);
`./script/run-tui` does the same for `warp-tui` / `warp-tui-oss` and always enables the `standalone`
cargo feature so bundled resources resolve next to the binary.

## Architecture: how the pieces fit

### Startup and view ownership (GUI)

- `app/src/lib.rs`: `run()` builds the `warpui::platform::AppBuilder`; `initialize_app()` calls each
  module's `init` (settings, experiments, ai, app_services, workspace, ...); `launch()` loads custom
  keybindings and dispatches the global action `root_view:open_from_restored` (falling back to
  `root_view:open_new`).
- Ownership chain: `RootView` (`app/src/root_view.rs`) → `Workspace` (`app/src/workspace/view.rs`, one
  per window, owns `tabs: Vec<TabData>`) → `TabData` (`app/src/tab.rs`) → `PaneGroup`
  (`app/src/pane_group/mod.rs`, a split tree of `Box<dyn AnyPaneContent>` keyed by `PaneId`) → pane
  kinds in `app/src/pane_group/pane/` (terminal, code, notebook, settings, ai_document, ...) →
  `TerminalPane` wraps a `TerminalView` whose `TerminalManager` exposes
  `Arc<FairMutex<TerminalModel>>`.
- Session restore mirrors that tree in `app/src/app_state.rs` (`AppState` → `WindowSnapshot` →
  `TabSnapshot` → `PaneNodeSnapshot` → `LeafContents`). Adding a new pane kind usually means touching
  both the pane tree and the snapshot types.

### warpui entities, actions, keybindings

- `Entity` and `View: Entity` live in `crates/warpui_core/src/core/`. Models come from `ctx.add_model`
  / `add_singleton_model` (`ModelHandle<T>`; singletons fetched via `T::handle(ctx)`); views from
  `add_view` / `add_typed_action_view` (`ViewHandle<T>`). Views observe models with
  `subscribe_to_model`.
- There is no `actions!` macro. A view implements `TypedActionView { type Action; fn handle_action }`
  with an enum implementing `Action`; dispatch with `ctx.dispatch_typed_action(...)`, which bubbles to
  ancestors. String-named global actions (`"workspace:..."`, `"root_view:..."`) use
  `add_global_action` / `dispatch_global_action`.
- Keybindings are registered in code inside each module's `init`, via
  `register_editable_bindings([EditableBinding::new("workspace:crash", desc, Action)
  .with_context_predicate(id!("Workspace"))])` and `register_fixed_bindings`. The matcher is
  `crates/warpui_core/src/keymap/`. User overrides come from a `keybindings.yaml` (binding name →
  keystrokes or `"none"`) loaded by `app/src/keyboard.rs`. The TUI has its own table in
  `crates/warp_tui/src/keybindings.rs`.

### Terminal layers

- `crates/warp_terminal` is the low-level emulator: an Alacritty-derived parser/grid under `model/`,
  plus the native PTY layer in `local_tty/` (not built for wasm).
- `app/src/terminal` builds Warp's block model on top: `model/` (`TerminalModel`, `BlockList`), several
  `TerminalManager` implementations wiring the model to I/O (`local_tty/`, `remote_tty/`,
  `shared_session/`), and the renderers (`block_list_element.rs`, `grid_renderer*`).

### AI and server transport

- `crates/ai_types` (plain types) → `crates/ai` (UI-agnostic agent, codebase index, skills, providers)
  → `app/src/ai` (UI and orchestration: agent_sdk, ambient agents, MCP UI, execution profiles).
  `crates/mcp` is the MCP client (rmcp); `crates/computer_use` is per-OS screen control.
- Agent requests go through `app/src/ai/agent/api/impl.rs`, which builds a protobuf request (from the
  `warp-proto-apis` git dependency) and streams responses with
  `warp_multi_agent_client::generate_multi_agent_output` over SSE. Everything else uses the
  `ServerApi` wrapper in `app/src/server/server_api.rs` on top of `crates/warp_server_client`
  (`BaseClient`), with GraphQL via `crates/graphql` and subscriptions via `crates/websocket`.

### Settings

- A setting is declared with `define_setting!` / `define_settings_group!` from `crates/settings`
  (`macros.rs`) in `app/src/settings/*.rs` and registered in `app/src/settings/init.rs`. Each
  declaration names `type`, `default`, `supported_platforms`, `sync_to_cloud`, `surface` (GUI/TUI),
  `private`, `toml_path`, and `description`.
- Values serialize through the `SettingsValue` trait (`crates/settings_value`, derive in
  `settings_value_derive`). Public settings persist to the user's `settings.toml`; private ones to
  platform-native prefs. `cloud_preferences_syncer.rs` pushes `SyncToCloud` settings as Drive objects.
- Remember the AGENTS.md rule: a toggleable setting also needs a Command Palette entry.

### Cloud objects (Drive)

Layered crates: `cloud_objects` (ids, metadata, sharing primitives) → `cloud_object_models` (concrete
object types) → `cloud_object_persistence` (SQLite helpers) → `cloud_object_client` (`ObjectClient`
trait over GraphQL, `#[automock]` under `test-util`). In the app, `app/src/cloud_object/model/` holds
`CloudModel`, `app/src/server/cloud_objects/update_manager.rs` plus `sync_queue.rs` push local changes,
and `listener.rs` applies server pushes from the websocket subscription.

### Platform backends

`crates/warpui/src/platform/` has `mac/` (native Cocoa/objc), `linux/` and `windows/` (winit-based),
`wasm/`, and `headless/`. Wasm is selected with `cfg(target_family = "wasm")` and excludes `local_tty`,
the MCP runtime, and the CLI. App-level platform code is in `app/src/platform/`.

### Testing infrastructure

- Unit tests that need an app use `App::test((), |mut app| async move { ... })`
  (`crates/warpui_core/src/core/app.rs`). App-level helpers (`add_window_with_terminal`, `VirtualFS`,
  `assert_eventually!`) are in `app/src/test_util/`.
- GUI integration tests: framework in `crates/integration/src/` (`Builder`, `TestStep`), test
  definitions in `crates/integration/src/test/`, nextest harness in
  `crates/integration/tests/integration.rs`, app-side steps/assertions in
  `app/src/integration_testing/`. Read `crates/integration/tests/INTEGRATION_TESTING.md` first.
- Logging: `safe_info!` / `safe_warn!` / `safe_error!` etc. are in `crates/warp_core/src/safe_log.rs`
  (`safe:` text always logs, `full:` text only on dogfood). Sentry reporting requires `report_error!`
  from `crates/warp_errors`; plain `log::error!` does not create Sentry events.

## Contribution workflow (from CONTRIBUTING.md)

- Branch from `master`; branch names are prefixed with your handle (e.g. `alice/fix-parser`).
- PRs must link a GitHub issue labeled `ready-to-implement` (bugs, settled designs) or
  `ready-to-spec` (features). Issues labeled `warp:reserved-internal` are off limits.
- Feature work needs a spec PR first: `specs/GH<issue-number>/product.md` (numbered, testable
  behavior invariants) and `tech.md` (Context, Proposed changes, Testing and validation). See
  `specs/GH408/` for an example. Internal specs use `specs/APP-<n>/`.
- PRs use `.github/pull_request_template.md`, need proof of manual testing (screenshots, or a
  recording for interactive changes), and a changelog line. Besides the prefixes in AGENTS.md, the
  template also accepts `CHANGELOG-OZ`, `CHANGELOG-TUI`, and `CHANGELOG-NONE` (explicit opt-out for
  refactors/CI).
- Review is two-stage: Oz reviews first, then a Warp SME. Comment `/warp-agent-review` to re-request
  Oz (max three times per PR).
- Project skills live in `.agents/skills/` (`.claude/skills` is a symlink to it). Shared skills are
  pinned in `skills-lock.json`; the `/write-product-spec` and `/write-tech-spec` skills become
  available once they are installed via `./script/bootstrap`.

## Licensing

`warpui_core` and `warpui` are MIT; everything else is AGPL v3.
