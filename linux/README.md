# Linux helper

`srr-play` runs the 32-bit `SRRLauncher.exe` under Wine with everything the
game needs already wired. It is a short configuration of the shared
rea-launcher-sdk play library, `rea-play.sh`, which ships beside it in the
zip and the AppImage (in this repository it is the vendored
`sdk/rea-play.sh`):

- a private Wine prefix (default `~/.local/share/srr-launcher/prefix`)
- Wine's SDL controller bridge configured for this game (`Map Controllers=1`,
  `Enable SDL=1`, raw duplicates off with `DisableHidraw=1`/`DisableInput=1`),
  seeded once per prefix before Wine starts; existing values are kept, so
  setting `DisableHidraw`/`DisableInput` to 0 in the prefix reverts it
  (see CHANGELOG)
- no winetricks: Wine's own DirectX 9 pieces run the game; Microsoft's
  DirectX 9 files, if you want them, come from the settings window's
  **Download** button (or `SRRLauncher.exe --download-directx`) and go into
  the private game copy, not the prefix
- the NVIDIA Vulkan ICD pinned (`VK_DRIVER_FILES` and the older
  `VK_ICD_FILENAMES`) on hybrid-GPU hosts that really have an NVIDIA GPU, so
  DXVK cannot render on the integrated GPU (that failure presents as a black screen while the
  process looks alive); Intel/AMD hybrids fall back to `DRI_PRIME=1`
- no gamescope of its own: on hybrid-GPU hosts gamescope routes this DXVK
  title's Vulkan to the integrated GPU; the launcher's isolated Wine
  desktop handles fullscreen instead

## Usage

```sh
srr-play            # settings window, then play
srr-play --play     # play with saved settings (no window); "start" works too
srr-play stop       # end this launch's session and the prefix's Wine server
srr-play status     # running or idle
```

Exit codes of the settings run: 0 play, 3 window closed, 4 no window. When
the window closes without Play, `srr-play` says so and starts nothing.

## Environment

A GPU choice you already made (`VK_DRIVER_FILES`, `VK_ICD_FILENAMES`,
`DRI_PRIME`, `__NV_PRIME_RENDER_OFFLOAD`) is left alone.

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRR_PREFIX` | `~/.local/share/srr-launcher/prefix` | private Wine prefix |
| `SRR_LAUNCHER` | next to `srr-play` | path to `SRRLauncher.exe` |
| `SRR_RUN` | empty | prefix for the wine commands, e.g. `distrobox enter gaming --` |
| `SRR_WAIT` | unset | run in the foreground (used by the AppImage) |

State lives in `~/.config/srr-launcher` (`running.pid`, `play.log`,
`settings.log`, `inner.sh`). `running.pid` holds the session leader's pid
and start time, so a reused pid or a game started some other way is never
mistaken for this launch, and `stop` signals only that session's process
group. The launcher path reaches `inner.sh` as an argument, never as
generated shell text. Run each `linux/tests/*.sh` with `sh` to exercise
the helper (the tests point the library's test seams, `REA_ICD_DIRS`,
`REA_SYSFS` and `REA_PROC`, at fake trees; they replace the old
`SRR_ICD_DIR`/`SRR_SYSFS` test variables).

## Steam

Add the AppImage as a non-Steam game: it runs this wrapper with everything
above. Adding `SRRLauncher.exe` with a forced Proton version also works, but
skips this wrapper: no GPU pinning. The launcher itself
seeds the controller bridge into Proton's prefix (values that are already
there are kept); it takes effect from the second start. The frame limit still applies through Proton's own DXVK.
