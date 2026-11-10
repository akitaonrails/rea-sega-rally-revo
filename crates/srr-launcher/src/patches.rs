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
//! - [`TIMER`]: with the stutter fix, the simulation thread gets a 1 ms
//!   timer (`timeBeginPeriod(1)` while it runs) and polls `Sleep(1)`, so
//!   its 16 ms beat holds on Windows too.
//! - The aspect sets: the renderer's 16:9 constant (`39 8E E3 3F`) in its
//!   two aspect tables becomes 21:9 or 32:9 for ultra-wide resolutions,
//!   the PCGamingWiki/WSGF hex edit at this build's own offsets.
//!
//! `docs/RESEARCH.md` explains the sites; `describe()` prints them.

use rea_launcher::identify::{self, hex32};
use rea_launcher::patch::{self, At, PatchCave, PatchError, PatchSet, PatchSite};

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

/// Where one build keeps what the borderless and keys sets touch (VAs;
/// `.text` file offset = VA - 0x400000 in both builds). `docs/RESEARCH.md`
/// ("Borderless", "Keyboard keys") has the evidence.
struct Build {
    /// `0x58E350`'s `mov ebx,1 / push 0x20 / mov [FULLSCREEN],ebx`: the
    /// device setup forces the game's "fullscreen" global to 1 before
    /// `Direct3DCreate9` (so `RunWindowed` does nothing).
    fullscreen_site: u32,
    fullscreen_global: u32,
    /// `mov edx,[REFRESH]`: the configured refresh rate, copied to
    /// `FullScreen_RefreshRateInHz` (must be 0 in a window).
    refresh_site: u32,
    refresh_global: u32,
    /// The five `push 0` (style, X, Y, width, height) and `push eax`
    /// (parent) before `CreateWindowExA("SegaClass")`, and where the push
    /// of the window name follows.
    window_site: u32,
    window_resume: u32,
    zero_reg_push: u8,
    /// IAT slot of `GetSystemMetrics`.
    get_system_metrics: u32,
    /// Zero `.text` slack past the game's last code byte.
    window_cave: u32,
    /// The input object's four key accessors (vtable `+0x24/+0x28/+0x20/
    /// +0x1C`): `movzx ecx, byte [esp+8]` loads the key code.
    key_sites: [u32; 4],
    key_cave: u32,
}

const SSE1: Build = Build {
    fullscreen_site: 0x0058_E360,
    fullscreen_global: 0x009E_0CBC,
    refresh_site: 0x0058_E3C6,
    refresh_global: 0x00A1_344C,
    window_site: 0x0055_86B7,
    window_resume: 0x0055_86BD,
    zero_reg_push: 0x53, // push ebx
    get_system_metrics: 0x0068_6298,
    window_cave: 0x0068_5200, // after the pacer's cave (0x685030..0x68510F)
    key_sites: [0x0057_8146, 0x0057_8178, 0x0057_81C3, 0x0057_8203],
    key_cave: 0x0068_5300,
};

const MAIN: Build = Build {
    fullscreen_site: 0x0058_BCB0,
    fullscreen_global: 0x009C_DCBC,
    refresh_site: 0x0058_BD16,
    refresh_global: 0x00A0_044C,
    window_site: 0x0055_6177,
    window_resume: 0x0055_617D,
    zero_reg_push: 0x57, // push edi
    get_system_metrics: 0x0068_326C,
    window_cave: 0x0068_2800,
    key_sites: [0x0057_7086, 0x0057_70B8, 0x0057_7103, 0x0057_7143],
    key_cave: 0x0068_2830,
};

const fn le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// `E8`/`E9` + rel32 from `from` to `to`.
const fn branch(op: u8, from: u32, to: u32) -> [u8; 5] {
    let rel = le(to.wrapping_sub(from.wrapping_add(5)));
    [op, rel[0], rel[1], rel[2], rel[3]]
}

