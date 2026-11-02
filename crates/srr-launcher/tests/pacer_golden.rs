//! Layer 1 (always runs): golden bytes, all-or-nothing, idempotence,
//! unpatch, refusal of unknown builds, header site semantics.

#[path = "pacer_common/mod.rs"]
mod common;

use rea_launcher::identify::sha256;
use rea_launcher::patch::{
    self, At, PatchError, PatchSet, apply_sets, describe, testing, to_markdown, unpatch,
};
use rea_launcher::pe::Pe;
use srr_launcher::pacer::*;

fn leak<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

/// PACER retargeted at the synthetic build, with its result hash.
fn synthetic_set(img: &[u8]) -> &'static PatchSet {
    let ed = sha256(img);
    let bare = PatchSet {
        editions: leak(vec![ed]),
        results: &[],
        ..PACER
    };
    let out = apply_sets(img, &[&bare]).unwrap();
    Box::leak(Box::new(PatchSet {
        results: leak(vec![(ed, sha256(&out))]),
        ..bare
    }))
}

#[test]
fn golden_bytes_on_a_synthetic_image() {
    testing::assert_golden(&PACER, &common::synthetic());
}

#[test]
fn every_site_lands_where_the_table_says() {
    let img = common::synthetic();
    let set = synthetic_set(&img);
    let out = apply_sets(&img, &[set]).unwrap();
    let pe = Pe::parse(&img).unwrap();
    for site in PACER.sites {
        let off = match site.at {
            At::Va(va) => pe.va_to_offset(u64::from(va)).unwrap(),
            At::Offset(o) => o,
        };
        assert_eq!(
            &out[off..off + site.patched.len()],
            site.patched,
            "{}",
            site.note
        );
    }
    // Only site bytes changed.
    let changed: usize = img.iter().zip(&out).filter(|(a, b)| a != b).count();
    let max: usize = PACER.sites.iter().map(|s| s.patched.len()).sum();
    assert!(changed <= max);
}

#[test]
fn header_site_raises_text_virtual_size_only() {
    let img = common::synthetic();
    let out = apply_sets(&img, &[synthetic_set(&img)]).unwrap();
    let before = Pe::parse(&img).unwrap();
    let after = Pe::parse(&out).unwrap();
    let (b, a) = (
        before.section(".text").unwrap(),
        after.section(".text").unwrap(),
    );
    assert_eq!(b.virtual_size, 0x28_4028);
    assert_eq!(a.virtual_size, 0x28_5000);
    assert_eq!(a.virtual_size, a.raw_size, "VirtualSize = SizeOfRawData");
    // Rounded to the 4 KiB section alignment it is unchanged, so
    // SizeOfImage and every later section stay where they were.
    assert_eq!(
        b.virtual_size.div_ceil(0x1000),
        a.virtual_size.div_ceil(0x1000)
    );
    assert_eq!(
        (a.rva, a.raw_offset, a.raw_size, a.characteristics),
        (b.rva, b.raw_offset, b.raw_size, b.characteristics)
    );
    // The cave is past the old VirtualSize and inside the new one.
    let cave_rva = CAVE_VA - 0x40_0000;
    assert!(cave_rva >= a.rva + b.virtual_size);
    assert!(cave_rva + CAVE_LEN as u32 <= a.rva + a.virtual_size);
    assert!(after.is_executable(cave_rva as usize));
    // .data grows by 8 bytes to declare the state dword; same page count.
    let (bd, ad) = (
        before.section(".data").unwrap(),
        after.section(".data").unwrap(),
    );
    assert_eq!((bd.virtual_size, ad.virtual_size), (0x43_C95C, 0x43_C964));
    assert_eq!(
        bd.virtual_size.div_ceil(0x1000),
        ad.virtual_size.div_ceil(0x1000)
    );
    let state_rva = LAST_HOOK_MS - 0x40_0000;
    assert!(state_rva >= bd.rva + bd.virtual_size && state_rva + 4 <= ad.rva + ad.virtual_size);
    assert!(
        state_rva >= bd.rva + bd.raw_size,
        "in the zero-initialised (BSS) part, no file bytes"
    );
    assert_eq!(after.sections[1], before.sections[1], ".rdata untouched");
}

