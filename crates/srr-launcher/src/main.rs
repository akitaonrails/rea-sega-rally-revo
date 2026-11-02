#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(any(windows, test))]
mod palette;
#[cfg(windows)]
mod ui;

#[cfg(windows)]
fn main() {
    use rea_launcher_win32::dialog::{self, Icon};
    let code = match app::run() {
        Ok(code) => code,
        Err(error) => {
            app::log_line(&format!("error: {error}"));
            dialog::message("Sega Rally Revo launcher", &error, Icon::Error);
            1
        }
    };
    std::process::exit(code as i32);
}

#[cfg(not(windows))]
fn main() {
    eprintln!("SRRLauncher is a Windows program; on Linux run it with Proton, Wine, or srr-play.");
    std::process::exit(2);
}

#[cfg(windows)]
mod app {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use rea_launcher::log::Log;
    use rea_launcher::{cfg, fsx, paths};
    use rea_launcher_win32::dialog::{self, Icon};
    use rea_launcher_win32::reg::Key;
    use rea_launcher_win32::{display as screen, gpu, wine, xinput};
    use srr_launcher::config::{self, Settings};
    use srr_launcher::edition;
    use srr_launcher::form::Picture;
    use srr_launcher::gamecfg;
    use srr_launcher::{display, patches, runtime};

    use crate::ui;

