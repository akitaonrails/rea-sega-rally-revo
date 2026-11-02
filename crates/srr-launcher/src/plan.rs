//! Everything one launch will do, worked out without touching the disk,
//! the registry or Wine: which executable runs, which patch sets go into
//! which executable, the `config.ini` keys, the registry values, the DXVK
//! file, `WINEDLLOVERRIDES` and the command line. `main.rs` carries the
//! plan out; the tests (and the golden snapshots in `tests/snapshots/`)
//! check it option by option.

use rea_launcher::patch::PatchSet;

use crate::config::{Aspect, Quality, Settings};
use crate::display;
use crate::edition::{GameEditions, MAIN_EXE, SSE1_EXE};
use crate::gamecfg::{self, Graphics, RegValue};
use crate::patches;

/// What the launch depends on besides the settings and the game folder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Env {
    /// Running under Wine or Proton.
    pub wine: bool,
    /// `--linux`: srr-play or the AppImage, which bring DXVK.
    pub linux: bool,
    /// Inside gamescope.
    pub gamescope: bool,
    /// The monitor's size in pixels.
    pub screen: Option<(u32, u32)>,
    /// Native Windows has a display mode of the game's size.
    pub mode_supported: bool,
    /// The resolution an existing `config.ini` holds.
    pub config_resolution: Option<(u32, u32)>,
    /// `WINEDLLOVERRIDES` as the launcher inherited it.
    pub dll_overrides: Option<String>,
    /// The DirectX 9 DLLs the player downloaded (file names).
    pub directx: Vec<String>,
    /// The private copy as Windows spells it.
    pub runtime: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The executable that runs.
    pub launch: &'static str,
    /// For each known executable of the folder, the patch sets its private
    /// copy gets (none: the pristine bytes).
    pub patches: Vec<(&'static str, Vec<&'static PatchSet>)>,
    /// The size the game runs at.
    pub resolution: (u32, u32),
    pub aspect: Aspect,
    /// The values a new `config.ini` is seeded with.
    pub seed: ((u32, u32), Quality, bool),
    /// The `config.ini` options set in `launcher.cfg`.
    pub graphics: Graphics,
    pub registry: Vec<RegValue>,
    /// `dxvk.conf` in the private copy (any DXVK reads it: ours or Proton's).
    pub dxvk_conf: Option<String>,
    /// Put the bundled DXVK `d3d9.dll` into the private copy.
    pub install_dxvk: bool,
    /// `WINEDLLOVERRIDES` for the game, when it changes (kept, but Wine
    /// ignores it in a child's environment block: see `app_dlls`).
    pub dll_overrides: Option<String>,
    /// Under Wine, the DLLs that get `native,builtin` in
    /// `HKCU\Software\Wine\AppDefaults\<launch>\DllOverrides` (SDK
    /// `app_dll_overrides`; values the player set are kept).
    pub app_dlls: Vec<String>,
    /// Under Wine, our earlier per-exe overrides to take back (components
    /// off or files gone; SDK `remove_app_dll_overrides`).
    pub app_dlls_off: Vec<String>,
    pub display: display::Plan,
    /// The program and its arguments.
    pub command: Vec<String>,
    /// Ask Windows for the fast GPU (native Windows only).
    pub gpu_preference: bool,
    /// Decisions worth a line in `launcher.log`.
    pub notes: Vec<String>,
}

