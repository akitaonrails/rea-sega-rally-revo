//! Layer 2 (env-gated): `SRR_GAME_DIR=<folder with SEGA Rally_SSE1.exe>`.
//! Nothing is written to that folder. With `SRR_WRITE_PATCHED=<path>` the
//! patched image is also written there (for REA inspection; the path must
//! not be inside the game folder).

use std::path::{Path, PathBuf};

use rea_launcher::identify::{hex, sha256};
use rea_launcher::patch::{At, PatchError, PatchSet, PatchSite, apply_sets, testing, unpatch};
use rea_launcher::pe::Pe;
use srr_launcher::pacer::*;

fn game_exe() -> Option<PathBuf> {
    let dir = std::env::var_os("SRR_GAME_DIR")?;
    let p = Path::new(&dir).join("SEGA Rally_SSE1.exe");
    assert!(p.is_file(), "{} not found", p.display());
    Some(p)
}

#[test]
fn real_game_sdk_checks() {
    // apply all / each alone / twice / unpatch, results verified.
    testing::assert_real_game("SRR_GAME_DIR", &[&PACER]);
}

#[test]
fn real_game_result_hash_and_pe() {
    let Some(exe) = game_exe() else {
        eprintln!("SRR_GAME_DIR not set; skipped");
        return;
    };
    let clean = std::fs::read(&exe).unwrap();
    assert_eq!(sha256(&clean), SSE1_CLEAN, "clean SSE1 build");
    // The hash with the table's own result check switched off, so a
    // mismatch prints the value to pin instead of only "Unverified".
    let unpinned = PatchSet {
        results: &[],
        ..PACER
    };
    let out = apply_sets(&clean, &[&unpinned]).unwrap();
    let got = sha256(&out);
    println!("pacer result SHA-256: {}", hex(&got));
    assert_eq!(
        got, SSE1_CLEANROOM_PACER,
        "pin this hash in SSE1_CLEANROOM_PACER"
    );
    assert_eq!(apply_sets(&clean, &[&PACER]).unwrap(), out);
    assert_ne!(got, SSE1_COMMUNITY_PACER);
    assert_eq!(out.len(), clean.len());

    // rea_launcher::pe parses the result; .text grew to cover the cave.
    let pe = Pe::parse(&out).unwrap();
    let text = pe.section(".text").unwrap();
    assert_eq!(
        (text.rva, text.virtual_size, text.raw_size, text.raw_offset),
        (0x1000, 0x28_5000, 0x28_5000, 0x1000)
    );
    assert_eq!(pe.image_base, 0x40_0000);
    assert!(pe.is_executable(pe.va_to_offset(u64::from(CAVE_VA)).unwrap()));
    let before = Pe::parse(&clean).unwrap();
    assert_eq!(pe.sections.len(), before.sections.len());
    for (a, b) in pe.sections.iter().zip(&before.sections).skip(1) {
        if a.name == ".data" {
            assert_eq!((b.virtual_size, a.virtual_size), (0x43_C95C, 0x43_C964));
            assert_eq!(
                (a.rva, a.raw_offset, a.raw_size, a.characteristics),
                (b.rva, b.raw_offset, b.raw_size, b.characteristics)
            );
        } else {
            assert_eq!(a, b, "other sections unchanged");
        }
    }

    // Only the declared bytes changed.
    let changed: Vec<usize> = (0..clean.len()).filter(|&i| clean[i] != out[i]).collect();
    let in_site = |i: usize| {
        PACER.sites.iter().any(|s| {
            let o = match s.at {
                At::Va(va) => pe.va_to_offset(u64::from(va)).unwrap(),
                At::Offset(o) => o,
            };
            (o..o + s.patched.len()).contains(&i)
        })
    };
    assert!(changed.iter().all(|&i| in_site(i)));
    println!("{} bytes changed", changed.len());

    // Idempotent, reversible.
    assert_eq!(apply_sets(&out, &[&PACER]).unwrap(), out);
    assert_eq!(unpatch(&out, &[&PACER]).unwrap(), clean);

    // Combines with a set at the aspect-table offsets (disjoint bytes).
    // Stand-in for ASPECT_*: flips those 4 bytes; only disjointness and the
    // union are under test here.
    let a1: &'static [u8] = Box::leak(clean[0x2F_7C44..0x2F_7C48].to_vec().into_boxed_slice());
    let a2: &'static [u8] = Box::leak(clean[0x2F_ADA8..0x2F_ADAC].to_vec().into_boxed_slice());
    let f = |b: &[u8]| -> &'static [u8] {
        Box::leak(b.iter().map(|x| !x).collect::<Vec<_>>().into_boxed_slice())
    };
    let sites: &'static [PatchSite] = Box::leak(
        vec![
            PatchSite {
                at: At::Offset(0x2F_7C44),
                original: a1,
                patched: f(a1),
                note: "aspect stand-in 1",
            },
            PatchSite {
                at: At::Offset(0x2F_ADA8),
                original: a2,
                patched: f(a2),
                note: "aspect stand-in 2",
            },
        ]
        .into_boxed_slice(),
    );
    let aspect = PatchSet {
        name: "aspect-standin",
        summary: "",
        editions: &[SSE1_CLEAN],
        sites,
        caves: &[],
        results: &[],
    };
    let both = apply_sets(&clean, &[&PACER, &aspect]).unwrap();
    let only_aspect = apply_sets(&clean, &[&aspect]).unwrap();
    for i in 0..clean.len() {
        let want = if out[i] != clean[i] {
            out[i]
        } else {
            only_aspect[i]
        };
        assert_eq!(both[i], want);
    }

    if let Some(path) = std::env::var_os("SRR_WRITE_PATCHED") {
        let path = PathBuf::from(path);
        assert!(
            !path.starts_with(exe.parent().unwrap()),
            "never write into the game folder"
        );
        std::fs::write(&path, &out).unwrap();
        println!("wrote {}", path.display());
    }
}

#[test]
fn real_game_refuses_other_builds() {
    let Some(exe) = game_exe() else { return };
    let mut other = std::fs::read(exe).unwrap();
    // A byte outside every site: an unknown build, refused whole.
    other[0x10_0000] ^= 0xFF;
    assert!(matches!(
        apply_sets(&other, &[&PACER]),
        Err(PatchError::UnknownEdition { .. })
    ));
}
