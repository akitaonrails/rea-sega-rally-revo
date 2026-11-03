# Changelog

## Unreleased

- Updated rea-launcher-sdk to v0.4.1.
- **The settings window opens on the very first start under Wine.** On a
  new Wine prefix the window could stay invisible until the next start;
  the launcher now restarts itself once, right away, so it draws.
- **The game can't be started twice from the same copy.** If the launcher
  was closed or killed while the game kept running, the next start copied
  files over the running game and started a second one. It now says the
  game is already running and stops (exit code 5).
- **A second start only finds this launcher's window.** It looked for any
  window named "Sega Rally Revo", which a folder of that name open in
  Explorer could match too.
- **Errors under srr-play and Proton reach you.** Whether a message box is
  shown now follows the rule all rea launchers share: a game started by
  srr-play, the AppImage or Proton shows its errors in a box, while
  srr-play's own `--check` and settings runs leave the reporting to
  srr-play. A second `srr-play --play` while the game runs now exits with
  5 instead of 0.
- The stutter fix's "why is this greyed out" note now shows once, under
  its buttons, also for pad players who can't hover: "This folder has no
  clean SEGA Rally_SSE1.exe, so the game runs without the stutter fix."
- The README says plainly that the launcher runs the clean and no-DVD
  builds as they are and can't and won't remove DRM.

## v0.2.0 - 2026-11-02

- **The stutter fix is now our own clean-room implementation.** It was
  written from a specification of what the fix must do and the game's own
  code, without stuken's `patch.py` (which has no licence); stuken is
  credited for finding the bugs and showing the way. Besides the single
  pacer thread, the race clock now follows the real time of every frame
  at any frame limit (measured: within 1 ms on 99.9 % of frames at 60,
  120 fps and uncapped, never backwards). A `SEGA Rally_SSE1.exe` you
  patched yourself with stuken's fix is still recognised and run as it
  is. See `docs/PACER-PROVENANCE.md`.
- **No more "Player Progress Data Corrupt" / "Local Leaderboard Corrupt"
  on a first start.** The launcher created empty save and leaderboard
  files, which the game reported as corrupt; it now lets the game make
  them. Empty files left by an earlier release are removed at the next
  launch (the game recreates them); a save the game wrote is never
  touched.
- **Polish menus work.** Polish needs the registry's `sku` set to `polish`;
  the launcher wrote `Europe`, so `language = Polish` showed English menus
  (seen in the game). Choosing another language sets `Europe` again; with
  no language set, your own `sku` is kept.
- Updated rea-launcher-sdk to v0.3.3.
- **Bundled DLLs really load under Wine and Proton.** Wine ignores
  `WINEDLLOVERRIDES` set for the game by the launcher, so DXVK's `d3d9.dll`
  only loaded thanks to Wine's defaults and the downloaded DirectX 9 files
  were never used. The launcher now writes them as per-game overrides in
  the Wine registry (`AppDefaults\<game exe>\DllOverrides`, `native,builtin`);
  a value you set there yourself is kept, and its own values are taken
  back when the files go.
- Options also take `--flag=value` (`--vsync=on`), and a flag spelled with
  an underscore works too (`--stutter_fix off`, `--frame_limit 60`), as
  the keys are spelled in `launcher.cfg`. Switches such as `--no-window`
  refuse a value.
- **Errors reach the player under Wine and Proton.** An error that stops
  the launcher (wrong game folder, unknown game version, a failed setup)
  now always shows a message box when you started it, and is printed too,
  so `srr-play`'s `launcher.log` keeps it. Before, under Wine it only went
  to the log. `--no-window` runs from a script still get no message box.
- `--download-directx` while the settings window is open says "The
  launcher window is open; use its Download button" and exits with code 5.
  It used to bring the window forward and exit 0, which a script read as
  a finished download.
- DirectX 9 files already in the private copy are not written again at
  each launch. If they cannot be put there (a full disk, a locked file),
  `launcher.log` gets a warning and the game starts without them instead
  of the launcher refusing to play.
- The DirectX 9 row and `--download-directx` say that Microsoft's DirectX
  End-User Runtime licence applies, with a link to Microsoft's page for
  the package. The launcher unpacks the files without running Microsoft's
  installer, which is where Microsoft shows that licence.
