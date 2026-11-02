//! Clean-room `pacer` fix (the stutter fix) for Sega Rally Revo (`SEGA Rally_SSE1.exe`
//! v4.0.6.0), as one [`PatchSet`] built with `rea_launcher::patch`.
//!
//! Written from a clean-room specification (invariants R1–R6, its text in
//! `docs/PACER-PROVENANCE.md`'s process; "SPEC.md" below) and the game's own code read
//! with REA; see `docs/PACER-PROVENANCE.md`. The cave and site bytes come from
//! `pacer/caves.asm` / `pacer/sites.asm` (NASM); `tests/pacer_asm.rs` re-assembles
//! them and compares.
//!
//! What it does:
//!
//! - **R1, one pacer.** The spawn routine `0x595660` first checks the pacer
//!   handle slot `0xB420A0` (`[ESI+0x1C]`). Non-zero means a pacer is alive
//!   (every stop path joins it through `0x586FF0`, which zeroes the slot),
//!   so the spawn is a no-op that reports success. Zero: the original
//!   routine runs unchanged.
//! - **R2/R4, race clock = master time, never backwards.** All five game
//!   writes to the race clock `0x9670D8` (pacer start, pacer tick, pacer
//!   catch-up snap, and the two init functions `0x650630`/`0x5D21C0`) call
//!   one cave routine that, under the master clock's own critical section,
//!   updates the master clock and stores `max(race, master_us / 1000)` with
//!   one `lock cmpxchg8b`. The pacer's `+= 16 ms` is gone.
//! - **R3, refreshed every frame on the main thread.** The in-race keyframe
//!   update `0x5D31F0` (the consumer: it reads the race clock at
//!   `0x5D31FF` and hands it to `0x594CB0`) calls the same routine on entry,
//!   so the value it consumes was sampled in that same call, whatever the
//!   pacer thread is doing. It also records that store in one u32
//!   (`0xB4B960`, `.data`'s zero-initialised tail); while it is less than
//!   100 ms old the pacer's tick and snap leave the race clock alone, so in
//!   a race it moves once per frame by the real frame time. In the front
//!   end and during loads the pacer moves it every 16 ms, with master time.
//! - **R3, every presented frame, in every state.** In the end-of-frame
//!   function `0x58E9B0`, the master-clock update whose result becomes the
//!   frame timestamp (`call 0x589890` at `0x58E9D3`) is replaced by a call
//!   to the cave, which performs that same update and sets the race clock
//!   from it (and marks the hook active). The race clock then equals the
//!   frame timestamp in ms at every present, also in the front end and the
//!   attract demo, which do not run `0x5D31F0`.
//! - The `.text` section header's VirtualSize is raised from `0x284028` to
//!   `0x285000` (= SizeOfRawData) so the cave lies inside the section's
//!   declared size for every loader (the rounded size, and so SizeOfImage,
//!   are unchanged). The PE checksum is left as it is (`0x4EB522`; not
//!   checked for executables). `.data`'s VirtualSize grows by 8 bytes for
//!   the state dword (same page rounding).

use rea_launcher::patch::{At, PatchCave, PatchSet, PatchSite};

/// Parses 64 hex digits at compile time.
const fn digest(hex: &str) -> [u8; 32] {
    let b = hex.as_bytes();
    assert!(b.len() == 64);
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (nibble(b[2 * i]) << 4) | nibble(b[2 * i + 1]);
        i += 1;
    }
    out
}

const fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("not hex"),
    }
}

/// `SEGA Rally_SSE1.exe` v4.0.6.0, clean (3,727,360 bytes): the only build
/// this set applies to.
pub const SSE1_CLEAN: [u8; 32] =
    digest("22655a54bad6932239ade58325703bd4b5a2020376737748026443fa5aadf76e");

