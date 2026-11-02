# Provenance of the stutter fix (`pacer`)

Copied from the clean-room implementation's `PROVENANCE.md` when it was
integrated into this launcher (2026-11-02). Paths below are those of the
clean-room working folder; in this repository the code is
`crates/srr-launcher/src/pacer.rs`, the sources
`crates/srr-launcher/pacer/{caves,sites}.asm` and the tests
`crates/srr-launcher/tests/pacer_*.rs`.


Clean-room implementation of the Sega Rally Revo `pacer` fix. Implementer:
a Claude agent (Claude Code, model Opus 5.5) acting for the owner
(boss@akitaonrails.com), 2026-11-02, as the IMPLEMENTER role of the
clean-room process. Everything here was written inside
`/tmp/claude-1026/cleanroom-srr/impl/`.

## Statement

The design (hook sites, the routine that writes the race clock, the spawn
guard, the per-frame hook placement, register use, cave layout, header
change, padding) came only from `SPEC.md` and from the game's own code in
the clean `SEGA Rally_SSE1.exe` (SHA-256 `22655a54…f76e`), read with REA
(Ghidra provider) and, for byte context the REA listing did not cover,
with `ndisasm` on byte strings returned by REA. I never read stuken's
`patch.py`, its README, its released executables, any
"SegaRallyRevoStutterFix" code, the existing `patches::PACER` sites or
caves, or anything in `~/Projects/rea-sega-rally-revo`. I never opened the
community-fixed build (`4c08c1de…`); only its hash, given in SPEC.md, is in
the crate. No such material was seen by accident.

## What I read

1. `/tmp/claude-1026/STOP-outside-changes.md` (rules).
2. `/tmp/claude-1026/cleanroom-srr/SPEC.md` (requirements).
3. rea-launcher-sdk at `/mnt/data/Projects/rea-launcher-sdk` (working tree
   = tag v0.3.0 plus one CI-only commit):
   `crates/rea-launcher/src/patch.rs`, `crates/rea-launcher/src/patch/testing.rs`
   (the `patch::testing` module the brief names), `crates/rea-launcher/src/pe.rs`
   (API and the `synthetic` builder), `docs/STANDARDS.md` (headings and the
   "Running the real game in a test" section), `docs/playbooks/binary-patching.md`;
   `grep` of `README.md` for the section "Binary patches as one table"
   (not present at this tag; nothing else of the README was used);
   `Cargo.toml`/`crates/rea-launcher/Cargo.toml` (workspace edition,
   dependency names) and `grep` of `src/identify.rs` / `src/lib.rs` for
   public function names (`sha256`, `hex`, module list).
4. The game executable, read-only, via REA MCP tools: `open_binary`,
   `procedure_assembly` (0x595660, 0x5951B0, 0x589890, 0x5D31F0, 0x650630,
   0x5D21C0, 0x4AF8F0, 0x595B00, 0x595980, 0x586FF0), `xrefs` (0x9670D8,
   0x6FA470, 0x6FA460, 0x6FA450, 0xB420A4, 0xB420A0),
   `inspect_native_instruction` (0x9670D8 incoming refs with access kind,
   0x65CB34, 0x594C67), `resolve_containing_procedure`, `procedure_callers`,
   `search_bytes` (vtable pointers), `read_bytes` (site bytes, the PE
   header at 0x400000/0x400208, the slack 0x685000–0x685FFF, vtables,
   context at 0x65CAE0), `inspect_pe_imports`.
5. Two reads of the clean exe made **without** REA, while REA could not
   start (the user's `/tmp` quota was full, Ghidra failed with "Disk quota
   exceeded"): `ndisasm` of the bytes of `0x58E9B0` (the end-of-frame
   function named in SPEC.md §2.5) and a read-only Python parse of the
   section table plus a scan of the file for 32-bit values pointing into
   `.data`'s tail (the state-dword proof). Same information REA's
   `procedure_assembly` / `search_bytes` give; when REA came back it
   confirmed both on the patched image (`0x58E9B0` listing; `xrefs` on
   `0xB4B960` = only my three cave instructions).
6. The patched copy I produced (`.work/SEGA_Rally_SSE1_pacer.exe`, my own
   output) via REA, to check it disassembles sanely.
