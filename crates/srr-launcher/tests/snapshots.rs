//! Golden snapshots of everything the launcher produces: `--help`, the
//! first-run `launcher.cfg`, and for each look and a few option
//! combinations the plan of a launch (executable, patch sets with their
//! sites, `config.ini`, registry values, `dxvk.conf`, `WINEDLLOVERRIDES`,
//! the command line). A diff after an SDK upgrade or a refactor must be
//! explained by an intended change or an SDK Migration item; never
//! regenerate blindly. Regenerate, then review every changed file:
//! `SRR_UPDATE_SNAPSHOTS=1 cargo test --test snapshots`.
//!
//! Only launcher-made bytes appear here (the patch tables are ours);
//! nothing comes from the game's files.

use std::fmt::Write;
use std::path::PathBuf;

use rea_launcher::ini::Owned;
use rea_launcher::patch;
use srr_launcher::config::{self, Settings};
use srr_launcher::edition::{GameEditions, MAIN_NODVD_BUILD, SSE1_CLEAN_BUILD};
use srr_launcher::form::{Form, Look};
use srr_launcher::gamecfg;
use srr_launcher::plan::{self, Env, Plan};

const RIP: GameEditions = GameEditions {
    main: Some(Ok(&MAIN_NODVD_BUILD)),
    sse1: Some(Ok(&SSE1_CLEAN_BUILD)),
};

const RUNTIME: &str = r"C:\users\steamuser\AppData\Local\SRRLauncher\SegaRally";

/// A config.ini the game saved earlier (CRLF, the player's pad, a
/// changed vsync and a custom key).
const PLAYER_CONFIG: &str = "[Audio]\r\nDriverGUID=00000000-0000-0000-0000-000000000000\r\nQuality=2\r\nSoftwareEmulation=0\r\n\r\n[GameController]\r\nGUID={6F1D2B61-D5A0-11CF-BFC7-444553540000}\r\nMapping=0001000100000001\r\n\r\n[Graphics]\r\nHorizontalResolution=1920\r\nVerticalResolution=1080\r\nGraphicQuality=2\r\nVSync=0\r\nRunWindowed=0\r\n";

fn snapshot(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));
    if std::env::var_os("SRR_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("no snapshot {}; see the module docs", path.display()));
    assert!(
        expected == actual,
        "snapshot {name} changed; review the diff:\n--- expected\n{expected}\n--- actual\n{actual}"
    );
}

/// What the settings window saves for `look` on `screen`.
fn window_saves(look: Look, screen: Option<(u32, u32)>) -> Settings {
    let mut form = Form::first_run(screen);
    form.game = r"D:\Games\Sega Rally".into();
    form.set_look(look);
    let text = form.save(&config::template(), screen);
    let (settings, warnings) = Settings::parse_file(&text);
    assert!(warnings.is_empty(), "{warnings:?}");
    settings
}

fn file(text: &str) -> Settings {
    let (settings, warnings) = Settings::parse_file(text);
    assert!(warnings.is_empty(), "{warnings:?}");
    settings
}

