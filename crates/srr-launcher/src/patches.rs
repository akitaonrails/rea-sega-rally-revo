//! Binary patches for the private runtime copy of the game, as SDK
//! `PatchSet` tables (`rea_launcher::patch`): every site is original →
//! patched bytes at an exact file offset of an exact build (SHA-256), and
//! the launcher applies them from the pristine bytes in the player's game
//! folder, all or nothing, never to an unidentified executable.
//!
//! - [`PACER`]: the stutter fix for the clean `SEGA Rally_SSE1.exe`, our
//!   clean-room implementation ([`crate::pacer`], `docs/PACER-PROVENANCE.md`;
//!   stuken/SegaRallyRevoStutterFix is credited for the idea and the
//!   problem analysis only). The result is checked against its SHA-256.
//!   The community-fixed build ([`SSE1_PACER`]) is recognised as already
//!   fixed and never patched.
//! - The aspect sets: the renderer's 16:9 constant (`39 8E E3 3F`) in its
//!   two aspect tables becomes 21:9 or 32:9 for ultra-wide resolutions,
//!   the PCGamingWiki/WSGF hex edit at this build's own offsets.
//!
//! `docs/RESEARCH.md` explains the sites; `describe()` prints them.

use rea_launcher::identify::hex32;
use rea_launcher::patch::{self, At, PatchError, PatchSet, PatchSite};

use crate::config::Aspect;

/// The clean `SEGA Rally_SSE1.exe` v4.0.6.0 (3,727,360 bytes).
pub const SSE1_CLEAN: [u8; 32] =
    hex32("22655a54bad6932239ade58325703bd4b5a2020376737748026443fa5aadf76e");
/// The same build with the community stutter fix (stuken's patch.py)
/// applied by the player: recognised as already fixed, never patched.
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

/// The stutter fix: the clean-room implementation in [`crate::pacer`].
pub use crate::pacer::PACER;

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
    fn the_stutter_fix_is_the_clean_room_set() {
        // The shipped pacer is our clean-room implementation
        // (docs/PACER-PROVENANCE.md): clean SSE1 in, cb9a5a34…ff65 out.
        assert_eq!(PACER.name, "pacer");
        assert_eq!(PACER.editions, &[SSE1_CLEAN]);
        assert_eq!(
            PACER.results,
            &[(SSE1_CLEAN, crate::pacer::SSE1_CLEANROOM_PACER)]
        );
        assert_eq!(PACER.sites.len(), crate::pacer::SITES.len());
        // The community-fixed build is still recognised, never patched.
        assert_eq!(SSE1_PACER, crate::pacer::SSE1_COMMUNITY_PACER);
        assert!(!PACER.editions.contains(&SSE1_PACER));
    }

    #[test]
    fn every_set_passes_the_golden_checks() {
        // The pacer has VA sites and its own golden tests on a synthetic
        // PE (tests/pacer_golden.rs).
        for set in ALL.iter().filter(|set| set.name != "pacer") {
            testing::assert_golden(set, &synthetic(set));
        }
    }

    #[test]
    fn unknown_builds_are_refused() {
        // A synthetic file is no known build: nothing is patched.
        // (The pacer's refusal of unknown builds: tests/pacer_golden.rs.)
        assert!(matches!(
            apply(&synthetic(&ASPECT_21_SSE1), &[&ASPECT_21_SSE1]),
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
