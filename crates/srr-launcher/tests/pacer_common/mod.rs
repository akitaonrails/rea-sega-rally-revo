//! A synthetic stand-in for `SEGA Rally_SSE1.exe`: the same PE header
//! layout (e_lfanew 0x118, section headers for `.text`, `.rdata` and
//! `.data` at file offsets 0x210/0x238/0x260 with the real sizes and flags,
//! image base 0x400000, file offset = RVA) and each site's original bytes
//! in place. Everything else is filler; no game bytes beyond the short
//! site originals are used.
#![allow(dead_code)]

use rea_launcher::patch::At;
use srr_launcher::pacer::SITES;

pub const TEXT_RAW: usize = 0x1000;
pub const TEXT_RAW_SIZE: usize = 0x28_5000;
pub const TEXT_VSIZE: u32 = 0x28_4028;
pub const DATA_VSIZE: u32 = 0x43_C95C;
/// (name, VirtualSize, RVA, raw size, raw offset, flags) as in the game.
type Section = (&'static [u8; 8], u32, u32, u32, u32, u32);
const SECTIONS: [Section; 3] = [
    (
        b".text\0\0\0",
        TEXT_VSIZE,
        0x1000,
        0x28_5000,
        0x1000,
        0x6000_0020,
    ),
    (
        b".rdata\0\0",
        0x8_8B15,
        0x28_6000,
        0x8_9000,
        0x28_6000,
        0x4000_0040,
    ),
    (
        b".data\0\0\0",
        DATA_VSIZE,
        0x30_F000,
        0x1_C000,
        0x30_F000,
        0xC000_0040,
    ),
];

pub fn synthetic() -> Vec<u8> {
    let mut f = vec![0u8; 0x32_B000];
    f[0..2].copy_from_slice(b"MZ");
    f[0x3c..0x40].copy_from_slice(&0x118u32.to_le_bytes());
    let pe = 0x118;
    f[pe..pe + 4].copy_from_slice(b"PE\0\0");
    let coff = pe + 4;
    f[coff..coff + 2].copy_from_slice(&0x14cu16.to_le_bytes()); // i386
    f[coff + 2..coff + 4].copy_from_slice(&3u16.to_le_bytes()); // sections
    f[coff + 16..coff + 18].copy_from_slice(&0xe0u16.to_le_bytes()); // optional size
    f[coff + 18..coff + 20].copy_from_slice(&0x0103u16.to_le_bytes()); // relocs stripped
    let opt = coff + 20; // 0x130
    f[opt..opt + 2].copy_from_slice(&0x10bu16.to_le_bytes());
    f[opt + 16..opt + 20].copy_from_slice(&0x1000u32.to_le_bytes()); // entry
    f[opt + 28..opt + 32].copy_from_slice(&0x40_0000u32.to_le_bytes()); // image base
    f[opt + 32..opt + 36].copy_from_slice(&0x1000u32.to_le_bytes()); // section align
    f[opt + 36..opt + 40].copy_from_slice(&0x1000u32.to_le_bytes()); // file align
    let sec = opt + 0xe0;
    assert_eq!(sec, 0x210, "same section-table offset as the game");
    for (i, (name, vsize, rva, raw_size, raw_off, flags)) in SECTIONS.iter().enumerate() {
        let h = sec + 40 * i;
        f[h..h + 8].copy_from_slice(*name);
        f[h + 8..h + 12].copy_from_slice(&vsize.to_le_bytes());
        f[h + 12..h + 16].copy_from_slice(&rva.to_le_bytes());
        f[h + 16..h + 20].copy_from_slice(&raw_size.to_le_bytes());
        f[h + 20..h + 24].copy_from_slice(&raw_off.to_le_bytes());
        f[h + 36..h + 40].copy_from_slice(&flags.to_le_bytes());
    }
    // Code filler up to the game's last code byte (VA 0x685027), zero slack
    // after it, like the real file.
    f[TEXT_RAW..0x28_5028].fill(0xCC);
    for site in SITES {
        let off = match site.at {
            At::Va(va) => (va - 0x40_0000) as usize,
            At::Offset(o) => o,
        };
        f[off..off + site.original.len()].copy_from_slice(site.original);
    }
    f
}
