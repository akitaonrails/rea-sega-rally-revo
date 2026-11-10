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

## 2026-11-02 — real-game verification pass (headless, owner's full rip)

Release build `SRRLauncher.exe` (i686, lld) plus `linux/srr-play` and the
pinned DXVK 3.1.1 `d3d9.dll`, against the owner's full rip (read-only,
reached as Wine drive `D:` through a symlink). Everything ran inside the
`gaming` distrobox under a supervisor with a finite timeout
(`.work/realgame/sv.sh`): a throwaway prefix, `HOME` and XDG dirs under
`.work/realgame/`, a scrubbed environment (`env -i` plus only the test's
variables), `gamescope --backend headless` on the RTX 5090 (1280x720 to
3840x1080), audio on a private PipeWire null sink (`srr_rg_null`,
unloaded afterwards). After every run the supervisor listed the test's
processes by an environment marker and `WINEPREFIX` (plus `Z` entries by
name), measured how long each kind took to disappear after the quit
action, then signalled exact PIDs (TERM, then KILL). Evidence (logs,
JPEG screenshots) is in `.work/realgame/{logs,keep}/`.

Harness lessons (not launcher bugs):

- **The game shows nothing until it gets input.** In headless gamescope
  the window never gets focus, and the game neither presents a frame nor
  advances (black screen, ~0.6 CPU ticks/s) until an input event arrives:
  one click plus a Shift key press and it runs (legal screen, title,
  attract demo). On a real desktop focus is given at start.
- `gamescopectl screenshot` writes the file asynchronously: wait before
  reading it.
- gamescope's environment can't be read (file capabilities); track it as
  `timeout`'s child. gamescope 3.17 often segfaults on exit (exit 139).
- A TERM to the game can leave a defunct leader with live threads (seen
  once: 33 threads in `S`); a KILL to the thread group removes it.
  After the isolated-desktop run, gamescope, Xwayland and two explorer
  threads sat in `D` (`drm_release`, NVIDIA driver) for about a minute
  after KILL, then went away by themselves.