const fn fullscreen_original(b: &Build) -> [u8; 13] {
    let g = le(b.fullscreen_global);
    [
        0xBB, 1, 0, 0, 0, 0x6A, 0x20, 0x89, 0x1D, g[0], g[1], g[2], g[3],
    ]
}

/// `xor ebx,ebx / mov [FULLSCREEN],ebx / inc ebx / push 0x20 / nop nop`:
/// the global is 0 (a window), EBX is 1 again for what follows.
const fn fullscreen_patched(b: &Build) -> [u8; 13] {
    let g = le(b.fullscreen_global);
    [
        0x31, 0xDB, 0x89, 0x1D, g[0], g[1], g[2], g[3], 0x43, 0x6A, 0x20, 0x90, 0x90,
    ]
}

const fn refresh_original(b: &Build) -> [u8; 6] {
    let g = le(b.refresh_global);
    [0x8B, 0x15, g[0], g[1], g[2], g[3]]
}

/// `xor edx,edx`: refresh rate 0, as Direct3D requires for a window.
const REFRESH_PATCHED: [u8; 6] = [0x31, 0xD2, 0x90, 0x90, 0x90, 0x90];

const fn window_original(b: &Build) -> [u8; 6] {
    let z = b.zero_reg_push;
    [0x50, z, z, z, z, z]
}

const fn window_patched(b: &Build) -> [u8; 6] {
    let j = branch(0xE9, b.window_site, b.window_cave);
    [j[0], j[1], j[2], j[3], j[4], 0x90]
}

pub const WINDOW_CAVE_LEN: usize = 33;

/// The window cave: the pushes `CreateWindowExA` gets instead of zeros:
/// parent (EAX, as before), height and width of the screen
/// (`GetSystemMetrics(SM_CYSCREEN/SM_CXSCREEN)`), X = Y = 0 and
/// `WS_POPUP | WS_VISIBLE` (no border, no caption); then back.
const fn window_cave(b: &Build) -> [u8; WINDOW_CAVE_LEN] {
    let g = le(b.get_system_metrics);
    let back = branch(0xE9, b.window_cave + 28, b.window_resume);
    [
        0x50, // push eax (parent)
        0x6A, 0x01, 0xFF, 0x15, g[0], g[1], g[2], g[3], // GetSystemMetrics(SM_CYSCREEN)
        0x50, // push eax (height)
        0x6A, 0x00, 0xFF, 0x15, g[0], g[1], g[2], g[3], // GetSystemMetrics(SM_CXSCREEN)
        0x50, // push eax (width)
        0x6A, 0x00, 0x6A, 0x00, // Y, X
        0x68, 0x00, 0x00, 0x00, 0x90, // WS_POPUP | WS_VISIBLE
        back[0], back[1], back[2], back[3], back[4],
    ]
}

/// `movzx ecx, byte [esp+8]`: the key code an accessor was asked about.
const KEY_ORIGINAL: [u8; 5] = [0x0F, 0xB6, 0x4C, 0x24, 0x08];
pub const KEY_CODE_LEN: usize = 16;
/// Code then table: the table starts `KEY_CODE_LEN` bytes into the cave.
pub const KEY_CAVE_LEN: usize = KEY_CODE_LEN + 256;

/// `movzx ecx, byte [esp+0xC]` (the same argument under the return
/// address) / `movzx ecx, byte [ecx+TABLE]` / `ret`, padded with int3,
/// then the 256-byte table, identity here (the launcher writes the
/// player's keys over it: [`apply`]).
const fn key_cave(b: &Build) -> [u8; KEY_CAVE_LEN] {
    let t = le(b.key_cave + KEY_CODE_LEN as u32);
    let mut out = [0u8; KEY_CAVE_LEN];
    let code = [
        0x0F, 0xB6, 0x4C, 0x24, 0x0C, 0x0F, 0xB6, 0x89, t[0], t[1], t[2], t[3], 0xC3,
    ];
    let mut i = 0;
    while i < KEY_CODE_LEN {
        out[i] = if i < code.len() { code[i] } else { 0xCC };
        i += 1;
    }
    while i < KEY_CAVE_LEN {
        out[i] = (i - KEY_CODE_LEN) as u8;
        i += 1;
    }
    out
}