/// The same exe with the community stutter fix applied by the player: to be
/// recognised as "already fixed" and never patched (SPEC.md §1). Only the
/// hash is known here; no bytes of that build were read.
pub const SSE1_COMMUNITY_PACER: [u8; 32] =
    digest("4c08c1deae28a329b6588138198556ac8eaded8444d0eb94a1fe2f1ad08f2520");

/// The NoDVD `SEGA Rally.exe`: out of scope, refused.
pub const NODVD: [u8; 32] =
    digest("ad84ecea6ac5e86e1f1a420f07595866429bba63e7c489b40661a8032ba391d7");

/// `SSE1_CLEAN` with [`PACER`] alone applied (computed on the real exe by
/// `tests/real_game.rs`).
pub const SSE1_CLEANROOM_PACER: [u8; 32] =
    digest("cb9a5a343feb850f2360a9eb01fb775c55ea0962b0278502430da8d51689ff65");

/// Cave entry points (see `pacer/caves.asm`).
pub const CAVE_VA: u32 = 0x0068_5030;
pub const STORE_RACE_CLOCK: u32 = 0x0068_5030;
pub const PACER_STORE: u32 = 0x0068_5038;
pub const SPAWN_GUARD: u32 = 0x0068_50C0;
pub const FRAME_HOOK: u32 = 0x0068_50E0;
pub const FRAME_END_HOOK: u32 = 0x0068_5100;
/// One u32 of state in .data's zero-initialised tail (see caves.asm).
pub const LAST_HOOK_MS: u32 = 0x00B4_B960;
/// The pacer stands aside this long after a frame-hook store.
pub const HOOK_HOLD_MS: u32 = 100;
pub const CAVE_LEN: usize = 0xE0;

/// `prefix`, then `jmp short` to the end of the `N`-byte site, then int3
/// filler (never executed). Same bytes as `skip_rest` in pacer/sites.asm.
const fn skip_fill<const N: usize>(prefix: &[u8]) -> [u8; N] {
    assert!(N >= prefix.len() + 2 && N - prefix.len() - 2 <= 0x7F);
    let mut out = [0xCCu8; N];
    let mut i = 0;
    while i < prefix.len() {
        out[i] = prefix[i];
        i += 1;
    }
    out[i] = 0xEB;
    out[i + 1] = (N - prefix.len() - 2) as u8;
    out
}

// ---- original bytes (clean exe, read with REA) ---------------------------

