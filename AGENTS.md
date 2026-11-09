# Sega Rally Revo launcher — instructions for coding agents

This repository makes the player's own PC copy of **Sega Rally Revo**
(2007) run on modern Windows and Linux (Proton/Wine, AppImage) with a
settings window, sensible defaults and no manual steps. It is not a
distribution of the game. It is built on
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk).

**Read `sdk/docs/STANDARDS.md` before any work; the playbooks are in
`sdk/docs/playbooks/`** (vendored from the SDK at the tag in `sdk/VERSION`).

## Always

- The game's files are read-only: never committed, copied into the repo or published.
- Everything outside this repository is the user's and read-only (game installs,
  other repos, Wine prefixes, saves, home configs, containers, the SDK checkout)
  unless the user asked for or approved that specific change: back up first,
  never delete by a broad glob, ask when unsure.
- Headless by default; open a visible window only when a check needs a person, and say so first.
- Never print, log or commit credentials.
- Push, tag, release or post only with the user's go-ahead.
- Research community fixes first (`docs/RESEARCH.md`); patch only what nobody solved.
- Never edit `sdk/`; SDK gaps and bugs go to the SDK repository as issues or PRs.
- Clean up after a release or a long session: `sdk/scripts/clean`, then `--yes` (log scratch in `.work/.task-created`).

## Checks (all must pass before a commit)

```sh
cargo fmt --all --check
cargo test --workspace --locked
for t in linux/tests/*.sh packaging/tests/*.sh; do [ -f "$t" ] && sh "$t"; done
cargo clippy --release --target i686-pc-windows-gnu --workspace -- -D warnings
cargo deny check
../rea-launcher-sdk/scripts/sdk-sync --check .   # from the SDK at the tag in sdk/VERSION
cargo build --release --locked --target i686-pc-windows-gnu
sdk/packaging/check-imports.sh target/i686-pc-windows-gnu/release/SRRLauncher.exe
sdk/scripts/wine-test --locked --workspace     # Windows-target tests under Wine, headless
sdk/scripts/launcher-check                     # family rules; waivers in .rea-check-waivers
```

MinGW, lld and Wine may live in the `gaming` distrobox: run the last three
as `distrobox enter gaming -- sh -c 'cd /absolute/path && …'`.

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
- **2026-11-02 — the stutter fix is our clean-room implementation.** The
  old `patches::PACER` caves (derived from stuken's unlicensed `patch.py`)
  are gone; `patches::PACER` is now `crate::pacer::PACER`, written from a
  specification and the game's own code (`docs/PACER-PROVENANCE.md`;
  clean `22655a54…` → `cb9a5a34…ff65`). stuken is credited for the idea
  and the analysis only. Never bring `patch.py` code or bytes (or those of
  the community-fixed exe `4c08c1de…`) into this repository; that exe is
  only recognised by hash as already fixed. Change the pacer through
  `pacer/*.asm` and keep `tests/pacer_*.rs` green.

- **2026-11-02 — the game makes its own save files.** The launcher no
  longer seeds empty `SEGA Rally.dat` / `SR_RANKTABLES.DAT` (the game
  called them corrupt: "Player Progress Data Corrupt", "Local Leaderboard
  Corrupt"); it only creates the Documents folders. A zero-byte file of
  that name (the old seed, never written by the game) is removed so the
  game recreates it; any other file is the player's save and is never
  touched.

## Game-specific rules

- Support the v4.0.6.0 PC build by exact hash (clean `SEGA Rally_SSE1.exe`,
  the SSE1 build with the stutter fix applied, the full rip's NoDVD
  `SEGA Rally.exe`). Refuse unknown executables with their SHA-256 and the
  issue link; never patch them.
- Only the private runtime copies are patched, always from the game
  folder's untouched executable: the stutter fix (`patches::PACER`, the
  clean-room set, clean SSE1 only, result hash verified), the ultra-wide
  aspect sets, and, when chosen, the borderless set (`display_mode`) and
  the keys set (`keys`; its 256-byte table is written from the option),
  both for the clean SSE1 and the NoDVD build only. Everything else is
  configuration the game reads.
- `config.ini` is seeded complete only when missing; afterwards only the
  keys of options set in `launcher.cfg` are written (`ini::Owned`). Never
  rewrite `[Audio]`. In `[GameController]` the only thing the launcher
  ever touches is the vibration field (chars 12–16) of `Mapping` — the
  `rumble` option owns exactly those four characters; the pad GUID,
  layout and the eleven button bindings are the player's, kept
  character for character, with `Owned` release semantics. The save
  files (`SEGA Rally.dat`, `SR_RANKTABLES.DAT`) are the game's: never
  seeded, never deleted (only the empty seeds of earlier releases are
  removed, see the owner decision).
- Keep options specific to Sega Rally Revo and verified in the game.
  Anti-aliasing and the 62.5 fps Championship cap are deliberately absent;
  unverified values are listed in `docs/RUNTIME_NOTES.md`.
- Every option is one row of `config::OPTIONS`; end-to-end cases live in
  `plan::tests`, the window's in `ui::tests`, and the golden snapshots in
  `crates/srr-launcher/tests/snapshots/` (`SRR_UPDATE_SNAPSHOTS=1` only
  for an intended change). The env-gated real-game test reads
  `SRR_GAME_DIR`.
- The window code draws over `ui::Host` (closures for the Win32 calls), so
  it builds and is tested on the host; keep Win32 calls out of `Window`.
- Game knowledge stays here: the edition table, patch sets, the runtime
  `Policy`, `config.ini` and registry values, the option table, the
  palette, art and tabs, `srr-play`'s configuration.