const fn key_call(b: &Build, i: usize) -> [u8; 5] {
    branch(0xE8, b.key_sites[i], b.key_cave)
}

const ZERO_WINDOW_CAVE: [u8; WINDOW_CAVE_LEN] = [0; WINDOW_CAVE_LEN];
const ZERO_KEY_CAVE: [u8; KEY_CAVE_LEN] = [0; KEY_CAVE_LEN];

macro_rules! borderless_set {
    ($build:expr, $editions:expr) => {
        PatchSet {
            name: "borderless",
            summary: "The game in a borderless window over the whole screen, no display-mode switch",
            editions: $editions,
            sites: &[
                PatchSite {
                    at: At::Va($build.fullscreen_site),
                    original: &fullscreen_original(&$build),
                    patched: &fullscreen_patched(&$build),
                    note: "device setup: the fullscreen global becomes 0 (D3DPRESENT_PARAMETERS.Windowed = 1)",
                },
                PatchSite {
                    at: At::Va($build.refresh_site),
                    original: &refresh_original(&$build),
                    patched: &REFRESH_PATCHED,
                    note: "FullScreen_RefreshRateInHz = 0 (required in a window)",
                },
                PatchSite {
                    at: At::Va($build.window_site),
                    original: &window_original(&$build),
                    patched: &window_patched(&$build),
                    note: "CreateWindowExA(\"SegaClass\"): jump to the window cave for its size and style",
                },
                PatchSite {
                    at: At::Va($build.window_cave),
                    original: &ZERO_WINDOW_CAVE,
                    patched: &window_cave(&$build),
                    note: "window cave: screen-sized WS_POPUP|WS_VISIBLE window at 0,0",
                },
            ],
            caves: &[PatchCave {
                name: "window",
                at: At::Va($build.window_cave),
                len: WINDOW_CAVE_LEN,
            }],
            results: &[],
        }
    };
}

macro_rules! keys_set {
    ($build:expr, $editions:expr) => {
        PatchSet {
            name: "keys",
            summary: "The player's keyboard keys: the game's key accessors read through a table",
            editions: $editions,
            sites: &[
                PatchSite {
                    at: At::Va($build.key_sites[0]),
                    original: &KEY_ORIGINAL,
                    patched: &key_call(&$build, 0),
                    note: "key accessor (last frame): call the key cave",
                },
                PatchSite {
                    at: At::Va($build.key_sites[1]),
                    original: &KEY_ORIGINAL,
                    patched: &key_call(&$build, 1),
                    note: "key accessor (down): call the key cave",
                },
                PatchSite {
                    at: At::Va($build.key_sites[2]),
                    original: &KEY_ORIGINAL,
                    patched: &key_call(&$build, 2),
                    note: "key accessor (just pressed): call the key cave",
                },
                PatchSite {
                    at: At::Va($build.key_sites[3]),
                    original: &KEY_ORIGINAL,
                    patched: &key_call(&$build, 3),
                    note: "key accessor (just released): call the key cave",
                },
                PatchSite {
                    at: At::Va($build.key_cave),
                    original: &ZERO_KEY_CAVE,
                    patched: &key_cave(&$build),
                    note: "key cave: ECX = table[key]; the 256-byte table follows the code",
                },
            ],
            caves: &[PatchCave {
                name: "keys",
                at: At::Va($build.key_cave),
                len: KEY_CAVE_LEN,
            }],
            results: &[],
        }
    };
}

