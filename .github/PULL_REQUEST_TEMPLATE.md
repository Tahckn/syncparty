## What this does

<!-- One or two sentences. Link an issue if there is one. -->

## Why

<!-- What problem this solves, or what prompted it. -->

## Checked

- [ ] `cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test`
      pass in `src-tauri`
- [ ] `pnpm build` passes (this also runs `tsc`)
- [ ] If a Rust type used by the frontend changed, `src/shared/types` was
      regenerated (`cargo test` does this) and committed
- [ ] If this touches `core/syncplay/protocol.rs`, it was checked against the
      [Syncplay source](https://github.com/Syncplay/syncplay), not inferred
