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
   linked with LLD (`.cargo/config.toml`); CI checks every built exe with
   the SDK's `packaging/check-imports.sh` (it replaced `tests/imports.rs`).
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

## Unreleased — cross-launcher audit fixes (needs a real-game check)

Changed by code audit and covered by unit/shell tests only; none of this
was exercised against the real game yet:

- **FIRST: winebus hidraw/evdev now really disabled — a behaviour change.**
  `DisableHidraw`/`DisableInput` are now really set. The old spaced names
  were ignored by Wine, so every verified run so far (including the user's
  recorded race) had hidraw/evdev devices enabled. This matches the
  documented design and OutRun's tested values, but on this game it is
  unverified. Check that the 8BitDo pad appears exactly once in the game's
  controller options, that steering, pedals and buttons work in a race,
  and that the settings window's pad navigation works, through `srr-play`,
  the AppImage, and Steam's Proton running `SRRLauncher.exe` directly
  (second start). If it regresses, revert per prefix (values that exist
  are never overwritten):

  ```sh
WINEPREFIX=~/.local/share/srr-launcher/prefix wine reg add \
  'HKLM\System\CurrentControlSet\Services\winebus' /v DisableHidraw /t REG_DWORD /d 0 /f
WINEPREFIX=~/.local/share/srr-launcher/prefix wine reg add \
  'HKLM\System\CurrentControlSet\Services\winebus' /v DisableInput /t REG_DWORD /d 0 /f
  ```
- **NVIDIA ICD pinning** now also requires an NVIDIA PCI device or the
  `nvidia` module, and exports `VK_DRIVER_FILES`. On the reference host
  (RTX 5090 + iGPU) `srr-play` should still print the NVIDIA pin and DXVK
  should still pick the RTX 5090.
- **srr-play supervision.** `srr-play --play` (detached) then `srr-play
  status` / `srr-play stop` on a real game session, and the AppImage
  (waiting mode) ending cleanly when the game exits or on Ctrl+C. Only
  covered with fake runners and `sleep` (`linux/tests/supervision.sh`).
- **Look cards on a Steam Deck.** The first-run look and badge on a real
  Deck (1280×800, Game Mode) are unit-tested only; check that the window
  shows Steam Deck selected and badged there.
- **config.ini editing** now rewrites a managed `[Graphics]` key as
  `Key=value` wherever it was spelled differently, drops duplicate managed
  keys, and replaces the file by rename. Check that the game still reads
  the resolution/quality after a launch over a config.ini the game itself
  saved (CRLF), and that controller bindings survive.
- **Uncanonicalizable drives.** The lexical fallback is tested with a
  permission-denied folder on Linux only; not tried on a Windows RAM disk,
  `subst` drive or network share.
- **config.ini bytes/BOM.** Non-UTF-8 and BOM handling is unit-tested
  only; no real config.ini with such a controller name was available.
- **Pacer result verification.** The patch is now only written when it
  hashes to `4c08c1de…f1ad08f2520`. Rerun `SRR_GAME_DIR=... cargo test
  real_game -- --ignored` (it now goes through `pacer_verified`) and one
  fresh-runtime launch with the stutter fix on.
- **Owned d3d9.dll.** On native Windows a player's own `d3d9.dll` in the
  game folder is synced; with `--linux` (or the `.srr-dxvk` marker) it is
  never copied over our DXVK. Not exercised on a real Windows install with
  a wrapper DLL.
- **WINEDLLOVERRIDES `d3d9=n,b`** is now passed to the game (through the
  isolated desktop's explorer). Check in `launcher.log` and DXVK's log that
  DXVK 3.1.1 still loads and picks the dGPU through srr-play and the
  AppImage.
- **Settings window at Wine DPI 120/144/192.** The creation zoom now
  divides out Wine's DPI scale (`LOGPIXELSY`/96, process made DPI aware
  first). Unit-tested only (fits 4K at 96–192 DPI); open the window with
  `winecfg` DPI 144 and 192 on the 4K desktop and check it fits and is
  sharp.
- **Disabled chips** now outline the saved value and show a hover reason;
  not yet seen in the real window (capture the Play tab on native Windows
  or with a folder that lacks the clean SSE1 exe).

## 2026-11-02 — move onto rea-launcher-sdk v0.1.0

Verified: `cargo test --workspace` (host, including the palette contrast
test), the Windows tests under Wine with the SDK's headless
`scripts/wine-test` in the `gaming` distrobox (baseline before the move:
all 66 pass; after: all 56 pass — the rest moved to the SDK's own
suites), the linked `SRRLauncher.exe` (lld) and its import-resolution
test, every `linux/tests/*.sh`, `cargo deny check`, the sdk/ drift check,
and a full local `packaging/package.sh` run: the zip passes
`packaging/package.allow`, the AppImage holds `usr/lib/srr/{SRRLauncher.exe,
srr-play,rea-play.sh,linux/d3d9.dll}` and its `srr-play status` answers.

