---
name: alefbet-checkpoint
description: Commit a compiling checkpoint of Alefbet fork work (M3 strip and later milestones) and log it in the plan doc. Use whenever `cargo check -p warp` is green after a sub-step, instead of batching large changes into one commit.
---

# Alefbet checkpoint commit

Commits are checkpoints (AGENTS.md: local commits are not validation boundaries), so commit as soon as
a coherent sub-step compiles. Tests and clippy may still be red; `alefbet-validate` gates "done".

The user authorized commits per milestone ("commit and start m3"). Never push. Never add attribution
lines (no `Co-Authored-By`, no "Generated with") to commit messages.

## Steps

1. Compile: `cargo check -p warp --all-targets 2>&1 | grep -E '^(error|warning: unused)' | head -40`.
   Stop and fix if there are errors.
2. Guard the keep list. All of these must still exist (they back the M5 opt-in AI):
   ```sh
   for p in crates/input_classifier crates/natural_language_detection \
     app/src/settings_view/ai_page.rs app/src/ai/custom_endpoints.rs \
     app/src/search/command_search/natural_language_data_source.rs; do
     [ -e "$p" ] || echo "MISSING keep-list path: $p"; done
   ```
   Restore any missing path from git instead of committing.
3. Guard against new server hosts: `git add -A && git diff --cached -U0 HEAD | grep -E '^\+.*(app|oz|rtc|sessions)\.warp\.dev'`
   must print nothing.
4. Commit: `git commit -q -m "<Milestone> <step>: <imperative summary>"`, e.g.
   `M3 4b.2: Remove the ai and multi-agent crates behind the opt-in facade`. Body: 1–3 lines on what
   was stubbed or kept. Print `git log --oneline -1` and `git diff --shortstat HEAD~1`.
5. Plan doc: update the M3 row note (col 2) of the progress table in the Alefbet plan doc
   (https://claude.ai/artifact/UMmrz1MJpWfK2ocVJYsNnn, Claude Docs project
   `dd8d442a-4449-4cdc-827b-808d15774d33`, table `m71rasezm9b.22089`) with the new short hash and
   step. Read the table first; edit only that cell.
