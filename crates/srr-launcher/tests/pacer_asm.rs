//! The byte tables are exactly what NASM makes of pacer/caves.asm and
//! pacer/sites.asm. Skipped (with a note) when nasm is not on PATH.

use std::path::PathBuf;
use std::process::Command;

use srr_launcher::pacer::*;

fn nasm(args: &[&str], out: &PathBuf) -> Option<Vec<u8>> {
    let status = Command::new("nasm")
        .args(args)
        .arg("-o")
        .arg(out)
        .status()
        .ok()?;
    assert!(status.success(), "nasm {args:?} failed");
    Some(std::fs::read(out).unwrap())
}

#[test]
fn tables_match_the_assembler_output() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("asm");
    std::fs::create_dir_all(&dir).unwrap();
    let caves = root.join("pacer/caves.asm");
    let Some(cave) = nasm(
        &["-f", "bin", caves.to_str().unwrap()],
        &dir.join("caves.bin"),
    ) else {
        eprintln!("warning: nasm not found; assembler check skipped");
        return;
    };
    assert_eq!(cave, CAVE);
    let sites = root.join("pacer/sites.asm");
    let expected: [&[u8]; 8] = [
        &SPAWN_NEW,
        &PACER_START_NEW,
        &PACER_TICK_NEW,
        &PACER_SNAP_NEW,
        &KF_INIT_NEW,
        &RACE_INIT_NEW,
        &KF_UPDATE_NEW,
        &FRAME_END_NEW,
    ];
    for (i, want) in expected.iter().enumerate() {
        let n = i + 1;
        let def = format!("-DSITE={n}");
        let got = nasm(
            &["-f", "bin", &def, sites.to_str().unwrap()],
            &dir.join(format!("site{n}.bin")),
        )
        .unwrap();
        assert_eq!(&got, want, "site {n}");
    }
}
