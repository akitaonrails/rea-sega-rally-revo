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
    the same pinned Kron4ek Wine WoW64 build the other REA launchers
    use (11.21 since v0.4.2), so system wine upgrades cannot change the runner mid-flight.

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

**The community fix** is [stuken/SegaRallyRevoStutterFix](https://github.com/stuken/SegaRallyRevoStutterFix)
(`patch.py`, no licence): it showed that both bugs can be fixed in the
clean `SEGA Rally_SSE1.exe` (whose reference MD5 matches our rip's file
byte-for-byte) with a pacer spawn guard (B) and a race clock taken from
the master clock (A).

**The shipped fix is our clean-room implementation** (owner decision,
2026-11-02): a specification of the required behaviour (invariants R1–R6),
then an independent implementation from that specification and the game's
own code read with REA, with no access to `patch.py`, its output or the
old table (`docs/PACER-PROVENANCE.md`). stuken is credited for the idea
and the problem analysis only. What it does (`crates/srr-launcher/src/pacer.rs`,
`pacer/*.asm`):

- R1: the pacer spawn routine returns early while a pacer is alive (one
  pacer thread: fixes B).
- R2/R4: all five writers of the race clock store `master_µs / 1000`
  through one routine, under the master clock's own lock, never backwards;
  the `+16 ms` tick is gone (fixes A).
- R3: the race clock is refreshed from the frame timestamp at every
  present and on entry to the in-race keyframe update; the pacer stands
  aside while that happens.

Eight code sites, one 224-byte cave in `.text` slack, `.text` and `.data`
VirtualSize raised; clean `22655a54…` → `cb9a5a34…ff65` (verified by the
SDK on every launch). Headless measurements (race clock vs. present
interval) are in `docs/RUNTIME_NOTES.md`. A player's exe patched with
stuken's fix (`4c08c1de…`) is recognised and run as it is.

A second patch, [Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix](https://github.com/Martijnsx/Sega-Rally-Revo-Frame-Pacing-Fix)
(P1: two timer initializers 15,999 µs → 0, four bytes), targets a
different exe build (3,700,786 bytes, SHA-256 `246b4a63…12db39`) and is
not applicable to ours; not shipped.

## Windows stutter: the simulation thread's timer (Quiver, issue #6, 2026-11-09)

A Windows 10 player (v0.4.2, stutter fix applied: `patches pacer
(cb9a5a34…)` in his `launcher.log`, 1080p, vsync) reported "noticeable
stutter … when you let the car roll on its own". Community fixes first:
PCGamingWiki, stuken's README, Martijnsx's patch and the Steam threads
(app 4790) describe the 62.5-fps/race-clock bug the stutter fix already
covers; none mentions the timer below. RTSS limits, G-Sync and NVIDIA
vsync were reported not to help ("an in-game code issue").

What the game does (clean `SEGA Rally_SSE1.exe`):

- The thread the pacer fix calls "the pacer" (`0x5951B0`, spawned by
  `0x595660`) is the game's **Simulation Thread**: it runs the tasks
  registered in list `0x728AAC`, named in the exe "Clock" (`0x5CF6D0`,
  samples the master clock into `0x950930`), "Keyframe" (`0x5D31F0`, the
  physics/AI keyframes due by the race clock, `0x594CB0`) and "Network";
  the render thread's list is `0x7285D4` ("Render AI", "Event Markers",
  …). It waits for each 16 ms beat by polling `Sleep(4)` (`0x5952D7`).
- The game never asks for a fine system timer while it plays. Its only
  `timeBeginPeriod(1)` (`0x46DFCC`) belongs to DirectShow video playback
  and is balanced by `timeEndPeriod` when the video stops; a relay trace of
  a title → attract race → quit session shows **zero** calls.
- Since Windows 10 version 2004, `timeBeginPeriod` is per process (another
  program's 1 ms request no longer speeds up this game's sleeps), and the
  default timer tick is 15.625 ms. So on current Windows each `Sleep(4)`
  lasts a full tick: the simulation beat runs on a 15.6 ms grid and, as
  the 16 ms target drifts, takes **two** ticks (31 ms) about once every
  0.65 s. The renderer then draws frames whose newest keyframe is more
  than a full step old — a hitch roughly every second. Wine's timer is
  always fine, so Linux never showed it (and our earlier measurements,
  all under Wine, could not).

The fix (`patches::TIMER`, the "timer" set, applied with the stutter
fix): the simulation thread calls `timeBeginPeriod(1)` when it starts
and `timeEndPeriod(1)` when it ends (hooks at its prologue `0x5951B0`
and its exit `0x5953CD`, a 38-byte cave at `0x685140`, both WINMM
imports already in the exe), and polls `Sleep(1)` instead of `Sleep(4)`.
Measured in `docs/RUNTIME_NOTES.md` (2026-11-09).

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

## Keyboard keys (issue #1, 2026-11-03; resolved 2026-11-09)

A player asked to change the keyboard keys. What is established:

- The keys are fixed. The manual lists "Configuration 1 (Keyboard)": A
  accelerate, Z brake/reverse, `,`/`.` steer, Space handbrake, S/X shift,
  Left Ctrl camera, Tab driver names, O/P Time Trial class.
- The PC game's Options menu has Audio, Display and Online only (checked
  in the real game, headless, keyboard as the player's device). The manual
  describes an Options > Controller page that this build doesn't show on
  the keyboard.
- `SEGARallyLauncher.exe` (the .NET tool, decompiled with ilspycmd for
  this note only) can't change keys either. `ControllerOptionsControl`
  enumerates DirectInput game controllers only (device class 4, types
  19–22, Xbox 360 pads excluded). Its `[GameController] Mapping=` string
  is `version(4 hex) 1 layout ff GUID(36) feedback` followed by eleven
  4-hex `eInput` values (axes, POV, buttons 0–15) for Accelerate, Brake,
  Handbrake, CameraChange, LookBack, GearUpshift, GearDownshift,
  ToggleDriverName, Start, SteerLeft, SteerRight.
- In `SEGA Rally_SSE1.exe` the keyboard is a DirectInput device
  (`GUID_SysKeyboard` used at 0x578468), read with `GetDeviceState` into a
  256-byte buffer and copied to the input object's key arrays (`+0x39`,
  previous frame `+0x139`, per player 0x2048 apart). `IsKeyDown` is vtable
  slot `+0x28` of the input object (vtable 0x6FB298, function 0x578160).
