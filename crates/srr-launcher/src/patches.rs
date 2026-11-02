//! Binary patches for the private runtime copy of the game, as SDK
//! `PatchSet` tables (`rea_launcher::patch`): every site is original →
//! patched bytes at an exact file offset of an exact build (SHA-256), and
//! the launcher applies them from the pristine bytes in the player's game
//! folder, all or nothing, never to an unidentified executable.
//!
//! - [`PACER`]: the community stutter fix for the clean
//!   `SEGA Rally_SSE1.exe` (reimplemented from
//!   stuken/SegaRallyRevoStutterFix, whose reference build is
//!   byte-identical to this one). Three hooks jump to three code caves:
//!   at most one pacer thread (the Championship stage-2 judder), a race
//!   clock sampled from the master clock (the 16/17 ms jitter), and a
//!   per-frame race-clock refresh. The result is checked against the
//!   fix's reference SHA-256.
//! - The aspect sets: the renderer's 16:9 constant (`39 8E E3 3F`) in its
//!   two aspect tables becomes 21:9 or 32:9 for ultra-wide resolutions,
//!   the PCGamingWiki/WSGF hex edit at this build's own offsets.
//!
//! `docs/RESEARCH.md` explains the sites; `describe()` prints them.

use rea_launcher::identify::hex32;
use rea_launcher::patch::{self, At, PatchCave, PatchError, PatchSet, PatchSite};

use crate::config::Aspect;

/// The clean `SEGA Rally_SSE1.exe` v4.0.6.0 (3,727,360 bytes).
pub const SSE1_CLEAN: [u8; 32] =
    hex32("22655a54bad6932239ade58325703bd4b5a2020376737748026443fa5aadf76e");
/// The same build with the stutter fix applied (the fix's reference hash).
pub const SSE1_PACER: [u8; 32] =
    hex32("4c08c1deae28a329b6588138198556ac8eaded8444d0eb94a1fe2f1ad08f2520");
/// The NoDVD `SEGA Rally.exe` v4.0.6.0 of the widely mirrored full rip.
pub const MAIN_NODVD: [u8; 32] =
    hex32("ad84ecea6ac5e86e1f1a420f07595866429bba63e7c489b40661a8032ba391d7");

/// `N` bytes from hex, at compile time.
const fn hex<const N: usize>(text: &str) -> [u8; N] {
    let text = text.as_bytes();
    assert!(text.len() == 2 * N);
    const fn digit(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            _ => panic!("lower-case hex"),
        }
    }
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = digit(text[2 * i]) << 4 | digit(text[2 * i + 1]);
        i += 1;
    }
    out
}

// Engine sites in the clean SSE1 build; in `.text` the file offset is the
// virtual address minus the image base 0x400000.
const SPAWN_HOOK: usize = 0x195_660; // pacer thread spawn
const TICK_HOOK: usize = 0x195_323; // race-clock tick (+16 ms)
const MAIN_HOOK: usize = 0x158_895; // main loop, power-poll deadline
const CAVE1: usize = 0x285_100; // spawn guard
const CAVE2: usize = 0x285_180; // race-clock tick from the master clock
const CAVE3: usize = 0x285_200; // main-loop race-clock refresh

const SPAWN_GUARD: [u8; 39] =
    hex("833da020b40000750a518d042450e95205f1ffc605a420b40001c605a520b40000b801000000c3");
const RACE_CLOCK: [u8; 57] = hex(
    "51e80a47f0ff8b152c099500a1280995006a0068e80300005250e851a7e2ff3b05d8709600760ba3d87096008915dc70960059f8e97101f1ff",
);
const MAIN_REFRESH: [u8; 34] =
    hex("3b05d8709600760ba3d87096008915dc7096008bf23b35748c72008bf8e97d36edff");