/// `0x595660`: push ecx / lea eax,[esp] / push eax.
pub const SPAWN_ORIG: [u8; 5] = [0x51, 0x8D, 0x04, 0x24, 0x50];
/// `0x595249`: pacer start, master ms → race clock (and the run-flag test).
pub const PACER_START_ORIG: [u8; 48] = [
    0xE8, 0x42, 0x46, 0xFF, 0xFF, 0x8B, 0x0D, 0x2C, 0x09, 0x95, 0x00, 0x8B, 0x15, 0x28, 0x09, 0x95,
    0x00, 0x57, 0x68, 0xE8, 0x03, 0x00, 0x00, 0x51, 0x52, 0xE8, 0x89, 0xA6, 0xF1, 0xFF, 0xA3, 0xD8,
    0x70, 0x96, 0x00, 0xA0, 0xA4, 0x20, 0xB4, 0x00, 0x84, 0xC0, 0x89, 0x15, 0xDC, 0x70, 0x96, 0x00,
];
/// `0x595323`: add [race],16 / mov [esp+0x18],ecx / adc [race+4],0.
pub const PACER_TICK_ORIG: [u8; 18] = [
    0x83, 0x05, 0xD8, 0x70, 0x96, 0x00, 0x10, 0x89, 0x4C, 0x24, 0x18, 0x83, 0x15, 0xDC, 0x70, 0x96,
    0x00, 0x00,
];
/// `0x595381`: pacer catch-up snap, master ms → race clock.
pub const PACER_SNAP_ORIG: [u8; 40] = [
    0xE8, 0x0A, 0x45, 0xFF, 0xFF, 0xA1, 0x2C, 0x09, 0x95, 0x00, 0x8B, 0x0D, 0x28, 0x09, 0x95, 0x00,
    0x57, 0x68, 0xE8, 0x03, 0x00, 0x00, 0x50, 0x51, 0xE8, 0x52, 0xA5, 0xF1, 0xFF, 0xA3, 0xD8, 0x70,
    0x96, 0x00, 0x89, 0x15, 0xDC, 0x70, 0x96, 0x00,
];
/// `0x650655`: keyframe-clock init, master ms → race clock.
pub const KF_INIT_ORIG: [u8; 41] = [
    0xE8, 0x36, 0x92, 0xF3, 0xFF, 0x8B, 0x15, 0x2C, 0x09, 0x95, 0x00, 0xA1, 0x28, 0x09, 0x95, 0x00,
    0x6A, 0x00, 0x68, 0xE8, 0x03, 0x00, 0x00, 0x52, 0x50, 0xE8, 0x7D, 0xF2, 0xE5, 0xFF, 0xA3, 0xD8,
    0x70, 0x96, 0x00, 0x89, 0x15, 0xDC, 0x70, 0x96, 0x00,
];
/// `0x5D21E5`: race-state init, master ms → race clock.
pub const RACE_INIT_ORIG: [u8; 41] = [
    0xE8, 0xA6, 0x76, 0xFB, 0xFF, 0x8B, 0x15, 0x2C, 0x09, 0x95, 0x00, 0xA1, 0x28, 0x09, 0x95, 0x00,
    0x6A, 0x00, 0x68, 0xE8, 0x03, 0x00, 0x00, 0x52, 0x50, 0xE8, 0xED, 0xD6, 0xED, 0xFF, 0xA3, 0xD8,
    0x70, 0x96, 0x00, 0x89, 0x15, 0xDC, 0x70, 0x96, 0x00,
];
/// `0x5D31F0`: push ebp / mov ebp,esp / and esp,-8.
pub const KF_UPDATE_ORIG: [u8; 6] = [0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8];
/// `0x58E9D3` in the end-of-frame function: call 0x589890 (master update
/// that becomes the frame timestamp).
pub const FRAME_END_ORIG: [u8; 5] = [0xE8, 0xB8, 0xAE, 0xFF, 0xFF];
/// `.text` VirtualSize (section header at file offset 0x210, field +8).
pub const TEXT_VSIZE_OFFSET: usize = 0x218;
pub const TEXT_VSIZE_ORIG: [u8; 4] = 0x0028_4028u32.to_le_bytes();

// ---- patched bytes (NASM output of pacer/sites.asm, SITE=1..7) -------------

/// jmp SPAWN_GUARD
pub const SPAWN_NEW: [u8; 5] = [0xE9, 0x5B, 0xFA, 0x0E, 0x00];
/// call STORE_RACE_CLOCK / mov al,[0xB420A4] / test al,al / jmp short end / int3…
pub const PACER_START_NEW: [u8; 48] = skip_fill(&[
    0xE8, 0xE2, 0xFD, 0x0E, 0x00, 0xA0, 0xA4, 0x20, 0xB4, 0x00, 0x84, 0xC0,
]);
/// mov [esp+0x18],ecx / call PACER_STORE / jmp short end / int3…
pub const PACER_TICK_NEW: [u8; 18] =
    skip_fill(&[0x89, 0x4C, 0x24, 0x18, 0xE8, 0x0C, 0xFD, 0x0E, 0x00]);
