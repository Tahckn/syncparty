# Contributing

Thanks for taking a look. Issues and pull requests are both welcome.

## Getting set up

You need [Rust](https://rustup.rs), Node 20+, pnpm, and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your
platform (MSVC Build Tools on Windows, Xcode command line tools on macOS).

```bash
pnpm install
pnpm tauri dev
```

## Before you open a pull request

```bash
cd src-tauri
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

```bash
pnpm build   # runs tsc, so this is the frontend type check too
```

CI runs all of the above on Windows and macOS.

## How the code is arranged

The rule that matters: **`core` must not depend on Tauri.**

```
src-tauri/src/
  ipc/     Tauri commands and the event bridge — thin, delegates to core
  core/    everything else, testable with plain `cargo test`
```

If you find yourself importing `tauri::` inside `core`, the logic belongs
somewhere else, or the dependency needs to go behind a trait. `EventBus` and
`ProgressSink` in `core/events.rs` exist for exactly that reason.

Concretely:

- **A new external program syncparty depends on** — implement `Dependency`
  in `core/deps/`, register it in `DependencyManager::standard`. Give it a
  working `manual_url()`; a user must never hit a dead end.
- **A new backend capability** — put the logic in `core/`, add a one-line
  handler in `ipc/commands.rs`, register it in the `generate_handler!` list.
- **A new thing the UI needs to be told about** — add a variant to `AppEvent`.

## Generated types

`src/shared/types/*.ts` is generated from the Rust types by `ts-rs` whenever
`cargo test` runs. Do not edit those files; change the Rust type and re-run
the tests. CI fails if they drift.

## Working on the protocol

`core/syncplay/protocol.rs` mirrors Syncplay's wire format. A mismatch there
fails quietly — the monitor connects and simply shows nothing — so please
check any change against the
[Syncplay source](https://github.com/Syncplay/syncplay) rather than inferring
the shape, and add a test with a real captured message.

Two properties the tests guard, both worth keeping:

- The monitor never sends `playstate`. If it did, it could pause, unpause or
  seek everybody's film.
- The server password and salt never appear in `argv`.

## Adding strings

Add the key to `en` in `src/shared/i18n/messages.ts` first. `Messages` is
derived from it, so TypeScript will then tell you the Turkish translation is
missing.
