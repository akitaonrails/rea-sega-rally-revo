# Changelog

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