/// The community stutter fix (see the module docs).
pub const PACER: PatchSet = PatchSet {
    name: "pacer",
    summary: "Removes the microstutter in every race and the judder from Championship stage 2",
    editions: &[SSE1_CLEAN],
    sites: &[
        PatchSite {
            at: At::Offset(SPAWN_HOOK),
            original: &[0x51, 0x8D, 0x04, 0x24, 0x50],
            patched: &[0xE9, 0x9B, 0xFA, 0x0E, 0x00],
            note: "pacer spawn: jmp to the spawn guard",
        },
        PatchSite {
            at: At::Offset(TICK_HOOK),
            original: &[0x83, 0x05, 0xD8, 0x70, 0x96, 0x00, 0x10],
            patched: &[0xE9, 0x58, 0xFE, 0x0E, 0x00, 0x90, 0x90],
            note: "race-clock tick: jmp to the master-clock tick",
        },
        PatchSite {
            at: At::Offset(MAIN_HOOK),
            original: &[0x8B, 0xF2, 0x3B, 0x35, 0x74],
            patched: &[0xE9, 0x66, 0xC9, 0x12, 0x00],
            note: "main loop: jmp to the race-clock refresh",
        },
        PatchSite {
            at: At::Offset(CAVE1),
            original: &[0; 39],
            patched: &SPAWN_GUARD,
            note: "spawn guard: at most one pacer thread",
        },
        PatchSite {
            at: At::Offset(CAVE2),
            original: &[0; 57],
            patched: &RACE_CLOCK,
            note: "race clock = master clock / 1000, never backwards",
        },
        PatchSite {
            at: At::Offset(CAVE3),
            original: &[0; 34],
            patched: &MAIN_REFRESH,
            note: "race clock refreshed every frame",
        },
    ],
    caves: &[
        PatchCave {
            name: "spawn guard",
            at: At::Offset(CAVE1),
            len: 39,
        },
        PatchCave {
            name: "race clock",
            at: At::Offset(CAVE2),
            len: 57,
        },
        PatchCave {
            name: "main-loop refresh",
            at: At::Offset(CAVE3),
            len: 34,
        },
    ],
    results: &[(SSE1_CLEAN, SSE1_PACER)],
};

const RATIO_16_9: [u8; 4] = hex("398ee33f");
const RATIO_21_9: [u8; 4] = hex("26b41740"); // 64/27, 2560x1080
const RATIO_32_9: [u8; 4] = hex("398e6340");

/// The two 16:9 slots of the aspect tables (`.rdata`), each flanked by
/// 4:3 `AB AA AA 3F` and 16:10 `CD CC CC 3F`.
const SSE1_SLOTS: [usize; 2] = [0x2F_7C44, 0x2F_ADA8];
const MAIN_SLOTS: [usize; 2] = [0x2F_4BF4, 0x2F_7D68];

macro_rules! aspect_set {
    ($name:literal, $summary:literal, $editions:expr, $slots:expr, $ratio:expr) => {
        PatchSet {
            name: $name,
            summary: $summary,
            editions: $editions,
            sites: &[
                PatchSite {
                    at: At::Offset($slots[0]),
                    original: &RATIO_16_9,
                    patched: &$ratio,
                    note: "aspect table 1: 16:9 slot",
                },
                PatchSite {
                    at: At::Offset($slots[1]),
                    original: &RATIO_16_9,
                    patched: &$ratio,
                    note: "aspect table 2: 16:9 slot",
                },
            ],
            caves: &[],
            results: &[],
        }
    };
}

pub const ASPECT_21_SSE1: PatchSet = aspect_set!(
    "aspect 21:9",
    "21:9 screens without stretching",
    &[SSE1_CLEAN, SSE1_PACER],
    SSE1_SLOTS,
    RATIO_21_9
);
pub const ASPECT_32_SSE1: PatchSet = aspect_set!(
    "aspect 32:9",
    "32:9 screens without stretching",
    &[SSE1_CLEAN, SSE1_PACER],
    SSE1_SLOTS,
    RATIO_32_9
);
pub const ASPECT_21_MAIN: PatchSet = aspect_set!(
    "aspect 21:9",
    "21:9 screens without stretching",
    &[MAIN_NODVD],
    MAIN_SLOTS,
    RATIO_21_9
);
pub const ASPECT_32_MAIN: PatchSet = aspect_set!(
    "aspect 32:9",
    "32:9 screens without stretching",
    &[MAIN_NODVD],
    MAIN_SLOTS,
    RATIO_32_9
);

/// Every set, for docs and the real-game test.
pub const ALL: [&PatchSet; 5] = [
    &PACER,
    &ASPECT_21_SSE1,
    &ASPECT_32_SSE1,
    &ASPECT_21_MAIN,
    &ASPECT_32_MAIN,
];

/// The aspect set for a build and an aspect, `None` for native aspects or
/// a build without a table.
pub fn aspect_set(build: [u8; 32], aspect: Aspect) -> Option<&'static PatchSet> {
    let main = build == MAIN_NODVD;
    let sse1 = build == SSE1_CLEAN || build == SSE1_PACER;
    match (aspect, main, sse1) {
        (Aspect::Ultra21, true, _) => Some(&ASPECT_21_MAIN),
        (Aspect::Ultra32, true, _) => Some(&ASPECT_32_MAIN),
        (Aspect::Ultra21, _, true) => Some(&ASPECT_21_SSE1),
        (Aspect::Ultra32, _, true) => Some(&ASPECT_32_SSE1),
        _ => None,
    }
}

