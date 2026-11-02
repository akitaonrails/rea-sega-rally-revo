# Research: Sega Rally Revo (PC, 2007) on modern systems

Snapshot: 2026-11-01. Sources: the local documented install in
`distrobox-gaming` (`docs/pc-racing.md`, the `sega-rally-revo` catalog
entry, its git history), the community patches and wikis linked below, and
byte-level verification against the game executables on this machine.
This file records *why* the launcher does what it does.

## The game

| | |
| --- | --- |
| Title | Sega Rally Revo (aka Sega Rally in EU) |
| Developer / publisher | Sega Racing Studio / Sega |
| Release | 2007-09-27 (PC: 2007) |
| API | Direct3D 9, 32-bit executables |
| Known PC build | file version 4.0.6.0 (the patched DVD release; the `v4.060` patch introduced the frame-pacing problems — PCGamingWiki) |

Executables in the widely mirrored EN-FR-DE-ES-IT-PL full-rip
(myabandonware lineage, the source this machine's install uses):

| File | SHA-256 | Notes |
| --- | --- | --- |
| `SEGA Rally.exe` (3,715,072 B) | `ad84ecea…2ba391d7` | NoDVD build; playable; no clean reference for community binary patches |
| `SEGA Rally_SSE1.exe` (3,727,360 B) | `22655a54…a5aadf76e` | **Byte-identical to the clean original** (MD5 `4e71704338c6d7f3ef57ea81fe5b015d` — exactly the reference build of the community stutter fix) |
| `SEGARallyLauncher.exe` (1,787,136 B) | `4a695903…2694a7a147b` | The .NET settings launcher; unreliable under Wine — replaced by this launcher |

Verified on this machine: all patch sites, cave space, and aspect tables
described below match these exact bytes.

## Install hurdles (each one was a real failure on the reference host)

From the `distrobox-gaming` install history (2026-05-01 through 2026-08-21):

1. **Registry gate.** The game reads
   `HKLM\SOFTWARE\WOW6432Node\Sega\SEGA Rally` (`installdir`, `sku`,
   `language`) and refuses to behave without it. The rip ships a
   `SEGA Rally.reg` pointing at `C:\Sega Rally\`. *Wired by:* the launcher
   seeds these values, pointing `installdir` at its private runtime copy
   (HKCU VirtualStore fallback on non-elevated native Windows).
2. **The .NET settings launcher is unreliable under Wine**, so
   `Documents\SEGA Rally\Saved Games\config.ini` must exist before launch.
   *Wired by:* the launcher writes config.ini itself (the `[Graphics]`
   keys it manages, preserving the player's `[Audio]`/`[GameController]`
   sections with the controller GUID and mapping).
3. **Missing saved-game layout crashes the main menu** (access violation
   after reaching the menu): the game expects
   `Documents\SEGA Rally\Saved Games\SEGA Rally.dat` and
   `Public Documents\SEGA Rally\Shared Data\Rank Table\SR_RANKTABLES.DAT`
   (plus `Ghosts/`) to exist. *Wired by:* the folders are created when
   missing. Empty seed files were dropped on 2026-11-02 (the game called
   them corrupt); the game creates both files itself (see RUNTIME_NOTES).
4. **Offline DirectX runtime needed.** The repeatable main-menu access
   violation was stopped by installing the June 2010 DirectX redist plus
   `d3dx9`, `directplay` into the prefix (the rip ships only
   `dxwebsetup.exe`). *Since 2026-11-02:* the owner's first AppImage run
   and the recorded race used a fresh prefix **without** winetricks, so
   Wine 11.20's built-in d3dx9/dplayx sufficed there; the old crash may
   have come from the then-missing saved-game layout (#3). *Wired by:* a
   Download button for Microsoft's D3DX9 files (see "DirectX 9 files"),
   which Windows 10/11 need; `srr-play`'s winetricks step is gone.
5. **Startup looks stuck on the logo videos.** Wine's Quartz/GStreamer
   path spends a long time on `Bootup/video/Rally/SEGA_Logo720p.wmv` and
   `SRS_Logo_1.wmv`. *Wired by:* renamed out of the way in the runtime
   copy (`.srr-disabled`).
6. **Exclusive fullscreen crashes on multi-monitor hosts.** The game
   ignores `RunWindowed=1` and requests exclusive 3840×2160@60; Wine turns
   that into a real XRandR `RRSetCrtcConfig` which the host rejects when
   the target monitor runs 240 Hz ("stopped loading" at swapchain
   creation). `UseXRandR=N` does not stop it. *Wired by:* under plain Wine
   the launcher always runs the game inside an isolated Wine desktop at
   the chosen resolution (the 2026-08-21 fix, verbatim).
7. **DXVK on hybrid GPUs can land on the integrated GPU** — the process
   runs (~13% CPU, DXVK threads up, audio mixing) but presents only black
   frames. Gamescope makes this worse (it routes Vulkan to the iGPU — the
   reason this launcher never wraps the game in gamescope). *Wired by:*
   `srr-play` pins the NVIDIA Vulkan ICD when an NVIDIA ICD coexists with
   another vendor (verified fix, 2026-08-08) and an NVIDIA GPU is present
   (PCI vendor `0x10de` or the `nvidia` module loaded); Intel/AMD hybrids fall back
   to `DRI_PRIME=1`.
8. **Controller bridge.** The game needs Wine's SDL-backed XInput device:
   `Map Controllers=1` with raw evdev/hidraw duplicates disabled
   (`DisableHidraw=1`, `DisableInput=1`, `Enable SDL=1`). *Wired by:*
   `SRRLauncher.exe` seeds the absent winebus values under any Wine or
   Proton, and `srr-play` seeds them before Wine boots on a new prefix.
   (Before Unreleased, `srr-play` wrote `Disable Hidraw`/`Disable Input`
   with a space, which Wine ignores.) The pad must be in
   X-input mode with a real `/dev/input/js*` device.
9. **Force feedback crashes the game** (Steam community reports on
   Windows 10; the launcher's controller options "allow force feedback"
   tick). *Wired by:* the GUI warns to keep it off; nothing enables it.
10. **A wine upgrade once regressed the game to a black screen**
    (11.12→11.13, 2026-07-12; pinned to a wine-11.8 sidecar). The later
    iGPU finding (#7) is the more likely true cause. This launcher bundles
    the same pinned Kron4ek Wine 11.20 WoW64 build the other REA launchers
    use, so system wine upgrades cannot change the runner mid-flight.

## Timing: the two stacked stutter bugs

- Symptom A (every mode, from the first race): steady 60 fps but a frame
  drops/duplicates roughly every 26–27 frames. Root cause: the master
  clock accumulates QPC microseconds correctly, but consumers truncate to
  integer milliseconds, so the per-frame step alternates 16/17 ms instead
  of a steady 16.67 ms. The game renders "exactly 62.5 frames every
  second… drawn in the incorrect order" (NeoGAF analysis, 2016).
- Symptom B (Championship only, from stage 2's countdown): a pacer thread
  is spawned unconditionally — twice per stage transition — so two pacers
  integrate the same race clock; the stale one wakes hundreds of
  milliseconds behind and snaps the clock (visible judder in countdown,
  timers, camera, HUD).
- Community workarounds: the 0.96 clock-scale speedhack (fixes A,
  exposes B); DXVK `d3d9.maxFrameRate = 60` (the Lutris installer
  standard); "enable high frame rate" (PCGamingWiki: also fixes the odd
  frame pacing at 60 Hz; Championship always caps at 62.5 fps).

**The proper fix** is [stuken/SegaRallyRevoStutterFix](https://github.com/stuken/SegaRallyRevoStutterFix):
three code caves in the clean `SEGA Rally_SSE1.exe` (whose reference MD5
matches our rip's file byte-for-byte):

1. *Pacer spawn guard* — at most one pacer thread (fixes B).
2. *Race-clock tick samples the master clock* instead of `+16 ms` (fixes A
   at the source).
3. *Main loop refreshes the race clock every frame* (makes A impossible
   even if a pacer stalls).

**Clean-room rewrite (owner decision, 2026-11-02).** The current caves
derive from stuken's unlicensed `patch.py` (the repository has no licence),
so they are being replaced by a clean-room implementation: a spec of what
the three changes must do, written from the behaviour above, then an
independent implementation from that spec alone. Until it lands the
current table stays as it is. Docs and issues describe the fix by
behaviour, never by its cave bytes.

This launcher reimplements that patch in Rust (`patches::pacer`), with the
same self-checks (clean hook-site bytes, zero-filled caves, size guard,
overlap guard) and the same normalisation sites. Golden bytes from the
reference assembly are pinned in the unit tests; applying the patch to the
real game file reproduces the reference SHA-256
`4c08c1de…f1ad08f2520` (verified on this machine).

A second patch, [Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix](https://github.com/Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix)
(P1: two timer initializers 15,999 µs → 0, four bytes), targets a
different exe build (3,700,786 bytes, SHA-256 `246b4a63…12db39`) and is
not applicable to ours; not shipped.

## Aspect ratio

- The game's renderer carries aspect tables in `.rdata`: two layouts
  flanking the 16:9 constant `39 8E E3 3F` (1.7777778f32) with 4:3
  (`ab aa aa 3f`) and 16:10 (`cd cc cc 3f`). In our builds the tables sit
  at file offsets `0x2F7C44`/`0x2FADA8` (SSE1, clean and stutter-fixed)
  and `0x2F4BF4`/`0x2F7D68` (NoDVD main) — PCGamingWiki's fixed offsets
  belong to a different build. Until v0.1.x the launcher found them by
  that layout pattern in any executable; since v0.2 they are exact
  `PatchSet` sites per build (`patches::ASPECT_*`), so an unknown build is
  never touched.
- 16:9 and 16:10 are native: the game picks between them from the
  resolution, so the old 16:9/16:10 chips changed nothing (owner decision
  2026-11-02: folded into the resolution). 21:9 (64:27, `26 B4 17 40`) and
  32:9 (`39 8E 63 40`) replace the 16:9 slot in both tables when the
  resolution is ultra-wide — the PCGamingWiki/WSGF hex edit.
- **Not verified**: no 21:9 or 32:9 screen has been tried
  (`docs/RUNTIME_NOTES.md`).

## Frame limiting

DXVK's `d3d9.maxFrameRate` (in `dxvk.conf` next to the exe) is the
community-standard pacing guard. Default 60; 120 and Off are offered for
high-refresh displays once the pacer fix is on. Championship mode caps
itself at 62.5 fps regardless (engine-side).

## Quality presets

The original .NET launcher's own enum is LOWEST/LOW/NORMAL/HIGH (0–3), and
its key list confirms the config.ini schema (including `RefreshRate`,
`ShaderVersion`, `ResolutionFilter`). This launcher maps:

- **Normal** — every slider at NORMAL (2), era-typical.
- **High (recommended)** — the exact configuration proven on the reference
  4K/DXVK/60 Hz setup (GraphicQuality 3, Shadow 2, Reflection 2, Track 3,
  Texture 2, Shader 1, DrawDistance 2, Particle 2, RearViewMirror 2,
  Mud 1).
- **Ultra** — every slider at HIGH (3).

Anti-aliasing stays off (`AntiAliasingLevel=0`), matching the proven
configuration; multisample stability under DXVK is untested.

## Community fixes: licence and maintenance

| Project | What it fixes | Licence | Maintained | Decision |
| --- | --- | --- | --- | --- |
| [stuken/SegaRallyRevoStutterFix](https://github.com/stuken/SegaRallyRevoStutterFix) | both stutter bugs, clean SSE1 v4.0.6.0 only (`patch.py`) | **none** (no licence file; README and `patch.py` state no terms; checked 2026-11-02) | yes (last push 2026-09-12) | reimplemented as `patches::PACER`; its cave machine code equals the fix's output. **Owner decision 2026-11-02:** the current caves derive from stuken's unlicensed `patch.py`; they are being replaced by a clean-room implementation (a written spec of the behaviour, then an independent implementation, done separately). The U1 licence request (`docs/CONTRIBUTIONS.md`) stays a draft. |
| [Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix](https://github.com/Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix) | timer initialisers 15,999 µs → 0 (P1), HTML/Python patcher | **none** (no licence file, checked 2026-11-02) | yes (last push 2026-10-21) | not used: another build (3,700,786 bytes). Nothing taken from it. |
| DXVK (`d3d9.maxFrameRate`) | Linux/Wine renderer, frame limiter | Zlib | yes | bundled (pinned, mirrored) |
| Widescreen Fixer / WSGF hex edit | ultra-wide aspect | WSGF technique (no code) | — | technique reused at our builds' offsets |

## DirectX 9 files: licence evidence (2026-11-02)

The game links one of Microsoft's D3DX9 DLLs (`d3dx9_NN.dll`, part of the
DirectX "optional components"); Windows 10/11 do not include them, and
Windows Update never installs them.

- **The only package with these DLLs** is Microsoft's DirectX End-User
  Runtime (`directx_Jun2010_redist.exe`, today served re-signed:
  100,275,120 bytes, SHA-256 `053f76dc…dbf89c73b`; winetricks still pins
  the older signature `8746ee1a…f011780c0d`). Its licence text (in its
  `dsetup32.dll`, shown by its installer DXSETUP) — *Microsoft Software License Terms,
  Microsoft DirectX End User Runtime* — grants "install and use any number
  of copies … on your devices" and forbids to "publish the software for
  others to copy" and to "transfer the software … to any third party".
  So we cannot extract its DLLs at package time and ship them.
- **The DirectX SDK's REDIST.TXT** lists `D3DX9_xx.dll` as redistributable,
  but only to SDK licensees and, per Microsoft's own deployment guidance,
  through DirectSetup: "Using the legacy DirectX SDK requires the use of
  the deprecated DirectSetup for components such as D3DX9…"
  ([DirectX Installation for Game Developers](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/directx-setup-for-game-developers)).
  The project has not accepted the SDK licence, and DirectSetup writes to
  System32 (admin rights). The one app-local D3DX Microsoft licenses,
  the `Microsoft.DXSDK.D3DX` NuGet package, holds only `d3dx9_43.dll`,
  which a 2007 game does not load.
- **Decision**: the window's Download button (and `--download-directx`)
  fetches the runtime package from download.microsoft.com on the
  player's request, pinned by SHA-256; the player's own launcher takes the
  32-bit `d3dx9_24…43.dll` and `D3DCompiler_*.dll` out of its cabinets
  (the `cab` crate) and puts them next to the game in the private copy
  (no admin, nothing in System32), with `n,b` Wine overrides. DXSETUP
  never runs, so it never shows the licence; the window's DirectX row and
  the `--download-directx` output carry the line "Microsoft's DirectX
  End-User Runtime licence applies" with a link to Microsoft's page for
  the package (`microsoft.com/en-us/download/details.aspx?id=8109`). Which
  `d3dx9_NN` the game imports was not checked (no executable on the
  analysis host); all versions are extracted (~70 MB).
- **DirectPlay** is not in that package (winetricks takes it from the
  February 2010 package's `dxnt.cab`); on Windows it is an optional Windows
  feature, and it is COM-registered, so it cannot sit in the game folder.
  Wine ships built-in `dplayx`/`dpnet`, and the runs without winetricks
  reached the menus and a race; no multiplayer was tried. Nothing is
  wired for it; Windows offers to enable it if the game asks.
- `d3dcompiler_47` (winetricks' third verb) was a DXVK safeguard from DXVK
  issue #3320 (falling trees, fixed by 2023 drivers); not needed for d3d9.

## Old-game trouble spots

| Spot | Status for this game |
| --- | --- |
| Timers / frame pacing on fast CPUs | the two stacked stutter bugs above; stutter fix + DXVK limit |
| Multi-core affinity | no problem seen on a 16-core 7950X3D |
| DirectSound3D / EAX | not analysed; sound worked under Wine in the recorded race; native Windows not tried |
| CD audio | none (music is on disk) |
| Intro-video codecs | the two WMV logo videos stall Wine's GStreamer path; switched off in the private copy |
| Resolution and aspect tables | any resolution via config.ini; ultra-wide via the aspect sets |
| Window losing focus | exclusive fullscreen; under Wine the isolated desktop keeps it contained; not tested on native Windows |
| Disc check | the SSE1 build avoids it (PCGamingWiki); the NoDVD main build is a supported edition (owner decision) |

## Platforms

| Platform | Status |
| --- | --- |
| Linux AppImage, 4K desktop (Hyprland, RTX 5090) | verified: menus, a full race with a pad (owner's recording) |
| Headless gamescope 4K | verified: title, attract demo, quality presets |
| Steam Deck Game Mode (1280×800, controller, suspend/resume) | **not verified** |
| Steam Proton running `SRRLauncher.exe` directly | **not verified** (frame limit through Proton's DXVK) |
| Native Windows 10/11 | **not verified** (DirectX Download path, VirtualStore registry) |
| Multi-monitor | the isolated Wine desktop exists for it (verified 2026-08-21 in distrobox-gaming); native Windows not tried |
| HiDPI / 4K | verified on Linux; Wine DPI 144/192 window fit unit-tested only |
| Ultra-wide 21:9 / 32:9 | **not verified** |

## Known-but-not-shipped

- Widescreen Fixer (aspect on the fly), the Martijnsx P1 patch (different
  build), the falling-trees DXVK bug (fixed by 2023 driver updates —
  DXVK issue #3320), 62.5 fps championship cap (engine-side).
- The `SSE1` executable name refers to the SSE1 CPU fallback build; both
  builds run on modern CPUs, and the clean SSE1 build is the patchable
  one, so it is the default launch target when the stutter fix is on.
  `SEGARallyLauncher.exe` (the .NET tool) is never run.

## Sources

- distrobox-gaming: `docs/pc-racing.md` (Sega Rally Revo section +
  controller policy), `ansible/group_vars/all/pc_racing.yml`
  (`sega-rally-revo` entry), git commits `625520d`, `736bd9a`, `961dd56`,
  `6fa10f6`, `d3acc46`, `8875b85`; `docs/pc-racing-launcher-opportunities.md`
- stuken/SegaRallyRevoStutterFix (README + patch.py, fetched 2026-11-01;
  licence and activity checked 2026-11-02 via the GitHub API)
- Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix (README, fetched 2026-11-01;
  checked 2026-11-02)
- Microsoft: the DirectX End User Runtime licence (inside
  `directx_Jun2010_redist.exe`), "DirectX Installation for Game
  Developers" (learn.microsoft.com), Chuck Walbourn's "Not So Direct
  Setup" (walbourn.github.io)
- PCGamingWiki "Sega Rally Revo" + its Talk page; WSGF "Sega Rally Revo"
- Lutris installer `sega-rally-revo-myabandonware-full-r`
- doitsujin/dxvk issue #3320 (D3DX9 falling trees, d3dcompiler_47)
- NeoGAF "Fighting Framerates — Help with Sega Rally Revo" (2016)
- Steam community: force-feedback crash reports (app 10400/4790)