- While the DirectX 9 files download or unpack, Play waits (the button
  says why), and closing the window asks first: a second close cancels the
  download, and the next one picks up where it stopped. Before, Play or
  closing cut the download off half way.
- **One launcher at a time.** Starting the launcher again while its window
  is open brings that window forward. While the game is running from it,
  a second start says so and doesn't start another game (exit code 5).
  `--check` still runs alongside.
- The DirectX 9 download shows its unpacking progress too (the SDK's
  downloader unpacks it now). The files now live in
  `directx\directx_Jun2010_redist\`. **If you used an unreleased build and
  downloaded them already, press Download again** (or run
  `--download-directx`): the old `directx\d3dx9\` folder is no longer read
  and can be deleted.
- The old `aspect` and `fullscreen` settings are now ignored without
  complaint, from `launcher.cfg` and on the command line.
  Saving from the window turns those lines into comments.
- `--help` lists the switches in the same column as the options.
- The Picture and Play tabs draw frame limit, language, quality and vsync
  with the SDK's option rows. Each description now sits under its row, and
  a screen reader hears why a disabled row is disabled.
- The game folder field is named "Game folder" for screen readers. The
  example path is now its description.
- CI: the SDK's lld and import checks replace `tests/imports.rs`, the
  x86_64 Windows target links with lld too, and a weekly workflow opens a
  pull request when a new SDK version is out.
- Issue forms: a **Bug report** form (test reports that everything works
  are welcome there too) and an **Unknown edition** form replace the test
  report. The message for an unknown game version links the Unknown
  edition form, and a link sends shared-code bugs to rea-launcher-sdk.
- `CLAUDE.md` and `AGENTS.md` point to the SDK's standards instead of
  repeating them. The stutter fix's clean-room rewrite is recorded as an
  owner decision.

- Golden snapshots (`crates/srr-launcher/tests/snapshots/`) of `--help`,
  the `launcher.cfg` template, and what each look and three option
  combinations write, so an SDK upgrade cannot change behaviour silently.
- Docs: the README covers the v0.2 options (one table for window, file and
  command line), the DirectX 9 download instead of winetricks, an
  Uninstall section that says where saves live and to back them up first
  (the old "Start over" deleted the Wine prefix with them), Building with
  the Wine tests, deny and the golden snapshots, Contributing (game
  issues here, shared code at rea-launcher-sdk) and Credits.
  `AGENTS.md` points to the SDK's standards and playbooks and records the
  owner's decisions; `docs/RESEARCH.md` records the community fixes'
  licences, the DirectX licence evidence, the trouble spots and the
  platform matrix; `docs/CONTRIBUTIONS.md` carries the drafts for
  PCGamingWiki and upstream.
- Packaging: DXVK and its licence are pins in `packaging/third-party.txt`
  (SDK `pins.sh`), with this repository's `thirdparty` release as the
  fallback; `.github/workflows/mirror.yml` fills that release.
  `packaging/dxvk.txt` is gone.
- The test-report template asks for the executables' SHA-256 lines from
  `launcher.log`.
- Settings window messages say how to fix the problem: a path that is no
  folder points at Browse…, a settings file that cannot be saved names
  it.
- **`srr-play` no longer runs winetricks.** It used to install `d3dx9`,
  `directplay` and `d3dcompiler_47` into the prefix when winetricks
  happened to be installed (a silent, unrequested download). Wine's own
  DirectX 9 pieces were enough in our runs, including the recorded race,
  which ran without winetricks; the DirectX 9 files are the window's
  Download button now. `SRR_WINETRICKS` is gone. Prefixes that already
  have them keep them.
- **One option table for the window, `launcher.cfg` and the command
  line.** `--help`, the first-run `launcher.cfg` and the window's labels
  and descriptions come from it, and a test fails if the window and the
  command line drift apart. Every option now has a plain one-line
  description in the window and the template.
  - Keys are spelled with underscores: `frame_limit`, `stutter_fix`. Files
    with the old `frame-limit` / `stutter-fix` keep working (and the flags
    are the same); saving from the window writes the new names and turns
    the old lines into comments.
  - The window writes `on`/`off` (it wrote `true`/`false`, which are still
    read).
  - New `--look modern|steam-deck|original|custom`: sets the look's
    resolution, quality and vsync for one run (other flags still win).
  - `--resolution auto` (and `resolution = auto`) uses the screen's size.
  - Error messages name the flag, the value and what it takes.
- **The aspect choice is gone; the aspect follows the resolution.** The
  16:9 and 16:10 chips changed nothing (the game picks between them from
  the resolution), and 21:9/32:9 only made sense with an ultra-wide
  resolution. Now an ultra-wide resolution gets the aspect fix by
  itself, and the Picture tab says what the game will run at. An old
  `aspect = …` line and `--aspect` are ignored (see "The old `aspect` and
  `fullscreen` settings" above). If you had forced 16:9 on an ultra-wide
  resolution, the picture is now unstretched instead.
- **The Picture tab** is regrouped (Screen: resolution and what it means;
  Detail: quality and vsync), with a description under each control.
- **Patches only go into identified executables.** The stutter fix and
  the ultra-wide aspect fix are site tables for the exact builds (SDK
  `PatchSet`), applied from the game folder's untouched executable every
  launch. Before, the aspect tables were found by a byte pattern in any
  executable, including unknown ones. An unknown executable is never
  patched; the error (and the window) show its SHA-256 and the issue link
  to report it, with launcher.log attached.
- **`config.ini` keeps your in-game choices.** A missing file is still
  seeded complete. After that only the keys of options set in
  `launcher.cfg` are written (resolution, quality, vsync) and recorded;
  when one goes back to unset, its key is restored only if it still
  holds the launcher's value. Earlier releases rewrote all 25 `[Graphics]`
  keys every launch. The menu language is written to the registry only
  when `language` is set (English stays a default for a missing value).
- **`--check`, `--help` and errors print to the terminal** the launcher
  was started from (a message box only when there is none). `--check`
  exits with 1 when the folder cannot be played. `launcher.log` records
  each executable's SHA-256, the patch sets applied with the result's
  hash, and DXVK's hash.
- **Microsoft DirectX 9 files: one button.** The Play tab has a Download
  button (and `--download-directx`) that fetches Microsoft's DirectX
  End-User Runtime (June 2010) from download.microsoft.com, pinned by
  SHA-256, only when you press it; the D3DX9 and D3DCompiler DLLs go next
  to the game in the private copy (never over a file of yours) and Wine
  is told to use them. Windows 10/11 lack these files; under Wine the
  built-in ones were enough in our runs. Remove takes them out again.
- The photo's credit lines sit on a solid band (no small text over the
  picture).
- **Updated rea-launcher-sdk to v0.2.0.** The settings window's control
  borders are lighter (`#5475a1` instead of `#1f4a84`) so they reach the
  3:1 contrast that WCAG asks of controls; text selection in fields keeps
  the half-strength orange. Windows-target tests also run under Wine in
  CI. `linux/tests/*.sh` load `srr-play` through the SDK's test
  convention (`REA_FUNCTIONS_ONLY=1 REA_PLAY_HERE=…`); the old
  `SRR_PLAY_FUNCTIONS_ONLY` hook is gone.