/// The plan for `settings` (file and command line merged) on a game folder
/// whose executables are `editions` (at least one known: see
/// [`GameEditions::playable`]).
pub fn plan(settings: &Settings, editions: &GameEditions, env: &Env) -> Plan {
    let mut notes = Vec::new();
    let main = editions.main_known();
    let sse1 = editions.sse1_known();

    // The stutter fix: the clean SSE1 build gets it; an SSE1 build that
    // already has it runs as it is.
    let wanted = settings.effective_stutter_fix();
    let pacer = wanted && sse1.is_some_and(|e| e.pacer_fixable);
    if wanted && !pacer {
        notes.push(format!(
            "stutter fix: needs the clean {SSE1_EXE}, which is not in the game folder; \
             running unpatched"
        ));
    }
    let launch = if pacer || main.is_none() {
        SSE1_EXE
    } else {
        MAIN_EXE
    };

    // The picture: an option set in launcher.cfg is written; unset keeps
    // the game's own value (the seed only for a new config.ini).
    let effective = settings.effective_resolution(env.screen);
    let resolution = match settings.resolution {
        Some(_) => effective,
        None => env.config_resolution.unwrap_or(effective),
    };
    let graphics = Graphics {
        resolution: settings.resolution.map(|_| effective),
        quality: settings.quality,
        vsync: settings.vsync,
    };
    let seed = (
        effective,
        settings.effective_quality(),
        settings.effective_vsync(),
    );
    let aspect = Aspect::for_resolution(resolution);
    if aspect != Aspect::Native {
        notes.push(format!(
            "aspect: {}x{} is {}; aspect fix in the private copy",
            resolution.0,
            resolution.1,
            aspect.key()
        ));
    }

    let mut patches = Vec::new();
    for (name, edition) in [(MAIN_EXE, main), (SSE1_EXE, sse1)] {
        let Some(edition) = edition else { continue };
        let mut sets: Vec<&'static PatchSet> = Vec::new();
        if name == SSE1_EXE && pacer && !edition.pacer_applied {
            sets.push(&patches::PACER);
        }
        if let Some(set) = patches::aspect_set(edition.sha256, aspect) {
            sets.push(set);
        }
        patches.push((name, sets));
    }

    let registry = gamecfg::registry_values(&env.runtime, settings.language.as_deref());
    let dxvk = env.linux || env.wine;
    let dxvk_conf = dxvk.then(|| gamecfg::dxvk_conf(settings.effective_frame_limit().fps()));
    let native = crate::directx::override_names(&env.directx);
    let (app_dlls, app_dlls_off) = if env.wine {
        let mut on: Vec<String> = Vec::new();
        if env.linux {
            on.push("d3d9".into());
        }
        on.extend(native.iter().cloned());
        let off = std::iter::once("d3d9".to_string())
            .chain(crate::directx::all_override_names())
            .filter(|name| !on.contains(name))
            .collect();
        (on, off)
    } else {
        (Vec::new(), Vec::new())
    };
    let dll_overrides = if dxvk {
        gamecfg::wine_overrides(env.dll_overrides.as_deref(), env.linux, &native)
    } else {
        None
    };

    let portrait = env.screen.is_some_and(|(width, height)| width < height);
    let display = display::plan(
        env.wine,
        env.gamescope,
        portrait,
        env.wine || env.mode_supported,
    );
    let exe = format!("{}\\{launch}", env.runtime.trim_end_matches('\\'));
    let command = match display {
        display::Plan::WineDesktop => vec![
            "explorer".to_string(),
            format!("/desktop=SegaRally,{}x{}", resolution.0, resolution.1),
            exe,
        ],
        _ => vec![exe],
    };
    Plan {
        launch,
        patches,
        resolution,
        aspect,
        seed,
        graphics,
        registry,
        dxvk_conf,
        install_dxvk: env.linux,
        dll_overrides,
        app_dlls,
        app_dlls_off,
        display,
        command,
        gpu_preference: !env.wine,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{FrameLimit, Look};
    use crate::edition::{MAIN_NODVD_BUILD, SSE1_CLEAN_BUILD, SSE1_PACER_BUILD};

    const RIP: GameEditions = GameEditions {
        main: Some(Ok(&MAIN_NODVD_BUILD)),
        sse1: Some(Ok(&SSE1_CLEAN_BUILD)),
    };

    fn linux() -> Env {
        Env {
            wine: true,
            linux: true,
            screen: Some((3840, 2160)),
            runtime: r"C:\users\me\AppData\Local\SRRLauncher\SegaRally".into(),
            ..Env::default()
        }
    }

    fn windows() -> Env {
        Env {
            screen: Some((2560, 1440)),
            mode_supported: true,
            runtime: r"C:\Users\José\AppData\Local\SRRLauncher\SegaRally".into(),
            ..Env::default()
        }
    }

    fn file(text: &str) -> Settings {
        let (settings, warnings) = Settings::parse_file(text);
        assert!(warnings.is_empty(), "{warnings:?}");
        settings
    }

    fn sets(plan: &Plan, exe: &str) -> Vec<&'static str> {
        plan.patches
            .iter()
            .find(|(name, _)| *name == exe)
            .map(|(_, sets)| sets.iter().map(|set| set.name).collect())
            .unwrap_or_default()
    }

    #[test]
    fn stutter_fix_on_runs_the_patched_sse1_build() {
        let plan = plan(&file("stutter_fix = on\n"), &RIP, &linux());
        assert_eq!(plan.launch, SSE1_EXE);
        assert_eq!(sets(&plan, SSE1_EXE), ["pacer"]);
        assert_eq!(sets(&plan, MAIN_EXE), Vec::<&str>::new());
        assert!(plan.notes.is_empty());
    }

    #[test]
    fn stutter_fix_off_runs_the_main_build_unpatched() {
        let plan = plan(&file("stutter_fix = off\n"), &RIP, &linux());
        assert_eq!(plan.launch, MAIN_EXE);
        assert!(plan.patches.iter().all(|(_, sets)| sets.is_empty()));
    }

    #[test]
    fn stutter_fix_without_the_clean_build_says_so() {
        let nodvd = GameEditions {
            main: Some(Ok(&MAIN_NODVD_BUILD)),
            sse1: None,
        };
        let plan = plan(&Settings::default(), &nodvd, &linux());
        assert_eq!(plan.launch, MAIN_EXE);
        assert!(plan.notes[0].contains("running unpatched"));
        // An SSE1 build that already has the fix runs as it is.
        let fixed = GameEditions {
            main: None,
            sse1: Some(Ok(&SSE1_PACER_BUILD)),
        };
        let plan = super::plan(&Settings::default(), &fixed, &linux());
        assert_eq!(plan.launch, SSE1_EXE);
        assert_eq!(sets(&plan, SSE1_EXE), Vec::<&str>::new());
        // An unknown SSE1 next to a known main build is never patched.
        let odd = GameEditions {
            main: Some(Ok(&MAIN_NODVD_BUILD)),
            sse1: Some(Err([7; 32])),
        };
        let plan = super::plan(&Settings::default(), &odd, &linux());
        assert_eq!(plan.patches.len(), 1);
        assert_eq!(plan.launch, MAIN_EXE);
    }

    #[test]
    fn resolution_reaches_config_ini_the_desktop_and_the_aspect_fix() {
        let plan = plan(&file("resolution = 3440x1440\n"), &RIP, &linux());
        assert_eq!(plan.graphics.resolution, Some((3440, 1440)));
        assert_eq!(plan.aspect, Aspect::Ultra21);
        assert_eq!(sets(&plan, SSE1_EXE), ["pacer", "aspect 21:9"]);
        assert_eq!(sets(&plan, MAIN_EXE), ["aspect 21:9"]);
        assert_eq!(plan.command[1], "/desktop=SegaRally,3440x1440");
        let plan = super::plan(&file("resolution = 5120x1440\n"), &RIP, &linux());
        assert_eq!(sets(&plan, MAIN_EXE), ["aspect 32:9"]);
        // auto: the screen's size, written.
        let plan = super::plan(&file("resolution = auto\n"), &RIP, &linux());
        assert_eq!(plan.graphics.resolution, Some((3840, 2160)));
        assert_eq!(plan.aspect, Aspect::Native);
        // unset: config.ini's own value stays and is what runs.
        let env = Env {
            config_resolution: Some((2560, 1080)),
            ..linux()
        };
        let plan = super::plan(&Settings::default(), &RIP, &env);
        assert_eq!(plan.graphics.resolution, None);
        assert_eq!(plan.resolution, (2560, 1080));
        assert_eq!(plan.aspect, Aspect::Ultra21);
        assert_eq!(plan.seed.0, (3840, 2160));
    }

    #[test]
    fn quality_and_vsync_reach_config_ini_only_when_set() {
        let plan = plan(&file("quality = ultra\nvsync = off\n"), &RIP, &linux());
        assert_eq!(plan.graphics.quality, Some(Quality::Ultra));
        assert_eq!(plan.graphics.vsync, Some(false));
        assert_eq!(plan.seed.1, Quality::Ultra);
        let plan = super::plan(&Settings::default(), &RIP, &linux());
        assert_eq!(plan.graphics, Graphics::default());
        assert_eq!(plan.seed, ((3840, 2160), Quality::High, true));
    }

    #[test]
    fn frame_limit_reaches_dxvk_conf_under_dxvk_only() {
        let plan = plan(&file("frame_limit = 120\n"), &RIP, &linux());
        assert_eq!(plan.dxvk_conf.as_deref(), Some("d3d9.maxFrameRate = 120\n"));
        let plan = super::plan(&file("frame_limit = off\n"), &RIP, &linux());
        assert!(
            plan.dxvk_conf
                .as_deref()
                .unwrap()
                .starts_with("# no frame limit")
        );
        let plan = super::plan(&file("frame_limit = 120\n"), &RIP, &windows());
        assert_eq!(plan.dxvk_conf, None);
        assert_eq!(
            Settings::default().effective_frame_limit(),
            FrameLimit::Fps60
        );
    }

    #[test]
    fn language_reaches_the_registry() {
        let plan = plan(&file("language = polish\n"), &RIP, &windows());
        let language = plan.registry.iter().find(|v| v.name == "language").unwrap();
        assert_eq!(
            (language.value.as_str(), language.if_missing),
            ("Polish", false)
        );
        let plan = super::plan(&Settings::default(), &RIP, &windows());
        let language = plan.registry.iter().find(|v| v.name == "language").unwrap();
        assert_eq!(
            (language.value.as_str(), language.if_missing),
            ("English", true)
        );
        assert_eq!(
            plan.registry[0].value,
            r"C:\Users\José\AppData\Local\SRRLauncher\SegaRally\"
        );
    }

    #[test]
    fn a_look_from_the_command_line_reaches_the_game() {
        let cli = Settings::parse_args(["--look", "original"].map(String::from)).unwrap();
        let settings = file("quality = ultra\nresolution = 3840x2160\n").merged(cli.settings);
        let plan = plan(&settings, &RIP, &linux());
        assert_eq!(plan.graphics.resolution, Some((1280, 720)));
        assert_eq!(plan.graphics.quality, Some(Quality::Normal));
        assert_eq!(plan.graphics.vsync, Some(true));
        assert_eq!(settings.look, Some(Look::Original));
    }

    #[test]
    fn wine_overrides_carry_dxvk_and_the_directx_files() {
        let env = Env {
            dll_overrides: Some("mscoree,mshtml=".into()),
            directx: vec!["d3dx9_35.dll".into()],
            ..linux()
        };
        let plan = plan(&Settings::default(), &RIP, &env);
        assert_eq!(
            plan.dll_overrides.as_deref(),
            Some("d3d9,d3dx9_35=n,b;mscoree,mshtml=")
        );
        assert!(plan.install_dxvk);
        // Proton running the launcher directly: no DXVK of ours.
        let proton = Env {
            linux: false,
            ..env.clone()
        };
        let plan = super::plan(&Settings::default(), &RIP, &proton);
        assert_eq!(
            plan.dll_overrides.as_deref(),
            Some("d3dx9_35=n,b;mscoree,mshtml=")
        );
        assert!(!plan.install_dxvk);
        // Native Windows: no Wine environment at all.
        let plan = super::plan(&Settings::default(), &RIP, &windows());
        assert_eq!(plan.dll_overrides, None);
        assert!(plan.gpu_preference);
    }

    #[test]
    fn wine_gets_per_exe_dll_overrides_for_what_we_bundle() {
        // SDK v0.3.3: Wine ignores WINEDLLOVERRIDES in the game's
        // environment block, so the bundled DLLs go into the registry
        // (AppDefaults\<exe>\DllOverrides, native,builtin).
        let env = Env {
            directx: vec!["d3dx9_35.dll".into(), "D3DCompiler_43.dll".into()],
            ..linux()
        };
        let plan = plan(&Settings::default(), &RIP, &env);
        assert_eq!(plan.app_dlls, ["d3d9", "d3dx9_35", "d3dcompiler_43"]);
        // Everything else we could have written earlier is taken back
        // (only values still equal to ours are removed).
        assert!(plan.app_dlls_off.contains(&"d3dx9_24".to_string()));
        assert!(plan.app_dlls_off.contains(&"d3dcompiler_42".to_string()));
        assert!(!plan.app_dlls_off.contains(&"d3dx9_35".to_string()));
        assert!(!plan.app_dlls_off.contains(&"d3d9".to_string()));
        // Proton without --linux: no DXVK of ours, so d3d9 is taken back.
        let proton = Env {
            linux: false,
            directx: vec![],
            ..env
        };
        let plan = super::plan(&Settings::default(), &RIP, &proton);
        assert!(plan.app_dlls.is_empty());
        assert!(plan.app_dlls_off.contains(&"d3d9".to_string()));
        assert!(plan.app_dlls_off.contains(&"d3dx9_35".to_string()));
        // Native Windows: no registry overrides at all.
        let plan = super::plan(&Settings::default(), &RIP, &windows());
        assert!(plan.app_dlls.is_empty() && plan.app_dlls_off.is_empty());
    }

    #[test]
    fn the_display_follows_the_host() {
        let plan = plan(&Settings::default(), &RIP, &linux());
        assert_eq!(plan.display, display::Plan::WineDesktop);
        assert_eq!(plan.command[0], "explorer");
        let gamescope = Env {
            gamescope: true,
            ..linux()
        };
        let plan = super::plan(&Settings::default(), &RIP, &gamescope);
        assert_eq!(plan.display, display::Plan::Fullscreen);
        assert_eq!(plan.command.len(), 1);
        let unsupported = Env {
            mode_supported: false,
            ..windows()
        };
        let plan = super::plan(&Settings::default(), &RIP, &unsupported);
        assert_eq!(plan.display, display::Plan::WarnUnsupportedMode);
    }

    /// Every option, from its flag to what it changes; a new option
    /// without a case here fails.
    #[test]
    fn every_option_reaches_what_it_changes() {
        type Check = fn(&Plan, &Settings) -> bool;
        let cases: &[(&str, &[&str], Check)] = &[
            ("game", &["--game", r"D:\Jogos d'Ação"], |_, s| {
                s.game.as_deref() == Some(std::path::Path::new(r"D:\Jogos d'Ação"))
            }),
            ("runtime", &["--runtime", r"E:\SRR"], |_, s| {
                s.runtime.as_deref() == Some(std::path::Path::new(r"E:\SRR"))
            }),
            ("look", &["--look", "steam-deck"], |p, _| {
                p.graphics.resolution == Some((1280, 800))
            }),
            ("resolution", &["--resolution", "2560x1080"], |p, _| {
                p.graphics.resolution == Some((2560, 1080))
                    && p.command[1] == "/desktop=SegaRally,2560x1080"
                    && p.aspect == Aspect::Ultra21
            }),
            ("quality", &["--quality", "normal"], |p, _| {
                p.graphics.quality == Some(Quality::Normal)
            }),
            ("vsync", &["--vsync", "off"], |p, _| {
                p.graphics.vsync == Some(false)
            }),
            ("frame_limit", &["--frame-limit", "off"], |p, _| {
                p.dxvk_conf.as_deref() == Some("# no frame limit (managed by SRRLauncher)\n")
            }),
            ("stutter_fix", &["--stutter-fix", "off"], |p, _| {
                p.launch == MAIN_EXE
            }),
            ("language", &["--language", "german"], |p, _| {
                p.registry
                    .iter()
                    .any(|v| v.name == "language" && v.value == "German" && !v.if_missing)
            }),
        ];
        for option in rea_launcher::cfg::spec::active(crate::config::OPTIONS) {
            assert!(
                cases.iter().any(|(key, _, _)| *key == option.key),
                "no end-to-end case for {}",
                option.key
            );
        }
        let base = super::plan(&Settings::default(), &RIP, &linux());
        for (key, args, check) in cases {
            let cli = Settings::parse_args(args.iter().map(|a| a.to_string())).unwrap();
            let settings = Settings::default().merged(cli.settings);
            let plan = super::plan(&settings, &RIP, &linux());
            assert!(check(&plan, &settings), "{key}: {plan:#?}");
            assert!(
                !check(&base, &Settings::default()),
                "{key} holds by default"
            );
        }
    }
}
