# Project rules

This repository builds a compatibility launcher for the player's own PC
copy of Sega Rally Revo (2007). It is not a distribution of the game. It is
built on [rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk).

## Read first: the SDK's standards and playbooks

The SDK's rules are binding here. The vendored copies live in `sdk/` once
`scripts/sdk-sync` brings them; otherwise read them in the SDK repository:

- `docs/STANDARDS.md` — non-negotiables, product rules, settings window and
  CLI, robustness and support, golden snapshots, testing.
- `docs/playbooks/` — new launcher, SDK change and bumps,
  community components (bundle first, then a download button), binary
  patching, contributing back.

In short, the ones that are easy to forget:

- **Research community fixes first** (PCGamingWiki, GitHub, mod sites) and
  record them in `docs/RESEARCH.md` with licence and maintenance status.
  Build on them; reverse-engineer only what nobody solved.
- **Community components**: bundle when the licence allows; otherwise one
  button in the window plus a `--download-…` flag, pinned by SHA-256,
  only when the player presses it. Never a manual step, never a silent
  download.
- **Binary patches** are SDK `PatchSet` tables for exact builds (SHA-256),
  applied from the game folder's pristine bytes, all or nothing.
- **Settings window changes go through a design skill** (`frontend-design`,
  then a `web-design-guidelines` review). Accessibility first: contrast
  (palette test), labels and descriptions on every control, keyboard and
  pad, disabled controls say why.
- **GUI/CLI parity**: every option is one row of `config::OPTIONS` (key,
  flag, values, label, description); the window, `--help`, the template and
  parsing come from it, and the parity test (`ui::tests`) fails on drift.
  Every option has an end-to-end test (`plan::tests`), the window has
  headless `egui_kittest` tests, and the golden snapshots
  (`crates/srr-launcher/tests/snapshots/`) change only on purpose.
- **Headless by default**: `sdk/scripts/wine-test` for Windows-target
  tests, `gamescope --backend headless` for real-game runs.
- **Contribute back**: findings the community lacks are drafted in
  `docs/CONTRIBUTIONS.md` for the owner to post; agents never post.
- **Credentials** (e.g. `~/.config/rea/pcgamingwiki.txt`) are never printed,
  logged, committed, passed on a command line or handed to other agents.
- **Everything outside this repository is read-only**: toolchains, `~/.config`,
  `~/.local`, other repositories, game folders. Tests point `HOME`,
  `XDG_*` or `LOCALAPPDATA` at temporary folders.

## Non-negotiable boundaries

- Never commit, vendor, embed, or publish original executables, DLLs,
  media, archives, XML, fonts, track/car data, or other game assets, or
  anything copied out of them. Snapshots hold only launcher-made bytes.
- The player's game folder is read only. The launcher builds a separate
  runtime folder from it.
- Never commit a developer's absolute game path. Commands and code take an
  explicit path (or `SRR_GAME_DIR` in the env-gated test).
- Local experiments, traces, test prefixes and investigation output go
  under `.work/`.

## Owner decisions

- **2026-11-02 — the NoDVD `SEGA Rally.exe` is a supported edition.** The
  widely mirrored full rip ships a NoDVD `SEGA Rally.exe`
  (`ad84ecea…2ba391d7`); the launcher identifies it by SHA-256 and runs it
  (unpatched, or with the aspect fix) when the stutter fix cannot run.
  We never write a crack ourselves.
- **2026-11-02 — DirectX 9 files: one button instead of winetricks.** The
  DirectX End-User Runtime's licence does not let us bundle its DLLs
  (`docs/RESEARCH.md`), so the window downloads Microsoft's package on
  request (`--download-directx`); `srr-play` no longer runs winetricks.
- **2026-11-02 — the aspect choice is folded into the resolution** (the
  16:9/16:10 chips did nothing); ultra-wide sizes get the aspect fix.
- **2026-11-02 — `launcher.cfg` keys use underscores**; the hyphenated
  `frame-limit`/`stutter-fix` stay readable as aliases.

## Game-specific rules

- Support the v4.0.6.0 PC build by exact hash (clean `SEGA Rally_SSE1.exe`,
  the SSE1 build with the stutter fix applied, the full rip's NoDVD
  `SEGA Rally.exe`). Refuse unknown executables with their SHA-256 and the
  test-report link; never patch them.
- Only the private runtime copies are patched: the community stutter fix
  (`patches::PACER`, clean SSE1 only, result hash verified) and the
  ultra-wide aspect sets. Everything else is configuration the game reads.
- `config.ini` is seeded complete only when missing; afterwards only the
  keys of options set in `launcher.cfg` are written (`ini::Owned`). Never
  rewrite `[Audio]`/`[GameController]`; never delete `SEGA Rally.dat` or
  `SR_RANKTABLES.DAT`.
- Keep options specific to Sega Rally Revo and verified in the game.
  Anti-aliasing and the 62.5 fps Championship cap are deliberately absent;
  unverified values are listed in `docs/RUNTIME_NOTES.md`.
- The window code draws over `ui::Host` (closures for the Win32 calls), so
  it builds and is tested on the host; keep Win32 calls out of `Window`.

## Code and verification

- Rust for our own code. No `unsafe` or Win32 FFI here: it lives in the
  SDK's `rea-launcher-win32`.
- Before each commit: `cargo fmt --all --check`, `cargo test --workspace`,
  `linux/tests/*.sh`, `cargo clippy --release --target i686-pc-windows-gnu
  --workspace -- -D warnings`, `cargo deny check`, `sdk-sync --check`, the
  i686 release build and `sdk/scripts/wine-test --locked --workspace`.
- Exercise the real game separately and record exactly what was and was not
  verified in `docs/RUNTIME_NOTES.md`.
- Conventional Commit subjects; commit only when asked; never push.

## Shared SDK

- Generic code comes from the SDK crates `rea-launcher`,
  `rea-launcher-win32` and `rea-launcher-ui`, the Linux play library
  `sdk/rea-play.sh`, `sdk/packaging/` and the reusable workflows.
- Game knowledge stays here: the edition table, patch sets, the runtime
  `Policy`, config.ini and registry values, the option table, the palette,
  art and tabs, `srr-play`'s configuration.
- Never fork or copy SDK code into this repo, and never edit `sdk/` by
  hand. A missing feature or an SDK bug is reported upstream; keep local
  code meanwhile and say so.
- Bumping: follow the SDK's `docs/playbooks/sdk-change.md` ("Upgrading a
  launcher"): Cargo tags, `cargo update -p rea-launcher -p
  rea-launcher-win32 -p rea-launcher-ui`, `scripts/sdk-sync`, the `@vX.Y.Z`
  of the workflows, Migration items, then compare the golden snapshots.