/// The names of `sets`, for the log (`pacer + aspect 21:9`, or `none`).
pub fn names(sets: &[&PatchSet]) -> String {
    if sets.is_empty() {
        return "none".to_string();
    }
    sets.iter()
        .map(|set| set.name)
        .collect::<Vec<_>>()
        .join(" + ")
}

/// `sets` applied to a pristine executable from the game folder; the
/// SDK refuses another build, a site with other bytes or a wrong result
/// hash, and nothing is returned then.
pub fn apply(pristine: &[u8], sets: &[&PatchSet]) -> Result<Vec<u8>, PatchError> {
    patch::apply_sets(pristine, sets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::patch::testing;

    /// `E9 rel32` from `from` to `to` (file offsets in `.text`).
    fn jmp(from: usize, to: usize) -> [u8; 5] {
        let rel = (to as i64 - (from as i64 + 5)) as i32;
        let mut out = [0xE9, 0, 0, 0, 0];
        out[1..].copy_from_slice(&rel.to_le_bytes());
        out
    }

    /// A buffer the size of the clean build with every site's original
    /// bytes in place.
    fn synthetic(set: &PatchSet) -> Vec<u8> {
        let mut data = vec![0x90u8; 3_727_360];
        for site in set.sites {
            let At::Offset(at) = site.at else {
                unreachable!()
            };
            data[at..at + site.original.len()].copy_from_slice(site.original);
        }
        data
    }

    #[test]
    fn hooks_jump_to_their_caves_and_back() {
        assert_eq!(PACER.sites[0].patched, jmp(SPAWN_HOOK, CAVE1));
        assert_eq!(&PACER.sites[1].patched[..5], jmp(TICK_HOOK, CAVE2));
        assert_eq!(PACER.sites[2].patched, jmp(MAIN_HOOK, CAVE3));
        // Each cave ends with a jmp back past its hook (the spawn guard
        // returns or resumes inside its own body).
        assert_eq!(
            RACE_CLOCK[RACE_CLOCK.len() - 5..],
            jmp(CAVE2 + 52, TICK_HOOK + 7)
        );
        assert_eq!(
            MAIN_REFRESH[MAIN_REFRESH.len() - 5..],
            jmp(CAVE3 + 29, 0x158_89F)
        );
    }

    #[test]
    fn every_set_passes_the_golden_checks() {
        for set in ALL {
            testing::assert_golden(set, &synthetic(set));
        }
    }

    #[test]
    fn unknown_builds_are_refused() {
        // A synthetic file is no known build: nothing is patched.
        let data = synthetic(&PACER);
        assert!(matches!(
            apply(&data, &[&PACER]),
            Err(PatchError::UnknownEdition { .. })
        ));
        assert!(matches!(
            apply(&synthetic(&ASPECT_21_MAIN), &[&ASPECT_21_MAIN]),
            Err(PatchError::UnknownEdition { .. })
        ));
    }

    #[test]
    fn the_aspect_set_follows_build_and_aspect() {
        assert_eq!(aspect_set(SSE1_CLEAN, Aspect::Native), None);
        assert_eq!(
            aspect_set(SSE1_CLEAN, Aspect::Ultra21),
            Some(&ASPECT_21_SSE1)
        );
        assert_eq!(
            aspect_set(SSE1_PACER, Aspect::Ultra32),
            Some(&ASPECT_32_SSE1)
        );
        assert_eq!(
            aspect_set(MAIN_NODVD, Aspect::Ultra21),
            Some(&ASPECT_21_MAIN)
        );
        assert_eq!(aspect_set([0; 32], Aspect::Ultra21), None);
        assert_eq!(names(&[&PACER, &ASPECT_21_SSE1]), "pacer + aspect 21:9");
        assert_eq!(names(&[]), "none");
    }

    #[test]
    fn the_site_table_describes_itself() {
        let rows = patch::describe(&ASPECT_21_SSE1, None).unwrap();
        let table = patch::to_markdown(&rows);
        assert!(
            table.contains("2F7C44") || table.contains("2f7c44"),
            "{table}"
        );
    }

    /// Every set on the real game when `SRR_GAME_DIR` names its folder
    /// (never committed): `SRR_GAME_DIR=... cargo test real_game -- --ignored`.
    #[test]
    #[ignore]
    fn real_game() {
        // The two ratios share their sites, so they are checked apart.
        testing::assert_real_game("SRR_GAME_DIR", &[&PACER, &ASPECT_21_SSE1, &ASPECT_21_MAIN]);
        testing::assert_real_game("SRR_GAME_DIR", &[&ASPECT_32_SSE1, &ASPECT_32_MAIN]);
    }
}
