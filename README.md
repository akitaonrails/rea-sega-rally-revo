# Sega Rally Revo Launcher

Play your own PC copy of **Sega Rally Revo** (2007) on Windows 10/11 or on
Linux, at 4K, without the stutter. Point the launcher at the game folder and
press **Play**: it builds a private, fixed-up copy of the game and starts it.
Your installed game folder is never changed.

![The launcher's Play tab](docs/images/launcher-play.png)

### See it running

[![Video: the launcher starts Sega Rally Revo at 4K on Linux, then a Safari race driven with a gamepad (click to play)](https://new-uploads-akitaonrails.s3.us-east-2.amazonaws.com/2026/11/02/sega-rally-revo-launcher-poster.jpg)](https://new-uploads-akitaonrails.s3.us-east-2.amazonaws.com/2026/11/02/sega-rally-revo-launcher.mp4)

The AppImage on a Linux desktop (Hyprland, RTX 5090, 4K): Play in the
launcher, the game's own start-up, then a Safari race on a gamepad with
the stutter fix applied (2:49; the first-run loading wait is cut out).

## What it fixes

The 2007 PC release is a great rally game in a fragile package. Each row
below was a real hurdle on a modern machine; the launcher handles all of
them for you.

| Problem with a plain 2007 install | What the launcher does |
| --- | --- |
| Every race microstutters, and Championship stages after the first judder (two stacked timing bugs in the game) | Applies the community stutter fix (pacer patch) to the private copy |
| The game wants registry entries, a `config.ini` and a Documents folder layout made by the installer or its .NET settings tool, which misbehaves under Wine | Writes all of them itself, the way the original tool would; your controller bindings in `config.ini` are kept |
| Exclusive fullscreen crashes on multi-monitor and high-refresh Linux desktops | Runs the game inside an isolated Wine desktop at the chosen resolution |
| Uneven frame pacing | DXVK frame limit: 60 fps by default, 120 or off for high-refresh screens |
| Startup stalls on the logo videos under Wine | Switches the two logo videos off in the private copy |
| Ultra-wide monitors get a stretched picture | Patches 21:9 and 32:9 into the game's aspect tables |
| DXVK picks the integrated GPU on hybrid laptops and desktops (black screen) | Pins the discrete GPU's Vulkan driver |
| Community fixes overwrite your install | Everything happens in a private copy; the source folder is only read |

## What you need

Your PC copy of the game, version **4.0.6.0** (the EN-FR-DE-ES-IT-PL
release). The launcher recognizes the game executables by exact SHA-256 and
refuses anything else instead of patching it blindly:

| Executable | SHA-256 | Used for |
| --- | --- | --- |
| `SEGA Rally_SSE1.exe` (clean) | `22655a54bad6932239ade58325703bd4b5a2020376737748026443fa5aadf76e` | the default; the stutter fix applies only to this build |
| `SEGA Rally.exe` (no-DVD build in common copies) | `ad84ecea6ac5e86e1f1a420f07595866429bba63e7c489b40661a8032ba391d7` | played when the stutter fix is off or unavailable |

## Install

Download the latest release from the
[Releases page](https://github.com/akitaonrails/rea-sega-rally-revo/releases).

**Linux (recommended): the AppImage.** It bundles Wine 11.20 and DXVK 3.1.1,
so nothing else needs to be installed:

```sh
chmod +x SRRLauncher-*-x86_64.AppImage
./SRRLauncher-*-x86_64.AppImage
```

**Windows: the zip.** Unzip it anywhere and run `SRRLauncher.exe`.

**Linux with your own Wine or Proton:** the zip also works. Run
`linux/srr-play` from it (see [linux/README.md](linux/README.md)).

**Steam / Steam Deck:** add the AppImage as a non-Steam game. To use Steam's
Proton instead, add `SRRLauncher.exe` as a non-Steam game and force a Proton
version; the frame limit then goes through Proton's own DXVK.

## First run

1. Start the launcher. On the **Play** tab, click **Browse…** and choose the
   folder that holds `SEGA Rally.exe` / `SEGA Rally_SSE1.exe`. Under Wine your
   Linux files are on drive `Z:`, e.g. `Z:\home\you\Games\Sega Rally`.
2. Pick a **Look** (Modern is right for most screens) and leave the stutter
   fix on.
3. Press **Play** (or Enter, or Start on the pad). The first launch copies the
   game (about 4.2 GB) into the private folder; later launches copy only
   files that changed.

On Linux, keep `winetricks` installed if you can: on the first run the
launcher uses it to add `d3dx9`, `directplay` and `d3dcompiler_47`, which
stops a known crash at the main menu. Without it the launcher prints a note
and carries on.

## The settings window

### Play

Game folder, the Look presets, the stutter fix, the frame limit, and the menu
language (screenshot at the top).

| Look | Resolution | Aspect | Quality | Vsync |
| --- | --- | --- | --- | --- |
| **Modern** (default) | your screen | from the screen | High | on |
| **Steam Deck** | 1280×800 | 16:10 | High | on |
| **Original 2007** | 1280×720 | 16:9 | Normal | on |
| **Custom** | whatever you set on the Picture tab | | | |

### Picture

![The Picture tab](docs/images/launcher-picture.png)

Resolution, aspect, quality and vsync. Changing any of them turns the Look
into Custom.

### Controls

![The Controls tab](docs/images/launcher-controls.png)

Shows the connected Xbox-style (XInput) pads and has a **Test rumble**
button, which only checks that the pad accepts rumble. The game uses its own
native gamepad support: nothing is remapped to the keyboard. Bind the pad in
the game's own Controls menu. Keep the game's force feedback **off**; it is a
known crash and adds nothing.

The window itself works with a pad: d-pad to move, **A** to change the
focused setting, **LB/RB** to switch tabs, **Start** to play.

## All options

Every setting is in the window, in `launcher.cfg`, and on the command line.

| Setting | Values | Default | `launcher.cfg` key | Command line |
| --- | --- | --- | --- | --- |
| Game folder | a folder | (asked on first run) | `game` | `--game <folder>` |
| Private copy | a folder outside the game folder | `%LOCALAPPDATA%\SRRLauncher\SegaRally` | `runtime` | `--runtime <folder>` |
| Resolution | Auto, your screen, 1080p, 1440p, 4K, or any `WxH` | Auto (screen size, or 1920×1080) | `resolution` | `--resolution 3840x2160` |
| Aspect | Auto, 16:9, 16:10, 21:9, 32:9 | Auto (from the resolution) | `aspect` | `--aspect 21:9` |
| Quality | Normal, High, Ultra | High | `quality` | `--quality high` |
| Vsync | on, off | on | `vsync` | `--vsync on` |
| Stutter fix | on, off | on | `stutter-fix` | `--stutter-fix on` |
| Frame limit | 60, 120, off | 60 | `frame-limit` | `--frame-limit 60` |
| Menu language | English, French, German, Spanish, Italian, Polish | English | `language` | `--language French` |

Notes:

- **Auto** resolution and aspect are resolved when you press Play, and the
  actual values are saved.
- **Aspect:** 21:9 and 32:9 patch the private copy's aspect tables. 16:9 and
  16:10 need no patch; the game picks between them from the resolution.
- **Quality** maps to the game's ten per-feature sliders (scale 0–3).
  *High* is the configuration proven on the reference 4K setup; *Normal*
  sets every slider to 2 and *Ultra* every slider to 3.
- **Frame limit** runs through DXVK, so it applies on Linux, Wine and
  Proton. On native Windows the game has no DXVK and follows vsync instead;
  the buttons are disabled there.
- **Stutter fix** needs the clean `SEGA Rally_SSE1.exe`. Without it the game
  runs unpatched from `SEGA Rally.exe`; your saved choice is kept for later.
- The game always runs fullscreen. Under Wine that fullscreen is the isolated
  Wine desktop, which is a window of the chosen size on your desktop.
- Championship mode caps itself at 62.5 fps inside the game; that cap cannot
  be changed from outside.

### Command line

```text
SRRLauncher [options]

  --game <folder>       folder with SEGA Rally.exe or SEGA Rally_SSE1.exe (remembered)
  --runtime <folder>    private writable game copy
  --resolution <WxH>    e.g. 3840x2160
  --aspect <ratio>      16:9, 16:10, 21:9 or 32:9
  --quality <preset>    normal, high or ultra
  --vsync on|off        wait for the display refresh
  --frame-limit <fps>   60, 120 or off
  --stutter-fix on|off  community pacer fix on the private copy
  --language <name>     English, French, German, Spanish, Italian or Polish
  --check               validate the game and exit
  --setup               prepare the runtime and settings without playing
  --no-window           play with saved settings, no launcher window
  --settings            only show the settings window
  --linux               running under srr-play/AppImage: install DXVK into the copy
  --help                show this help
```

Without options the settings window opens. Options on the command line
override `launcher.cfg` for that run only; only the game folder is saved
(the window's **Play** button saves everything). Invalid values are
rejected with an error.

The AppImage and `srr-play` add their own commands:

```sh
./SRRLauncher-x86_64.AppImage          # settings window, then play
./SRRLauncher-x86_64.AppImage --play   # play with saved settings, no window
./SRRLauncher-x86_64.AppImage stop     # end the game
./SRRLauncher-x86_64.AppImage status   # running or idle
```

## Where things are

| What | Windows | Linux (AppImage / `srr-play`) |
| --- | --- | --- |
| Launcher settings | `%LOCALAPPDATA%\SRRLauncher\launcher.cfg` (or `launcher.cfg` next to the exe) | `~/.local/share/srr-launcher/prefix/drive_c/users/<you>/AppData/Local/SRRLauncher/launcher.cfg` |
| Private game copy | `%LOCALAPPDATA%\SRRLauncher\SegaRally` | the same path inside that Wine prefix |
| Saves and `config.ini` | `Documents\SEGA Rally\Saved Games` | `Documents` inside the prefix, which Wine usually links to your `~/Documents` |
| Logs | `%LOCALAPPDATA%\SRRLauncher\launcher.log` | the same, plus `~/.config/srr-launcher/play.log` and `settings.log` |

Saves (`SEGA Rally.dat`, `SR_RANKTABLES.DAT`) are created only when missing
and never deleted. An existing `config.ini` keeps its `[Audio]` and
`[GameController]` sections untouched; only the `[Graphics]` keys above are
updated.

## Troubleshooting

- **"This launcher supports the Sega Rally Revo PC v4.0.6.0 build"**: the
  executables are not the supported ones. The message shows the SHA-256 the
  launcher found.
- **Black screen while the game seems to run** (hybrid-GPU Linux): check the
  `GPU:` line `srr-play` prints at startup; it should name the discrete GPU's
  Vulkan driver.
- **Crash at the main menu on a new prefix**: install `winetricks`, delete
  `~/.local/share/srr-launcher/prefix`, and start again.
- **The pad does nothing in the game**: switch the pad to X-input mode, then
  bind it in the game's Controls menu.
- **Start over**: delete `%LOCALAPPDATA%\SRRLauncher` (Windows) or
  `~/.local/share/srr-launcher` and `~/.config/srr-launcher` (Linux). Your
  game folder was never modified.

## Building

```sh
cargo test --workspace                                 # host tests
sh linux/tests/gpu-env.sh && sh linux/tests/game-alive.sh
cargo clippy --release --target i686-pc-windows-gnu -- -D warnings
cargo build --release --target i686-pc-windows-gnu     # needs i686 mingw-w64 and lld
packaging/appimage/build.sh <package-folder> <output-folder>
```

CI (`.github/workflows/build.yml`) runs all of this on every push, checks
that every import of the built exe resolves, uploads the Windows zip and the
AppImage, and publishes both as a GitHub Release for `v*` tags.

Further reading: [docs/RESEARCH.md](docs/RESEARCH.md) (the fixes and the
evidence behind them) and [docs/RUNTIME_NOTES.md](docs/RUNTIME_NOTES.md)
(what was verified on real hardware, and what was not).

## Legal

Sega Rally Revo © SEGA. This is a fan-made compatibility tool, not affiliated
with or endorsed by SEGA. It ships no game files; it only writes a private
copy of the game you own plus its own settings. Code: MIT OR Apache-2.0. See
[THIRD_PARTY.md](THIRD_PARTY.md) for everything bundled.