#[test]
fn caves_are_in_the_verified_slack_and_checked() {
    assert!(CAVE_VA >= 0x68_5028 && CAVE_VA + CAVE_LEN as u32 <= 0x68_6000);
    assert_eq!(CAVE_VA + CAVE_LEN as u32, 0x68_5110);
    let img = common::synthetic();
    let caves = [patch::Cave {
        name: "pacer",
        offset: (CAVE_VA - 0x40_0000) as usize,
        len: CAVE_LEN,
    }];
    // Limit: end of .text raw data (what the SDK uses).
    assert_eq!(
        patch::check_caves(&img, &caves, common::TEXT_RAW + common::TEXT_RAW_SIZE),
        Ok(())
    );
    let _ = common::DATA_VSIZE;
    // A non-zero byte in the cave refuses the set.
    let mut dirty = img.clone();
    dirty[(CAVE_VA - 0x40_0000) as usize + 0x40] = 1;
    assert!(matches!(
        apply_sets(&dirty, &[synthetic_set(&img)]),
        Err(PatchError::Unexpected { .. }
            | PatchError::CaveNotZero { .. }
            | PatchError::UnknownEdition { .. })
    ));
}

#[test]
fn idempotent_and_reversible() {
    let img = common::synthetic();
    let set = synthetic_set(&img);
    let once = apply_sets(&img, &[set]).unwrap();
    assert_ne!(once, img);
    assert_eq!(apply_sets(&once, &[set]).unwrap(), once, "applying twice");
    assert_eq!(unpatch(&once, &[set]).unwrap(), img);
}

#[test]
fn unknown_builds_are_refused_whole() {
    let img = common::synthetic();
    // The real PACER only accepts the clean SSE1 hash.
    assert_eq!(
        apply_sets(&img, &[&PACER]).unwrap_err(),
        PatchError::UnknownEdition {
            sha256: sha256(&img)
        }
    );
    // A different byte at one site: refused, nothing returned.
    let set = synthetic_set(&img);
    let mut other = img.clone();
    other[0x19_5323] = 0x42; // inside the pacer-tick site
    assert!(matches!(
        apply_sets(&other, &[set]),
        Err(PatchError::Unexpected { .. })
    ));
    // Wrong pinned result: refused.
    let wrong = PatchSet {
        results: leak(vec![(sha256(&img), [0; 32])]),
        ..*set
    };
    assert!(matches!(
        apply_sets(&img, &[&wrong]),
        Err(PatchError::Unverified { .. })
    ));
}

#[test]
fn table_shape() {
    assert_eq!(PACER.name, "pacer");
    assert_eq!(PACER.editions, &[SSE1_CLEAN]);
    assert_eq!(PACER.results.len(), 1);
    assert_eq!(PACER.results[0].0, SSE1_CLEAN);
    for bad in [SSE1_COMMUNITY_PACER, NODVD] {
        assert!(!PACER.editions.contains(&bad));
        assert_ne!(
            PACER.results[0].1, bad,
            "new result hash, not the community build's"
        );
    }
    for s in PACER.sites {
        assert_eq!(s.original.len(), s.patched.len());
        assert!(!s.note.is_empty());
    }
    // The aspect tables (other sets) are at file offsets 0x2F7C44 and
    // 0x2FADA8: disjoint from every pacer site and the cave.
    for aspect in [0x2F_7C44usize, 0x2F_ADA8] {
        for s in PACER.sites {
            let off = match s.at {
                At::Va(va) => (va - 0x40_0000) as usize,
                At::Offset(o) => o,
            };
            assert!(
                aspect + 8 <= off || off + s.patched.len() <= aspect,
                "{}",
                s.note
            );
        }
    }
}

#[test]
fn describe_table_for_the_docs() {
    let img = common::synthetic();
    let rows = describe(&PACER, Some(&Pe::parse(&img).unwrap())).unwrap();
    assert_eq!(rows.len(), PACER.sites.len());
    let md = to_markdown(&rows);
    assert!(md.contains("`0x00595660`"));
    println!("{md}");
}