- **Built on the shared rea-launcher-sdk v0.1.0.** The launcher's
  generic code (runtime copy, config editing, Win32 helpers, the settings
  window's widgets, the Linux helper's session handling, AppImage
  packaging and CI) now comes from the SDK the five rea launchers share;
  what you see and set stays the same. Small differences:
  - `launcher.log` starts afresh once it passes 1 MiB.
  - Settings window: a row of choices is one stop for the d-pad
    (left/right changes the value), focused controls show a ring, and
    the Browse/Test rumble labels are white on the orange outline.
  - `launcher.cfg`: a `#key =` line in the file becomes the setting
    instead of a new line at the end; a file saved by Notepad in ANSI is
    read instead of being treated as missing.
  - Native Windows without admin rights: the game's registry values go to
    the 32-bit VirtualStore location (`WOW6432Node`), and only when HKLM
    is refused; the screen size is the monitor's real resolution.
  - `srr-play`: the `GPU:` line shows only when something starts; the old
    test-only `SRR_ICD_DIR`/`SRR_SYSFS` variables are gone.
  - The AppImage also carries the README, licences and changelog.
- **Controller behaviour change on Linux.** The controller bridge now
  really disables Wine's raw hidraw/evdev devices. `srr-play` used to
  write `Disable Hidraw`/`Disable Input` (with a space), names Wine never
  reads, so every earlier release ran with those devices enabled. It now
  writes `DisableHidraw`/`DisableInput` (the documented design, and the
  values the OutRun 2006 launcher was tested with), only when absent, and
  `SRRLauncher.exe` seeds the same values itself, so it also works when
  Proton runs the launcher directly. Not yet checked with a real pad. If
  your pad stops working or appears twice, switch them back off (the
  launcher keeps values that already exist):

  ```sh
WINEPREFIX=~/.local/share/srr-launcher/prefix wine reg add \
  'HKLM\System\CurrentControlSet\Services\winebus' /v DisableHidraw /t REG_DWORD /d 0 /f
WINEPREFIX=~/.local/share/srr-launcher/prefix wine reg add \
  'HKLM\System\CurrentControlSet\Services\winebus' /v DisableInput /t REG_DWORD /d 0 /f
  ```

  For Steam's Proton, run the same with `WINEPREFIX` set to the game's
  `compatdata/<appid>/pfx`.