/// Borderless and keys exist for the clean SSE1 build (with or without
/// the stutter fix) and the NoDVD build. The community-fixed SSE1 build
/// is not covered: its caves are unknown to us (never brought in).
pub const BORDERLESS_SSE1: PatchSet = borderless_set!(SSE1, &[SSE1_CLEAN]);
pub const BORDERLESS_MAIN: PatchSet = borderless_set!(MAIN, &[MAIN_NODVD]);
pub const KEYS_SSE1: PatchSet = keys_set!(SSE1, &[SSE1_CLEAN]);
pub const KEYS_MAIN: PatchSet = keys_set!(MAIN, &[MAIN_NODVD]);

/// The simulation thread (`0x5951B0`, the game's "Simulation Thread"
/// running the Clock, Keyframe and Network tasks) waits for its 16 ms beat
/// by polling `Sleep(4)`. The game never asks for a fine timer during
/// play (its only `timeBeginPeriod` belongs to video playback), and since
/// Windows 10 2004 another program's request no longer counts, so on
/// Windows each `Sleep(4)` lasts a 15.6 ms timer tick: the beat runs late
/// and doubles to 31 ms about once every 0.65 s (`docs/RUNTIME_NOTES.md`,
/// 2026-11-09). Wine's timer is always fine, which is why Linux never
/// showed it.
const SIM_THREAD: u32 = 0x0059_51B0;
/// `push ebp / mov ebp,esp / and esp,-8`: the thread's prologue.
const SIM_PROLOGUE: [u8; 6] = [0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8];
/// `push 4` before `call [Sleep]` in the thread's wait loop.
const SIM_WAIT: u32 = 0x0059_52D7;
/// `push 0x6CBDCC` (its exit message), where the thread ends.
const SIM_EXIT: u32 = 0x0059_53CD;
const SIM_EXIT_PUSH: [u8; 5] = [0x68, 0xCC, 0xBD, 0x6C, 0x00];
/// IAT slots of `timeBeginPeriod` and `timeEndPeriod` (WINMM).
const TIME_BEGIN_PERIOD: u32 = 0x0068_62E4;
const TIME_END_PERIOD: u32 = 0x0068_62DC;
/// `.text` slack between the pacer's cave (ends 0x685110) and the window cave.
pub const TIMER_CAVE: u32 = 0x0068_5140;
pub const TIMER_CAVE_LEN: usize = 38;
/// Where the exit routine starts inside the cave.
const TIMER_END_AT: usize = 20;

const fn sim_entry_patched() -> [u8; 6] {
    let j = branch(0xE9, SIM_THREAD, TIMER_CAVE);
    [j[0], j[1], j[2], j[3], j[4], 0x90]
}

const fn sim_exit_patched() -> [u8; 5] {
    branch(0xE9, SIM_EXIT, TIMER_CAVE + TIMER_END_AT as u32)
}

/// `timeBeginPeriod(1)`, the prologue, back to the thread (19 bytes);
/// `int3`; then `timeEndPeriod(1)`, the exit push, back (18 bytes).
const fn timer_cave() -> [u8; TIMER_CAVE_LEN] {
    let b = le(TIME_BEGIN_PERIOD);
    let e = le(TIME_END_PERIOD);
    let back = branch(0xE9, TIMER_CAVE + 14, SIM_THREAD + 6);
    let end = TIMER_CAVE + TIMER_END_AT as u32;
    let out = branch(0xE9, end + 13, SIM_EXIT + 5);
    let p = SIM_PROLOGUE;
    let x = SIM_EXIT_PUSH;
    [
        0x6A, 0x01, 0xFF, 0x15, b[0], b[1], b[2], b[3], // timeBeginPeriod(1)
        p[0], p[1], p[2], p[3], p[4], p[5], // the replaced prologue
        back[0], back[1], back[2], back[3], back[4], // jmp 0x5951B6
        0xCC, 0x6A, 0x01, 0xFF, 0x15, e[0], e[1], e[2], e[3], // timeEndPeriod(1)
        x[0], x[1], x[2], x[3], x[4], // the replaced push
        out[0], out[1], out[2], out[3], out[4], // jmp 0x5953D2
    ]
}

