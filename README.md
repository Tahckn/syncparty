# syncparty

Synchronised movie nights over [Tailscale](https://tailscale.com). One app for
the person hosting and the people joining.

syncparty runs a private [Syncplay](https://syncplay.pl) server bound to your
Tailscale address, then hands your friends a single link. They click it and land
in the room — no address to copy, no password to retype, nothing exposed to the
public internet.

> **Nothing is streamed.** Everyone plays their own local copy of the file;
> syncparty only keeps playback in sync.

## What it does

**For the host**

- Brings Tailscale up, starts the Syncplay server and generates the invite in
  one action
- Live room panel showing real nicknames, the file each person has open and
  who is ready — pushed by the server, not polled
- Warns when two people opened *different* files, which is the usual reason a
  movie night desyncs
- Optional Discord announcement with the join link

**For the guest**

- Paste an invite code, or just click the `syncparty://` link
- Checks for Tailscale, Syncplay and mpv, and installs whatever is missing
- One button to join

## Install

Grab the latest build from [Releases](https://github.com/Tahckn/syncparty/releases).

Releases are currently unsigned, so Windows SmartScreen and macOS Gatekeeper
will both warn on first run.

Everything else — Tailscale, the Syncplay client, mpv, and the Python
environment the server needs — is detected on first launch and installed for
you if it is missing.

## How it works

```
Host machine                                Guest machine
┌────────────────────────┐                  ┌────────────────────────┐
│ syncparty              │                  │ syncparty              │
│  ├─ Syncplay server ───┼── Tailscale ─────┼─→ Syncplay client      │
│  │   (tailnet IP only) │   (WireGuard)    │      └─ mpv            │
│  └─ room monitor       │                  │                        │
└────────────────────────┘                  └────────────────────────┘
```

The server binds to the machine's Tailscale IPv4 address rather than
`0.0.0.0`, so it is unreachable from the local network and from the internet.
All traffic rides the existing WireGuard tunnel.

## Design notes

A few decisions worth knowing about:

- **Portable builds can be pointed at by hand.** Detection covers installers
  and `PATH`, which misses an mpv or Syncplay zip extracted to some folder.
  Rather than guessing at where people keep those, the setup screen has a
  "Locate…" button — give it the program or the folder holding it. A location
  that turns out not to work is rejected rather than saved.
- **The room panel attaches a real Syncplay client.** The server has no admin
  API, so the only way to learn nicknames and open files is to be a
  participant. It appears in everyone's user list as `syncparty-panel`, and can
  be switched off in Settings at the cost of the detail it provides.
- **Secrets never touch the command line.** The server password and salt reach
  Syncplay through `SYNCPLAY_PASSWORD` and `SYNCPLAY_SALT`, so they stay out of
  the process table. They are stored in Windows Credential Manager or the macOS
  Keychain.
- **The salt is generated once and kept.** Syncplay derives room operator
  passwords from it; a new salt on every start would silently invalidate them.
- **Stopping a party does not stop Tailscale.** Your tailnet is used for other
  things.

## Building from source

Requires [Rust](https://rustup.rs), Node 20+, pnpm, and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform.

```bash
pnpm install
pnpm tauri dev
```

Run the backend tests:

```bash
cd src-tauri && cargo test
```

TypeScript types under `src/shared/types` are generated from the Rust types by
`ts-rs` when the tests run — do not edit them by hand.

## Layout

```
src/                     React frontend
  features/              onboarding · host · guest · settings
  shared/                ipc wrappers, generated types, UI primitives
src-tauri/
  src/ipc/               Tauri commands and the event bridge
  src/core/              all logic, no Tauri dependency
    deps/                dependency detection and installation
    tailscale/           tailnet status and addresses
    syncplay/            protocol, server process, room monitor, launcher
    invite/              invite codes and deep links
    session/             the host/guest state machine
```

`core` never imports from `ipc`, which is what lets the whole of it run under
`cargo test` without a webview.

## Licence

MIT — see [LICENSE](LICENSE).

syncparty is not affiliated with Syncplay or Tailscale.
