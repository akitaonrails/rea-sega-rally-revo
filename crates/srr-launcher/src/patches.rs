//! Binary patches applied to the private runtime copy of the game.
//!
//! Two patches exist, both gated by exact byte verification:
//!
//! - `pacer` — the community stutter fix for the clean `SEGA Rally_SSE1.exe`
//!   (studied from stuken/SegaRallyRevoStutterFix, whose reference build is
//!   byte-identical to the rip this launcher targets). It fixes two stacked
//!   timing bugs: the whole-game 16/17 ms truncation jitter and the
//!   campaign-only stage-2+ stutter caused by a duplicated pacer thread.
//! - `aspect` — replaces the renderer's 16:9 aspect constant with the
//!   chosen ratio so ultra-wide resolutions render without stretching
//!   (the approach documented on PCGamingWiki, applied pattern-based so it
//!   also fits builds whose file offsets differ).
//!
//! The user's source folder is never written; patches are only ever applied
//! to the runtime copy, and every write is checked against the expected
//! original bytes first.

/// A tiny x86 assembler for literal opcodes plus computed rel32 branches.
struct Cave {
    base: u32,
    bytes: Vec<u8>,
}

impl Cave {
    fn new(base: u32) -> Cave {
        Cave {
            base,
            bytes: Vec::new(),
        }
    }

    fn va(&self) -> u32 {
        self.base + self.bytes.len() as u32
    }

    fn raw(&mut self, bytes: &[u8]) -> &mut Cave {
        self.bytes.extend_from_slice(bytes);
        self
    }

    /// `E8` (call) / `E9` (jmp) to an absolute VA.
    fn rel32(&mut self, opcode: u8, target: u32) -> &mut Cave {
        self.raw(&[opcode]);
        let end = self.va() + 4;
        self.bytes.extend_from_slice(
            &i32::checked_sub(target as i32, end as i32)
                .expect("branch distance fits i32")
                .to_le_bytes(),
        );
        self
    }

    /// Short conditional branch over an already-assembled block.
    fn branch_over(&mut self, opcode: u8, block: &[u8]) -> &mut Cave {
        assert!(block.len() < 128);
        self.raw(&[opcode, block.len() as u8]);
        self.bytes.extend_from_slice(block);
        self
    }
}

fn le32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

// Engine addresses in the clean SSE1 build (image base 0x400000; for
// .text the file offset equals VA - 0x400000).
const IMAGE_BASE: u32 = 0x400_000;
const CLOCK_TICK: u32 = 0x589_890; // Clock_Tick_QpcToUsecTotal
const AULLDIV: u32 = 0x4AF_8F0; //   __aulldiv (ret 0x10)
const TOTAL_USEC_L: u32 = 0x950_928; // master clock, microseconds (64-bit)
const TOTAL_USEC_H: u32 = 0x950_92C;
const RACE_MS_L: u32 = 0x967_0D8; // race clock, milliseconds (64-bit)
const RACE_MS_H: u32 = 0x967_0DC;
const PACER_HANDLE: u32 = 0xB42_0A0; // pacer object +0x1C: thread handle
const PACER_FLAG: u32 = 0xB42_0A4; //                       +0x20: run flag
const PACER_GATE: u32 = 0xB42_0A5; //                       +0x21: list gate
const POWER_NEXT_H: u32 = 0x728_C74; // main loop power-poll deadline (hi)

const SPAWN_HOOK: u32 = 0x595_660;
const SPAWN_RESUME: u32 = 0x595_665;
const TICK_HOOK: u32 = 0x595_323;
const TICK_RESUME: u32 = 0x595_32A;
const MAIN_HOOK: u32 = 0x558_895;
const MAIN_RESUME: u32 = 0x558_89F;

const CAVE1: u32 = 0x685_100; // spawn guard
const CAVE2: u32 = 0x685_180; // race-clock tick
const CAVE3: u32 = 0x685_200; // main-loop refresh