    use runtime::DXVK_MARKER;

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("SRRLauncher")
    }

    pub fn log_line(line: &str) {
        Log::new(home().join("launcher.log")).line(line);
    }

    fn settings_path() -> PathBuf {
        let portable = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join(config::FILE_NAME)));
        match portable {
            Some(path) if path.is_file() => path,
            _ => home().join(config::FILE_NAME),
        }
    }

    fn find_game(settings: &Settings) -> Result<PathBuf, String> {
        if let Some(game) = &settings.game {
            return Ok(game.clone());
        }
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            && (dir.join(edition::MAIN_EXE).is_file() || dir.join(edition::SSE1_EXE).is_file())
        {
            return Ok(dir);
        }
        Err(format!(
            "Choose the folder containing SEGA Rally.exe or {}, or run:\n\nSRRLauncher --game <folder>",
            edition::SSE1_EXE
        ))
    }

    fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
        let found = args.iter().any(|arg| arg == flag);
        args.retain(|arg| arg != flag);
        found
    }

    fn launcher_dir() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()?
            .parent()
            .map(Path::to_path_buf)
    }

    /// Copy the bundled DXVK d3d9.dll next to the runtime exe (Linux only).
    fn configure_dxvk(runtime: &Path, linux: bool) -> Result<(), String> {
        let marker = runtime.join(DXVK_MARKER);
        let destination = runtime.join("d3d9.dll");
        if !linux {
            if marker.is_file() {
                let _ = fsx::remove(&destination);
                let _ = fsx::remove(&marker);
            }
            return Ok(());
        }
        let source = launcher_dir()
            .map(|dir| dir.join("linux").join("d3d9.dll"))
            .ok_or("cannot locate the launcher folder")?;
        if !source.is_file() {
            return Err(format!(
                "DXVK is missing: {}. Keep the linux folder next to SRRLauncher.exe.",
                source.display()
            ));
        }
        let same = fs::read(&source)
            .ok()
            .zip(fs::read(&destination).ok())
            .is_some_and(|(a, b)| a == b);
        if !same {
            fsx::install_copy(&source, &destination)
                .map_err(|error| format!("cannot install DXVK into the runtime: {error}"))?;
        }
        fsx::install(&marker, b"d3d9.dll is managed by SRRLauncher\n")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn read_exe(dir: &Path, name: &str) -> Option<Vec<u8>> {
        fs::read(dir.join(name)).ok()
    }

    fn write_exe(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
        fsx::install(&dir.join(name), bytes)
            .map_err(|error| format!("cannot write the private copy of {name}: {error}"))
    }

    /// Writes the game's install values under `HKLM\SOFTWARE\Sega\SEGA
    /// Rally` (its 32-bit view), or its HKCU VirtualStore twin when native
    /// Windows refuses HKLM without elevation; 32-bit games read either.
    fn seed_game_registry(values: &[(&str, String)]) -> Result<&'static str, String> {
        let (key, place) = Key::machine_or_virtual_store(r"SOFTWARE\Sega\SEGA Rally")?;
        for (name, value) in values {
            key.set_string(name, value)?;
        }
        Ok(place.describe())
    }

    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

    pub fn run() -> Result<u32, String> {
        log_line(concat!("SRRLauncher ", env!("CARGO_PKG_VERSION")));
        let settings_path = settings_path();
        let mut cli: Vec<String> = std::env::args().skip(1).collect();
        if take_flag(&mut cli, "--help") {
            dialog::message("Sega Rally Revo launcher", config::USAGE, Icon::Info);
            return Ok(0);
        }
        let settings_only = take_flag(&mut cli, "--settings");
        let linux = take_flag(&mut cli, "--linux");
        let no_window = take_flag(&mut cli, "--no-window");
        let check_only = take_flag(&mut cli, "--check");
        let setup_only = take_flag(&mut cli, "--setup");
        // Validate options up front, so a typo is reported even when the
        // window opens instead of playing.
        let overrides = Settings::parse_args(cli.clone())?;
        if settings_only && !cli.is_empty() {
            return Err(format!(
                "--settings opens the window and takes no other options (got {})",
                cli.join(" ")
            ));
        }
        // Before any window or game: the settings window's pad navigation
        // and the game both read XInput through Wine's bus.
        if wine::is_wine() {
            match wine::seed_winebus(&gamecfg::WINEBUS_VALUES) {
                Ok(true) => log_line(
                    "wine: controller bridge seeded (winebus: SDL mapped, hidraw/evdev off)",
                ),
                Ok(false) => {}
                Err(error) => log_line(&format!("warning: {error}")),
            }
        }
        if settings_only || (cli.is_empty() && !no_window && !check_only && !setup_only) {
            if wine::is_wine() {
                match gpu::prefer_glx() {
                    Ok(true) => log_line("Wine settings window: OpenGL through GLX (UseEGL=N)"),
                    Ok(false) => {}
                    Err(error) => log_line(&format!("warning: {error}")),
                }
            }
            let saved = cfg::read_file(&settings_path).ok().flatten();
            match ui::show(&settings_path, saved.as_deref(), linux) {
                Ok(ui::Outcome::Play) => {
                    if settings_only {
                        return Ok(0);
                    }
                }
                Ok(ui::Outcome::Closed) => {
                    return Ok(if settings_only { WINDOW_CLOSED } else { 0 });
                }
                Err(error) => {
                    log_line(&format!("settings window unavailable: {error}"));
                    if settings_only {
                        return Ok(WINDOW_UNAVAILABLE);
                    }
                }
            }
        }

        let existing = cfg::read_file(&settings_path).ok().flatten();
        let file_text = existing.as_deref().unwrap_or(config::TEMPLATE);
        let (saved, warnings) = Settings::parse_file(file_text);
        for warning in warnings {
            log_line(&format!("warning: {warning}"));
        }
        let settings = saved.clone().merged(overrides);
        let game = find_game(&settings)?;

        let main_bytes = read_exe(&game, edition::MAIN_EXE);
        let sse1_bytes = read_exe(&game, edition::SSE1_EXE);
        let mut editions = edition::GameEditions {
            main: edition::identify(main_bytes.as_deref().unwrap_or(&[])),
            sse1: edition::identify(sse1_bytes.as_deref().unwrap_or(&[])),
        };
        if !editions.playable() {
            let unknown = editions
                .unknown_note()
                .unwrap_or_else(|| "no known executable".into());
            return Err(format!(
                "This launcher supports the Sega Rally Revo PC v4.0.6.0 build \
                 (the clean SEGA Rally_SSE1.exe or the full-rip NoDVD SEGA Rally.exe).\n\n{unknown}"
            ));
        }
        log_line(&format!(
            "game: {} ({})",
            game.display(),
            [editions.main.ok(), editions.sse1.ok()]
                .into_iter()
                .flatten()
                .map(|edition| edition.name)
                .collect::<Vec<_>>()
                .join(" · ")
        ));
        if check_only {
            dialog::message(
                "Sega Rally Revo launcher",
                &format!(
                    "Ready to set up.\n\nGame folder: {}\n{}",
                    game.display(),
                    [editions.main.ok(), editions.sse1.ok()]
                        .into_iter()
                        .flatten()
                        .map(|edition| edition.name)
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
                Icon::Info,
            );
            return Ok(0);
        }

        let runtime_dir = settings
            .runtime
            .clone()
            .unwrap_or_else(|| home().join("SegaRally"));
        paths::check_separate(&game, &runtime_dir)?;
        if !linux {
            // Back on native Windows after a Linux run: drop our DXVK first,
            // so the sync brings the player's own d3d9.dll (if any) back.
            configure_dxvk(&runtime_dir, false)?;
        }
        let report = runtime::sync(&game, &runtime_dir, linux)
            .map_err(|error| format!("cannot prepare {}: {error}", runtime_dir.display()))?;
        for warning in &report.warnings {
            log_line(&format!("warning: {warning}"));
        }
        log_line(&format!(
            "runtime: {} ({} files copied)",
            runtime_dir.display(),
            report.brought
        ));

        let renamed = runtime::disable_bootup_videos(&runtime_dir);
        if renamed > 0 {
            log_line(&format!(
                "disabled {renamed} bootup logo video(s) in the runtime"
            ));
        }

        // Stutter fix: apply the pacer patch to the private SSE1 copy.
        let mut pacer_done = false;
        let mut stutter_fix = settings.effective_stutter_fix();
        if stutter_fix && !editions.pacer_available() {
            log_line(
                "stutter fix: requested but the clean SEGA Rally_SSE1.exe is not present; \
                 running unpatched",
            );
            stutter_fix = false;
        }
        if stutter_fix {
            let mut bytes = read_exe(&runtime_dir, edition::SSE1_EXE)
                .ok_or("the private SSE1 copy disappeared")?;
            // A previous ultra-wide launch rewrote the aspect tables; judge the
            // build without them (the aspect step below sets them again).
            match patches::identify_ignoring_aspect(&bytes) {
                Ok(edition) if std::ptr::eq(edition, &edition::SSE1_PACER) => {
                    pacer_done = true;
                    log_line("stutter fix: already applied to the private copy");
                }
                Ok(edition) if std::ptr::eq(edition, &edition::SSE1_CLEAN) => {
                    let _ = patches::aspect(&mut bytes, 16.0 / 9.0);
                    match patches::pacer_verified(&mut bytes) {
                        Ok(true) => {
                            write_exe(&runtime_dir, edition::SSE1_EXE, &bytes)?;
                            pacer_done = true;
                            log_line("stutter fix: pacer patch applied to the private copy");
                        }
                        Ok(false) => {
                            pacer_done = true;
                        }
                        Err(error) => return Err(format!("stutter fix: {error}")),
                    }
                }
                _ => {
                    return Err(
                        "stutter fix: the private SSE1 copy is not the clean build".to_string()
                    );
                }
            }
        }
        editions.sse1 = patches::identify_ignoring_aspect(
            &read_exe(&runtime_dir, edition::SSE1_EXE).ok_or("cannot re-read the SSE1 copy")?,
        );

        // The exe this run launches: the patched SSE1 build when the fix is
        // active, otherwise the rip's main NoDVD exe when present.
        let launch_name = if pacer_done {
            edition::SSE1_EXE
        } else if editions.main.is_ok() {
            edition::MAIN_EXE
        } else {
            edition::SSE1_EXE
        };
        log_line(&format!("launch executable: {launch_name}"));

        let screen = screen::screen_size();
        let picture = Picture {
            resolution: settings.resolution,
            aspect: settings.aspect,
            quality: settings.effective_quality(),
            vsync: settings.vsync.unwrap_or(true),
        }
        .resolved(screen);
        let wine = wine::is_wine();
        let gamescope = std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some();
        let (width, height) = picture.resolution.unwrap();
        let mode_supported = wine || screen::mode_supported(width, height);
        let display_plan = display::plan(
            wine,
            gamescope,
            screen.is_some_and(|(width, height)| width < height),
            mode_supported,
        );
        match display_plan {
            display::Plan::Fullscreen => {}
            display::Plan::WineDesktop => log_line(&format!(
                "display: using an isolated Wine desktop at {width}x{height}"
            )),
            display::Plan::WarnPortrait => log_line(
                "warning: the current monitor is portrait; the game's fullscreen switch may fail",
            ),
            display::Plan::WarnUnsupportedMode => log_line(&format!(
                "warning: the monitor has no {width}x{height} mode; the game's fullscreen switch may fail"
            )),
        }

        // Aspect tables: patch every game exe in the runtime so the choice
        // survives a later switch between the SSE1 and NoDVD builds.
        let aspect = settings.effective_aspect(screen);
        if !patches::is_native_aspect(aspect.ratio()) {
            for name in [edition::MAIN_EXE, edition::SSE1_EXE] {
                if let Some(mut bytes) = read_exe(&runtime_dir, name) {
                    let written = patches::aspect(&mut bytes, aspect.ratio())
                        .map_err(|error| format!("aspect patch: {error}"))?;
                    if written > 0 {
                        write_exe(&runtime_dir, name, &bytes)?;
                        log_line(&format!(
                            "aspect: {} patched into {written} table(s) of {name}",
                            aspect.key()
                        ));
                    }
                }
            }
        } else if let Some(mut bytes) = read_exe(&runtime_dir, launch_name) {
            let restored = patches::aspect(&mut bytes, 16.0 / 9.0)
                .map_err(|error| format!("aspect restore: {error}"))?;
            if restored > 0 {
                write_exe(&runtime_dir, launch_name, &bytes)?;
                log_line(&format!(
                    "aspect: restored the native 16:9 constant in {restored} table(s)"
                ));
            }
        }

        // The game's own configuration, written the way the original .NET
        // launcher would have written it.
        let profile = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .ok_or("cannot locate the profile folder")?;
        let public = std::env::var_os("PUBLIC")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                profile
                    .parent()
                    .map(|parent| parent.join("Public"))
                    .unwrap_or_else(|| profile.clone())
            });
        let paths = gamecfg::documents_paths(&profile, &public);
        for dir in [&paths.saved_games, &paths.rank_table, &paths.ghosts] {
            fs::create_dir_all(dir)
                .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
        }
        // Earlier releases built config.ini at `config.ini.srr-tmp`.
        if let Err(error) = fsx::remove_stale_temps(&paths.saved_games, &[gamecfg::OLD_TEMP_SUFFIX])
        {
            log_line(&format!("warning: {error}"));
        }
        let config_path = paths.saved_games.join("config.ini");
        // Seed only a missing file; a file that cannot be read is the
        // player's and is left exactly as it is.
        match gamecfg::next_config(
            fs::read(&config_path),
            picture.resolution.unwrap(),
            picture.quality,
            picture.vsync,
        ) {
            Ok(bytes) => fsx::write_atomic(&config_path, bytes)
                .map_err(|error| format!("cannot write {}: {error}", config_path.display()))?,
            Err(error) => log_line(&format!(
                "warning: cannot read {} ({error}); left unchanged",
                config_path.display()
            )),
        }
        for file in [
            (paths.saved_games.join("SEGA Rally.dat"), "save file"),
            (paths.rank_table.join("SR_RANKTABLES.DAT"), "rank table"),
        ] {
            if !file.0.exists() && fs::write(&file.0, b"").is_err() {
                return Err(format!("cannot seed the {}", file.1));
            }
        }
        log_line(&format!(
            "configured: {}x{} {}, {}, vsync {}",
            picture.resolution.unwrap().0,
            picture.resolution.unwrap().1,
            aspect.key(),
            picture.quality.key(),
            picture.vsync,
        ));

        // Registry values the game reads before it will start.
        let runtime_windows = runtime_dir.display().to_string();
        let values = gamecfg::registry_values(&runtime_windows, &settings.effective_language());
        match seed_game_registry(&values) {
            Ok(place) => log_line(&format!(
                "registry: installdir + language {} seeded ({place})",
                settings.effective_language()
            )),
            Err(error) => log_line(&format!("warning: {error}")),
        }

        if linux {
            configure_dxvk(&runtime_dir, true)?;
        }
        let dxvk_conf = runtime_dir.join("dxvk.conf");
        // Any DXVK reads dxvk.conf from the game folder: ours (srr-play,
        // AppImage) or Proton's own when Steam starts the launcher directly.
        if linux || wine {
            fsx::install(
                &dxvk_conf,
                gamecfg::dxvk_conf(settings.effective_frame_limit().fps()),
            )
            .map_err(|error| format!("cannot write dxvk.conf: {error}"))?;
            log_line(&format!(
                "dxvk: frame limit {}",
                settings
                    .effective_frame_limit()
                    .fps()
                    .map(|fps| fps.to_string())
                    .unwrap_or_else(|| "off".into())
            ));
        }

        if existing.is_none() || saved.game.as_ref() != Some(&game) {
            let text = cfg::set_key(file_text, "game", &game.display().to_string());
            fsx::write_atomic(&settings_path, text)
                .map_err(|error| format!("cannot save {}: {error}", settings_path.display()))?;
        }
        if !wine::is_wine() {
            let runtime_exe = runtime_dir.join(launch_name);
            match gpu::prefer_fast_gpu(&runtime_exe) {
                Ok(true) => log_line("Windows high-performance GPU preference set"),
                Ok(false) => {}
                Err(error) => log_line(&format!("warning: {error}")),
            }
        }
        if setup_only {
            log_line("setup complete");
            return Ok(0);
        }

        let runtime_exe = runtime_dir.join(launch_name);
        let mut command = if display_plan == display::Plan::WineDesktop {
            let mut command = Command::new("explorer");
            command
                .arg(format!("/desktop=SegaRally,{width}x{height}"))
                .arg(&runtime_exe);
            command
        } else {
            Command::new(&runtime_exe)
        };
        // Under Wine, make sure the DXVK d3d9.dll installed above is the one
        // loaded (the isolated desktop's explorer passes this on).
        if wine || linux {
            let existing = std::env::var("WINEDLLOVERRIDES").ok();
            if let Some(overrides) = gamecfg::wine_overrides(existing.as_deref(), linux) {
                log_line(&format!("wine: DLL overrides {overrides}"));
                command.env("WINEDLLOVERRIDES", overrides);
            }
        }
        let status = command
            .current_dir(&runtime_dir)
            .status()
            .map_err(|error| format!("cannot start {}: {error}", runtime_exe.display()))?;
        xinput::stop_rumble();
        let code = status.code().unwrap_or(1) as u32;
        log_line(&format!("game exited with {code}"));
        Ok(code)
    }
}