- The default keys are not constants in the code (no `push`/`mov`
  operands, byte or word stores), not a table in the exe or in the
  live input object, and not stored as characters. Driving reads keys
  through small binding objects (key at `+0xC`, e.g. thunk 0x414EA0); the
  code that creates them, and so the key source, is not found yet.
- Not in the data files either (checked 2026-11-05):
  `Control/control_data.sbf` holds neither the manual's DIK sequence
  (A Z , . Space S X LCtrl Tab O P = `1E 2C 33 34 39 1F 2D 1D 0F 18 19`)
  packed nor as 16-bit words.

Paused by owner decision on 2026-11-03, resumed 2026-11-09 with a
different approach: the key source does not need to be found. Every key
read goes through four accessors of the input object, each loading the
asked-for DirectInput code with `movzx ecx, byte [esp+8]` (5 bytes):

| Accessor (vtable slot) | SSE1 | NoDVD | Reads |
| --- | --- | --- | --- |
| `+0x24` | `0x578146` | `0x577086` | last frame's state (`+0x139`) |
| `+0x28` (`IsKeyDown`) | `0x578178` | `0x5770B8` | this frame's state (`+0x39`) |
| `+0x20` | `0x5781C3` | `0x577103` | just pressed (`+0x339` set, `+0x139` clear) |
| `+0x1C` | `0x578203` | `0x577143` | just released |

