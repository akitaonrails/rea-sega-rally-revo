# Linux helper

`srr-play` runs the 32-bit `SRRLauncher.exe` under Wine with everything the
game needs already wired:

- a private Wine prefix (default `~/.local/share/srr-launcher/prefix`)
- Wine's SDL controller bridge configured for this game (`Map Controllers=1`,
  raw duplicate devices disabled), seeded once per prefix
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
srr-play --play     # play with saved settings (no window)
srr-play stop       # end the game and the Wine server
srr-play status     # running or idle
```

Exit codes of the settings run: 0 play, 3 window closed, 4 no window.

## Environment

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRR_PREFIX` | `~/.local/share/srr-launcher/prefix` | private Wine prefix |
| `SRR_LAUNCHER` | next to `srr-play` | path to `SRRLauncher.exe` |
| `SRR_RUN` | empty | prefix for wine/gamescope commands, e.g. `distrobox enter gaming --` |
| `SRR_WAIT` | unset | run in the foreground (used by the AppImage) |
| `SRR_WINETRICKS` | `auto` | `off` skips the winetricks bootstrap |
| `SRR_ICD_DIR` | `/usr/share/vulkan/icd.d` | Vulkan ICD directory (tests) |
| `SRR_SYSFS` | `/sys` | sysfs root (tests) |

State lives in `~/.config/srr-launcher` (`running.pid`, `play.log`,
`inner.sh`). Run `sh linux/tests/gpu-env.sh` and
`sh linux/tests/config-keys.sh`, and `sh linux/tests/game-alive.sh` to
exercise the host-side helpers.

## Steam

Add `SRRLauncher.exe` as a non-Steam game with
`WINEPREFIX=$HOME/.local/share/srr-launcher/prefix` in the launch options
(Proton works too), or add the AppImage directly.