fn render(
    title: &str,
    settings: &Settings,
    editions: &GameEditions,
    env: &Env,
    existing: Option<&str>,
) -> String {
    let plan: Plan = plan::plan(settings, editions, env);
    let mut out = String::new();
    let w = &mut out;
    writeln!(w, "# {title}").unwrap();
    writeln!(w, "\n## settings\n{settings:#?}").unwrap();
    writeln!(w, "\n## environment\n{env:#?}").unwrap();
    writeln!(w, "\n## launch\nexecutable: {}", plan.launch).unwrap();
    writeln!(w, "command: {:?}", plan.command).unwrap();
    writeln!(w, "display: {:?}", plan.display).unwrap();
    writeln!(
        w,
        "resolution: {}x{} (aspect {})",
        plan.resolution.0,
        plan.resolution.1,
        plan.aspect.key()
    )
    .unwrap();
    writeln!(w, "WINEDLLOVERRIDES: {:?}", plan.dll_overrides).unwrap();
    writeln!(
        w,
        "app DLL overrides (AppDefaults\\<exe>\\DllOverrides = native,builtin): [{}]",
        plan.app_dlls.join(", ")
    )
    .unwrap();
    writeln!(
        w,
        "app DLL overrides taken back: [{}]",
        plan.app_dlls_off.join(", ")
    )
    .unwrap();
    writeln!(w, "install DXVK d3d9.dll: {}", plan.install_dxvk).unwrap();
    writeln!(w, "Windows GPU preference: {}", plan.gpu_preference).unwrap();
    writeln!(w, "notes: {:#?}", plan.notes).unwrap();
    writeln!(w, "\n## patches").unwrap();
    for (exe, sets) in &plan.patches {
        writeln!(
            w,
            "{exe}: {}",
            if sets.is_empty() {
                "pristine".to_string()
            } else {
                sets.iter().map(|s| s.name).collect::<Vec<_>>().join(" + ")
            }
        )
        .unwrap();
        for set in sets {
            for site in set.sites {
                // No PE here: offset sites print their file offset, VA
                // sites their address (the pacer's).
                let at = match site.at {
                    patch::At::Offset(offset) => format!("{offset:#08X}"),
                    patch::At::Va(va) => format!("VA {va:#08X}"),
                };
                writeln!(
                    w,
                    // The patched bytes only: the originals are the game's.
                    "  {at}: {}  ({})",
                    hex(site.patched),
                    site.note
                )
                .unwrap();
            }
        }
    }
    writeln!(
        w,
        "\n## registry (HKLM\\SOFTWARE\\Sega\\SEGA Rally, 32-bit view)"
    )
    .unwrap();
    for value in &plan.registry {
        writeln!(
            w,
            "{} = {:?}{}",
            value.name,
            value.value,
            if value.if_missing {
                " (only when missing)"
            } else {
                ""
            }
        )
        .unwrap();
    }
    writeln!(
        w,
        "\n## dxvk.conf\n{}",
        plan.dxvk_conf.as_deref().unwrap_or("(not written)\n")
    )
    .unwrap();
    let read = match existing {
        Some(text) => Ok(text.as_bytes().to_vec()),
        None => Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
    };
    let mut owned = Owned::default();
    let config = gamecfg::next_config(read, plan.seed, &plan.graphics, &mut owned).unwrap();
    writeln!(
        w,
        "## config.ini ({})\n{}",
        if existing.is_some() {
            "updated over the player's file"
        } else {
            "seeded"
        },
        String::from_utf8(config).unwrap().replace("\r\n", "\\r\n")
    )
    .unwrap();
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn linux(screen: (u32, u32)) -> Env {
    Env {
        wine: true,
        linux: true,
        screen: Some(screen),
        runtime: RUNTIME.into(),
        dll_overrides: Some("mscoree,mshtml=".into()),
        ..Env::default()
    }
}

#[test]
fn help() {
    snapshot(
        "help",
        &config::usage().replace(env!("CARGO_PKG_VERSION"), "<version>"),
    );
}

#[test]
fn template() {
    snapshot("launcher-cfg-template", &config::template());
}

#[test]
fn look_modern_recommended_linux_4k() {
    let screen = (3840, 2160);
    let settings = window_saves(Look::Modern, Some(screen));
    snapshot(
        "look-modern",
        &render(
            "Modern (recommended), AppImage on a 4K desktop",
            &settings,
            &RIP,
            &linux(screen),
            None,
        ),
    );
}

#[test]
fn look_steam_deck_in_gamescope() {
    let screen = (1280, 800);
    let settings = window_saves(Look::SteamDeck, Some(screen));
    let env = Env {
        gamescope: true,
        ..linux(screen)
    };
    snapshot(
        "look-steam-deck",
        &render(
            "Steam Deck (recommended there), AppImage in Game Mode",
            &settings,
            &RIP,
            &env,
            None,
        ),
    );
}

#[test]
fn look_original_native_windows() {
    let screen = (2560, 1440);
    let settings = window_saves(Look::Original, Some(screen));
    let env = Env {
        screen: Some(screen),
        mode_supported: true,
        runtime: r"C:\Users\José\AppData\Local\SRRLauncher\SegaRally".into(),
        ..Env::default()
    };
    snapshot(
        "look-original",
        &render(
            "Original 2007, native Windows, over the player's config.ini",
            &settings,
            &RIP,
            &env,
            Some(PLAYER_CONFIG),
        ),
    );
}

#[test]
fn combo_ultrawide_120fps_no_stutter_fix_french_directx() {
    let settings = file(
        "game = D:\\Jogos d'Ação\\Sega Rally\nresolution = 3440x1440\nquality = ultra\n\
         vsync = off\nframe_limit = 120\nstutter_fix = off\nlanguage = French\n",
    );
    let env = Env {
        directx: vec!["D3DCompiler_43.dll".into(), "d3dx9_35.dll".into()],
        ..linux((3440, 1440))
    };
    snapshot(
        "combo-ultrawide",
        &render(
            "21:9, Ultra, vsync off, 120 fps, stutter fix off, French, DirectX files",
            &settings,
            &RIP,
            &env,
            None,
        ),
    );
}

#[test]
fn combo_nothing_set_under_proton_keeps_the_players_config() {
    let env = Env {
        wine: true,
        screen: Some((2560, 1440)),
        config_resolution: gamecfg::resolution_in(PLAYER_CONFIG.as_bytes()),
        runtime: RUNTIME.into(),
        ..Env::default()
    };
    snapshot(
        "combo-unset-proton",
        &render(
            "Nothing set (command line only), Steam's Proton, existing config.ini",
            &Settings::default(),
            &RIP,
            &env,
            Some(PLAYER_CONFIG),
        ),
    );
}

#[test]
fn combo_nodvd_only_with_a_command_line_look() {
    let nodvd = GameEditions {
        main: Some(Ok(&MAIN_NODVD_BUILD)),
        sse1: None,
    };
    let cli =
        Settings::parse_args(["--look", "original", "--resolution", "5120x1440"].map(String::from))
            .unwrap();
    let settings = file("stutter_fix = on\n").merged(cli.settings);
    snapshot(
        "combo-nodvd-32-9",
        &render(
            "NoDVD exe only, --look original --resolution 5120x1440",
            &settings,
            &nodvd,
            &linux((5120, 1440)),
            None,
        ),
    );
}