const ZERO_TIMER_CAVE: [u8; TIMER_CAVE_LEN] = [0; TIMER_CAVE_LEN];

/// The simulation thread's 1 ms timer, part of the stutter fix (clean SSE1).
pub const TIMER: PatchSet = PatchSet {
    name: "timer",
    summary: "The simulation thread keeps its 16 ms beat on Windows: a 1 ms timer while it runs, Sleep(1) polling",
    editions: &[SSE1_CLEAN],
    sites: &[
        PatchSite {
            at: At::Va(SIM_THREAD),
            original: &SIM_PROLOGUE,
            patched: &sim_entry_patched(),
            note: "simulation thread entry: jump to the cave for timeBeginPeriod(1)",
        },
        PatchSite {
            at: At::Va(SIM_WAIT),
            original: &[0x6A, 0x04],
            patched: &[0x6A, 0x01],
            note: "simulation thread wait: Sleep(4) becomes Sleep(1)",
        },
        PatchSite {
            at: At::Va(SIM_EXIT),
            original: &SIM_EXIT_PUSH,
            patched: &sim_exit_patched(),
            note: "simulation thread exit: jump to the cave for timeEndPeriod(1)",
        },
        PatchSite {
            at: At::Va(TIMER_CAVE),
            original: &ZERO_TIMER_CAVE,
            patched: &timer_cave(),
            note: "timer cave: timeBeginPeriod(1)/timeEndPeriod(1) around the simulation thread",
        },
    ],
    caves: &[PatchCave {
        name: "timer",
        at: At::Va(TIMER_CAVE),
        len: TIMER_CAVE_LEN,
    }],
    results: &[],
};

/// The addresses and the two sets of a build that has them.
fn build_of(build: [u8; 32]) -> Option<(&'static Build, &'static PatchSet, &'static PatchSet)> {
    if build == SSE1_CLEAN {
        Some((&SSE1, &BORDERLESS_SSE1, &KEYS_SSE1))
    } else if build == MAIN_NODVD {
        Some((&MAIN, &BORDERLESS_MAIN, &KEYS_MAIN))
    } else {
        None
    }
}

/// File offset of the key table in a build's executable.
fn key_table_offset(build: [u8; 32]) -> Option<usize> {
    build_of(build).map(|(b, _, _)| (b.key_cave - 0x0040_0000) as usize + KEY_CODE_LEN)
}

/// The borderless set for a build, `None` for a build without one.
pub fn borderless_set(build: [u8; 32]) -> Option<&'static PatchSet> {
    build_of(build).map(|(_, set, _)| set)
}

/// The keys set for a build, `None` for a build without one.
pub fn keys_set(build: [u8; 32]) -> Option<&'static PatchSet> {
    build_of(build).map(|(_, _, set)| set)
}

