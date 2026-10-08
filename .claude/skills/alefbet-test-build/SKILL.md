---
name: alefbet-test-build
description: Build a clickable Alefbet.app/.dmg, smoke-test startup, and hand the user a manual test checklist. Use at each milestone test point or when the user asks to try the current state.
---

# Alefbet test build

Never touch the user's live Warp OSS install: `/Applications/WarpOss.app`, `~/.warp-oss`,
`~/Library/Logs/warp-oss.log`, the `dev.warp.WarpOss` prefs, port 9282. Do not install into
`/Applications` yourself.

1. Smoke test (debug):
   ```sh
   cargo build --bin alefbet
   : > ~/Library/Logs/alefbet.log
   WARP_DATA_PROFILE=smoke target/debug/alefbet & pid=$!
   ```
   After ~20 s: `lsof -nP -a -p $pid -i` (expect no remote connections), then `kill $pid` and
   `grep -c panicked ~/Library/Logs/alefbet.log` (expect 0).
2. Bundle: `./script/bundle -c oss --nosign --nouniversal` (long; run in background). Output:
   `target/release-lto/bundle/osx/Alefbet.app` and the `.dmg` next to it.
3. Tell the user the commit hash, the paths, and how to open it:
   `open target/release-lto/bundle/osx/Alefbet.app` (unsigned: right-click → Open the first time).
4. Give this checklist:
   - Opens straight to a terminal; no login, onboarding or AI promo appears.
   - New tab, split panes, close/undo-close tab.
   - Tab completion (e.g. `git checkout <Tab>`), history autosuggestion (→ to accept).
   - Open a file in the editor pane; open the git diff / code review view in a repo.
   - Workflows (Ctrl-Shift-R): bundled and local YAML workflows.
   - `#` in the input: NL command search shows "Turn on AI and add a model in Settings > AI (optional)".
   - Settings: no Account, Billing, Teams, Drive or Agent pages; "AI (optional)" is off.
   - `lsof -nP -i | grep -i alefbet` shows nothing while idle.
