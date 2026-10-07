# Alefbet

Alefbet is a fast, modern terminal built on the open-source code of
[Warp](https://github.com/warpdotdev/warp), with the cloud taken out.

- **Offline by default.** The app is built with no server configured and never opens a
  connection to one. There is no account, no login screen, and no onboarding flow: it starts in
  the terminal.
- **No telemetry, no crash reporting.** Nothing about your usage leaves your machine.
- **No cloud.** Drive sync, session sharing, hosted agents, and auto-update are not part of the
  build.
- **Your own AI, later.** AI features are off in the default build. Letting you plug in a model
  or provider of your own choosing is planned.

## Status

Alefbet is an early fork. macOS is the first target; Linux and Windows builds come later. Expect
rough edges.

## Building and running

The Rust toolchain is pinned in `rust-toolchain.toml`, so `rustup` picks it up automatically.

```bash
./script/bootstrap --skip-gcloud-auth        # one-time platform setup
./script/run                                 # build and launch a debug build
./script/bundle -c oss --nosign --nouniversal   # unsigned release .app and .dmg under target/
```

Settings and local state live in `~/.alefbet`. The log file is `alefbet.log` (on macOS,
`~/Library/Logs/alefbet.log`).

See [AGENTS.md](AGENTS.md) for the engineering guide (build, test, lint, code style),
[CONTRIBUTING.md](CONTRIBUTING.md) for how to send changes, and [FAQ.md](FAQ.md) for the
background on the fork.

## License

The application and most crates are licensed under the [AGPL v3](LICENSE-AGPL). The UI
framework crates `crates/warpui` and `crates/warpui_core` are licensed under the
[MIT license](LICENSE-MIT).

## Attribution

Alefbet is based on [Warp](https://github.com/warpdotdev/warp), copyright Denver Technologies,
Inc., and used under the terms of its open-source licenses. Alefbet is not affiliated with or
endorsed by Warp. "Warp" is a trademark of Denver Technologies, Inc.