| Check | Result | Evidence |
| --- | --- | --- |
| Fresh runtime, `--setup` | VERIFIED: 1418 files copied, pacer applied (private SSE1 = `4c08c1de…f1ad08f2520`), config.ini/save/rank table/registry/DXVK written | `logs/setup.log`, `launcher.log` |
| `real_game` test (`SRR_GAME_DIR=… cargo test real_game -- --ignored`) | VERIFIED (1 passed) | terminal |
| Look Modern (1920x1080 screen) | VERIFIED: config.ini 1920x1080 High vsync, title and attract demo render through DXVK on the RTX 5090 | `keep/perf-60-030.jpg` |
| Look Original 2007 | VERIFIED (files): config.ini 1280x720, Normal, vsync; DXVK swapchain 1280x720. The screenshot is black (taken before the input fix above) | `logs/look-original.log` |
| Look Steam Deck | VERIFIED: config.ini 1280x800 High, DXVK buffer 1280x800, renders | `keep/look-steamdeck-030.jpg` |
| Stutter fix off | VERIFIED: `SEGA Rally.exe` (NoDVD, pristine) launched | `logs/diag-nopacer.log` |
| 21:9 (2560x1080) | VERIFIED (files): aspect 21:9 set on both exes, the patched SSE1 still runs; **screenshot for the owner** (is the picture unstretched?) | `keep/opt-219-120-fr-040.jpg` |
| 32:9 (3840x1080) | VERIFIED (files): aspect 32:9, runs; **screenshot for the owner** | `keep/opt-329-off-de-040.jpg` |
| Frame limit 60 / 120 / off | VERIFIED (gamescope at 240 Hz, DXVK HUD): 60.0, 119.9–120.0, uncapped (610–670 fps in the attract demo, ~4000 on the title) | `keep/perf-*.jpg` |
| Vsync off | VERIFIED: config.ini `VSync=0`, frame rate far above the 240 Hz refresh | `keep/perf-off-040.jpg` |
| Languages | VERIFIED French, German, Spanish, Italian (title text); Polish FAILED, then fixed (below) and VERIFIED ("WCIŚNIJ PRZYCISK START") | `keep/opt-*`, `keep/lang-*.jpg` |
| config.ini over the game's own file | VERIFIED: the game's keys (e.g. `ResolutionFilter`, `[GameController]`) kept, set options rewritten, unset quality kept the game's value | config.ini after the runs |
| NVIDIA ICD pin (srr-play) | VERIFIED: `GPU: VK_DRIVER_FILES=…nvidia_icd.json`; DXVK picks the RTX 5090 | `logs/srrplay-stop.log` |
| `WINEDLLOVERRIDES` reaches the game | **FAILED** (see bugs): DXVK loads only because Wine's default order already prefers the app-folder `d3d9.dll` | `logs/dx-*.log` |
| `--help`, `--check`, `--check` on a wrong folder (Wine) | VERIFIED: usage exit 0; "Ready to play" exit 0; plain message with the issue link, exit 1 | `logs/cli.log` |
| `--download-directx` | VERIFIED: Microsoft's package downloaded into the prefix, 22 DLLs in `directx\directx_Jun2010_redist`, copied into the private copy at the next launch | `logs/cli.log`, `launcher.log` |
| Game's own Quit (Escape at the title → "Are you sure you want to exit?" → Enter) | VERIFIED: game, launcher, wineserver gone in 0.08 s, gamescope 0.4 s | `logs/ownquit.log`, `keep/ownquit-confirm.jpg` |
| Close request (`wine cmd /c taskkill /im …`, no `/f`) | VERIFIED on a running game (8 runs): game and launcher 0.36–0.41 s, wineserver ~1 s, all ≤ 1.3 s (≤ 6.2 s once, gamescope teardown). A game still waiting for its first input ignores it | `logs/opt-*.log`, `logs/lang-it.log`, `logs/look-original.log` |
| `srr-play stop` (detached `--play`, `status` = running) | VERIFIED: all gone in 0.66 s | `logs/srrplay-stop.log` |
| SIGINT / SIGTERM to srr-play in waiting mode (`SRR_WAIT=1`, as the AppImage) | VERIFIED: helper exits 143, all gone in 1.6 s | `logs/srrplay-sig*.log` |
| gamescope killed (TERM) | Launcher and wineserver gone at once; the game was left as a defunct leader and gamescope hung until the supervisor's TERM. Only relevant inside gamescope (srr-play uses none) | `logs/lang-pl-killgs.log` |
| Second launcher start while the game runs | VERIFIED: "Sega Rally Revo is already running from this launcher. Quit the game first…", exit 5, still one game; log `another launcher is running: Busy` (the game's window title does not collide with the launcher's) | `logs/second-launcher.log` |
| Second `srr-play --play` while the game runs | VERIFIED: "already running", exit 0, no second game | `logs/second-srrplay.log` |
| winebus: pad appears once, steering/pedals | NOT CHECKABLE HEADLESS: needs the 8BitDo pad; a person must open the game's controller options through srr-play, the AppImage and Proton | — |
| Settings window (Wine, private Xvfb 2560x1440, 96 DPI) | Captured with the real game folder; README screenshots retaken | `docs/images/launcher-*.jpg` |

Bugs found:

1. **Polish menus showed English** (fixed, `c1d3c5e`). The full rip's
   readme: Polish needs `sku=polish`; the launcher always wrote `Europe`.
2. **The game's `WINEDLLOVERRIDES` is not honoured** (open). The launcher
   logs `wine: DLL overrides d3d9,d3dcompiler_42,…,d3dx9_43=n,b` and sets
   it on the game's command, but the game's Unix environment has no
   `WINEDLLOVERRIDES` and Wine loads its builtin `d3dx9_35` even though
   Microsoft's DLL is in the private copy — in both the gamescope
   (direct) and isolated-desktop (`explorer /desktop`) paths. The same
   string exported in the Unix environment loads the native DLL; with no
   override at all the result matches the launcher's (app-folder
   `d3d9.dll` native by Wine's default, `d3dx9_35` builtin). So the
   downloaded DirectX files are not used under Wine, and DXVK works only
   thanks to Wine's default order. The game ran fine with Wine's builtin
   D3DX9 in every run. Candidate fixes: `srr-play` exports the overrides
   before starting the launcher (not Proton), or per-executable
   `HKCU\Software\Wine\AppDefaults\<exe>\DllOverrides` registry values
   (Proton too). This touches how every rea launcher passes overrides, so
   it goes to the SDK first.