/// .rdata raw data starts here; caves must stay below it.
const TEXT_RAW_END: usize = 0x286_000;
/// Exact size of the clean build; a different size is another build.
const SSE1_SIZE: usize = 3_727_360;

const SPAWN_ORIGINAL: &[u8; 5] = &[0x51, 0x8D, 0x04, 0x24, 0x50];
const TICK_ORIGINAL: &[u8; 7] = &[0x83, 0x05, 0xD8, 0x70, 0x96, 0x00, 0x10];
const MAIN_ORIGINAL: &[u8; 5] = &[0x8B, 0xF2, 0x3B, 0x35, 0x74];

/// Sites written unconditionally to normalise earlier community
/// experiments (the 0.96 clock scale, a resync test, a thread-priority
/// test). All are no-ops on the clean base.
const NORMALISE: [(&[u8], usize, &str); 3] = [
    (&[0x40, 0x42, 0x0F, 0x00], 0x1898CE, "0.96 clock scale"),
    (
        &[0xA3, 0xD8, 0x70, 0x96, 0x00],
        0x19539E,
        "pacer resync site",
    ),
    (&[0x01], 0x1723EE, "engine thread priority"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchError {
    NotPatchable,
    NotClean(&'static str),
    CaveNotZero(u32),
    CaveTooLarge(u32),
    CavesOverlap(u32, u32),
    SiteChanged(u32),
    Truncated,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::NotPatchable => {
                write!(f, "this executable is not the supported SSE1 build")
            }
            PatchError::NotClean(what) => {
                write!(f, "patch site for {what} does not match the clean build")
            }
            PatchError::CaveNotZero(va) => {
                write!(f, "cave space at {va:#x} is not zero-filled")
            }
            PatchError::CaveTooLarge(va) => write!(f, "cave at {va:#x} runs past .text"),
            PatchError::CavesOverlap(a, b) => {
                write!(f, "caves at {a:#x} and {b:#x} overlap")
            }
            PatchError::SiteChanged(va) => {
                write!(f, "unexpected byte change at {va:#x}; refusing to write")
            }
            PatchError::Truncated => write!(f, "the executable is truncated"),
        }
    }
}

fn file_offset(va: u32) -> usize {
    (va - IMAGE_BASE) as usize
}

fn spawn_guard_cave() -> Vec<u8> {
    let mut cave = Cave::new(CAVE1);
    cave.raw(&[0x83, 0x3D])
        .raw(&le32(PACER_HANDLE))
        .raw(&[0x00]);
    let mut prologue = Cave::new(cave.va() + 2);
    prologue.raw(&[0x51, 0x8D, 0x04, 0x24, 0x50]);
    prologue.rel32(0xE9, SPAWN_RESUME);
    let prologue = std::mem::take(&mut prologue.bytes);
    cave.branch_over(0x75, &prologue);
    cave.raw(&[0xC6, 0x05]).raw(&le32(PACER_FLAG)).raw(&[0x01]);
    cave.raw(&[0xC6, 0x05]).raw(&le32(PACER_GATE)).raw(&[0x00]);
    cave.raw(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3]);
    cave.bytes
}

fn race_clock_cave() -> Vec<u8> {
    let store = [
        &[0xA3][..],
        &le32(RACE_MS_L)[..],
        &[0x89, 0x15],
        &le32(RACE_MS_H)[..],
    ]
    .concat();
    let mut cave = Cave::new(CAVE2);
    cave.raw(&[0x51]);
    cave.rel32(0xE8, CLOCK_TICK);
    cave.raw(&[0x8B, 0x15]).raw(&le32(TOTAL_USEC_H));
    cave.raw(&[0xA1]).raw(&le32(TOTAL_USEC_L));
    cave.raw(&[0x6A, 0x00]);
    cave.raw(&[0x68]).raw(&le32(1000));
    cave.raw(&[0x52]);
    cave.raw(&[0x50]);
    cave.rel32(0xE8, AULLDIV);
    cave.raw(&[0x3B, 0x05]).raw(&le32(RACE_MS_L));
    cave.branch_over(0x76, &store);
    cave.raw(&[0x59, 0xF8]);
    cave.rel32(0xE9, TICK_RESUME);
    cave.bytes
}

