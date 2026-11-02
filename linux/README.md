# Linux helper

`srr-play` runs the 32-bit `SRRLauncher.exe` under Wine with everything the
game needs already wired:

- a private Wine prefix (default `~/.local/share/srr-launcher/prefix`)
- Wine's SDL controller bridge configured for this game (`Map Controllers=1`,
  `Enable SDL=1`, raw duplicates off with `DisableHidraw=1`/`DisableInput=1`),
  seeded once per prefix before Wine starts; existing values are kept
- optional `winetricks d3dx9 directplay d3dcompiler_47` when `winetricks`
  is installed — the offline DirectX runtime equivalents that stop this
  game's main-menu access violation (skip with `SRR_WINETRICKS=off`)
- the NVIDIA Vulkan ICD pinned on hybrid-GPU hosts so DXVK cannot render on
  the integrated GPU (that failure presents as a black screen while the
  process looks alive); Intel/AMD hybrids fall back to `DRI_PRIME=1`
- no gamescope of its own: on hybrid-GPU hosts gamescope routes this DXVK
  title's Vulkan to the integrated GPU; the launcher's isolated Wine
  desktop handles fullscreen instead

## Usage

```sh
srr-play            # settings window, then play
srr-play --play     # play with saved settings (no window); "start" works too
srr-play stop       # end the game and the Wine server
srr-play status     # running or idle
```

Exit codes of the settings run: 0 play, 3 window closed, 4 no window. When
the window closes without Play, `srr-play` says so and starts nothing.

## Environment

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRR_PREFIX` | `~/.local/share/srr-launcher/prefix` | private Wine prefix |
| `SRR_LAUNCHER` | next to `srr-play` | path to `SRRLauncher.exe` |
| `SRR_RUN` | empty | prefix for the wine commands, e.g. `distrobox enter gaming --` |
| `SRR_WAIT` | unset | run in the foreground (used by the AppImage) |
| `SRR_WINETRICKS` | `auto` | `off` skips the winetricks bootstrap |
| `SRR_ICD_DIR` | `/usr/share/vulkan/icd.d` | Vulkan ICD directory (tests) |
| `SRR_SYSFS` | `/sys` | sysfs root (tests) |

State lives in `~/.config/srr-launcher` (`running.pid`, `play.log`,
`settings.log`, `inner.sh`). Run `sh linux/tests/gpu-env.sh` and
`sh linux/tests/game-alive.sh` and `sh linux/tests/winebus.sh` to exercise the host-side helpers.

## Steam

Add the AppImage as a non-Steam game: it runs this wrapper with everything
above. Adding `SRRLauncher.exe` with a forced Proton version also works, but
skips this wrapper: no GPU pinning, no winetricks. The launcher itself
seeds the controller bridge into Proton's prefix (values that are already
there are kept); it takes effect from the second start. The frame limit still applies through Proton's own DXVK.
