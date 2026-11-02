# Third-party components

Add a row before bundling anything new.

## Compiled into SRRLauncher.exe

| Component | Licence | Source |
| --- | --- | --- |
| rea-launcher, rea-launcher-win32, rea-launcher-ui (the shared launcher SDK) | MIT OR Apache-2.0 | github.com/akitaonrails/rea-launcher-sdk (tag pinned in `Cargo.toml`) |
| sha2 | MIT OR Apache-2.0 | crates.io |
| eframe / egui (and dependencies) | MIT OR Apache-2.0 | crates.io |
| jpeg-decoder | MIT OR Apache-2.0 | crates.io |
| Rust standard library (pre-release) | MIT OR Apache-2.0 | rust-lang.org |
| MinGW-w64 runtime | BSD-3-Clause and others | mingw-w64.org |

## Bundled for Linux

`rea-play.sh`, the SDK's play library (MIT OR Apache-2.0), ships beside
`srr-play`.


| Component | Licence | Source |
| --- | --- | --- |
| DXVK 3.1.1, 32-bit `d3d9.dll` | Zlib | github.com/doitsujin/dxvk (tarball and licence pinned by SHA-256 in `packaging/dxvk.txt`) |
| Wine 11.20 (Kron4ek amd64-wow64 build of unmodified WineHQ sources) | LGPL-2.1+ | github.com/Kron4ek/Wine-Builds (pinned in the vendored SDK's `sdk/packaging/appimage/tools.pins`, with appimagetool and the runtime); licence and source notes inside the AppImage at `usr/share/doc/wine` |
| AppImage runtime (appimagetool 1.9.1, type2 runtime 20251108) | MIT | github.com/AppImage/appimagetool, github.com/AppImage/type2-runtime |

## Artwork and fonts

| Component | Licence | Source |
| --- | --- | --- |
| Window portrait | ours (AI-generated for this launcher; nothing taken from the game) | — |
| App icon | ours (AI-generated for this launcher) | — |
| Barlow, Barlow Condensed (window fonts, bundled by `rea-launcher-ui`) | SIL OFL 1.1 | google.com/fonts (licence file beside the fonts in the SDK's `crates/rea-launcher-ui/assets/fonts`) |

## Not bundled, but wired

- The community pacer fix is **reimplemented in Rust from its public
  description** (`patches::pacer`); no code or bytes from
  stuken/SegaRallyRevoStutterFix are shipped. The game exes it patches
  belong to the user's own copy.
- `winetricks` verbs (`d3dx9`, `directplay`, `d3dcompiler_47`) are
  installed into the private prefix only when the user already has
  winetricks; the DirectX runtime itself is Microsoft's and never shipped.

The launcher never bundles official game patches, cracks, or registry
files from the rip; the registry values it writes are its own
configuration of the user's install.