fn main_loop_cave() -> Vec<u8> {
    let store = [
        &[0xA3][..],
        &le32(RACE_MS_L)[..],
        &[0x89, 0x15],
        &le32(RACE_MS_H)[..],
    ]
    .concat();
    let mut cave = Cave::new(CAVE3);
    cave.raw(&[0x3B, 0x05]).raw(&le32(RACE_MS_L));
    cave.branch_over(0x76, &store);
    cave.raw(&[0x8B, 0xF2]);
    cave.raw(&[0x3B, 0x35]).raw(&le32(POWER_NEXT_H));
    cave.raw(&[0x8B, 0xF8]);
    cave.rel32(0xE9, MAIN_RESUME);
    cave.bytes
}

fn hook_bytes(hook: u32, target: u32, pad: usize) -> Vec<u8> {
    let mut cave = Cave::new(hook);
    cave.rel32(0xE9, target);
    let mut bytes = cave.bytes;
    bytes.resize(bytes.len() + pad, 0x90);
    bytes
}

fn caves() -> [(u32, Vec<u8>); 3] {
    [
        (CAVE1, spawn_guard_cave()),
        (CAVE2, race_clock_cave()),
        (CAVE3, main_loop_cave()),
    ]
}

fn read(data: &[u8], offset: usize, len: usize) -> Result<&[u8], PatchError> {
    data.get(offset..offset + len).ok_or(PatchError::Truncated)
}

/// Apply the community pacer fix in place. Returns `Ok(true)` when bytes
/// were written, `Ok(false)` when the fix is already present.
pub fn pacer(data: &mut [u8]) -> Result<bool, PatchError> {
    if data.len() != SSE1_SIZE {
        return Err(PatchError::NotPatchable);
    }
    let spawn_hook = hook_bytes(SPAWN_HOOK, CAVE1, 0);
    let tick_hook = hook_bytes(TICK_HOOK, CAVE2, 2);
    let main_hook = hook_bytes(MAIN_HOOK, CAVE3, 0);
    if read(data, file_offset(SPAWN_HOOK), spawn_hook.len())? == spawn_hook.as_slice() {
        return Ok(false); // already patched
    }

    // Static checks, before anything is written.
    let mut spans = Vec::new();
    for (base, bytes) in caves() {
        let start = file_offset(base);
        let end = start + bytes.len();
        if end > TEXT_RAW_END {
            return Err(PatchError::CaveTooLarge(base));
        }
        if read(data, start, bytes.len())?.iter().any(|b| *b != 0) {
            return Err(PatchError::CaveNotZero(base));
        }
        spans.push((start, end, base));
    }
    spans.sort_unstable();
    for (a, b) in spans.iter().zip(spans.iter().skip(1)) {
        if a.1 > b.0 {
            return Err(PatchError::CavesOverlap(a.2, b.2));
        }
    }
    for (original, offset, what) in [
        (
            SPAWN_ORIGINAL as &[u8],
            file_offset(SPAWN_HOOK),
            "pacer spawn",
        ),
        (TICK_ORIGINAL, file_offset(TICK_HOOK), "race-clock tick"),
        (MAIN_ORIGINAL, file_offset(MAIN_HOOK), "main-loop refresh"),
    ] {
        if read(data, offset, original.len())? != original {
            return Err(PatchError::NotClean(what));
        }
    }

    // Writes: normalise first (no-ops on a clean base), then the fix.
    let writes: Vec<(usize, Vec<u8>, String)> = NORMALISE
        .iter()
        .map(|(bytes, offset, what)| (*offset, (*bytes).to_vec(), (*what).to_string()))
        .chain([
            (
                file_offset(SPAWN_HOOK),
                spawn_hook,
                "spawn-guard hook".to_string(),
            ),
            (
                file_offset(CAVE1),
                caves()[0].1.clone(),
                "spawn-guard cave".to_string(),
            ),
            (
                file_offset(TICK_HOOK),
                tick_hook,
                "raceclock hook".to_string(),
            ),
            (
                file_offset(CAVE2),
                caves()[1].1.clone(),
                "raceclock cave".to_string(),
            ),
            (
                file_offset(MAIN_HOOK),
                main_hook,
                "mainloop hook".to_string(),
            ),
            (
                file_offset(CAVE3),
                caves()[2].1.clone(),
                "mainloop cave".to_string(),
            ),
        ])
        .collect();
    for (offset, bytes, _) in &writes {
        let target = data
            .get_mut(*offset..*offset + bytes.len())
            .ok_or(PatchError::Truncated)?;
        target.copy_from_slice(bytes);
    }
    Ok(true)
}

