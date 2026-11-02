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
   (plus `Ghosts/`) to exist. *Wired by:* seeded when missing, never
   overwritten afterwards.
4. **Offline DirectX runtime needed.** The repeatable main-menu access
   violation was stopped by installing the June 2010 DirectX redist plus
   `d3dx9`, `directplay` into the prefix (the rip ships only
   `dxwebsetup.exe`). *Wired by:* `srr-play` runs
   `winetricks d3dx9 directplay d3dcompiler_47` once per prefix when
   winetricks is available (DXVK issue #3320 also names d3dcompiler_47 as
   a crash-stopper); documented fallback otherwise.
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
   another vendor (verified fix, 2026-08-08); Intel/AMD hybrids fall back
   to `DRI_PRIME=1`.
8. **Controller bridge.** The game needs Wine's SDL-backed XInput device:
   `Map Controllers=1` with raw evdev/hidraw duplicates disabled
   (`DisableHidraw=1`, `DisableInput=1`, `Enable SDL=1`). *Wired by:*
   `srr-play` seeds the winebus key once per prefix. The pad must be in
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
  at file offsets `0x2F7C44`/`0x2FADA8` (SSE1) and `0x2F4BF4`/`0x2F7D68`
  (main) — PCGamingWiki's fixed offsets belong to a different build, so
  this launcher locates them by layout pattern instead of offset.
- 16:9 and 16:10 are native. 21:9 (64:27) and 32:9 replace the 16:9 slot
  in both tables (`patches::aspect`), the same technique as the
  PCGamingWiki/WSGF hex edit but computed from the chosen ratio.

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
- stuken/SegaRallyRevoStutterFix (README + patch.py, fetched 2026-11-01)
- Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix (README, fetched 2026-11-01)
- PCGamingWiki "Sega Rally Revo" + its Talk page; WSGF "Sega Rally Revo"
- Lutris installer `sega-rally-revo-myabandonware-full-r`
- doitsujin/dxvk issue #3320 (D3DX9 falling trees, d3dcompiler_47)
- NeoGAF "Fighting Framerates — Help with Sega Rally Revo" (2016)
- Steam community: force-feedback crash reports (app 10400/4790)
