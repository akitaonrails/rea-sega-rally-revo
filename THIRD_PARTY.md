# Third-party components

Add a row before bundling anything new.

## Compiled into SRRLauncher.exe

| Component | Licence | Source |
| --- | --- | --- |
| rea-launcher, rea-launcher-win32, rea-launcher-ui (the shared launcher SDK) | MIT OR Apache-2.0 | github.com/akitaonrails/rea-launcher-sdk (tag pinned in `Cargo.toml`) |
| sha2 | MIT OR Apache-2.0 | crates.io |
| eframe / egui (and dependencies) | MIT OR Apache-2.0 | crates.io |
| jpeg-decoder | MIT OR Apache-2.0 | crates.io |
| cab (and lzxd, flate2, byteorder, time) — reads Microsoft's DirectX package after the player downloads it | MIT (cab, lzxd); MIT OR Apache-2.0 (the others) | crates.io |
| Rust standard library (pre-release) | MIT OR Apache-2.0 | rust-lang.org |
| MinGW-w64 runtime | BSD-3-Clause and others | mingw-w64.org |

## Bundled for Linux

`rea-play.sh`, the SDK's play library (MIT OR Apache-2.0), ships beside
`srr-play`.


| Component | Licence | Source |
| --- | --- | --- |
| DXVK 3.1.1, 32-bit `d3d9.dll` | Zlib | github.com/doitsujin/dxvk (tarball and licence pinned by SHA-256 in `packaging/third-party.txt`, mirrored in this repository's `thirdparty` release) |
| Wine 11.21 (Kron4ek amd64-wow64 build of unmodified WineHQ sources) | LGPL-2.1+ | github.com/Kron4ek/Wine-Builds (pinned in the vendored SDK's `sdk/packaging/appimage/tools.pins`, with appimagetool and the runtime); licence and source notes inside the AppImage at `usr/share/doc/wine` |
| AppImage runtime (appimagetool 1.9.1, type2 runtime 20251108) | MIT | github.com/AppImage/appimagetool, github.com/AppImage/type2-runtime |

## Artwork and fonts

| File | What | Author | Licence | Source |
| --- | --- | --- | --- | --- |
| `crates/srr-launcher/assets/portrait.jpg` | the settings window's art (560×650) | made for this launcher with an AI image tool; the tool and prompt were not recorded | ours, as stated since v0.1.0 (nothing taken from the game) — **provenance to be confirmed by the owner** | — |
| `packaging/appimage/srr-launcher.png` | the app icon (256×256) | made for this launcher with an AI image tool; tool not recorded | ours, as above — **to be confirmed** | — |
| `docs/images/launcher-*.jpg` | screenshots of the settings window | this project (captured headless) | MIT OR Apache-2.0, like the code | — |
| README video and poster (hosted outside the repository) | gameplay recording through the launcher | the owner | the owner's | S3 links in the README |
| Barlow, Barlow Condensed (window fonts, bundled by `rea-launcher-ui`) | fonts | Jeremy Tribby | SIL OFL 1.1 | google.com/fonts (licence file beside the fonts in the SDK's `crates/rea-launcher-ui/assets/fonts`) |

## Downloaded on the player's request (never bundled)

| Component | Licence | Source | Why not bundled |
| --- | --- | --- | --- |
| Microsoft DirectX End-User Runtime, June 2010 (`directx_Jun2010_redist.exe`, 100,275,120 bytes, SHA-256 `053f76dcbb28802e23341b6a787e3b0791c0fa5c8d4d011b1044172dbf89c73b`); the launcher keeps its 32-bit `d3dx9_*.dll` and `D3DCompiler_*.dll` | Microsoft Software License Terms, DirectX End User Runtime (inside the package; it lets the player install and use the software, and forbids publishing it for others to copy). The launcher takes the DLLs out of the cabinets without running the package's installer (DXSETUP), which is what would show the terms, so the window's DirectX row and `--download-directx` say "Microsoft's DirectX End-User Runtime licence applies" with a link to Microsoft's page for the package | download.microsoft.com (pinned in `crates/srr-launcher/src/directx.rs`) | the runtime's licence forbids redistribution; the DirectX SDK's redistribution rights require Microsoft's DirectSetup installer (see `docs/RESEARCH.md`) |

## Owner decisions

- **2026-11-02 — NoDVD build supported.** The full rip's NoDVD
  `SEGA Rally.exe` (SHA-256 `ad84ecea…2ba391d7`) is a supported edition;
  the launcher never ships, writes or downloads a crack (see `AGENTS.md`).
- **2026-11-02 — DirectX 9 files** are a download on request, not
  winetricks and not bundled (above).

## Not bundled, but wired

- The stutter fix (`patches::PACER`, `crates/srr-launcher/src/pacer.rs`
  and `pacer/*.asm`) is **this project's own clean-room implementation**
  (MIT OR Apache-2.0, like the code), written from a specification of the
  required behaviour and the game's own code; provenance in
  `docs/PACER-PROVENANCE.md`. stuken/SegaRallyRevoStutterFix is credited
  for the idea and the problem analysis only; no code or bytes from its
  `patch.py` (which has no licence) are in this repository since
  2026-11-02. A player's exe already patched with stuken's fix
  (SHA-256 `4c08c1de…`) is recognised by hash and run as it is. The game
  executables it patches belong to the player's own copy.

The launcher never bundles official game patches, cracks, or registry
files from the rip; the registry values it writes are its own
configuration of the user's install.