- `srr-play` no longer pins the NVIDIA Vulkan driver on hosts with a
  leftover NVIDIA ICD but no NVIDIA GPU (that left DXVK with no device). It
  also finds ICDs in `/etc/vulkan/icd.d`, sets `VK_DRIVER_FILES` next to the
  deprecated `VK_ICD_FILENAMES`, and keeps a `VK_DRIVER_FILES` you set.
- `srr-play` owns its game session: the lock records pid and start time
  (a reused pid no longer reads as "already running"), `stop` signals only
  that session's process group before shutting down the prefix's Wine
  server, and a stray game process no longer blocks a start. The launcher
  path is passed to the generated `inner.sh` as an argument, so a folder
  name with a quote can no longer break or inject into it. Ctrl+C on a
  waiting launch (the AppImage) waits for the session's own cleanup, and a
  lock left by v0.1.x (pid only) with a live pid still reads as running,
  so upgrading mid-session does not start a second game.
- The GitHub test-report template was copied from the DiRT launcher; it now
  asks about Sega Rally Revo (`SRRLauncher`, `srr-play`, which executable).
- Look cards: Custom now stays selected when clicked (it used to jump back
  to the preset the values matched), and on a Steam Deck the Steam Deck
  look is recommended, picked on first run and shown as selected (it
  showed as Modern, whose values are the same at 1280×800). The
  "Recommended" badge follows the screen instead of always marking Modern.
  The window also saves the chosen look (`look = …` in `launcher.cfg`, read
  only by the window), so on a Deck a saved Modern reopens as Modern; a
  hand edit that no longer matches the preset shows as Custom.
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
- On Windows, replacing `config.ini` retries for half a second when
  antivirus or OneDrive briefly holds the file.
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
  the environment (also spelled `d3d9.dll`, in any case) cannot swap DXVK
  for Wine's own d3d9; every other rule you set is kept. Wine is also recognised from `WINEPREFIX`,
  `WINELOADER` or `STEAM_COMPAT_DATA_PATH` when a Wine build hides
  `wine_get_version`.
- The settings window's first-run defaults (the recommended look) apply
  only when `launcher.cfg` is missing or blank; a saved file that happens
  to match the template is read as the player's settings.
- CI builds and tests with `--locked`; the DXVK licence is fetched from the
  pinned v3.1.1 commit and checked against its SHA-256, the DXVK folder is
  derived from `packaging/dxvk.txt` instead of hard-coded, and releases
  carry a `SHA256SUMS` file.
- Under Wine with a raised DPI (`winecfg` 144 or 192) the settings window
  came out 2.25× or 3× its size on a 4K screen, taller than the screen.
  Wine's own DPI scale is now divided out of the window's starting zoom.
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