/// Every set, for docs and the real-game test.
pub const ALL: [&PatchSet; 10] = [
    &PACER,
    &TIMER,
    &ASPECT_21_SSE1,
    &ASPECT_32_SSE1,
    &ASPECT_21_MAIN,
    &ASPECT_32_MAIN,
    &BORDERLESS_SSE1,
    &BORDERLESS_MAIN,
    &KEYS_SSE1,
    &KEYS_MAIN,
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
///
/// With a keys set among `sets`, `key_table` (see `keys::KeyMap::table`)
/// replaces the set's identity table.
pub fn apply(
    pristine: &[u8],
    sets: &[&PatchSet],
    key_table: Option<&[u8; 256]>,
) -> Result<Vec<u8>, PatchError> {
    let mut out = patch::apply_sets(pristine, sets)?;
    if let Some(table) = key_table
        && sets.iter().any(|set| set.name == "keys")
        && let Some(offset) = key_table_offset(identify::sha256(pristine))
    {
        let identity: [u8; 256] = std::array::from_fn(|k| k as u8);
        patch::apply(
            &mut out,
            &[patch::Site {
                name: "key table",
                offset,
                expected: &identity,
                patched: table,
            }],
        )?;
    }
    Ok(out)
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
        for set in ALL.iter().filter(|set| {
            set.sites
                .iter()
                .all(|site| matches!(site.at, At::Offset(_)))
        }) {
            testing::assert_golden(set, &synthetic(set));
        }
    }

    #[test]
    fn unknown_builds_are_refused() {
        // A synthetic file is no known build: nothing is patched.
        // (The pacer's refusal of unknown builds: tests/pacer_golden.rs.)
        assert!(matches!(
            apply(&synthetic(&ASPECT_21_SSE1), &[&ASPECT_21_SSE1], None),
            Err(PatchError::UnknownEdition { .. })
        ));
        assert!(matches!(
            apply(&synthetic(&ASPECT_21_MAIN), &[&ASPECT_21_MAIN], None),
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

    /// The rel32 target of a 5-byte `E8`/`E9` at `from`.
    fn target(bytes: &[u8], from: u32) -> u32 {
        let rel = i32::from_le_bytes(bytes[1..5].try_into().unwrap());
        from.wrapping_add(5).wrapping_add(rel as u32)
    }

    #[test]
    fn the_borderless_code_returns_where_it_left() {
        for build in [&SSE1, &MAIN] {
            let jump = window_patched(build);
            assert_eq!(jump[0], 0xE9);
            assert_eq!(target(&jump, build.window_site), build.window_cave);
            // The six replaced pushes are exactly the bytes up to the resume.
            assert_eq!(build.window_resume - build.window_site, 6);
            let cave = window_cave(build);
            assert_eq!(&cave[23..28], &[0x68, 0, 0, 0, 0x90], "WS_POPUP|WS_VISIBLE");
            assert_eq!(&cave[5..9], &build.get_system_metrics.to_le_bytes());
            assert_eq!(&cave[14..18], &build.get_system_metrics.to_le_bytes());
            assert_eq!(
                target(&cave[28..], build.window_cave + 28),
                build.window_resume
            );
            // EBX ends at 1, as the original `mov ebx,1` left it.
            assert_eq!(fullscreen_patched(build)[8], 0x43);
            assert_eq!(
                &fullscreen_patched(build)[4..8],
                &build.fullscreen_global.to_le_bytes()
            );
        }
    }

    #[test]
    fn every_key_accessor_calls_the_table_lookup() {
        for build in [&SSE1, &MAIN] {
            for (i, site) in build.key_sites.iter().enumerate() {
                let call = key_call(build, i);
                assert_eq!(call[0], 0xE8);
                assert_eq!(target(&call, *site), build.key_cave);
            }
            let cave = key_cave(build);
            // movzx ecx,[esp+0xC]; movzx ecx,[ecx+TABLE]; ret
            assert_eq!(
                &cave[..8],
                &[0x0F, 0xB6, 0x4C, 0x24, 0x0C, 0x0F, 0xB6, 0x89]
            );
            let table = build.key_cave + KEY_CODE_LEN as u32;
            assert_eq!(&cave[8..12], &table.to_le_bytes());
            assert_eq!(cave[12], 0xC3);
            assert!(
                cave[KEY_CODE_LEN..]
                    .iter()
                    .enumerate()
                    .all(|(k, v)| k as u8 == *v)
            );
        }
        // The caves stay apart from each other and from the pacer's.
        let pacer = (0x0068_5030u32, 0x0068_5110u32);
        assert!(SSE1.window_cave >= pacer.1);
        assert!(SSE1.window_cave + WINDOW_CAVE_LEN as u32 <= SSE1.key_cave);
        assert!(SSE1.key_cave + KEY_CAVE_LEN as u32 <= 0x0068_6000);
        assert!(MAIN.window_cave + WINDOW_CAVE_LEN as u32 <= MAIN.key_cave);
        assert!(MAIN.key_cave + KEY_CAVE_LEN as u32 <= 0x0068_3000);
        assert_eq!(key_table_offset(SSE1_CLEAN), Some(0x28_5310));
        assert_eq!(key_table_offset(MAIN_NODVD), Some(0x28_2840));
        assert_eq!(key_table_offset(SSE1_PACER), None);
    }

    #[test]
    fn the_timer_set_gives_the_simulation_thread_a_1_ms_timer() {
        // Answers Quiver/issue #6 (Windows stutter): the simulation thread
        // (0x5951B0) polls Sleep(4) for its 16 ms beat; on Windows 10 2004+
        // without the game's own timeBeginPeriod that sleep lasts 15.6 ms.
        assert_eq!(TIMER.name, "timer");
        assert_eq!(TIMER.editions, &[SSE1_CLEAN]);
        let site = |va: u32| {
            TIMER
                .sites
                .iter()
                .find(|s| s.at == At::Va(va))
                .unwrap_or_else(|| panic!("no site at {va:#x}"))
        };
        // Thread entry: the prologue jumps to the cave, which calls
        // timeBeginPeriod(1), replays the prologue and comes back.
        let entry = site(0x0059_51B0);
        assert_eq!(entry.original, &[0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8]);
        assert_eq!(entry.patched[0], 0xE9);
        assert_eq!(entry.patched[5], 0x90);
        assert_eq!(target(entry.patched, 0x0059_51B0), TIMER_CAVE);
        let cave = site(TIMER_CAVE).patched;
        assert_eq!(cave.len(), TIMER_CAVE_LEN);
        assert_eq!(
            &cave[..8],
            &[0x6A, 0x01, 0xFF, 0x15, 0xE4, 0x62, 0x68, 0x00]
        );
        assert_eq!(&cave[8..14], &[0x55, 0x8B, 0xEC, 0x83, 0xE4, 0xF8]);
        assert_eq!(cave[14], 0xE9);
        assert_eq!(target(&cave[14..], TIMER_CAVE + 14), 0x0059_51B6);
        // The wait: Sleep(4) becomes Sleep(1).
        let wait = site(0x0059_52D7);
        assert_eq!(
            (wait.original, wait.patched),
            (&[0x6A, 0x04][..], &[0x6A, 0x01][..])
        );
        // Thread exit: timeEndPeriod(1), then the original push and back.
        let exit = site(0x0059_53CD);
        assert_eq!(exit.original, &[0x68, 0xCC, 0xBD, 0x6C, 0x00]);
        let end = target(exit.patched, 0x0059_53CD);
        let at = (end - TIMER_CAVE) as usize;
        assert_eq!(
            &cave[at..at + 8],
            &[0x6A, 0x01, 0xFF, 0x15, 0xDC, 0x62, 0x68, 0x00]
        );
        assert_eq!(&cave[at + 8..at + 13], &[0x68, 0xCC, 0xBD, 0x6C, 0x00]);
        assert_eq!(target(&cave[at + 13..], end + 13), 0x0059_53D2);
        // Between the pacer's cave and the window cave.
        assert!(TIMER_CAVE >= 0x0068_5110);
        assert!(TIMER_CAVE + TIMER_CAVE_LEN as u32 <= SSE1.window_cave);
        assert!(ALL.contains(&&TIMER));
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
        // Borderless and keys together with the pacer and an aspect set.
        testing::assert_real_game(
            "SRR_GAME_DIR",
            &[
                &PACER,
                &TIMER,
                &ASPECT_21_SSE1,
                &BORDERLESS_SSE1,
                &KEYS_SSE1,
                &ASPECT_21_MAIN,
                &BORDERLESS_MAIN,
                &KEYS_MAIN,
            ],
        );
    }
}
