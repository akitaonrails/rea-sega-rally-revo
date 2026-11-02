# Project rules

This repository builds a compatibility launcher for the user's own PC copy
of Sega Rally Revo (2007). It is not a distribution of the game.

## Non-negotiable boundaries

- Never commit, vendor, embed, or publish original executables, DLLs,
  media, archives, XML, fonts, track/car data, or other game assets, or
  anything copied out of them.
- The user's game folder is read only. The launcher builds a separate
  runtime folder from it.
- Never commit a developer's absolute game path. Commands and code take an
  explicit path (or `SRR_GAME_DIR` in the env-gated test).
- Put local experiments, traces, test prefixes, and investigation output
  under `.work/`.

## Product rules

- Support the v4.0.6.0 PC build by exact hash (clean `SEGA Rally_SSE1.exe`
  plus the full-rip NoDVD `SEGA Rally.exe`). Reject unknown executables
  instead of patching them blindly.
- Self-contained: the user points at the game and plays. Do not require
  third-party patchers, compatibility modes, codec packs, or manual
  registry/INI edits.
- Windows first, easy on Linux: ship a 32-bit Windows launcher that also
  runs under Proton/Wine, plus an AppImage with portable Wine and DXVK.
- Reuse maintained open-source components where they fit. Record every
  bundled component and its licence in `THIRD_PARTY.md`.
- Only the private runtime copies may be patched. The current binary
  changes are the community pacer fix (clean SSE1 build only, byte-verified)
  and the aspect-table constants. Everything else is configuration the
  game itself reads.
- Preserve player settings and saves. Never rewrite the `[Audio]` /
  `[GameController]` sections of an existing `config.ini`; never delete
  `SEGA Rally.dat` or `SR_RANKTABLES.DAT`; seed only when missing.
- Keep GUI and CLI options specific to Sega Rally Revo. Do not add options
  whose behavior has not been implemented and verified (anti-aliasing and
  the 62.5 fps Championship cap are deliberately absent).

## Code and verification

- Rust for our own code. Keep `unsafe` and Win32 FFI in `win32.rs`.
- Unit-test edition detection, binary patching (golden bytes pinned from
  the reference assembly), runtime synchronization, config.ini editing,
  settings, and form behavior.
- The env-gated `real_game` test (`SRR_GAME_DIR=... cargo test real_game
  -- --ignored`) exercises the real executables without committing them.
- Run `cargo test --workspace`, formatting, Windows-target clippy, and
  Linux helper tests before each milestone. Exercise the real game
  separately and record exactly what was and was not verified in
  `docs/RUNTIME_NOTES.md`.
- Use Conventional Commit subjects. Commit only when asked.
