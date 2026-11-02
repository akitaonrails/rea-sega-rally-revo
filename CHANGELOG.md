# Changelog

## Unreleased

- The controller bridge now actually disables Wine's raw hidraw/evdev
  duplicates: `srr-play` wrote `Disable Hidraw`/`Disable Input` (with a
  space), names Wine never reads. It now writes `DisableHidraw`/
  `DisableInput`, only when absent, and `SRRLauncher.exe` seeds the same
  values itself, so it also works when Proton runs the launcher directly.
- `srr-play` no longer pins the NVIDIA Vulkan driver on hosts with a
  leftover NVIDIA ICD but no NVIDIA GPU (that left DXVK with no device). It
  also finds ICDs in `/etc/vulkan/icd.d`, sets `VK_DRIVER_FILES` next to the
  deprecated `VK_ICD_FILENAMES`, and keeps a `VK_DRIVER_FILES` you set.
- `srr-play` owns its game session: the lock records pid and start time
  (a reused pid no longer reads as "already running"), `stop` signals only
  that session's process group before shutting down the prefix's Wine
  server, and a stray game process no longer blocks a start. The launcher
  path is passed to the generated `inner.sh` as an argument, so a folder
  name with a quote can no longer break or inject into it.
- The GitHub test-report template was copied from the DiRT launcher; it now
  asks about Sega Rally Revo (`SRRLauncher`, `srr-play`, which executable).
- Look cards: Custom now stays selected when clicked (it used to jump back
  to the preset the values matched), and on a Steam Deck the Steam Deck
  look is recommended, picked on first run and shown as selected (it
  showed as Modern, whose values are the same at 1280×800). The
  "Recommended" badge follows the screen instead of always marking Modern.
- `config.ini`: a `[Graphics]` key written with spaces or another case
  (`HorizontalResolution = 1280`) is now updated in place instead of being
  left alone with a second copy appended; duplicates an earlier version
  appended are removed. Comments, unknown keys, CRLF line endings, and the
  `[Audio]`/`[GameController]` sections are kept, and the file is replaced
  atomically (no half-written file after a crash).
- A `config.ini` that could not be read as UTF-8 (a controller name in
  another encoding) or not at all was replaced by a fresh seed, losing the
  player's `[Audio]` and `[GameController]` sections. Only a missing file
  is seeded now; an unreadable one is left alone, and the file is edited
  byte for byte, so names in any encoding survive. A UTF-8 byte-order mark
  no longer hides `[Graphics]` on the first line, and is kept.
- The stutter fix checks its result against the known patched SHA-256
  before writing the private exe, and refuses (writing nothing) on a
  mismatch.
- The runtime folder is refused inside the game folder even before it
  exists or when reached through `..` or a symlink (the nearest existing
  parent is resolved). A drive that cannot be canonicalized (Windows RAM
  disks, `subst` drives, some network shares) is compared lexically
  instead of stopping the launch.
- Syncing the game folder never overwrites the launcher's own runtime
  files: `dxvk.conf` and the launcher's bookkeeping always, and `d3d9.dll`
  while the launcher manages DXVK (Linux, or its marker is present). On
  native Windows a `d3d9.dll` of your own (ReShade, dgVoodoo, DXVK for
  Windows) is still copied and kept up to date, and is restored when you
  come back to Windows after a Linux run.
- With the bundled DXVK (srr-play, AppImage) the game starts with
  `WINEDLLOVERRIDES` carrying `d3d9=n,b` first, so a `d3d9=b` elsewhere in
  the environment cannot swap DXVK for Wine's own d3d9; every other rule
  you set is kept. Wine is also recognised from `WINEPREFIX`,
  `WINELOADER` or `STEAM_COMPAT_DATA_PATH` when a Wine build hides
  `wine_get_version`.
- The settings window's first-run defaults (the recommended look) apply
  only when `launcher.cfg` is missing or blank; a saved file that happens
  to match the template is read as the player's settings.
