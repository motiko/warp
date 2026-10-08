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
- `cargo check -p <package>` is the fastest feedback loop while editing.
- `./script/format --check` verifies formatting. The formatter runs `cargo fmt` with
  `imports_granularity=Module` and `group_imports=StdExternalCrate` under `RUSTC_BOOTSTRAP=1`, so
  plain `cargo fmt` will not produce the same output.
- `./script/check_no_inline_test_modules` fails on any inline `mod tests {` block (enforces the
  `_tests.rs` convention from AGENTS.md).
- Toolchain is pinned in `rust-toolchain.toml` (currently 1.92.0).
- Nextest config lives in `.config/nextest.toml`: tests are marked slow at 30s and killed at 60s;
  integration tests count double against parallelism.

## Channel binaries

`app/` defines two binaries in `app/src/bin/`: `alefbet` (`oss.rs`, the OSS channel) and
`integration` (`integration.rs`, launched by the GUI integration test harness). Each `main` builds a
`ChannelState` from an inline `ChannelConfig` (`WarpServerConfig::offline()` / `OzConfig::offline()`
for `alefbet`, so the build has no server to talk to) and calls `warp::run()`. `./script/run` always
builds `alefbet` / channel `oss`.

## Architecture: how the pieces fit

### Startup and view ownership (GUI)

- `app/src/lib.rs`: `run()` builds the `warpui::platform::AppBuilder`; `initialize_app()` calls each
  module's `init` (settings, experiments, ai, app_services, workspace, ...); `launch()` loads custom
  keybindings and dispatches the global action `root_view:open_from_restored` (falling back to
  `root_view:open_new`).
- Ownership chain: `RootView` (`app/src/root_view.rs`) → `Workspace` (`app/src/workspace/view.rs`, one
  per window, owns `tabs: Vec<TabData>`) → `TabData` (`app/src/tab.rs`) → `PaneGroup`
  (`app/src/pane_group/mod.rs`, a split tree of `Box<dyn AnyPaneContent>` keyed by `PaneId`) → pane
  kinds in `app/src/pane_group/pane/` (terminal, code, file, settings, ...) →
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
  keystrokes or `"none"`) loaded by `app/src/keyboard.rs`.

### Terminal layers

- `crates/warp_terminal` is the low-level emulator: an Alacritty-derived parser/grid under `model/`,
  plus the native PTY layer in `local_tty/` (not built for wasm).
- `app/src/terminal` builds Warp's block model on top: `model/` (`TerminalModel`, `BlockList`), several
  `TerminalManager` implementations wiring the model to I/O (`local_tty/`, `remote_tty/`,
  `shared_session/`), and the renderers (`block_list_element.rs`, `grid_renderer*`).

### Opt-in AI and network

- There is no Warp server, account or agent. `alefbet` uses `WarpServerConfig::offline()`; the
  server transport, auth, Drive, teams, billing and Agent Mode were deleted in M3. Do not add code
  that talks to `*.warp.dev`.
- What remains for the M5 opt-in AI: `AISettings::is_any_ai_enabled` (`app/src/settings/ai.rs`, a
  local setting, off by default), custom endpoint types and `LLMId` in `crates/ai_config`, endpoint
  storage in `app/src/ai/custom_endpoints.rs`, the "AI (optional)" settings page
  (`settings_view/ai_page.rs`), and the `#` natural-language command search, which asks a
  `NaturalLanguageCommandProvider` (`app/src/search/command_search/natural_language.rs`) that has no
  implementation yet. `crates/input_classifier` and `crates/natural_language_detection` are local
  heuristics kept for that work.
- Outbound HTTP for user-triggered downloads (LSP servers, Node runtime) goes through
  `crates/http_client`.

### Settings

- A setting is declared with `define_setting!` / `define_settings_group!` from `crates/settings`
  (`macros.rs`) in `app/src/settings/*.rs` and registered in `app/src/settings/init.rs`. Each
  declaration names `type`, `default`, `supported_platforms`, `sync_to_cloud`, `surface`,
  `private`, `toml_path`, and `description`.
- Values serialize through the `SettingsValue` trait (`crates/settings_value`, derive in
  `settings_value_derive`). Public settings persist to the user's `settings.toml`; private ones to
  platform-native prefs.
- Remember the AGENTS.md rule: a toggleable setting also needs a Command Palette entry.

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
  (`safe:` text always logs, `full:` text only on dogfood). `report_error!` from `crates/warp_errors`
  is the error-reporting macro (Sentry only in builds with the `crash_reporting` feature).

## Contribution workflow (from CONTRIBUTING.md)

- Branch from `master`; branch names are prefixed with your handle (e.g. `alice/fix-parser`).
- Run the targeted validation from AGENTS.md (relevant `cargo nextest` tests, `cargo clippy` for
  the touched crates, `./script/format`); CI runs the full workspace checks.
- PRs use `.github/pull_request_template.md`, need proof of manual testing (screenshots, or a
  recording for interactive changes), and a changelog line using the prefixes in AGENTS.md
  (`CHANGELOG-NONE` opts out for refactors/CI). Linking an issue is welcome but not required.
- There is no CLA, no spec requirement, and no issue-label gating.

## Licensing

`warpui_core` and `warpui` are MIT; everything else is AGPL v3.