7. General x86 knowledge (instruction encodings, `cmpxchg8b`, MSVC
   thiscall / stdcall, `_aulldiv` contract as observed in the listing).
8. For the runtime run only (SPEC.md §5): the game folder listing, its
   `readme.txt` (first lines: system requirements), the rip's
   `Readme.txt` and `SEGA Rally.reg` one folder up (registry keys the game
   needs: `installdir`, `sku`, `language`); the GE-Proton 11-1 DXVK
   directory listing and `version` file, whose 32-bit `d3d9.dll` was copied
   into the private runtime under `.work/`. Nothing in the game folder was
   modified.

## Incidental exposure (reported)

While listing processes to check for leftover game processes
(`ps … | grep -i sega`), the output included the command line of another
agent's unrelated shell, which held the text of compliance-audit notes
for other launchers. One line of it mentions that the existing SRR pacer
is "derived from patch.py" and that its licence matters. It contains no
code, bytes, addresses or design details of `patch.py` or of the existing
pacer, and nothing from it was used. Reported here for completeness.

## What I did not read

- stuken/SegaRallyRevoStutterFix in any form (repo, README, `patch.py`,
  releases, patched executables).
- `~/Projects/rea-sega-rally-revo` (source, docs, tests, git history), and
  any launcher runtime copy it makes.
- Any patched executable other than my own output.
- The SDK's other sources, templates, launchers or skills beyond item 3.

## Timeline (2026-11-02, -03:00)

- 12:53 read rules, SPEC.md, SDK files; verified the exe hash
  (`sha256sum`, matches SPEC.md).
- 12:55–13:00 REA analysis of the pacer thread, spawn, master clock,
  keyframe update and its vtable, all race-clock writers and run-flag
  writers; found the five writers and the third stop path `0x65CB24`.
- 13:00 design fixed: one monotonic, locked store routine for every
  race-clock write; spawn guard on the handle slot; per-frame refresh at
  the consumer's entry; raise `.text` VirtualSize.
- 13:00–13:03 wrote `src/caves.asm` / `src/sites.asm`, assembled with
  NASM 3.02, wrote the crate and tests (golden, assembler, iced-x86 path
  analysis, env-gated real exe).
- 13:04 first real-exe run; pinned result hash. REA on the patched copy
  showed Ghidra stopping inside a long NOP run; replaced NOP tails with
  `jmp short end` + `int3`; re-pinned (`198cff5b…d260`, v1); REA re-check clean.
- 13:09–13:15 first headless runs (clean, fixed v1). The fixed build moved
  the race clock by real time, not 16-ms chunks, but the pacer's own
  16-ms writes still jittered the value seen at frame end.
- 13:17 v2: pacer tick/snap stand aside for 100 ms after a frame-hook
  store (state dword in `.data`'s tail; REA unavailable, proof by
  read-only byte scan; `.data` VirtualSize raised). Hash `a7b370e5…`.
- 13:21 v2 measured: race clock still on the pacer grid in the attract
  demo → `0x5D31F0` does not run there. Read `0x58E9B0` (ndisasm) and
  added `frame_end_hook` replacing the frame-timestamp master update at
  `0x58E9D3`. Final hash `cb9a5a34…`. REA back: final image checked.
- 13:25–13:45 first measurement batch; discarded (another workload was
  using the GPU). Secrets notice received: all runs since use `env -i`;
  scan of all outputs for credentials: none found.
- ~14:05–14:25 final batch, GPU idle, clean/fixed back to back, DXVK
  vsync60 / lim60 / lim120 / off; see `RUNTIME_NOTES.md`.

## Tools

REA MCP (Ghidra 12.1.2 provider), NASM 3.02, ndisasm, Rust 1.99.1
(toolchain binary called directly, private `CARGO_HOME` under `.work/`;
nothing written to `~/.cargo` or `~/.rustup`), iced-x86 1.x (dev-dependency),
`sha256sum`, Python 3 (byte slicing, sampler, analysis), distrobox `gaming`
(Wine 11.21, gamescope 3.17.0), DXVK 2.7.1 from GE-Proton 11-1 (copied).
