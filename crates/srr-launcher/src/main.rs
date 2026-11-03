#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(any(windows, test))]
mod palette;
#[cfg(any(windows, test))]
mod ui;

#[cfg(windows)]
fn main() {
    let code = match app::run() {
        Ok(code) => code,
        Err(error) => {
            app::log_line(&format!("error: {error}"));
            app::fatal(&error);
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

    use rea_launcher::identify;
    use rea_launcher::ini::Owned;
    use rea_launcher::log::Log;
    use rea_launcher::{cfg, download, fsx, paths};
    use rea_launcher_win32::reg::Key;
    use rea_launcher_win32::report::{self, Mode};
    use rea_launcher_win32::{disk, display as screen, gpu, http, wine, xinput};
    use srr_launcher::config::{self, Settings};
    use srr_launcher::edition::{self, GameEditions};
    use srr_launcher::plan::{self, Env};
    use srr_launcher::{directx, gamecfg, instance, patches, runtime};

    use crate::ui;

    use runtime::DXVK_MARKER;

    fn home() -> PathBuf {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("SRRLauncher")
    }

    /// Where the DirectX 9 files are kept after the player downloads them.
    pub fn directx_root() -> PathBuf {
        home().join("directx")
    }

    pub fn log_line(line: &str) {
        Log::new(home().join("launcher.log")).line(line);
    }

    const TITLE: &str = "Sega Rally Revo launcher";

    /// Who may see this run's messages: the SDK's rule
    /// (`report::audience`) for this command line (`config::run_kind`).
    /// Under Wine a game run started with `--no-window` (srr-play, the
    /// AppImage, Proton) has a player watching; `--settings --linux` and
    /// `--check --linux` report through srr-play instead.
    fn mode() -> Mode {
        let args: Vec<String> = std::env::args().skip(1).collect();
        report::audience(config::run_kind(&args)).mode()
    }

    /// An error the launcher stops on: a dialog when a person may be
    /// watching, and stdout, which `srr-play` and Proton keep in a log.
    pub fn fatal(text: &str) {
        report::fatal(TITLE, text, mode());
    }

    /// A message while preparing to play (another launcher is running).
    fn play_note(text: &str) {
        report::note(TITLE, text, mode(), true);
    }

    /// Output of a command (`--help`, `--check`, `--download-directx`):
    /// the terminal, or a dialog when nobody sees it.
    fn command(text: &str) {
        report::note(TITLE, text, mode(), false);
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
        if let Some(dir) = launcher_dir()
            && (dir.join(edition::MAIN_EXE).is_file() || dir.join(edition::SSE1_EXE).is_file())
        {
            return Ok(dir);
        }
        Err(format!(
            "Choose the folder containing {} or {}, or run:\n\nSRRLauncher --game <folder>",
            edition::MAIN_EXE,
            edition::SSE1_EXE
        ))
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
        log_line(&format!(
            "dxvk: d3d9.dll installed (SHA-256 {})",
            identify::sha256_file(&destination)
                .map(|digest| identify::hex(&digest))
                .unwrap_or_else(|_| "unreadable".into())
        ));
        Ok(())
    }

    /// Writes the game's install values under `HKLM\SOFTWARE\Sega\SEGA
    /// Rally` (its 32-bit view), or its HKCU VirtualStore twin when native
    /// Windows refuses HKLM without elevation; 32-bit games read either.
    fn seed_game_registry(values: &[gamecfg::RegValue]) -> Result<&'static str, String> {
        let (key, place) = Key::machine_or_virtual_store(r"SOFTWARE\Sega\SEGA Rally")?;
        for value in values {
            if value.if_missing {
                key.set_string_if_missing(value.name, &value.value)?;
            } else {
                key.set_string(value.name, &value.value)?;
            }
        }
        Ok(place.describe())
    }

    /// `--download-directx`: the window's DirectX download without the
    /// window, logging progress every 10 percent.
    fn download_directx() -> Result<u32, String> {
        let pin = directx::pin();
        let root = directx_root();
        if download::installed(&pin, &root).is_some() {
            let note = directx::already_note();
            log_line(&note);
            command(&note);
            return Ok(0);
        }
        let job = std::sync::Arc::new(download::Job::default());
        let worker = job.clone();
        let handle = std::thread::spawn(move || {
            let fetch = http::WinHttp::new(concat!("SRRLauncher/", env!("CARGO_PKG_VERSION")));
            directx::install(&pin, &root, &disk::free_space, &fetch, &worker)
        });
        let mut logged = None;
        while !handle.is_finished() {
            if let download::Phase::Downloading { done, total } = job.phase() {
                let tenth = done * 10 / total.max(1);
                if logged != Some(tenth) {
                    log_line(&format!("directx: downloading {}%", tenth * 10));
                    logged = Some(tenth);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        let dir = handle
            .join()
            .map_err(|_| "the download stopped unexpectedly".to_string())??;
        let note = directx::done_note(directx::dlls(&dir).len(), &dir);
        log_line(&note);
        command(&note);
        Ok(0)
    }

    const WINDOW_CLOSED: u32 = 3;
    const WINDOW_UNAVAILABLE: u32 = 4;

    pub fn run() -> Result<u32, String> {
        log_line(concat!("SRRLauncher ", env!("CARGO_PKG_VERSION")));
        let settings_path = settings_path();
        // Validated up front, so a typo is reported even when the window
        // opens instead of playing.
        let cli = match Settings::parse_args(std::env::args().skip(1)) {
            Ok(cli) => cli,
            Err(message) if message == config::usage() => {
                command(&message);
                return Ok(0);
            }
            Err(message) => return Err(message),
        };
        // One launcher at a time, held until the game exits: a second
        // start brings the window forward or refuses to play.
        let _instance = if instance::needs_instance(&cli.switches) {
            match rea_launcher_win32::instance::single(instance::NAME) {
                Ok(guard) => Some(guard),
                Err(running) => {
                    let window =
                        rea_launcher_win32::instance::Window::Title(instance::WINDOW_TITLE);
                    let second = instance::Second::of(
                        instance::is_action(&cli.switches),
                        || running.is_window_found(window),
                        || running.bring_to_front(window),
                    );
                    log_line(&format!("another launcher is running: {second:?}"));
                    match second.message() {
                        // An action's outcome goes where its output goes.
                        Some(message) if instance::is_action(&cli.switches) => command(message),
                        Some(message) => play_note(message),
                        None => {}
                    }
                    return Ok(second.exit_code());
                }
            }
        } else {
            None
        };
        let settings_only = cli.has("--settings");
        let linux = cli.has("--linux");
        let no_window = cli.has("--no-window");
        let check_only = cli.has("--check");
        let setup_only = cli.has("--setup");
        if cli.has("--download-directx") {
            return download_directx();
        }
        if settings_only && cli.options_given() {
            return Err(
                "--settings opens the window and takes no other options; save them in the \
                 window or launcher.cfg"
                    .to_string(),
            );
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
        let wants_window =
            settings_only || (!cli.options_given() && !no_window && !check_only && !setup_only);
        if wants_window {
            if wine::is_wine() {
                match gpu::prefer_glx() {
                    Ok(true) => log_line("Wine settings window: OpenGL through GLX (UseEGL=N)"),
                    Ok(false) => {}
                    Err(error) => log_line(&format!("warning: {error}")),
                }
            }
            let saved = cfg::read_file(&settings_path).ok().flatten();
            match ui::show(&settings_path, &directx_root(), saved.as_deref(), linux) {
                Ok(ui::Outcome::Play) => {
                    log_line("settings window: play");
                    if settings_only {
                        return Ok(0);
                    }
                }
                Ok(ui::Outcome::Closed) => {
                    log_line("settings window: closed");
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
        let template = config::template();
        let file_text = existing.as_deref().unwrap_or(&template);
        let (saved, warnings) = Settings::parse_file(file_text);
        for warning in warnings {
            log_line(&format!("warning: {warning}"));
        }
        let settings = saved.clone().merged(cli.settings.clone());
        let game = find_game(&settings)?;

        let read_exe = |dir: &Path, name: &str| fs::read(dir.join(name)).ok();
        let main_bytes = read_exe(&game, edition::MAIN_EXE);
        let sse1_bytes = read_exe(&game, edition::SSE1_EXE);
        let editions = GameEditions::from_bytes(main_bytes.as_deref(), sse1_bytes.as_deref());
        for (name, found) in [
            (edition::MAIN_EXE, editions.main),
            (edition::SSE1_EXE, editions.sse1),
        ] {
            match found {
                Some(Ok(known)) => log_line(&format!(
                    "{name}: {} (SHA-256 {})",
                    known.name,
                    identify::hex(&known.sha256)
                )),
                Some(Err(digest)) => log_line(&format!(
                    "{name}: unknown build, SHA-256 {}; never patched",
                    identify::hex(&digest)
                )),
                None => log_line(&format!("{name}: not in the game folder")),
            }
        }
        if !editions.playable() {
            return Err(editions.refusal());
        }
        log_line(&format!("game: {}", game.display()));
        if check_only {
            command(&format!(
                "Ready to play.\n\nGame folder: {}\n{}",
                game.display(),
                editions.names().join("\n")
            ));
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
        let report_sync = runtime::sync(&game, &runtime_dir, linux)
            .map_err(|error| format!("cannot prepare {}: {error}", runtime_dir.display()))?;
        for warning in &report_sync.warnings {
            log_line(&format!("warning: {warning}"));
        }
        log_line(&format!(
            "runtime: {} ({} files copied)",
            runtime_dir.display(),
            report_sync.brought
        ));
        let renamed = runtime::disable_bootup_videos(&runtime_dir);
        if renamed > 0 {
            log_line(&format!(
                "disabled {renamed} bootup logo video(s) in the runtime"
            ));
        }

        // The DirectX 9 files the player downloaded, if any.
        let directx_dir = download::installed(&directx::pin(), &directx_root());
        let directx_files = directx::for_launch(&game, &runtime_dir, directx_dir.as_deref());
        for line in &directx_files.log {
            log_line(line);
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
        let documents = gamecfg::documents_paths(&profile, &public);
        let config_path = documents.saved_games.join("config.ini");
        let config_read = fs::read(&config_path);
        let screen = screen::screen_size();
        let wine = wine::is_wine();
        let env = Env {
            wine,
            linux,
            gamescope: std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some(),
            screen,
            mode_supported: true,
            config_resolution: config_read
                .as_ref()
                .ok()
                .and_then(|bytes| gamecfg::resolution_in(bytes)),
            dll_overrides: std::env::var("WINEDLLOVERRIDES").ok(),
            directx: directx_files.dlls.clone(),
            runtime: runtime_dir.display().to_string(),
        };
        let mut plan = plan::plan(&settings, &editions, &env);
        if !wine && !screen::mode_supported(plan.resolution.0, plan.resolution.1) {
            plan = plan::plan(
                &settings,
                &editions,
                &Env {
                    mode_supported: false,
                    ..env
                },
            );
        }
        for note in &plan.notes {
            log_line(note);
        }

        // Patches: from the game folder's pristine bytes into the private
        // copy, every launch, so a choice switched off needs no restore.
        for (name, sets) in &plan.patches {
            let pristine = if *name == edition::MAIN_EXE {
                main_bytes.as_deref()
            } else {
                sse1_bytes.as_deref()
            }
            .ok_or("a game executable disappeared")?;
            let patched = patches::apply(pristine, sets)
                .map_err(|error| format!("{name}: {} refused: {error}", patches::names(sets)))?;
            let target = runtime_dir.join(name);
            if fs::read(&target).ok().as_deref() != Some(&patched[..]) {
                fsx::install(&target, &patched)
                    .map_err(|error| format!("cannot write the private copy of {name}: {error}"))?;
            }
            log_line(&format!(
                "{name}: patches {} (SHA-256 {})",
                patches::names(sets),
                identify::hex(&identify::sha256(&patched))
            ));
        }
        log_line(&format!("launch executable: {}", plan.launch));
        match plan.display {
            srr_launcher::display::Plan::Fullscreen => {}
            srr_launcher::display::Plan::WineDesktop => log_line(&format!(
                "display: an isolated Wine desktop at {}x{}",
                plan.resolution.0, plan.resolution.1
            )),
            srr_launcher::display::Plan::WarnPortrait => log_line(
                "warning: the current monitor is portrait; the game's fullscreen switch may fail",
            ),
            srr_launcher::display::Plan::WarnUnsupportedMode => log_line(&format!(
                "warning: the monitor has no {}x{} mode; the game's fullscreen switch may fail",
                plan.resolution.0, plan.resolution.1
            )),
        }

        gamecfg::prepare_documents(&documents)?;
        let removed = gamecfg::remove_old_seeds(&documents);
        if removed > 0 {
            log_line(&format!(
                "saves: removed {removed} empty save file(s) left by an earlier release; \
                 the game creates them"
            ));
        }
        // Earlier releases built config.ini at `config.ini.srr-tmp`.
        if let Err(error) =
            fsx::remove_stale_temps_here(&documents.saved_games, &[gamecfg::OLD_TEMP_SUFFIX])
        {
            log_line(&format!("warning: {error}"));
        }
        let owned_path = runtime_dir.join(runtime::CONFIG_OWNED);
        let mut owned = Owned::load(&owned_path).unwrap_or_default();
        let seeded = config_read.is_err();
        match gamecfg::next_config(config_read, plan.seed, &plan.graphics, &mut owned) {
            Ok(bytes) => {
                fsx::write_atomic(&config_path, bytes)
                    .map_err(|error| format!("cannot write {}: {error}", config_path.display()))?;
                owned
                    .save(&owned_path)
                    .map_err(|error| format!("cannot save {}: {error}", owned_path.display()))?;
                log_line(&format!(
                    "config.ini: {} ({}x{}; set in launcher.cfg: {:?})",
                    if seeded { "seeded" } else { "updated" },
                    plan.resolution.0,
                    plan.resolution.1,
                    plan.graphics
                ));
            }
            Err(error) => log_line(&format!(
                "warning: cannot read {} ({error}); left unchanged",
                config_path.display()
            )),
        }

        match seed_game_registry(&plan.registry) {
            Ok(place) => log_line(&format!(
                "registry: installdir, sku and language ({}) written ({place})",
                settings.effective_language()
            )),
            Err(error) => log_line(&format!("warning: {error}")),
        }

        if plan.install_dxvk {
            configure_dxvk(&runtime_dir, true)?;
        }
        if let Some(conf) = &plan.dxvk_conf {
            fsx::install(&runtime_dir.join("dxvk.conf"), conf)
                .map_err(|error| format!("cannot write dxvk.conf: {error}"))?;
            log_line(&format!("dxvk.conf: {}", conf.trim()));
        }

        if existing.is_none() || saved.game.as_ref() != Some(&game) {
            let text = config::set_key(file_text, "game", &game.display().to_string());
            fsx::write_atomic(&settings_path, text)
                .map_err(|error| format!("cannot save {}: {error}", settings_path.display()))?;
        }
        if plan.gpu_preference {
            match gpu::prefer_fast_gpu(&runtime_dir.join(plan.launch)) {
                Ok(true) => log_line("Windows high-performance GPU preference set"),
                Ok(false) => {}
                Err(error) => log_line(&format!("warning: {error}")),
            }
        }
        if setup_only {
            log_line("setup complete");
            return Ok(0);
        }

        let mut command = Command::new(&plan.command[0]);
        command.args(&plan.command[1..]);
        if let Some(overrides) = &plan.dll_overrides {
            log_line(&format!("wine: DLL overrides {overrides}"));
            command.env("WINEDLLOVERRIDES", overrides);
        }
        // Wine ignores WINEDLLOVERRIDES in a child's environment block
        // (SDK v0.3.3): the bundled DLLs go into the game's per-exe
        // registry overrides; values the player set are kept.
        if wine {
            use rea_launcher_win32::wine::{self as winereg, DllMode};
            let on: Vec<(&str, DllMode)> = plan
                .app_dlls
                .iter()
                .map(|dll| (dll.as_str(), DllMode::NativeThenBuiltin))
                .collect();
            let off: Vec<(&str, DllMode)> = plan
                .app_dlls_off
                .iter()
                .map(|dll| (dll.as_str(), DllMode::NativeThenBuiltin))
                .collect();
            match winereg::remove_app_dll_overrides(plan.launch, &off) {
                Ok(removed) if !removed.is_empty() => log_line(&format!(
                    "wine: per-exe DLL overrides removed for {}: {}",
                    plan.launch,
                    removed.join(",")
                )),
                Ok(_) => {}
                Err(error) => log_line(&format!("warning: {error}")),
            }
            match winereg::app_dll_overrides(plan.launch, &on) {
                Ok(applied) => {
                    if !on.is_empty() {
                        log_line(&format!(
                            "wine: per-exe DLL overrides for {} (native,builtin): written {:?}, already ours {:?}, the player's kept {:?}",
                            plan.launch, applied.written, applied.ours, applied.player
                        ));
                    }
                }
                Err(error) => log_line(&format!("warning: {error}")),
            }
        }
        let status = command
            .current_dir(&runtime_dir)
            .status()
            .map_err(|error| format!("cannot start {}: {error}", plan.command.join(" ")))?;
        xinput::stop_rumble();
        let code = status.code().unwrap_or(1) as u32;
        log_line(&format!("game exited with {code}"));
        Ok(code)
    }
}
