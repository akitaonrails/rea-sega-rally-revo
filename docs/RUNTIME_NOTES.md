# Runtime notes

Dated log of what was exercised for real and how. Anything not listed here
is design intent from `docs/RESEARCH.md`, not a verified behavior.

## 2026-11-01 — v0.1.0 bring-up (this machine)

Host: the reference gaming box (RTX 5090 + 7950X3D iGPU, 4K Samsung
Odyssey G8 on DP-1, Omarchy/Hyprland, `gaming` distrobox, 8BitDo Ultimate
2 pad over its 2.4G dongle in X-input mode). The game itself is installed
from the same full-rip by the distrobox-gaming pipeline and was already
proven playable there.

Verified, in order:

1. **Executable identification.** SHA-256/MD5 of both local game exes
   match the documented builds; `SEGA Rally_SSE1.exe` is byte-identical
   to the stutter-fix reference build.
2. **Pacer patch, byte level.** Every hook site, both normalisation
   sites, and all three cave regions in the local `SEGA Rally_SSE1.exe`
   match the reference expectations (checked with a byte-level diff
   before any code was written). The Rust patcher applied to the real
   file reproduces the reference patched SHA-256
   (`4c08c1de…f1ad08f2520`) exactly — `cargo test real_game -- --ignored`
   against `SRR_GAME_DIR`.
3. **Aspect tables.** Both tables located by pattern in both local exes
   (`0x2F7C44`/`0x2FADA8` SSE1, `0x2F4BF4`/`0x2F7D68` main); slot
   rewriting round-trips (patch → restore) in unit tests.
4. **Host suite.** `cargo test --workspace` 36 tests green;
   `cargo clippy --target i686-pc-windows-gnu -- -D warnings` clean;
   `cargo check --target i686-pc-windows-gnu` clean; `sh
   linux/tests/gpu-env.sh`, `sh linux/tests/config-keys.sh`, and `sh -n`
   on all shipped shell scripts pass.
5. **The .NET launcher's own schema.** Quality enum LOWEST/LOW/NORMAL/HIGH
   and the full `[Graphics]` key list extracted from
   `SEGARallyLauncher.exe` strings, confirming the config.ini writer.

Not exercised in this bring-up: the GUI under Wine, a launcher-started
game session, and the AppImage. See the next entry.

## 2026-11-01 — first real runs of the built launcher