Not exercised: the settings window (no window was opened), a game
session, `real_game` (no game folder on this host), and native Windows.
Worth a look on the next real run: the new chip focus ring and d-pad
stepping in the window, and the VirtualStore path on a non-admin Windows.

## Unreleased — v0.2 pass on rea-launcher-sdk v0.2.0 (needs real-game checks)

Verified (no game run, no window opened): host tests including the
headless window tests (egui_kittest), the same tests under Wine with
`sdk/scripts/wine-test` (i686), the golden snapshots, `cargo deny`, the
sdk/ drift check, the shell tests, and a local `packaging/package.sh` run
(zip within the allowlist, AppImage built). The DirectX download was
tested against Microsoft's real package once by hand (hash pinned, the
D3DX9 DLLs extracted with the `cab` crate) and with a fake server in the
unit tests; not through the window.

Pending, in the real game:

- **Option values never seen in the game** (the window offers them):
  21:9 and 32:9 aspect fixes (no ultra-wide screen), frame limit 120 and
  off, vsync off, and the five non-English menu languages. Each needs one
  run that shows the effect.
- **config.ini written key by key.** Only the set options' keys are
  written now; check a launch over a config.ini the game saved, that the
  game still reads resolution/quality/vsync, and that unsetting an option
  in launcher.cfg leaves the game's own value.
- **Patches from the pristine exe every launch.** Rerun
  `SRR_GAME_DIR=… cargo test real_game -- --ignored` (it now checks every
  `PatchSet` through the SDK, incl. the result hash), then one launch with
  the stutter fix on and one off, and one with an ultra-wide resolution.
- **DirectX 9 files.** On native Windows 10/11 without the DirectX
  runtime: does the game start only after Download? Which `d3dx9_NN.dll`
  does it load (Process Monitor or `WINEDEBUG=+loaddll`)? Under Wine, a race
  with the downloaded files (`n,b` overrides) and one without.
- **DirectPlay**: not wired; check whether the menus touch it on native
  Windows (Windows would offer the optional feature).
- **No winetricks**: a new prefix through srr-play/the AppImage reaching a
  race (the owner's earlier run suggests it does).
- **--check / --help / errors in a terminal** on native Windows (console
  attach) and under Wine.
- **Settings window**: the regrouped Picture tab, the DirectX row on the
  Play tab, the photo credit band, and the lighter control borders —
  judge them in a deliberate window; `docs/images/launcher-picture.png`
  still shows the old aspect row and should be recaptured.

## Unreleased — rea-launcher-sdk v0.3.0 (needs real-game checks)

Verified (no game run, no window opened): host tests including the
headless window tests, the same tests under Wine (`sdk/scripts/wine-test`,
i686), the golden snapshots (only `--help`'s switch padding changed), the
lld and import checks on the i686 release build, `cargo deny`, the sdk/
drift check against v0.3.0 and the shell tests. The single-instance
decision is tested with a fake; the SDK's mutex and window lookup were not
exercised with two real launchers.

The README screenshots (`docs/images/launcher-*.jpg`) were retaken with
this build: Xvfb at 2560x1440, Wine at 96 DPI in a throwaway prefix under
`.work/`, no game folder (so the Play tab shows the first run), tabs
switched with XTest clicks. They replace the PNGs, whose Picture tab still
showed the removed aspect row.

Pending, in the real game (STANDARDS "Clean exit, single instance"):

- **Second start while the window is open** (native Windows and Wine):
  the first window comes forward, no second window, exit code 0.
- **Second start while the game runs** from the launcher: the message
  "already running … Quit the game first", exit code 5, no second game.
  Under Wine with srr-play's isolated desktop the launcher can return
  before the game; there the helper's session lock is what refuses (check
  `srr-play` twice, and `srr-play status`).
- **Every way of quitting** leaves no `SEGA Rally*.exe`, launcher,
  `wineserver` or gamescope behind within a few seconds: the game's own
  Quit, Alt+F4 / window close, Steam's Stop, `srr-play stop` and Ctrl+C.
  The launcher only waits for the game (it never stops it), so if the game
  ignores `WM_CLOSE`, it needs a close hook (SDK `rea-launcher-close-hook`).
- **DirectX 9 download** through the window after the move to the SDK's
  `install_with`: progress through Unpacking, the DLLs in
  `directx\directx_Jun2010_redist\` (was `directx\d3dx9\` in unreleased
  builds; an old folder there is now unused and can be deleted), Remove.