The keys set (`patches::KEYS_*`) turns each of those loads into a call to
a 16-byte routine in `.text` slack (`movzx ecx,[esp+0xC]; movzx
ecx,[ecx+TABLE]; ret`) followed by a 256-byte table, written by the
launcher from `keys` (`keys::KeyMap::table`): entry K is the key whose
state the game gets when it asks for K. Moved actions answer from their
new key; every other entry is itself, so menus and pause are untouched.
Caves: SSE1 `0x685300` (after the pacer's `0x685030..0x68510F`), NoDVD
`0x682830`; both lie past `.text`'s VirtualSize but inside its raw data
and its page-rounded mapping (the SDK checks they are zero and in an
executable section; Wine maps them, verified below).

Verified 2026-11-09, headless (gamescope 1920x1080, Wine 11.21, DXVK,
the owner's full rip read-only) in a Safari race of the Amateur league:
with `keys = accelerate=up`, holding A for 4 s left the car at 0 mph and
holding Up for 5 s took it to 59–61 mph in 3rd gear, on the SSE1 build
(stutter fix on, with and without borderless) and on the NoDVD build;
without the option A accelerates (58 mph). Menus were driven with
Return/Escape throughout with the patch in place.

## Borderless (issue #4, 2026-11-09)

A player asked for a borderless window. `RunWindowed=1` does nothing
because the game never reads it (no such string in either executable,
ASCII or UTF-16; its config keys are UTF-16 and only the .NET tool
writes `RunWindowed`). The device setup `0x58E350` (SSE1; NoDVD
`0x58BCA0`) stores 1 into the "fullscreen" global (`0x9E0CBC`; NoDVD
`0x9CDCBC`) right before `Direct3DCreate9`, and fills
`D3DPRESENT_PARAMETERS` at `0x7E7194` with `Windowed = (global == 0)` and
`FullScreen_RefreshRateInHz = [0xA1344C]` (the configured refresh). The
window is `CreateWindowExA(0, "SegaClass", "SEGA Rally", 0, 0, 0, 0, 0,
GetDesktopWindow(), …)`: style 0 and size 0, left to Direct3D's
fullscreen switch. Nothing in the game resizes or restyles it
(no `SetWindowPos`/`MoveWindow`/`SetWindowLong` imports).

The borderless set (`patches::BORDERLESS_*`):

1. `mov ebx,1 / push 0x20 / mov [global],ebx` →
   `xor ebx,ebx / mov [global],ebx / inc ebx / push 0x20 / nop nop`: the
   global is 0, so `Windowed = 1`; EBX is 1 again for the fields that
   follow (back buffer count, swap effect, auto depth stencil).
2. `mov edx,[refresh]` → `xor edx,edx`: refresh 0, which Direct3D
   requires for a window.
3. The six pushes before `CreateWindowExA` (`push eax` parent, five zero
   pushes) → a jump to a 33-byte cave (SSE1 `0x685200`, NoDVD
   `0x682800`) that pushes the parent, `GetSystemMetrics(SM_CYSCREEN)`,
   `GetSystemMetrics(SM_CXSCREEN)`, 0, 0 and `WS_POPUP | WS_VISIBLE`, then
   jumps back to the window-name push.

A windowed Direct3D 9 device presents its back buffer (the game's
resolution) stretched to the window, so a smaller resolution is scaled to
the screen. Under plain Wine the launcher sizes the isolated desktop to
the screen in this mode (the game's window then covers it).

Verified 2026-11-09, headless (same harness as the keys): the unpatched
run logs DXVK's `Setting display mode: 1920x1080@60` (exclusive
fullscreen); the borderless runs never switch modes, and with the game at
1280x720 DXVK creates the 1280x720 device with a 1920x1080 swap chain
(the screen-sized window) — title, menus and a race rendered full screen,
directly in gamescope and inside `explorer /desktop=SegaRally,1920x1080`,
on SSE1 (with the stutter fix) and NoDVD. Not verified: native Windows
(DPI virtualisation scales `GetSystemMetrics` for this non-DPI-aware game,
so the window still covers the screen) and multi-monitor Windows (the
window covers the primary monitor).

## Pad rumble (issue #2, 2026-11-05)

A player asked for a way to disable rumble. What the game does, verified
in the clean `SEGA Rally_SSE1.exe` and the game's own settings tool
(`SEGARallyLauncher.exe`, decompiled with ilspycmd):

- The game's pad vibration is **config-driven**, not hardcoded. Its
  settings tool writes `[GameController] Mapping` as
  `version(4) const(4) layout(4) allow-vibration(4) GUID(36)
  dinput-feedback(4)` plus eleven 4-hex button bindings — 100 characters
  (`c_controllerMapStringLength`; the tool's "Allow vibration" checkbox,
  default **on**, is field 4, chars `[12..16]`).
- The game parses the same string at startup: `0x589cd0` reads
  `[GameController] Mapping` (string table `0x6cb094`, source
  `Sega_LL_Input_Manager.cpp`) into a 0x30-byte struct stored at
  `0xA13450` (the input manager at `0xA13420`, set up by `0x58a310`).
- The rumble dispatcher copies that struct to `0x785af4` (`0x66e2b3`,
  once, latched at `0x9502c8`) and **extracts the allow-vibration word
  with `shr edx, 0x10` of the dword at `+4`** (`0x66e2f8`): when the flag
  is 0 it calls `0x58b370`, which sends the controller object's
  "set force-feedback output to zero" vtable call (`[vt+0x44](player,
  0)`); when it is 1 the normal effect path runs.
- The effects themselves are DirectInput 8 force feedback
  (`DINPUT8.dll` import; `CreateEffect` with the ConstantForce, Square
  and Ramp GUIDs at `0x6a2b48`/`0x6a2b38`/`0x6a2b28`). `XINPUT1_3.dll`
  is imported by ordinal but its thunks are never called — vestigial.

So the launcher's `rumble` option rewrites only chars `[12..16]` of the
player's existing `Mapping` (version- and length-checked; a mapping in
any other format is left untouched), preserving the pad GUID and the
eleven bindings, with the SDK `Owned` release semantics when the option
goes back to unset. Verified statically end to end; the one live check —
a race with a pad after the flip — still needs a pad connected to the
test box (none was, 2026-11-05).

## Community fixes: licence and maintenance

| Project | What it fixes | Licence | Maintained | Decision |
| --- | --- | --- | --- | --- |
| [stuken/SegaRallyRevoStutterFix](https://github.com/stuken/SegaRallyRevoStutterFix) | both stutter bugs, clean SSE1 v4.0.6.0 only (`patch.py`) | **none** (no licence file; README and `patch.py` state no terms; checked 2026-11-02) | yes (last push 2026-09-12) | not used as code: credited for the idea and the analysis; the shipped fix is our clean-room implementation (2026-11-02, `docs/PACER-PROVENANCE.md`); a player's exe patched with it is recognised by hash. The U1 licence request (`docs/CONTRIBUTIONS.md`) is no longer needed for the launcher. |
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

## Owner decisions

- **2026-11-02 — the NoDVD `SEGA Rally.exe` is a supported edition**
  (recorded in `AGENTS.md` and `THIRD_PARTY.md` that day). The widely
  mirrored full rip ships a NoDVD `SEGA Rally.exe` (`ad84ecea…2ba391d7`,
  table above); the launcher identifies it by SHA-256 and runs it
  unpatched, or with the aspect fix, when the stutter fix cannot run. We
  never write, ship, download or apply a crack, and never tell a player
  how to get such a build. The waiver line is in `.rea-check-waivers`
  (`drm-exception`). Since 2026-11-09 (owner decision, `AGENTS.md`) the
  borderless and keyboard patches apply to it too: window and input code
  only, never the NoDVD bytes.

## Old-game trouble spots

| Spot | Status for this game |
| --- | --- |
| Timers / frame pacing on fast CPUs | the two stacked stutter bugs above; stutter fix + DXVK limit; on Windows the simulation thread's 15.6 ms sleeps (the timer set) |
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
