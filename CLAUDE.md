# nsmpl — notes for Claude

Two-track sample tool and publisher. Tauri 2 · React · WaveSurfer. See
[`nsmpl-introduction.md`](nsmpl-introduction.md).

## Read SUITE.md first

[`../ndisc/SUITE.md`](https://github.com/xjmzx/ndisc/blob/main/SUITE.md) is
authoritative for anything shared across the suite. Read it **before making a
platform-sensitive choice** — audio above all, which is this app's whole
surface. It records constraints invisible on the machine you are working on:
`nchat` shipped Web Audio tones that worked on macOS and were silent on Linux,
a constraint SUITE.md had already recorded.

## Build and verify

```
make dev      # hot reload
make check    # npm run build (tsc + vite) + cargo check
make build    # release
```

Release path is `tauri build`, which runs Vite. **Never `cargo build --release`**.

## Traps specific to this repo

- **All playback goes through WaveSurfer's `HTMLMediaElement`, deliberately.**
  A sample-accurate `AudioBufferSourceNode` with `loop`/`loopStart`/`loopEnd`
  was tried for region loops and abandoned: on WebKit2GTK the audio thread emits
  frames into the destination that never reach the sound card — visible cursor
  motion, audible silence — regardless of keep-alive tricks or user-gesture
  timing. Region loop is a rAF-polled wrap on top of the media element instead,
  costing ~16ms of overshoot at the boundary. Read the comment in
  `src/components/Player.tsx` before "fixing" that.
- **Volume goes through `HTMLMediaElement.volume`** (via `WaveSurfer.setVolume`),
  not a Web Audio gain node. Note WebKitGTK pins media-element volume at ~0.1
  and overwrites what the page sets — `nchat` had to bake levels into its
  samples to work around it.
- **Single-view app**: it omits the top bar's view-switch group entirely, per
  the suite top-bar grammar. Don't add one to "match" the others.
- **No database.** Works against the filesystem live.

## Not here

Machine-local paths, server addresses, credentials and per-box ops belong in a
machine-local `CLAUDE.md`, never in this file. **This repo is public.**