3. **"Player Progress Data Corrupt" and "Local Leaderboard Corrupt" on a
   fresh profile** (open, by design so far). The launcher seeds an empty
   `SEGA Rally.dat` and `SR_RANKTABLES.DAT` (RESEARCH #3, against the
   main-menu crash); the game calls both corrupt and offers to overwrite
   them. Answering Yes creates valid files (23,696 bytes and 464 KB) and
   the prompts do not come back. Worth an owner decision: keep the
   prompts, or seed the files the game writes.

Performance figures: an LLM was running on the same GPU during the first
runs, so the frame rates measured then were discarded; the figures above
come from three runs after the GPU was free. The GPU showed 41 % before
the first of them, and another agent's game test started during the last
(uncapped) run, so treat the uncapped figures as a lower bound; the 60
and 120 caps were reached exactly.

Not exercised here: the AppImage (not rebuilt), Steam's Proton, native
Windows, the Settings window at Wine DPI 120–192, a driven race with a
pad, the game at more than 60 fps in a race (the stutter fix with
`frame_limit 120/off`).

## 2026-11-02 — the game makes its own save files (owner decision)

The launcher no longer seeds empty `SEGA Rally.dat` / `SR_RANKTABLES.DAT`
(`7bb11e7`) and removes zero-byte copies left by earlier releases
(`ec2877e`). Same headless harness as the entry above (new throwaway
prefix, `--setup` from the owner's full rip, gamescope headless 1920x1080,
`env -i`, XTest keys).

- **Fresh profile.** After `--setup`, `Saved Games` and
  `Shared Data/{Rank Table,Ghosts}` exist and hold no `.dat` file. First
  launch: title → Enter → "Saving Data" → "Would you like to adjust your
  brightness setting?" (the game's own first-run question). There was no
  "Player Progress Data Corrupt" and no "Local Leaderboard Corrupt". The
  game wrote `SEGA Rally.dat` (23,696 bytes) and `SR_RANKTABLES.DAT`
  (464,008 bytes) itself. VERIFIED. Evidence: `.work/realgame/logs/fresh.log`,
  `keep/saves-fresh-0{2,3}.jpg`.
- **No main-menu crash** (the reason the seed existed, RESEARCH #3). Second
  launch on that profile: main menu (Championship), back to the title,
  the attract race (Alpine), then Championship → Premier → Amateur League
  → AZA Challenge → car selection. The game stayed up for 2.5 minutes,
  until the supervisor closed it. VERIFIED. Evidence: `logs/menu2.log`,
  `keep/saves-menu2-0{3,8}.jpg`.
- **Upgrade over the old empty seeds.** Both files truncated to 0 bytes as
  earlier releases left them, then a launch: `launcher.log` says "saves:
  removed 2 empty save file(s) left by an earlier release; the game
  creates them". There was no corrupt prompt (only the brightness
  question). The game recreated both files (23,696 / 464,008 bytes) and
  went from Championship through car and livery selection to the Safari
  race start ("Press ENTER to Start"), without crashing. VERIFIED. Evidence:
  `logs/upgrade.log`, `keep/saves-up-{01,12}.jpg`.
- Non-empty saves are left alone: covered by the host test
  `the_old_empty_seeds_go_and_real_saves_stay`. The real non-empty saves
  above were kept across the second and third launches.

## 2026-11-02 — rea-launcher-sdk v0.3.3: per-exe DLL overrides (bundled DLLs proven native)

Same headless harness. The prefix was made new for the save-file check;
Microsoft's DirectX package was downloaded into it with
`--download-directx`, and `--setup --linux` put the 22 D3DX9/D3DCompiler
DLLs into the private copy. Every run started from `env -i` with no
`WINEDLLOVERRIDES`, under `WINEDEBUG=+loaddll`. Logs are in
`.work/realgame/logs/dx-*.log`.