/// The 16:9 aspect constant `39 8E E3 3F` (1.7777778f32) as it appears in
/// the game's aspect tables, flanked by the 4:3 (`ab aa aa 3f`) and 16:10
/// (`cd cc cc 3f`) constants. Patching the middle slot retargets the
/// renderer's aspect without touching anything else.
const ASPECT_43: [u8; 4] = 1.333_333_4_f32.to_le_bytes();
const ASPECT_169: [u8; 4] = (16.0f32 / 9.0).to_le_bytes();
const ASPECT_1610: [u8; 4] = 1.6_f32.to_le_bytes();

/// The two aspect-table layouts in the clean build, as (slot, tail)
/// offsets from the 4:3 anchor: the enum-style table has the 16:9 slot
/// right after 4:3 with 16:10 four bytes later; the id-interleaved table
/// has an id between each constant and 16:10 sixteen bytes after the
/// anchor.
const ASPECT_LAYOUTS: [(usize, usize); 2] = [(4, 8), (8, 16)];

/// Aspect ratios the game's tables already contain, so the original 16:9
/// constant is kept.
pub fn is_native_aspect(ratio: f32) -> bool {
    (ratio - 16.0 / 9.0).abs() < 0.02 || (ratio - 1.6).abs() < 0.02
}

/// Identify a private copy whatever ultra-wide ratio a previous launch
/// patched into its aspect tables: the hash is taken with the tables reset
/// to the original 16:9 constant.
pub fn identify_ignoring_aspect(exe: &[u8]) -> Result<&'static crate::edition::Edition, [u8; 32]> {
    let mut probe = exe.to_vec();
    // aspect() cannot fail; it only reports how many slots changed.
    let _ = aspect(&mut probe, 16.0 / 9.0);
    crate::edition::identify(&probe)
}

/// Every 16:9 slot of an aspect table, whatever value it currently holds.
fn aspect_slots(data: &[u8]) -> Vec<usize> {
    let mut slots = Vec::new();
    for anchor in 0..data.len().saturating_sub(20) {
        if data[anchor..anchor + 4] != ASPECT_43 {
            continue;
        }
        for (slot, tail) in ASPECT_LAYOUTS {
            let tail = anchor + tail;
            if tail + 4 <= data.len() && data[tail..tail + 4] == ASPECT_1610 {
                let slot = anchor + slot;
                // The slot must hold a plausible aspect ratio; this keeps a
                // coincidental 4:3 … 16:10 pair from steering the patch.
                let value =
                    f32::from_le_bytes(data[slot..slot + 4].try_into().expect("four bytes"));
                if (1.0..=4.0).contains(&value) {
                    slots.push(slot);
                }
            }
        }
    }
    slots
}