/// call PACER_STORE / jmp short end / int3…
pub const PACER_SNAP_NEW: [u8; 40] = skip_fill(&[0xE8, 0xB2, 0xFC, 0x0E, 0x00]);
/// call STORE_RACE_CLOCK / jmp short end / int3…
pub const KF_INIT_NEW: [u8; 41] = skip_fill(&[0xE8, 0xD6, 0x49, 0x03, 0x00]);
/// call STORE_RACE_CLOCK / jmp short end / int3…
pub const RACE_INIT_NEW: [u8; 41] = skip_fill(&[0xE8, 0x46, 0x2E, 0x0B, 0x00]);
/// jmp FRAME_HOOK / int3
pub const KF_UPDATE_NEW: [u8; 6] = [0xE9, 0xEB, 0x1E, 0x0B, 0x00, 0xCC];
/// call FRAME_END_HOOK
pub const FRAME_END_NEW: [u8; 5] = [0xE8, 0x28, 0x67, 0x0F, 0x00];
pub const TEXT_VSIZE_NEW: [u8; 4] = 0x0028_5000u32.to_le_bytes();
/// `.data` VirtualSize (section header at file offset 0x260, field +8).
pub const DATA_VSIZE_OFFSET: usize = 0x268;
pub const DATA_VSIZE_ORIG: [u8; 4] = 0x0043_C95Cu32.to_le_bytes();
pub const DATA_VSIZE_NEW: [u8; 4] = 0x0043_C964u32.to_le_bytes();

/// The cave, VA 0x685030..0x68510F (NASM output of pacer/caves.asm; listing
/// there). 0xCC pads between the three routines.
pub const CAVE: [u8; CAVE_LEN] = [
    // store_race_clock @ 0x685030, pacer_store @ 0x685038 (shared body),
    // spawn_guard @ 0x6850C0, frame_hook @ 0x6850E0, frame_end_hook @ 0x685100
    0x53, 0x51, 0x31, 0xDB, 0xEB, 0x09, 0xCC, 0xCC, 0x53, 0x51, 0xBB, 0x01, 0x00, 0x00, 0x00, 0x68,
    0x80, 0x34, 0xA1, 0x00, 0xFF, 0x15, 0x14, 0x62, 0x68, 0x00, 0xE8, 0x41, 0x48, 0xF0, 0xFF, 0x6A,
    0x00, 0x68, 0xE8, 0x03, 0x00, 0x00, 0xFF, 0x35, 0x2C, 0x09, 0x95, 0x00, 0xFF, 0x35, 0x28, 0x09,
    0x95, 0x00, 0xE8, 0x89, 0xA8, 0xE2, 0xFF, 0x85, 0xDB, 0x74, 0x0D, 0x89, 0xC1, 0x2B, 0x0D, 0x60,
    0xB9, 0xB4, 0x00, 0x83, 0xF9, 0x64, 0x72, 0x29, 0x89, 0xC3, 0x89, 0xD1, 0xA1, 0xD8, 0x70, 0x96,
    0x00, 0x8B, 0x15, 0xDC, 0x70, 0x96, 0x00, 0x39, 0xD1, 0x72, 0x21, 0x77, 0x04, 0x39, 0xC3, 0x76,
    0x1B, 0xF0, 0x0F, 0xC7, 0x0D, 0xD8, 0x70, 0x96, 0x00, 0x75, 0xEC, 0x89, 0xD8, 0x89, 0xCA, 0xEB,
    0x0B, 0xA1, 0xD8, 0x70, 0x96, 0x00, 0x8B, 0x15, 0xDC, 0x70, 0x96, 0x00, 0x52, 0x50, 0x68, 0x80,
    0x34, 0xA1, 0x00, 0xFF, 0x15, 0x18, 0x62, 0x68, 0x00, 0x58, 0x5A, 0x59, 0x5B, 0xC3, 0xCC, 0xCC,
    0x83, 0x7E, 0x1C, 0x00, 0x75, 0x0A, 0x51, 0x8D, 0x04, 0x24, 0x50, 0xE9, 0x95, 0x05, 0xF1, 0xFF,
    0xC6, 0x46, 0x22, 0x01, 0xB0, 0x01, 0xC3, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC,
    0xE8, 0x4B, 0xFF, 0xFF, 0xFF, 0xA3, 0x60, 0xB9, 0xB4, 0x00, 0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8,
    0xE9, 0x01, 0xE1, 0xF4, 0xFF, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC,
    0xE8, 0x2B, 0xFF, 0xFF, 0xFF, 0xA3, 0x60, 0xB9, 0xB4, 0x00, 0xC3, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC,
];