**Why `d3dx9_35` was builtin before.** It was not an inherited
`WINEDLLOVERRIDES`: the environment held none, and `[Software\Wine\DllOverrides]`
was empty in both prefixes. The DLL was where the game loads it from
(the private copy, the game's folder). `WINEDEBUG=+module` shows the
reason (`dx-why.log`): `version_heuristics got default vendor "Microsoft
Corporation"` for the copy's `d3dx9_35.dll`, then `find_builtin_dll`. With
no explicit override, Wine's loader replaces a DLL whose version resource
names Microsoft as the vendor with its own builtin, even from the program
folder. DXVK's `d3d9.dll` (not Microsoft's) does win from the program
folder by default. That is why `d3d9` was native and `d3dx9_35` builtin.
On top of that, the launcher's `WINEDLLOVERRIDES` never reached the game
(SDK v0.3.3: Wine ignores it in a child's environment block), so nothing
overrode the vendor rule.

| Run | `d3d9.dll` | `d3dx9_35.dll` |
| --- | --- | --- |
| Before: the game started directly, no override (`dx-before.log`) | native (copy, DXVK) | **builtin** |
| Before: the v0.3.2 launcher, env override only (`dx-launcher.log`, `dx-desktop.log`, first prefix) | native (copy) | **builtin** |
| After: `wine SRRLauncher.exe --no-window`, plain Wine, no `--linux` (`dx-after-plain.log`) | builtin (system32; no DXVK of ours without `--linux`) | **native** (copy) |
| After: `wine SRRLauncher.exe --no-window --linux` (`dx-after-linux.log`) | native (copy, DXVK) | **native** (copy) |
| After: `srr-play --play` (`dx-after-srrplay.log`) | native (copy, DXVK) | **native** (copy) |

`launcher.log` after: `wine: per-exe DLL overrides for SEGA Rally_SSE1.exe
(native,builtin): written [...]`, then `already ours [...]` on the next
launch. The game loads only `d3dx9_35` of the D3DX9 set. In every "after"
run it kept running until the supervisor closed it (taskkill, or `srr-play
stop`); everything was gone in under 1 s.

This settles bug 2 of the verification entry above. The Wine-side
registry round trip (ours written, the player's value kept, ours taken
back) is covered by `tests/wine_app_overrides.rs` under
`sdk/scripts/wine-test`.

## 2026-11-03 — rea-launcher-sdk v0.4.1 on the real game (headless)

Release build of this branch packaged with `packaging/package.sh` (DXVK
3.1.1 `d3d9.dll`), against the owner's full rip (read-only, Wine drive
`D:` through a symlink in the test prefix; both exe hashes unchanged and
no file in the game folder newer than the runs). Everything ran in the
`gaming` distrobox from `env -i`: a new throwaway prefix, `HOME`, XDG dirs
and `XDG_RUNTIME_DIR` under `.work/rg2/`, sound to a private PipeWire
null sink (WirePlumber's device monitors off), each run inside its own
`gamescope --backend headless` (as Game Mode: srr-play starts no gamescope
of its own) with a finite timeout. Input by XTest on gamescope's Xwayland;
screenshots by `gamescopectl`. The supervisor found the test's processes
by name plus their working directory (under `.work/rg2/`, or the prefix's
wineserver directory) and never read a command line or an environment;
after each run it timed how long each kind took to go and swept leftovers
by exact PID. Evidence: `.work/rg2/{logs,shots}/` (local).

| Check | Result | Evidence |
| --- | --- | --- |
| Fresh prefix, first start of `srr-play` (settings window) | VERIFIED: the window shows on the first start; launcher.log `OpenGL through GLX (UseEGL=N); the restarted launcher exited with 3`; Alt+F4 closes it, srr-play exits 0 | `r0-fresh-window` |
| Look Modern (`look = modern`, 1920x1080) | VERIFIED: 1418 files copied, pacer applied (`cb9a5a34…ff65`), config.ini 1920x1080, the attract demo renders | `r1-modern-ownquit` |
| Look Original 2007 (`--look original`) | VERIFIED: config.ini 1280x720 Normal vsync, renders | `r2-original-altf4` |
| Look Steam Deck (`--look steam-deck`, 1280x800) | VERIFIED: config.ini 1280x800 High, renders | `r3-steamdeck-stop` |
| The game's own Quit (Escape → "Are you sure…" → Enter) | VERIFIED: game, launcher, helper gone in 1.5 s, wineserver 3.1 s | `r1-modern-ownquit` |
| Alt+F4 (XTest) | VERIFIED: game and launcher 1.5 s, wineserver 3.3 s | `r2-original-altf4` |
| `srr-play stop` | VERIFIED: everything gone in 1.4 s | `r3-steamdeck-stop` |
| SIGINT / SIGTERM to a waiting srr-play (`SRR_WAIT=1`) | VERIFIED: game, launcher, helper 1.4 s, wineserver 1.4 s / 3.0 s | `r4-sigint`, `r5-sigterm` |
| gamescope closed (TERM) | VERIFIED for the launcher: game, launcher, helper, wineserver gone in 1.6 s. gamescope 3.17 itself needed a KILL (it often hangs or segfaults on exit; not ours) | `r6-killgs` |
| Second launcher start while the game runs | VERIFIED: the "already running" message box (screenshot), exit 5, still one game; log `second start: … refused` | `r7-second-launcher` |
| Second `srr-play --play` while the game runs | VERIFIED: "already running", exit 5 (was 0), one game | `r8-second-srrplay` |
| Launcher killed while the game runs, then started again | VERIFIED: refused before the sync by `process::find_running` (`the game from C:\users\…\SegaRally runs: process [288]`), message box, exit 5, one game | `r9-orphan` |
| Play-time error under srr-play (`--play --game` an empty folder) | VERIFIED: a message box with the reason and the unknown-edition link (screenshot); launcher.log has it too | `r10-play-error` |
| `--check --linux` | VERIFIED: exit 0 in 2 s, no message box | `r11-check-linux` |
| Settings window with the real folder | Same look as the README screenshots (the SDK changes only touch disabled rows, which a found folder doesn't show); not retaken | `r12-window` |
| Per-exe DLL overrides from a v0.2.0 prefix (`Applied::matching`) | NOT CHECKED: needs a prefix set up by v0.2.0; covered by the SDK's tests and `tests/wine_app_overrides.rs` | — |

Observations: srr-play in waiting mode (`SRR_WAIT=1`) exited 0 after the
launcher refused an empty folder with exit 1 (the error box did show);
the helper's exit code is the SDK's. The harness's own gamescope stayed
up to ~3 minutes after runs without a game (its child waits for one),
then exited; nothing of the launcher's was left.

## 2026-11-02 — the clean-room stutter fix (integration)

`patches::PACER` is now the clean-room set (`crates/srr-launcher/src/pacer.rs`,
`pacer/*.asm`; provenance in `docs/PACER-PROVENANCE.md`). On the owner's
rip (read-only), headless as above:

- `SRR_GAME_DIR=… cargo test real_game -- --ignored` and the env-gated
  `tests/pacer_real_game.rs`: clean `22655a54…` → **`cb9a5a34…ff65`**,
  405 bytes changed, all inside the declared sites, PE intact,
  idempotent, reversible, and combined with sets at the aspect offsets.
  VERIFIED.
- Launch with `--stutter-fix on --resolution 1920x1080`: `launcher.log`
  "SEGA Rally_SSE1.exe: patches pacer (SHA-256 cb9a5a34…ff65)"; the game
  reached the attract race (Canyon, then the Skoda replay) and ran 60 s
  until the supervisor's close request (gone in 0.39 s). VERIFIED,
  `keep/cr-pacer-060.jpg`.
- With 21:9 (`--resolution 2560x1080`): "patches pacer + aspect 21:9
  (SHA-256 d3f4a417…d47e)"; the attract race ran 60 s. VERIFIED,
  `keep/cr-pacer-219-060.jpg`.

Measurements by the clean-room implementer (their harness: the race clock
`0x9670D8` and the frame timestamp read from `/proc/<pid>/mem` once per
frame, attract demo race, DXVK in headless gamescope at 60 Hz, GPU
otherwise idle; an earlier batch on a shared GPU was discarded). Clean
build vs. clean-room build (`cb9a5a34…`), back to back per mode. Their
DXVK was 2.7.1 (vsync60 = `presentInterval = 1`; lim60/lim120 =
`maxFrameRate = 60/120` with vsync off; off = no limit):

| Metric (demo race) | Clean vsync60 | Fixed vsync60 | Clean lim60 | Fixed lim60 | Clean lim120 | Fixed lim120 | Clean off | Fixed off | Target |
|---|---|---|---|---|---|---|---|---|---|
| frames scored | 5404 | 5582 | 5631 | 5606 | 11203 | 11406 | 80869 | 102461 | ≥ 3000 |
| Δrace top values (ms) | 16, 32, 0 | 17, 16, 18 | 16, 32, 0 | 17, 16, 18 | 16, 0 | 8, 9, 7 | 0, 16 | 1, 0, 2 | |
| Δrace 16–17 ms (60 fps) | 93.2 % | 87.4 % | 93.1 % | 88.6 % | | | | | ≥ 99 % |
| Δrace 8–9 ms (120 fps) | | | | | 0 % | 84.8 % | | | ≥ 99 % |
| Δrace = 0 or ≥ 32 ms | 6.8 % | 0 % | 6.9 % | 0 % | 47.9 % | 0 % | (uncapped) | (uncapped) | 0 |
| \|Δrace − Δpresent\| p99 | 15.3 ms | **0.92 ms** | 16.0 ms | **0.90 ms** | 9.8 ms | **0.88 ms** | 15.2 ms | **0.89 ms** | ≤ 1.5 ms |
| \|Δrace − Δpresent\| ≤ 1 ms | 54.6 % | 99.87 % | 78.7 % | 99.98 % | 0.03 % | 99.96 % | 40.6 % | 99.94 % | |
| race clock backwards | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| snaps (>64 ms, no gap) | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Δpresent p50 (ms) | 16.94 | 16.95 | 16.66 | 16.66 | 8.32 | 8.33 | 1.06 | 0.82 | |
| Δpresent p99 − p50 (ms) | 1.12 | 1.55 | 1.36 | 2.15 | 2.82 | 2.83 | 1.66 | 1.51 | ≤ clean + 0.5 |
| presents 16–18 ms apart | 97.5 % | 95.8 % | 95.0 % | 88.6 % | | | | | |
| presents 8–10 ms apart | | | | | 71.6 % | 76.6 % | | | |
| handle replaced while live | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

Reading (theirs, condensed): the clean build moves the race clock only in
16-ms steps (0/16/32 ms at 60 fps). The fixed build follows each
presented frame's real time: p99 error 0.88–0.92 ms, within 1 ms on
99.87–99.98 % of frames, never backwards, no snaps. The "16–17 ms on
≥ 99 %" target was not met headless because the present cadence itself
varies there (88.6 % of presents 16–18 ms apart); it needs a real 60-Hz
display. The lim60 present-cadence spread (+0.79 ms) is one run each and
more likely DXVK limiter jitter; repeat runs are needed. One pacer (R1)
was not exercised: the attract loop never starts a second pacer.

Still for a person (not automatable headless):

1. Championship, stages 1 → 2 → 3 (frame limit 60): no stutter or time
   jump in the stage-2+ countdowns (the old judder); at most one pacer
   thread.
2. Pause mid-race for 10 s: the lap timer behaves as in the clean build.
3. Alt-tab (device lost) and a loading screen: no crash; the race clock
   jumps forward once, never backwards.
4. A full race with the same inputs against the community-fixed build:
   lap times within ±1 frame.
5. A real 60-Hz display with vsync: the 16–17 ms ≥ 99 % row.
6. Quit to the menu, a new Quick Race, quit the game: clean exit, no hang.

## 2026-11-05 — the rumble switch (issue #2)

Verified statically, end to end, on the clean `SEGA Rally_SSE1.exe`
(addresses in `docs/RESEARCH.md`, "Pad rumble"): the game parses
`[GameController] Mapping` at `0x589cd0`, keeps it at `0xA13450`, and its
rumble dispatcher (`0x66e2f8`) extracts the allow-vibration word and
gates the zero-output call `0x58b370` on it. The decompiled settings tool
confirms the field layout and the default (vibration on). The option's
edit is golden-tested (`gamecfg::tests::rumble_*`, plan end-to-end case,
snapshots: Written, Seeded and Foreign paths).

Verified live under Wine (sandbox: a two-exe game folder in /tmp, a
throwaway prefix, the freshly built `SRRLauncher.exe --setup`):
`--rumble off` seeds `Mapping` with the vibration field `0000` and the
owned record notes the previous value; the next `--setup` without the
option releases it and `Mapping` is back to `0001` with an empty owned
record. The log lines (`rumble: off …`, `config.ini: … rumble written`)
landed as designed.

Still to feel: no pad was connected to this box on 2026-11-05, so the
flip has not been *felt* in a race. First run with a pad: confirm the
pad is silent with `rumble = off` and rumbles with `on`, and note it
here.

## 2026-11-07 — Wine 11.21: the handheld startup crash (issue #3)

Two players on SteamOS-class handhelds crashed at
`ntdll.so+0x787c9` before any launcher code ran. Reproduced exactly on this
machine: Wine 11.20's host-address-space probe calls `mmap(..., 0x22,
...)` without `MAP_FIXED_NOREPLACE`, so the hint is ignored, the probe
breaks at `1 << 63`, and `pages_vprot` is sized to 32 GiB (plus a 32 GiB
reservation): **65 557 MiB** of VmPeak per Wine process, measured with
`wineboot --init`. Where that cannot be reserved (16 GB handheld), the
`calloc` returns NULL, 11.20's assert compares against `MAP_FAILED` and
passes for NULL, and the first `create_view()` dereferences it. Capping
the 11.20 process at 8 GiB reproduced the reporters' exact offset in
`coredumpctl`.

Wine 11.21 (upstream fix: `3c49b97b`, Wine-Bug 60528) measures **8 215
MiB**. The AppImage pin moves to 11.21 (Kron4ek amd64-wow64,
sha256 `33f3c5b5…fb9dc83`) through the SDK; `packaging/tests/wine-startup.sh`
now fails a build whose bundled Wine dies at startup or reserves more than
32 GiB (red on 11.20 at 64 GiB, green on 11.21).

Verified with the launcher: the 11.21 AppImage ran
`SRRLauncher.exe --setup --linux` end to end (prefix, config.ini,
registry, DXVK install, `setup complete`, exit 0).

Not yet verified: a driven race under 11.21 + DXVK. The fleet has been
sensitive to Wine version changes (an 11.13 upgrade black-screened the
game in the distrobox-gaming setup), so drive one race before tagging the
release.

## 2026-11-09 — borderless and keyboard keys (issues #4, #1)

Headless, in the `gaming` distrobox: release `SRRLauncher.exe` with the
AppImage v0.4.2's Wine 11.21 and DXVK, a throwaway prefix and `HOME`
under `.work/live/`, `env -i` with only the test's variables, no audio
device, `gamescope --backend headless` at 1920x1080, the owner's full rip
read-only as drive `D:`, XTest keys. Route to a race: title → Return →
Championship → Amateur → AZA Challenge → Safari → car → race.

| Check | Result |
| --- | --- |
| `--setup --display-mode borderless --keys accelerate=up --resolution 1280x720` | `SEGA Rally_SSE1.exe: patches pacer + borderless + keys`, `SEGA Rally.exe: patches borderless + keys`; the log names the keys |
| SSE1 (stutter fix on), in gamescope | VERIFIED: no `Setting display mode`; DXVK device 1280x720, swap chain 1920x1080; A held 4 s: 0 mph; Up held 5 s: 61 mph |
| NoDVD `SEGA Rally.exe`, in gamescope | VERIFIED: same, 61 mph on Up, 0 on A |
| SSE1 inside `explorer /desktop=SegaRally,1920x1080` (the plain-Wine path) | VERIFIED: swap chain 1920x1080, Up 47 mph after 5 s, A nothing |
| Clean SSE1 with borderless + keys and no pacer (caves past `.text` VirtualSize 0x284028) | VERIFIED: runs, Up 61 mph |
| Default settings (control) | DXVK `Setting display mode: 1920x1080@60`; A held 5 s: 58 mph |
| `SRR_GAME_DIR=… cargo test real_game -- --ignored` | VERIFIED: every set alone and together (pacer, aspect, borderless, keys) on both builds |

Not checked: native Windows, a pad and changed keys at once, a 21:9
screen in borderless mode.


## 2026-11-09 — Windows frame pacing and the black start (Quiver, issues #6, #5)

Headless in the `gaming` distrobox: this branch's `SRRLauncher.exe
--setup --linux` with Wine 11.21 and DXVK 3.1.1, a throwaway prefix
under `.work/`, `env -i`, `gamescope --backend headless` 1920x1080 at
60 Hz, the owner's full rip read-only as `D:`. The attract race (the
car driving itself) after 30 s, sampled for 60 s from `/proc/<pid>/mem`
every 0.2 ms: the frame timestamp `0x9E0D08`, the race clock `0x9670D8`
and the simulation thread's Clock-task sample `0x950930` (one per
simulation beat). "Stale frame" = a presented frame whose newest
simulation beat is older than 20 ms (more than one 16 ms step behind).

Windows' default timer cannot be had under Wine, so it was emulated in
measurement-only copies (never shipped): the simulation thread's
`Sleep(n)` returns at the first 64 Hz (15.625 ms) boundary at or after
now + n ms; a 1 ms grid stands for Windows with `timeBeginPeriod(1)`.

| Run (60 s, 3 751 beats, ~3 600 frames each) | beat p50 / p99 / max (ms) | beats > 24 ms | 0 / 1 / 2 beats per frame | stale frames per minute |
| --- | --- | --- | --- | --- |
| v0.5.0 under Wine (`pacer`) | 16.84 / 18.76 / 23.1 | 0 | 2.1 / 91.7 / 6.2 % | 1 |
| v0.5.0, Windows timer emulated | 15.61 / **31.35** / 32.0 | **92 (2.45 %)** | 2.9 / 90.1 / 7.0 % | **62** (35 over 24 ms) |
| v0.5.0, Windows with a 1 ms timer emulated | 16.00 / 20.97 / 21.3 | 0 | 3.8 / 88.3 / 7.9 % | 15 |
| `Sleep(1)` + 1 ms timer emulated | 15.80 / 17.80 / 27.4 | 1 | 1.6 / 92.7 / 5.7 % | 0 |
| **This release (`pacer + timer + video`) under Wine** | 15.92 / 16.97 / 17.2 | 0 | 0.4 / 95.1 / 4.5 % | **0** |

So on current Windows the stutter fix's own beat ran on a 15.6 ms grid
with a 31 ms gap every ~0.65 s; with the timer set it holds 16 ms within
about 1 ms everywhere (the residual 2-beat frames are the 62.5 Hz
simulation under a 60 Hz display, interpolated by the race clock).

- A relay trace (`RelayInclude=winmm.timeBeginPeriod;winmm.timeEndPeriod`)
  through title → attract race → the game's own quit: v0.5.0 makes
  **no** call; this release makes one `timeBeginPeriod(1)` from
  `0x685148` and one `timeEndPeriod(1)` from `0x68515C` per simulation
  thread, balanced, and the game exits by itself on Yes.
- Not checked: native Windows itself (the 15.6 ms behaviour is
  Microsoft's documented `timeBeginPeriod` model since Windows 10 2004),
  and Championship stage transitions with the timer set.

Black start (issue #5), no input at all for 20–40 s after the game
process appears:

| Run | frames presented, per 5 s |
| --- | --- |
| v0.5.0 in gamescope (Steam Game Mode's path) | 1 frame in 40 s; one Shift press → title, 863 frames in 15 s |
| v0.5.0 with the logo videos restored | 1 frame in 40 s (Wine's playback stalls too) |
| v0.5.0 in `explorer /desktop=…` (plain Wine) | 1 frame in 20 s |
| This release in gamescope | 196, 277, 300, 300: title "PRESS ANY BUTTON" |
| This release, borderless + keys, in the Wine desktop | 261, 282, 300, 300 |

`winedbg` on the black game: the main thread in `WaitForSingleObject`
from `0x4DC553` (the video frame wait); slot 0 =
`.\bootup\video\Rally\SEGA_Logo720p.wmv`, renderer set, end of stream 0;
Wine's trace: `reader_Open … error 2`, `MediaControl_Run Failed to pause,
hr 0x80070057`. `SRR_GAME_DIR=… cargo test real_game -- --ignored`:
every set alone and together (pacer, timer, video, aspect, borderless,
keys) on the owner's rip.
