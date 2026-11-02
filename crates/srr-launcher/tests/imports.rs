//! Guards against a linker fault that produced a launcher which crashed on
//! its first Win32 call: GNU ld left some import address table slots outside
//! every import descriptor, so the loader never resolved them.
//!
//! `SRR_LAUNCHER_EXE=target/i686-pc-windows-gnu/release/SRRLauncher.exe
//! cargo test --test imports -- --ignored`

use std::collections::BTreeSet;

fn u16_at(bytes: &[u8], at: usize) -> usize {
    u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize
}

fn u32_at(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}

/// Names of import-table slots that no import descriptor covers.
fn orphaned_imports(exe: &[u8]) -> Vec<String> {
    let pe = u32_at(exe, 0x3c);
    let optional = pe + 24;
    assert_eq!(
        u16_at(exe, optional),
        0x10b,
        "expected a PE32 (32-bit) image"
    );
    let sections: Vec<(usize, usize, usize)> = (0..u16_at(exe, pe + 6))
        .map(|i| optional + u16_at(exe, pe + 20) + 40 * i)
        .map(|s| {
            let size = u32_at(exe, s + 8).max(u32_at(exe, s + 16));
            (u32_at(exe, s + 12), size, u32_at(exe, s + 20))
        })
        .collect();
    let offset = |rva: usize| {
        sections
            .iter()
            .find(|(va, size, _)| (*va..va + size).contains(&rva))
            .map(|(va, _, raw)| rva - va + raw)
            .expect("RVA inside a section")
    };
    let directories = optional + 96;
    let import_rva = u32_at(exe, directories + 8);
    let (iat_rva, iat_size) = (
        u32_at(exe, directories + 12 * 8),
        u32_at(exe, directories + 12 * 8 + 4),
    );

    let mut covered = BTreeSet::new();
    let mut descriptor = offset(import_rva);
    while (0..20).step_by(4).any(|i| u32_at(exe, descriptor + i) != 0) {
        let mut slot = u32_at(exe, descriptor + 16);
        loop {
            covered.insert(slot);
            if u32_at(exe, offset(slot)) == 0 {
                break;
            }
            slot += 4;
        }
        descriptor += 20;
    }

    (iat_rva..iat_rva + iat_size)
        .step_by(4)
        .filter(|slot| !covered.contains(slot))
        .map(|slot| u32_at(exe, offset(slot)))
        .filter(|&hint_name| hint_name != 0 && hint_name & 0x8000_0000 == 0)
        .map(|hint_name| {
            let name = &exe[offset(hint_name) + 2..];
            String::from_utf8_lossy(&name[..name.iter().position(|&b| b == 0).unwrap()])
                .into_owned()
        })
        .collect()
}

#[test]
#[ignore = "needs SRR_LAUNCHER_EXE pointing at a built SRRLauncher.exe"]
fn every_import_is_resolvable() {
    let path = std::env::var("SRR_LAUNCHER_EXE").expect("set SRR_LAUNCHER_EXE");
    let exe = std::fs::read(&path).expect("read SRR_LAUNCHER_EXE");
    let orphans = orphaned_imports(&exe);
    assert!(
        orphans.is_empty(),
        "{path}: unresolvable imports {orphans:?}"
    );
}