Built `SRRLauncher.exe` (`i686-pc-windows-gnu`) and the AppImage inside the
`gaming` distrobox (mingw-w64, Wine 11.21, lld 23). All runs used a
throwaway prefix and `HOME` under `.work/`, inside headless gamescope at
3840x2160 on the RTX 5090 (nothing on the user's desktop), audio on a
private null sink. The user's game folder hashes were identical before and
after.

Bugs found by these runs, all fixed with regression tests:

1. **Linker: unresolved imports.** GNU ld 2.47 merged the per-crate
   raw-dylib `kernel32` import groups wrongly: `Sleep`, `CloseHandle`,
   `GetModuleHandleA/W` and `LoadLibraryW` sat outside every import
   descriptor, so the first `is_wine()` call jumped into `.rdata`. Now
   linked with LLD (`.cargo/config.toml`); `tests/imports.rs` checks every
   built exe in CI.
2. **Stutter fix refused the clean exe.** `EDITIONS` held copies of the
   edition statics, so the address comparison in `main.rs` never matched
   and every launch failed with "the private SSE1 copy is not the clean
   build". `EDITIONS` now references the statics.
3. **Bootup videos came back on the second launch.** Sync re-copied the
   renamed videos; it now treats a `.srr-disabled` copy as present.
4. **Settings window overlap.** The Play tab overflowed into the footer;
   tabs now scroll. The "stutter fix unavailable" warning no longer shows
   before a folder is chosen.
5. **`srr-play` counted zombie game processes** as running ("already
   running", and the wait loop would never end). Zombies are now ignored
   (`linux/tests/game-alive.sh`).

Verified:

- `--setup` twice: first run copied 1418 files and applied the pacer
  patch (private SSE1 SHA-256 = reference `4c08c1de…f1ad08f2520`); second
  run copied 0 files and reported the fix already applied. `config.ini`
  (3840x2160, High, vsync), the empty save and rank table, registry, DXVK
  `d3d9.dll`, and `dxvk.conf` (`d3d9.maxFrameRate = 60`) were written.
- Settings window rendered (artwork, theme, tabs, Look cards) and was
  captured with `gamescopectl screenshot`.
- `SRRLauncher.exe --no-window --linux` started the patched SSE1 exe
  through DXVK 3.1.1 on the RTX 5090; the "SEGA RALLY — press any button"
  title showed at ~60 s and the attract demo loaded the Canyon stage at
  3840x2160.
- The AppImage built (181 MB; Wine 11.20, appimagetool and runtime all
  matched their pinned hashes). `SRRLauncher-x86_64.AppImage --play`
  seeded the prefix, ran the launcher with the bundled Wine, and started
  the game: DXVK 3.1.1 picked the RTX 5090, read `dxvk.conf`, and created
  its swapchain under gamescope.

Not verified:

- Driving through a full stage with the pad was not recorded in detail
  (see the next entry for what the user confirmed).

- Driving with a pad, rumble, audio output, in-race frame pacing.
- The isolated Wine desktop path (`srr-play` on a real desktop, outside
  gamescope); the headless runs take the launcher's gamescope path.
- The winetricks bootstrap (skipped: it downloads Microsoft runtimes).
- Native Windows.

Harness gotcha: run any `wine` command before starting gamescope only if
`wineserver -w` follows; otherwise the display-less wineserver it starts
is reused inside gamescope and every window falls back to the null driver.

## 2026-11-01 — the user's first AppImage run on the real desktop

The user ran the locally built `SRRLauncher-x86_64.AppImage` on the host
(Hyprland, 4K Odyssey G8, RTX 5090), with no winetricks installed: the
settings window appeared, the game folder was chosen with Browse, Play
copied the game into a fresh prefix, and the game worked ("game is working
fine now"). This settles the black frames seen only in headless gamescope
with the AppImage: the real-desktop path (isolated Wine desktop) renders.

Found along the way: closing the settings window without Play ended
`srr-play` silently, and the settings step's stderr went to `/dev/null`.
It now says why it stopped and keeps that output in
`~/.config/srr-launcher/settings.log`.

## 2026-11-02 — v0.1.1: option audit and quality presets

Same headless harness (gaming distrobox, gamescope headless at 3840x2160,
RTX 5090, throwaway prefix). The game folder was reached through a Wine
drive `D:` mapped to a symlink, so screenshots show no developer path.

- `--quality ultra`: config.ini got every quality slider at 3; the game
  reached its title and ran the Alpine attract demo at 4K.
- `--quality normal`: every slider at 2; the game ran its attract-mode
  menus at 4K.
- The settings window's three tabs were captured for the README
  (`docs/images/`) with the rebuilt exe, driving the tabs with XTest clicks.
- The ultra-wide + stutter fix failure (aspect patch changing the exe's
  hash) was found by code audit, not by a run: the reference host is 16:9.
  The fix is covered by the env-gated `real_game` test (clean → pacer →
  21:9 → still identified as the stutter-fixed build), which passed against
  the real exe.

Not verified: the gamepad mapping change on a real pad (unit-tested
against the XInput bit values only); the frame limit under Steam's Proton.

## 2026-11-02 — the user's gameplay recording

The user recorded a session with OBS on the reference host (AppImage,
Hyprland, 4K): the launcher window, Play, about two minutes of first-run
loading (game files on a NAS), the game's legal and title screens, then a
Safari race driven with the 8BitDo pad in X-input mode through lap 2.
This is the first recorded in-race gamepad control through the launcher
(the game's native pad support, no remapping). The trimmed 1080p cut is
linked from the README (hosted outside the repository).