- CI builds and tests with `--locked`; the DXVK licence is fetched from the
  pinned v3.1.1 commit and checked against its SHA-256, the DXVK folder is
  derived from `packaging/dxvk.txt` instead of hard-coded, and releases
  carry a `SHA256SUMS` file.
- Disabled choice chips (stutter fix without the clean exe or before a
  folder is chosen, frame limit on native Windows) still outline the saved
  value and say why on hover.

## v0.1.1 - 2026-11-02

Fixes from an audit of every option against its wiring, tests and docs:

- Ultra-wide aspect (21:9, 32:9) no longer breaks the stutter fix from the
  second launch on. The patched aspect tables changed the exe's hash, so
  the launcher refused its own copy. Auto picks 21:9 on 3440×1440 screens,
  so ultra-wide users hit this with default settings.
- The settings window's gamepad buttons now do what the window says: d-pad
  moves, A changes a setting, LB/RB switch tabs, Start plays (A used to
  start the game and the bumpers did nothing).
- The frame limit now applies under any Wine or Proton, including Steam
  starting `SRRLauncher.exe` directly. On native Windows, which has no DXVK,
  the buttons are disabled with an explanation.
- Removed the Fullscreen option: the game ignores windowed mode, and turning
  it off under Wine skipped the isolated Wine desktop that prevents the
  multi-monitor crash. `--fullscreen` is now an unknown option; old
  `fullscreen` keys in `launcher.cfg` are ignored.
- `language` in `launcher.cfg` is validated like `--language`.
- The window no longer saves the stutter fix as off when the chosen folder
  lacks the clean exe.
- `--settings` reports other options instead of ignoring them.
- Look cards no longer promise a 60 fps cap the presets do not set; the
  rumble test and aspect texts say what they actually do.
- Normal and Ultra quality were run on the real game for the first time
  (both render at 4K).
- New README: what the launcher fixes, install, first run, every option
  across the window, `launcher.cfg` and the command line, and screenshots
  of each tab.
- Removed dead code (an unused key list, an unused config reader in
  `srr-play`).

## v0.1.0 - 2026-11-01

First release. A plug-and-play launcher for the user's own Sega Rally Revo
(PC v4.0.6.0) with the whole fix stack wired in:

- Recognized editions by exact SHA-256 (clean `SEGA Rally_SSE1.exe` and
  the full-rip NoDVD `SEGA Rally.exe`); unknown executables rejected.
- Community pacer/stutter fix reimplemented in Rust and applied to the
  private SSE1 copy (byte-verified, golden-tested, idempotent).
- Aspect-table patching for 21:9 and 32:9; 16:9 and 16:10 native.
- config.ini writer replacing the unreliable-under-Wine .NET launcher,
  preserving the player's controller sections; saved-game layout and
  registry seeding; bootup logo videos disabled in the private copy.
- DXVK 3.1.1 installed into the private copy with a `d3d9.maxFrameRate`
  limit (60 default, 120/Off optional).
- egui settings window (SEGA navy + Revo orange) with Look presets,
  timing options, language, and full pad navigation; gamepad rumble test;
  force-feedback crash warning.
- `srr-play` Linux wrapper: private Wine prefix, winebus controller bridge
  seeding, optional winetricks bootstrap, NVIDIA Vulkan ICD pinning on
  hybrid-GPU hosts, isolated Wine desktop (no gamescope), single-instance
  guard, stop/status.
- AppImage packaging with pinned portable Wine 11.20 WoW64; CI builds,
  tests, lints, and publishes Windows zip + AppImage.

Fixed before release, found by the first real runs (see
`docs/RUNTIME_NOTES.md`):

- The Windows exe is linked with LLD; GNU ld left five `kernel32`
  imports unresolved and the launcher crashed on its first Win32 call. CI
  now checks every import of the built exe.
- The stutter fix recognizes the clean SSE1 copy (it refused every launch).
- Bootup logo videos stay disabled after the first launch.
- The settings tabs scroll instead of running into the footer.
- `srr-play` ignores zombie game processes, and says why when the settings
  window closes without starting the game.
- The AppImage runtime is pinned to type2-runtime 20251108 instead of the
  rolling `continuous` build.
