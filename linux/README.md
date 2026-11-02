# Sega Rally Revo on Linux

`SRRLauncher.exe` is a Windows program; on Linux it runs through Wine or
Proton. There are three ways to play: the AppImage (it brings its own
Wine), Steam's Proton, or `srr-play` from the zip with your own Wine.
Every way ends at the same settings window.

## The AppImage

```sh
chmod +x SRRLauncher-<version>-x86_64.AppImage
./SRRLauncher-<version>-x86_64.AppImage          # settings window, then play
./SRRLauncher-<version>-x86_64.AppImage --play   # play with the saved settings, no window
./SRRLauncher-<version>-x86_64.AppImage stop     # end the game
./SRRLauncher-<version>-x86_64.AppImage status   # running or idle
```

It brings Wine and DXVK, and runs `srr-play` inside. The Wine prefix is
`~/.local/share/srr-launcher/prefix`; your Linux files are on drive `Z:`.
It doesn't use gamescope: on hybrid-GPU machines gamescope sends this game
to the integrated GPU, so the game runs in its own Wine desktop instead.

## Steam (Proton)

Add the AppImage as a non-Steam game to get everything above. Adding
`SRRLauncher.exe` from the zip with a forced Proton version also works,
but skips `srr-play`: the GPU choice is left to Steam, and the frame limit
goes through Proton's own DXVK. The launcher sets up the controller bridge
in Proton's prefix itself (values already there are kept); it takes
effect from the second start.

### Steam Deck

In Desktop Mode, add the AppImage as a non-Steam game, then start it from
Game Mode. The **Steam Deck** look is recommended there. Game Mode is not
tested yet.

## Your own Wine

```sh
srr-play            # settings window, then play
srr-play --play     # play with the saved settings, no window; "start" works too
srr-play stop       # end this launch's session and the prefix's Wine server
srr-play status     # running or idle
```

`srr-play --play` takes the launcher's options for one run, e.g.
`srr-play --play --resolution 2560x1440`. To run Wine from a distrobox,
set `SRR_RUN="distrobox enter gaming --"`.

Exit codes of the settings run: 0 play, 3 window closed, 4 no window, 5
already running (another launcher is waiting for the game). When the
window closes without Play, `srr-play` says so and starts nothing.

## Environment

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRR_PREFIX` | `~/.local/share/srr-launcher/prefix` | the private Wine prefix |
| `SRR_LAUNCHER` | next to `srr-play` | path to `SRRLauncher.exe` |
| `SRR_RUN` | empty | put in front of the Wine commands, e.g. `distrobox enter gaming --` |
| `SRR_WAIT` | unset | run in the foreground (the AppImage sets it) |

A GPU choice you already made (`VK_DRIVER_FILES`, `VK_ICD_FILENAMES`,
`DRI_PRIME`, `__NV_PRIME_RENDER_OFFLOAD`) is left alone.

## Where things are

| What | Where |
| --- | --- |
| Settings (`launcher.cfg`) and `launcher.log` | the prefix's `drive_c/users/<you>/AppData/Local/SRRLauncher/` |
| The launcher's copy of the game, DirectX 9 files | the same folder, `SegaRally` and `directx` |
| Your saves | the prefix's `drive_c/users/<you>/Documents/SEGA Rally` (often linked to your `~/Documents`) and `drive_c/users/Public/Documents/SEGA Rally` |
| Wine prefix | `~/.local/share/srr-launcher/prefix` |
| `srr-play`'s logs and lock | `~/.config/srr-launcher` (`play.log`, `settings.log`, `running.pid`) |

## Troubleshooting

- **Black screen while the game seems to run** (hybrid GPU): check the
  `GPU:` line `srr-play` prints when it starts; it should name the
  discrete GPU's Vulkan driver.
- **The pad does nothing**: switch it to X-input mode and bind it in the
  game's Controls menu.
- **The game crashes at the main menu**: press **Download** under
  *Microsoft DirectX 9 files* on the Play tab, then Play again.
- **"already running"**: `srr-play status` tells you; `srr-play stop` ends
  the old session.