const ZERO_CAVE: [u8; CAVE_LEN] = [0; CAVE_LEN];

/// One entry per site: VA (or file offset), original, patched, note.
pub const SITES: &[PatchSite] = &[
    PatchSite {
        at: At::Va(0x0059_5660),
        original: &SPAWN_ORIG,
        patched: &SPAWN_NEW,
        note: "R1: spawn 0x595660 jumps to the guard (no new pacer while handle 0xB420A0 is set)",
    },
    PatchSite {
        at: At::Va(0x0059_5249),
        original: &PACER_START_ORIG,
        patched: &PACER_START_NEW,
        note: "R2/R4: pacer start stores master ms through the monotonic store",
    },
    PatchSite {
        at: At::Va(0x0059_5323),
        original: &PACER_TICK_ORIG,
        patched: &PACER_TICK_NEW,
        note: "R2/R3: pacer tick stores master ms (not +16), and only while no frame hook ran in the last 100 ms",
    },
    PatchSite {
        at: At::Va(0x0059_5381),
        original: &PACER_SNAP_ORIG,
        patched: &PACER_SNAP_NEW,
        note: "R2/R4: pacer catch-up snap through the same conditional store as the tick",
    },
    PatchSite {
        at: At::Va(0x0065_0655),
        original: &KF_INIT_ORIG,
        patched: &KF_INIT_NEW,
        note: "R4: keyframe-clock init 0x650630 through the monotonic store",
    },
    PatchSite {
        at: At::Va(0x005D_21E5),
        original: &RACE_INIT_ORIG,
        patched: &RACE_INIT_NEW,
        note: "R4: race-state init 0x5D21C0 through the monotonic store",
    },
    PatchSite {
        at: At::Va(0x005D_31F0),
        original: &KF_UPDATE_ORIG,
        patched: &KF_UPDATE_NEW,
        note: "R3: keyframe update 0x5D31F0 refreshes the race clock on entry (main thread) and marks the hook active",
    },
    PatchSite {
        at: At::Va(0x0058_E9D3),
        original: &FRAME_END_ORIG,
        patched: &FRAME_END_NEW,
        note: "R2/R3: end-of-frame 0x58E9B0 sets the race clock from the frame-timestamp master sample, every presented frame",
    },
    PatchSite {
        at: At::Va(CAVE_VA),
        original: &ZERO_CAVE,
        patched: &CAVE,
        note: "R1-R4: cave code (store_race_clock/pacer_store, spawn_guard, frame_hook) in .text slack",
    },
    PatchSite {
        at: At::Offset(TEXT_VSIZE_OFFSET),
        original: &TEXT_VSIZE_ORIG,
        patched: &TEXT_VSIZE_NEW,
        note: ".text VirtualSize 0x284028 -> 0x285000 so the cave is inside the declared section",
    },
    PatchSite {
        at: At::Offset(DATA_VSIZE_OFFSET),
        original: &DATA_VSIZE_ORIG,
        patched: &DATA_VSIZE_NEW,
        note: ".data VirtualSize 0x43C95C -> 0x43C964 to declare the u32 state at 0xB4B960 (zero-initialised)",
    },
];

/// The clean-room stutter fix.
pub const PACER: PatchSet = PatchSet {
    name: "pacer",
    summary: "Smooth frame pacing: one pacer thread, race clock follows real time every frame",
    editions: &[SSE1_CLEAN],
    sites: SITES,
    caves: &[PatchCave {
        name: "pacer",
        at: At::Va(CAVE_VA),
        len: CAVE_LEN,
    }],
    results: &[(SSE1_CLEAN, SSE1_CLEANROOM_PACER)],
};
