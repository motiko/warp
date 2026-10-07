# Frequently Asked Questions

## Why does Alefbet exist?

Warp's client is a well-built terminal, but it is designed around an account, a cloud backend,
and telemetry. Alefbet keeps the terminal and removes the rest: it is for people who want a
modern terminal that works entirely on their own machine.

## What is the relationship to Warp?

Alefbet is a hard fork of the open-source Warp repository
(https://github.com/warpdotdev/warp). It is an independent project, not affiliated with or
endorsed by Warp. The code is used under the licenses Warp published it under (AGPL v3 for the
app, MIT for the UI framework crates), and the upstream copyright notices are retained.

## What was removed?

- Login, account creation, and the onboarding flow. The app starts directly in the terminal.
- All server communication. The build carries no server configuration, and every network client
  bails out before opening a socket.
- Telemetry, crash reporting, auto-update, and the changelog fetch.
- Cloud features such as Drive sync, session sharing, and hosted agents.
- The upstream release pipeline, repository automation, and agent tooling.

AI features are off by default. Supporting a user-supplied model or provider is planned.

## Where does Alefbet keep its data?

Settings, state, and the local database live under `~/.alefbet`. The log file is `alefbet.log`
(on macOS, `~/Library/Logs/alefbet.log`). Nothing is sent anywhere.

## How do I build it?

```bash
./script/bootstrap --skip-gcloud-auth
./script/run
```

`./script/bundle -c oss --nosign --nouniversal` produces an unsigned `.app` and `.dmg` under
`target/`. See [README.md](README.md) and [AGENTS.md](AGENTS.md) for details.

## Which platforms are supported?

macOS first. The Linux and Windows code paths are still in the tree and CI keeps them compiling,
but they are not yet a supported target for Alefbet.

## How do I report a bug or a security problem?

Open a GitHub issue for bugs and feature requests. For security problems, use the private
reporting channel described in [SECURITY.md](SECURITY.md).
