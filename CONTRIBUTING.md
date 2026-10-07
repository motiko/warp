# Contributing to Alefbet

Alefbet is a small fork, so the process is light.

## Workflow

1. Branch from `master`. Prefix the branch with your handle, for example `alice/fix-parser`.
2. Make the change and add tests where they make sense (see [Testing](#testing)).
3. Run the targeted validation described in
   [AGENTS.md](AGENTS.md#implementation-validation-order): the relevant `cargo nextest` tests,
   `cargo clippy` for the crates you touched, and `./script/format`. CI runs the full workspace
   checks.
4. Open a pull request against `master` using the
   [pull request template](.github/pull_request_template.md). Describe what changed and why, and
   include proof of manual testing (screenshots, or a short recording for interactive changes).

There is no CLA, no spec requirement, and no issue-label gating. Linking an issue is welcome but
not required.

## Testing

- Bug fixes should include a regression test that would have caught the bug.
- Non-trivial logic needs unit tests, placed in `*_tests.rs` files as described in
  [AGENTS.md](AGENTS.md#testing).
- User-facing GUI flows can be covered by the integration framework under
  `crates/integration/`.

Run unit tests with `cargo nextest run -p <package>` and the app with `./script/run`.

## Code style

`./script/format --check` and `cargo clippy --workspace --all-targets --tests -- -D warnings`
must pass. The full style guide, including comment rules and terminal-model locking, is in
[AGENTS.md](AGENTS.md).

## Reporting security issues

See [SECURITY.md](SECURITY.md). Do not open public issues for security vulnerabilities.

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
