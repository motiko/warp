---
name: alefbet-validate
description: Full validation gate for an Alefbet fork step (tests, clippy, format, inline-test check, lockfile). Use before declaring a milestone step done or building a test bundle for the user.
---

# Alefbet validation gate

Order follows AGENTS.md. Run long commands in the background and wait on them; fix failures and
rerun only the affected stage.

1. Tests:
   `cargo nextest run --workspace --no-fail-fast -E 'not test(/test_ssh_/)' 2>&1 | tail -40`
   - The 5 `test_ssh_*` tests need gcloud and are excluded.
   - `ui_tests::test_undo_close_stack_timeout_cleanup` is a known flake: rerun it alone before
     treating it as a failure.
   - Record the pass count; report it.
2. Clippy: `cargo clippy --workspace --all-targets --tests -- -D warnings 2>&1 | grep -E '^(error|warning)' | sort | uniq -c | head`
3. Format once: `./script/format`, then `./script/format --check`.
4. `./script/check_no_inline_test_modules`
5. `cargo metadata --locked --format-version 1 > /dev/null`
6. LOC for the progress note: `find app crates -name '*.rs' | xargs cat | wc -l`.

If formatting changed files, follow with the `alefbet-checkpoint` skill to commit them.
