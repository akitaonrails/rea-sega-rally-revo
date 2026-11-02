# SRRLauncher

A compatibility launcher for your own copy of **Sega Rally Revo** (PC,
2007). One download, point it at the game, press Play.

The 2007 PC release is a great rally game wrapped in a fragile package:
an installer era of registry gates, a .NET settings tool that misbehaves
under Wine, an exclusive-fullscreen mode that crashes multi-monitor hosts,
DirectX-runtime quirks that crash the main menu, and two stacked timing
bugs that stutter every race. This launcher wires all of it — plus the
community stutter fix — behind a small settings window.

| A standalone 2007 install | SRRLauncher |
| --- | --- |
| Registry + `config.ini` + saved-game layout must be hand-made or come from the flaky .NET launcher | seeded automatically, controller bindings preserved |
| Main-menu access violation without the offline DirectX runtime | `srr-play` installs the winetricks equivalents when available |
| Whole-game microstutter + stage-2+ Championship judder (two stacked timing bugs) | the community pacer fix, applied to the private copy |
| Frame pacing at 60 Hz | DXVK frame limiter wired in (60 default, 120/Off for high-refresh) |
| Exclusive fullscreen crashes when the monitor runs a high refresh rate | isolated Wine desktop at the chosen resolution |
| DXVK can render on the integrated GPU → black screen | the discrete GPU's Vulkan ICD is pinned |
| Startup stalls on logo videos under Wine | logo videos disabled in the private copy |
| Ultra-wide stretches | 21:9 / 32:9 aspect tables patched in |
| Your install is modified by whatever fix you applied | the source folder is read-only; everything happens in a private copy |

## What you need

The PC game itself — the widely mirrored EN-FR-DE-ES-IT-PL full-rip
(file version 4.0.6.0) is the supported source. Two executables are
recognized by exact hash:

| Executable | SHA-256 | Role |
| --- | --- | --- |
| `SEGA Rally_SSE1.exe` (clean) | `22655a54bad6932239ade58325703bd4b5a2020376737748026443fa5aadf76e` | default target; the only build the stutter fix applies to |
| `SEGA Rally.exe` (NoDVD rip) | `ad84ecea6ac5e86e1f1a420f07595866429bba63e7c489b40661a8032ba391d7` | playable fallback when the fix is off |

Unknown executables are rejected, never patched.

## Download

- **Windows**: `SRRLauncher-*.zip` — unzip next to the game (or anywhere)
  and run `SRRLauncher.exe`.
- **Linux**: `SRRLauncher-*.AppImage` — self-contained (bundles Wine 11.20
  WoW64 and DXVK 3.1.1; nothing is installed system-wide). Make it
  executable and run it.
- Or the plain zip on Linux: `linux/srr-play` with your own Wine/Proton
  (see `linux/README.md`).
- Steam: add the AppImage (or `SRRLauncher.exe` with a Proton
  compatibility tool) as a non-Steam game.

## Options

| Setting | Values | Default |
| --- | --- | --- |
| Look | Modern · Steam Deck · Original 2007 · Custom | Modern (screen size, High, 60 fps cap) |
| Resolution | Auto (screen) · 1080p · 1440p · 4K | Auto |
| Aspect | Auto · 16:9 · 16:10 · 21:9 · 32:9 | Auto (16:9) |
| Quality | Normal · High · Ultra | High |
| Fullscreen | on / off | on |
| Vsync | on / off | on |
| Stutter fix | on / off | on |
| Frame limit | 60 · 120 · Off | 60 |
| Language | English · French · German · Spanish · Italian · Polish | English |

The same settings work from the command line (`SRRLauncher --help`);
options override `launcher.cfg` for one run and only `--game` is
remembered. The window is fully gamepad-navigable (d-pad, A, LB/RB,
Start).

Championship mode caps itself at 62.5 fps engine-side — that one is not
fixable from outside. Keep in-game force feedback **off**; it is a known
crash and adds nothing (the pad still rumbles in supported menus via the
launcher's Test button).

## Where things are

| What | Windows | Linux (Wine prefix) |
| --- | --- | --- |
| Settings | `%LOCALAPPDATA%\SRRLauncher\launcher.cfg` (or next to the exe) | `<prefix>/drive_c/users/<you>/AppData/Local/SRRLauncher/launcher.cfg` |
| Private game copy | `%LOCALAPPDATA%\SRRLauncher\SegaRally` | same path inside `~/.local/share/srr-launcher/prefix` |
| Log | `%LOCALAPPDATA%\SRRLauncher\launcher.log` | same, plus `~/.config/srr-launcher/play.log` |

The private copy is refreshed from your source folder (only changed files
are copied); your saves live in `Documents\SEGA Rally` and are never
touched.

## Troubleshooting

- **"Unsupported game executables"** — you need the v4.0.6.0 full-rip;
  the launcher prints the SHA-256 it found.
- **Black screen but the game seems to run** — on hybrid-GPU Linux hosts
  make sure the discrete GPU's Vulkan ICD is what `srr-play` prints at
  startup (`GPU: VK_ICD_FILENAMES=…`).
- **Crash at the main menu on a fresh prefix** — install `winetricks` and
  rerun `srr-play` once (it installs `d3dx9 directplay d3dcompiler_47`).
- **No controller** — switch the pad to X-input mode; under Linux check
  `Map Controllers` guidance in the Controls tab.
- Delete `%LOCALAPPDATA%\SRRLauncher` and `~/.local/share/srr-launcher`
  to start over; the source folder was never modified.

## Building

```sh
cargo test --workspace                                # host tests
sh linux/tests/gpu-env.sh && sh linux/tests/config-keys.sh
cargo clippy --release --target i686-pc-windows-gnu -- -D warnings
cargo build --release --target i686-pc-windows-gnu    # needs gcc-mingw-w64-i686
packaging/appimage/build.sh <pkg-dir> <out-dir>       # AppImage with pinned Wine
```

CI (`.github/workflows/build.yml`) runs all of the above and publishes
the zip + AppImage for `v*` tags. Design references:
`docs/RESEARCH.md` (the fix stack and its evidence),
`docs/RUNTIME_NOTES.md` (what was verified live).

## Legal

Sega Rally Revo © Sega. This launcher is a fan-made compatibility tool,
not affiliated with or endorsed by Sega; it ships no game files, and the
only files it ever writes are a private copy of the game you own plus its
own configuration. Code: MIT OR Apache-2.0 — see `THIRD_PARTY.md` for
everything bundled.
