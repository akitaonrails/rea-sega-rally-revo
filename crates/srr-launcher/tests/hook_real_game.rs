//! Env-gated (`SRR_GAME_DIR=<the game folder>`; never committed): the
//! close hook import goes into the runtime copies of the owner's real
//! executables after every patch set a launch can apply, the result still
//! parses, imports the DLL once, and a second pass leaves it unchanged.

use std::path::PathBuf;

use rea_launcher::pe::{self, Pe};
use srr_launcher::config::Settings;
use srr_launcher::edition::{GameEditions, MAIN_EXE, SSE1_EXE};
use srr_launcher::plan::{self, Env};
use srr_launcher::{patches, registry};

fn game_dir() -> Option<PathBuf> {
    std::env::var_os("SRR_GAME_DIR").map(PathBuf::from)
}

#[test]
fn real_game_exes_take_the_hook_import_after_every_patch_set() {
    let Some(dir) = game_dir() else {
        eprintln!("SRR_GAME_DIR not set; skipped");
        return;
    };
    let main = std::fs::read(dir.join(MAIN_EXE)).ok();
    let sse1 = std::fs::read(dir.join(SSE1_EXE)).ok();
    let editions = GameEditions::from_bytes(main.as_deref(), sse1.as_deref());
    assert!(editions.playable(), "{}", editions.refusal());
    let env = Env {
        wine: true,
        linux: true,
        screen: Some((2560, 1080)),
        runtime: r"C:\x".into(),
        ..Env::default()
    };
    let choices: [&[&str]; 3] = [
        &[],
        &["--stutter-fix", "off"],
        &[
            "--resolution",
            "2560x1080",
            "--display-mode",
            "borderless",
            "--keys",
            "accelerate=up",
        ],
    ];
    let mut checked = 0;
    for args in choices {
        let cli = Settings::parse_args(args.iter().map(|a| a.to_string())).unwrap();
        let plan = plan::plan(&Settings::default().merged(cli.settings), &editions, &env);
        for (name, sets) in &plan.patches {
            let pristine = if *name == MAIN_EXE { &main } else { &sse1 };
            let patched =
                patches::apply(pristine.as_deref().unwrap(), sets, plan.key_table.as_ref())
                    .unwrap();
            let hooked = registry::with_hook(&patched).unwrap();
            assert_eq!(registry::with_hook(&hooked).unwrap(), hooked, "{name}");
            let imports = pe::edit::imports(&hooked).unwrap();
            assert_eq!(
                imports
                    .iter()
                    .filter(|i| i.dll.eq_ignore_ascii_case(registry::HOOK_DLL))
                    .count(),
                1,
                "{name}"
            );
            // The game's own imports are all still there.
            for import in pe::edit::imports(&patched).unwrap() {
                assert!(imports.contains(&import), "{name}: {import:?}");
            }
            // Inside the game's sections only the import descriptors' stale
            // time stamps and forwarder chains change (`clear_stale_binding`).
            let (before, after) = (Pe::parse(&patched).unwrap(), Pe::parse(&hooked).unwrap());
            let descriptors = import_descriptors(&patched);
            for section in &before.sections {
                for at in section.raw_range() {
                    if patched[at] != hooked[at] {
                        let field = (at - descriptors.start) % 20;
                        assert!(
                            descriptors.contains(&at) && (4..12).contains(&field),
                            "{name}: byte {at:#x} in {} changed",
                            section.name
                        );
                    }
                }
            }
            assert_eq!(after.image_base, before.image_base);
            checked += 1;
        }
    }
    assert!(checked >= 3, "{checked}");
}

/// The file range of `bytes`' import descriptors (PE32).
fn import_descriptors(bytes: &[u8]) -> std::ops::Range<usize> {
    let pe = Pe::parse(bytes).unwrap();
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let directories = u32_at(0x3C) as usize + 24 + 96;
    let start = pe
        .va_to_offset(pe.image_base + u64::from(u32_at(directories + 8)))
        .unwrap();
    let count = (start..)
        .step_by(20)
        .take_while(|at| bytes[*at..*at + 20].iter().any(|&b| b != 0))
        .count();
    start..start + count * 20
}