/// Set the renderer's aspect tables to `ratio` (native ratios restore the
/// original 16:9 constant). Returns how many slots were rewritten.
pub fn aspect(data: &mut [u8], ratio: f32) -> Result<usize, PatchError> {
    let value = if is_native_aspect(ratio) {
        ASPECT_169
    } else {
        ratio.to_le_bytes()
    };
    let mut written = 0;
    for slot in aspect_slots(data) {
        if data[slot..slot + 4] != value {
            data[slot..slot + 4].copy_from_slice(&value);
            written += 1;
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden bytes captured from the reference implementation.
    #[test]
    fn caves_match_the_reference_assembly() {
        assert_eq!(
            spawn_guard_cave(),
            bytes_from_hex(
                "833da020b40000750a518d042450e95205f1ffc605a420b40001c605a520b40000b801000000c3"
            )
        );
        assert_eq!(
            race_clock_cave(),
            bytes_from_hex(
                "51e80a47f0ff8b152c099500a1280995006a0068e80300005250e851a7e2ff3b05d8709600760ba3d87096008915dc70960059f8e97101f1ff"
            )
        );
        assert_eq!(
            main_loop_cave(),
            bytes_from_hex("3b05d8709600760ba3d87096008915dc7096008bf23b35748c72008bf8e97d36edff")
        );
        assert_eq!(
            hook_bytes(SPAWN_HOOK, CAVE1, 0),
            bytes_from_hex("e99bfa0e00")
        );
        assert_eq!(
            hook_bytes(TICK_HOOK, CAVE2, 2),
            bytes_from_hex("e958fe0e009090")
        );
        assert_eq!(
            hook_bytes(MAIN_HOOK, CAVE3, 0),
            bytes_from_hex("e966c91200")
        );
    }

    #[test]
    fn caves_do_not_overlap_and_stay_in_text() {
        let mut spans: Vec<(usize, usize)> = caves()
            .iter()
            .map(|(base, bytes)| (file_offset(*base), file_offset(*base) + bytes.len()))
            .collect();
        spans.sort_unstable();
        for (a, b) in spans.iter().zip(spans.iter().skip(1)) {
            assert!(a.1 <= b.0, "caves overlap at {a:?} {b:?}");
            assert!(a.1 <= TEXT_RAW_END);
        }
        assert!(spans[2].1 <= TEXT_RAW_END);
    }

    fn bytes_from_hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn wrong_size_or_dirty_site_is_refused() {
        let mut small = vec![0u8; 10];
        assert_eq!(pacer(&mut small), Err(PatchError::NotPatchable));

        let mut data = vec![0u8; SSE1_SIZE];
        data[file_offset(SPAWN_HOOK)] = 0x90; // dirty hook site
        assert_eq!(pacer(&mut data), Err(PatchError::NotClean("pacer spawn")));

        let mut data = vec![0xFFu8; SSE1_SIZE];
        data[file_offset(SPAWN_HOOK)..file_offset(SPAWN_HOOK) + 5].copy_from_slice(SPAWN_ORIGINAL);
        data[file_offset(TICK_HOOK)..file_offset(TICK_HOOK) + 7].copy_from_slice(TICK_ORIGINAL);
        data[file_offset(MAIN_HOOK)..file_offset(MAIN_HOOK) + 5].copy_from_slice(MAIN_ORIGINAL);
        assert_eq!(pacer(&mut data), Err(PatchError::CaveNotZero(CAVE1)));
    }

    #[test]
    fn patch_is_idempotent_on_a_clean_synthetic_build() {
        let mut data = vec![0u8; SSE1_SIZE];
        for (bytes, offset, _) in [
            (SPAWN_ORIGINAL as &[u8], file_offset(SPAWN_HOOK), "spawn"),
            (TICK_ORIGINAL, file_offset(TICK_HOOK), "tick"),
            (MAIN_ORIGINAL, file_offset(MAIN_HOOK), "main"),
        ] {
            data[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        assert_eq!(pacer(&mut data), Ok(true));
        // Every normalise site was already at its clean value; hooks + caves
        // are in place, and a second run reports "already patched".
        assert_eq!(
            &data[file_offset(SPAWN_HOOK)..file_offset(SPAWN_HOOK) + 5],
            &hook_bytes(SPAWN_HOOK, CAVE1, 0)[..]
        );
        assert_eq!(pacer(&mut data), Ok(false));
    }

    #[test]
    fn aspect_slots_follow_the_table_layouts() {
        let mut data = vec![0u8; 128];
        // table A: enum-style (4:3, 16:9, 16:10 back to back)
        data[16..20].copy_from_slice(&ASPECT_43);
        data[20..24].copy_from_slice(&ASPECT_169);
        data[24..28].copy_from_slice(&ASPECT_1610);
        // table B: id-interleaved (4:3, 0, 16:9, 1, 16:10, 2)
        data[64..68].copy_from_slice(&ASPECT_43);
        data[72..76].copy_from_slice(&ASPECT_169);
        data[80..84].copy_from_slice(&ASPECT_1610);
        assert_eq!(aspect_slots(&data), vec![20, 72]);

        let written = aspect(&mut data, 64.0 / 27.0).unwrap();
        assert_eq!(written, 2);
        assert_eq!(data[20..24], (64.0f32 / 27.0).to_le_bytes());
        assert_eq!(data[72..76], (64.0f32 / 27.0).to_le_bytes());

        // the patched slots are still found and restored to native
        assert_eq!(aspect_slots(&data), vec![20, 72]);
        let restored = aspect(&mut data, 16.0 / 9.0).unwrap();
        assert_eq!(restored, 2);
        assert_eq!(data[20..24], ASPECT_169);
        assert_eq!(data[72..76], ASPECT_169);
        // a second native run rewrites nothing
        assert_eq!(aspect(&mut data, 16.0 / 9.0).unwrap(), 0);
    }

    #[test]
    fn implausible_slot_values_are_left_alone() {
        let mut data = vec![0u8; 64];
        data[8..12].copy_from_slice(&ASPECT_43);
        data[12..16].copy_from_slice(&9.9_f32.to_le_bytes()); // not an aspect
        data[16..20].copy_from_slice(&ASPECT_1610);
        assert!(aspect_slots(&data).is_empty());
        assert_eq!(aspect(&mut data, 64.0 / 27.0).unwrap(), 0);
    }

    /// Applies the pacer fix to the real game folder when `SRR_GAME_DIR`
    /// points at it (never committed data). Run manually with:
    /// `SRR_GAME_DIR=... cargo test real_game -- --nocapture --ignored`
    #[test]
    #[ignore]
    fn real_game() {
        let Some(dir) = std::env::var_os("SRR_GAME_DIR") else {
            panic!("set SRR_GAME_DIR to the game folder");
        };
        let path = std::path::Path::new(&dir).join(crate::edition::SSE1_EXE);
        let mut data = std::fs::read(&path).expect("read the SSE1 exe");
        let applied = pacer(&mut data).expect("the clean build is patchable");
        println!("pacer applied: {applied}");
        let digest = crate::edition::sha256(&data);
        if applied {
            assert_eq!(
                crate::edition::to_hex(&digest),
                "4c08c1deae28a329b6588138198556ac8eaded8444d0eb94a1fe2f1ad08f2520"
            );
        }
        let slots = aspect_slots(&data);
        println!("aspect slots: {slots:#x?}");
        assert_eq!(slots, vec![0x2F7C44, 0x2FADA8]);
        // An ultra-wide launch must not hide the stutter-fixed build.
        aspect(&mut data, 21.0 / 9.0).unwrap();
        assert!(std::ptr::eq(
            identify_ignoring_aspect(&data).unwrap(),
            &crate::edition::SSE1_PACER
        ));
    }
}
