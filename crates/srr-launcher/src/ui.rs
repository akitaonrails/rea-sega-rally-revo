//! The settings window. The drawing is plain egui over a [`Host`] (the
//! Win32 calls it needs, as closures), so the headless tests drive the
//! real window on the host and under `sdk/scripts/wine-test`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rea_launcher::cfg;
use rea_launcher::download::{self, Job};
use rea_launcher::fsx;
use rea_launcher_ui::download::{Download, DownloadAction, DownloadState, download_row};
use rea_launcher_ui::egui::{self, Align2, Key, Rect, Response, TextureHandle, pos2, vec2};
use rea_launcher_ui::pad::{self, Nav, PadNav};
use rea_launcher_ui::spec::{RowOptions, mark_rendered, spec_row_with};
use rea_launcher_ui::theme::{body, decal, semibold};
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::Layout;
use srr_launcher::config::{self, Aspect, FrameLimit, Quality};
use srr_launcher::directx;
use srr_launcher::edition::{self, GameEditions};
use srr_launcher::form::{self, Form, Look, Picture};

use crate::palette::{BLUE, ERROR, INK, MUTED, ORANGE};

const PHOTO_W: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Play,
    Closed,
}

/// What the window needs from the system.
pub struct Host {
    pub screen: Option<(u32, u32)>,
    /// Linux helper (`--linux`): hints use Wine's `Z:` drive.
    pub linux: bool,
    /// DXVK will run the game (Linux helper, Wine or Proton).
    pub dxvk: bool,
    pub pads: usize,
    pub pick_folder: Box<dyn Fn() -> Option<PathBuf>>,
    /// Rumbles every pad briefly; how many took it.
    pub test_rumble: Box<dyn Fn() -> usize>,
    /// The XInput buttons held now (for pad navigation).
    pub buttons: Box<dyn Fn() -> u16>,
    /// Where the DirectX 9 files are kept.
    pub directx_root: PathBuf,
    /// Starts the DirectX download on a worker thread.
    pub start_download: Box<dyn Fn(Arc<Job>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Play,
    Picture,
    Controls,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Play, Tab::Picture, Tab::Controls];
    const NAMES: [(Tab, &str); 3] = [
        (Tab::Play, "Play"),
        (Tab::Picture, "Picture"),
        (Tab::Controls, "Controls"),
    ];

    fn step(self, by: isize) -> Tab {
        let at = Tab::ALL.iter().position(|&tab| tab == self).unwrap_or(0) as isize;
        Tab::ALL[(at + by).rem_euclid(Tab::ALL.len() as isize) as usize]
    }
}

pub enum Check {
    Empty,
    Found {
        names: Vec<&'static str>,
        pacer: bool,
    },
    Problem(String),
}

pub fn check_game(folder: &str) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem(
            "There is no folder at this path. Check it, or choose the folder with Browse…".into(),
        );
    }
    let read = |name: &str| std::fs::read(dir.join(name)).ok();
    let (main, sse1) = (read(edition::MAIN_EXE), read(edition::SSE1_EXE));
    let editions = GameEditions::from_bytes(main.as_deref(), sse1.as_deref());
    if editions.playable() {
        Check::Found {
            names: editions.names(),
            pacer: editions.pacer_available(),
        }
    } else {
        Check::Problem(editions.refusal())
    }
}

pub struct Window {
    pub form: Form,
    pub tab: Tab,
    check: Check,
    checked: String,
    resolutions: Vec<(Option<(u32, u32)>, String)>,
    host: Host,
    portrait: Option<Option<TextureHandle>>,
    path: PathBuf,
    text: String,
    error: Option<String>,
    rumble_test: Option<String>,
    pad: PadNav,
    pending: Vec<Nav>,
    download: Option<Arc<Job>>,
    /// A close was held back once because the download was running.
    close_held: bool,
    pub outcome: Arc<Mutex<Outcome>>,
}

impl Window {
    pub fn new(path: PathBuf, saved: Option<String>, host: Host) -> Window {
        let form = Form::from_file(saved.as_deref(), host.screen);
        let text = saved.unwrap_or_else(config::template);
        let check = check_game(&form.game);
        Window {
            checked: form.game.clone(),
            check,
            resolutions: form::resolutions(host.screen, form.picture.resolution),
            form,
            tab: Tab::Play,
            host,
            portrait: None,
            path,
            text,
            error: None,
            rumble_test: None,
            pad: PadNav::default(),
            pending: Vec::new(),
            download: None,
            close_held: false,
            outcome: Arc::new(Mutex::new(Outcome::Closed)),
        }
    }

    fn ready(&self) -> bool {
        matches!(self.check, Check::Found { .. })
    }

    /// `None` before a usable folder is chosen, else whether it holds the
    /// clean SSE1 exe the stutter fix needs.
    fn folder_has_pacer(&self) -> Option<bool> {
        match self.check {
            Check::Found { pacer, .. } => Some(pacer),
            _ => None,
        }
    }

    /// Why Play waits while the DirectX files download or unpack (closing the
    /// window for the game would stop the worker half way).
    pub const PLAY_WAITS: &'static str =
        "Play waits until the DirectX 9 download finishes (or press Cancel).";

    /// What the window says the first time it is closed during a download.
    pub const CLOSE_HELD: &'static str = "The DirectX 9 download is still running. Close the \
         window again to stop it; the next download picks up where it stopped.";

    /// Whether the DirectX download (or its unpacking) is running.
    fn downloading(&self) -> bool {
        self.download
            .as_ref()
            .is_some_and(|job| job.phase().running())
    }

    /// The window was asked to close; true to let it. During a download the
    /// first request is held back with a warning; the second cancels the
    /// download (what was fetched stays for the next one) and closes.
    pub fn close_requested(&mut self) -> bool {
        if !self.downloading() {
            return true;
        }
        if !self.close_held {
            self.close_held = true;
            self.error = Some(Self::CLOSE_HELD.to_string());
            return false;
        }
        if let Some(job) = &self.download {
            job.cancel();
        }
        true
    }

    fn play(&mut self, ctx: &egui::Context) {
        if self.downloading() {
            return;
        }
        if !self.ready() {
            self.tab = Tab::Play;
            return;
        }
        // A folder without the clean SSE1 exe only skips the fix at launch;
        // the saved choice stays for when the clean exe is there.
        let saved = self.form.save(&self.text, self.host.screen);
        match fsx::write_atomic(&self.path, saved) {
            Ok(()) => {
                *self.outcome.lock().unwrap_or_else(|e| e.into_inner()) = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => {
                self.error = Some(format!(
                    "Can't save the settings ({error}). Check that {} can be written.",
                    self.path.display()
                ))
            }
        }
    }

    /// Pad input for this frame (from `raw_input_hook`).
    pub fn pad_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let navs = self.pad.update((self.host.buttons)());
        pad::inject(ctx, raw, &navs);
        self.pending.extend(navs);
    }

    /// The whole window.
    pub fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.close_held && !self.downloading() {
            // The download ended (or was cancelled): the warning is moot.
            self.close_held = false;
            if self.error.as_deref() == Some(Self::CLOSE_HELD) {
                self.error = None;
            }
        }
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BLUE);
        let layout = Layout::new(full, PHOTO_W);
        self.draw_photo(ui, layout.photo);
        widgets::stripes(
            ui.painter(),
            full,
            &[(30.0, ORANGE), (16.0, ORANGE.gamma_multiply(0.65))],
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(layout.body), |ui| {
            ui.label(
                egui::RichText::new("Sega Rally Revo")
                    .font(decal(43.0))
                    .color(INK),
            );
            ui.add_space(-6.0);
            self.status(ui);
            ui.add_space(6.0);
            widgets::tabs_bar(ui, &Tab::NAMES, &mut self.tab);
            ui.add_space(14.0);
            // Tall tabs scroll inside the body instead of running into the footer.
            widgets::tab_body(ui, self.tab, |ui| match self.tab {
                Tab::Play => self.play_tab(ui),
                Tab::Picture => self.picture_tab(ui),
                Tab::Controls => self.controls_tab(ui),
            });
        });
        let downloading = self.downloading();
        let hint = if downloading {
            Self::PLAY_WAITS
        } else {
            "The game folder is never changed. Settings are saved when you press Play."
        };
        if widgets::footer(ui, layout.footer, hint, self.ready() && !downloading).clicked() {
            self.play(&ctx);
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play(&ctx);
        }
        // Taken, so a second egui pass in one frame cannot play twice.
        for nav in std::mem::take(&mut self.pending) {
            match nav {
                Nav::Play => self.play(&ctx),
                Nav::NextTab => self.tab = self.tab.step(1),
                Nav::PreviousTab => self.tab = self.tab.step(-1),
                _ => {}
            }
        }
        if self.form.game != self.checked {
            self.checked = self.form.game.clone();
            self.check = check_game(&self.form.game);
        }
    }

    fn draw_photo(&mut self, ui: &egui::Ui, rect: Rect) {
        let portrait = self
            .portrait
            .get_or_insert_with(|| {
                widgets::load_jpeg(
                    ui.ctx(),
                    "portrait",
                    include_bytes!("../assets/portrait.jpg"),
                )
            })
            .clone();
        let painter = ui.painter();
        widgets::hero_image(painter, rect, portrait.as_ref(), Crop::Start, BLUE);
        // The credit sits on a solid backing (no small text over a photo).
        let backing = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 84.0), rect.max);
        painter.rect_filled(backing, 0.0, BLUE);
        let x = rect.left() + 20.0;
        let y = rect.bottom() - 20.0;
        painter.text(
            pos2(x, y),
            Align2::LEFT_BOTTOM,
            "Original artwork made for this launcher",
            body(11.0),
            MUTED,
        );
        painter.text(
            pos2(x, y - 16.0),
            Align2::LEFT_BOTTOM,
            "Sega Racing Studio, 2007",
            body(13.0),
            MUTED,
        );
        painter.text(
            pos2(x, y - 34.0),
            Align2::LEFT_BOTTOM,
            "SEGA RALLY REVO",
            semibold(19.0),
            INK,
        );
    }

    fn status(&self, ui: &mut egui::Ui) {
        let (text, color) = match (&self.error, &self.check) {
            (Some(error), _) => (error.clone(), ERROR),
            (None, Check::Found { .. }) if self.downloading() => {
                (Self::PLAY_WAITS.to_string(), MUTED)
            }
            (None, Check::Found { .. }) => ("Ready. Press Play or Enter.".to_string(), MUTED),
            (None, _) => (
                "First, show the launcher where the game is.".to_string(),
                MUTED,
            ),
        };
        ui.label(egui::RichText::new(text).font(body(14.0)).color(color));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        let game = config::option("game");
        widgets::heading(ui, "Game folder (PC v4.0.6.0)");
        mark_rendered(ui.ctx(), game.key);
        let hint = if self.host.linux {
            "Game folder, e.g. Z:\\home\\you\\Games\\Sega Rally"
        } else {
            "Game folder: the one with SEGA Rally.exe"
        };
        let pick = &self.host.pick_folder;
        if widgets::folder_field(ui, &mut self.form.game, "Game folder", hint, pick) {
            self.error = None;
        }
        match &self.check {
            Check::Empty => {
                widgets::note(ui, game.description);
            }
            Check::Found { names, .. } => {
                ui.label(
                    egui::RichText::new(format!("Found: {}", names.join(", ")))
                        .font(body(13.0))
                        .color(ORANGE),
                );
            }
            Check::Problem(problem) => {
                widgets::error_note(ui, problem);
            }
        }
        ui.add_space(10.0);

        widgets::heading(ui, "Look");
        mark_rendered(ui.ctx(), "look");
        ui.add_space(6.0);
        let screen = self.host.screen;
        let modern_note = match Picture::resolved(
            Picture {
                resolution: None,
                quality: Quality::High,
                vsync: true,
            },
            screen,
        )
        .resolution
        {
            Some((width, height)) => format!("{width}×{height}, High."),
            None => "High graphics at your screen's size.".to_string(),
        };
        let cards = [
            (Look::Modern, "Modern", modern_note),
            (Look::SteamDeck, "Steam Deck", "1280×800, High.".to_string()),
            (
                Look::Original,
                "Original 2007",
                "1280×720, Normal.".to_string(),
            ),
            (
                Look::Custom,
                "Custom",
                "Pick each setting in Picture.".to_string(),
            ),
        ];
        let gap = 10.0;
        let width = (ui.available_width() - 3.0 * gap) / 4.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for (look, title, note) in cards {
                let card = LookCard {
                    title,
                    note: &note,
                    selected: self.form.look == look,
                    badge: (look == form::recommended(screen)).then_some("Recommended"),
                    ..LookCard::default()
                };
                if widgets::look_card(ui, vec2(width, 116.0), &card).clicked() {
                    self.form.set_look(look);
                    self.refresh_resolutions();
                    if look == Look::Custom {
                        self.tab = Tab::Picture;
                    }
                }
            }
        });
        ui.add_space(14.0);

        widgets::heading(ui, "Timing");
        let stutter_disabled = form::stutter_fix_disabled(self.folder_has_pacer());
        option_row(ui, "stutter_fix", |ui| {
            widgets::chips(
                ui,
                &[(true, "On (recommended)"), (false, "Off")],
                &mut self.form.stutter_fix,
                stutter_disabled,
            )
        });
        let frame_disabled = form::frame_limit_disabled(self.host.dxvk);
        if let Some(text) = table_row(
            ui,
            "frame_limit",
            self.form.frame_limit.key(),
            frame_disabled,
        ) && let Some(limit) = FrameLimit::parse(&text)
        {
            self.form.frame_limit = limit;
        }
        ui.add_space(8.0);

        widgets::heading(ui, "Language");
        if let Some(text) = table_row(ui, "language", &self.form.language, None) {
            self.form.language = text;
        }
        ui.add_space(8.0);

        self.directx_section(ui);
    }

    fn directx_section(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, directx::LABEL);
        widgets::note(
            ui,
            "Windows 10 and 11 lack the DirectX 9 files the game was built with. One download \
             from Microsoft (100 MB) puts them in the private copy. Under Linux the game \
             usually runs without them.",
        );
        let pin = directx::pin();
        let root = self.host.directx_root.clone();
        let installed = download::installed(&pin, &root).is_some();
        let phase = self.download.as_ref().map(|job| job.phase());
        let state =
            DownloadState::from_phase(phase.as_ref(), installed, download::partial(&pin, &root));
        let row = Download {
            label: directx::LABEL,
            size: pin.size,
            state,
        };
        match download_row(ui, &row) {
            Some(DownloadAction::Start) => {
                let job = Arc::new(Job::default());
                self.download = Some(job.clone());
                (self.host.start_download)(job);
            }
            Some(DownloadAction::Cancel) => {
                if let Some(job) = &self.download {
                    job.cancel();
                }
            }
            Some(DownloadAction::Remove) => {
                if let Err(error) = download::remove(&pin, &root) {
                    self.error = Some(format!("Can't remove the DirectX 9 files: {error}"));
                }
                self.download = None;
            }
            None => {}
        }
        widgets::note(ui, directx::LICENCE);
        if state.running() {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }

    fn refresh_resolutions(&mut self) {
        self.resolutions = form::resolutions(self.host.screen, self.form.picture.resolution);
    }

    /// Picture: the screen size (with what the game will run at, and
    /// whether the ultra-wide fix applies), then detail and vsync.
    fn picture_tab(&mut self, ui: &mut egui::Ui) {
        let screen = self.host.screen;
        let mut picture = self.form.picture;
        widgets::heading(ui, "Screen");
        option_row(ui, "resolution", |ui| {
            widgets::chips(ui, &self.resolutions, &mut picture.resolution, None)
        });
        let size = picture.resolved(screen).resolution.expect("resolved");
        let outcome = match Aspect::for_resolution(size) {
            Aspect::Native => format!("Runs at {}×{}, in the game's own aspect.", size.0, size.1),
            ultra => format!(
                "Runs at {}×{}: {}, so the private copy gets the ultra-wide fix.",
                size.0,
                size.1,
                ultra.key()
            ),
        };
        ui.horizontal(|ui| {
            ui.add_space(rea_launcher_ui::theme::metrics(ui.ctx()).field_width);
            ui.label(egui::RichText::new(outcome).font(body(14.0)).color(ORANGE));
        });
        ui.add_space(12.0);

        widgets::heading(ui, "Detail");
        if let Some(text) = table_row(ui, "quality", picture.quality.key(), None)
            && let Some(quality) = Quality::parse(&text)
        {
            picture.quality = quality;
        }
        if let Some(text) = table_row(ui, "vsync", cfg::on_off(picture.vsync), None) {
            picture.vsync = text == cfg::on_off(true);
        }
        self.form.edit_picture(picture, screen);
    }

    fn controls_tab(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, "Xbox-style pads");
        let pads = match self.host.pads {
            0 => "No Xbox-style pad is connected now. Connect one before starting the game."
                .to_string(),
            1 => "One Xbox-style pad connected.".to_string(),
            count => format!("{count} Xbox-style pads connected."),
        };
        ui.label(egui::RichText::new(pads).font(body(15.0)).color(INK));
        widgets::note(
            ui,
            "Switch the pad to X-input mode and set it up in the game's own Controls menu. \
             On Linux the launcher prepares Wine's controller support for this game.",
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "Leave force feedback off in the game's controller options: it is a known \
                 crash (reported on Windows 10) and is not needed to play.",
            )
            .font(body(13.0))
            .color(ORANGE),
        );
        ui.add_space(8.0);
        if widgets::outline_button(ui, "Test rumble").clicked() {
            let took = (self.host.test_rumble)();
            self.rumble_test = Some(if took == 0 {
                "No pad accepted rumble. Check the pad connection and X-input mode.".to_string()
            } else {
                "Sent for half a second: the pad accepts rumble. This only tests the pad; \
                 keep in-game force feedback off."
                    .to_string()
            });
        }
        if let Some(result) = &self.rumble_test {
            ui.label(egui::RichText::new(result).font(body(13.0)).color(ORANGE));
        }
        ui.add_space(10.0);
        widgets::note(
            ui,
            "This window works with the pad too: d-pad to move, A to change a setting, \
             LB/RB for tabs, Start to play.",
        );
    }
}

/// One option's row drawn by the SDK from the option table
/// (`spec_row_with`: label, control, description, the disabled reason)
/// over the window's typed value, given as its `launcher.cfg` text; the
/// new text when the player changed it.
fn table_row(ui: &mut egui::Ui, key: &str, text: &str, disabled: Option<&str>) -> Option<String> {
    let mut value = text.to_string();
    let options = RowOptions {
        disabled,
        ..RowOptions::default()
    };
    let response = spec_row_with(ui, config::option(key), &mut value, options);
    ui.add_space(6.0);
    (response.changed() && value != text).then_some(value)
}

/// A row the SDK cannot draw from the table yet, laid out as
/// `spec_row_with` lays out a chip row: the label, the control, the
/// description under them; the control is named and described for
/// screen readers and the key is recorded for the parity test. Used for
/// the resolution (a `Size` row as chips of the screen's sizes) and the
/// stutter fix (a `Toggle` as chips that keep showing the saved value
/// while disabled).
fn option_row(ui: &mut egui::Ui, key: &str, control: impl FnOnce(&mut egui::Ui) -> Response) {
    let spec = config::option(key);
    mark_rendered(ui.ctx(), spec.key);
    ui.vertical(|ui| {
        let response = ui
            .horizontal_top(|ui| {
                widgets::field(ui, spec.label);
                control(ui)
            })
            .inner;
        widgets::name_control(&response, Some(spec.label), Some(spec.description));
        widgets::note(ui, spec.description);
    });
    ui.add_space(6.0);
}

#[cfg(windows)]
pub use native::show;

#[cfg(windows)]
mod native {
    use super::*;
    use crate::palette::{METRICS, PALETTE};
    use rea_launcher_ui::eframe;
    use rea_launcher_ui::theme;
    use rea_launcher_ui::window::{SIZE, WindowFit};
    use rea_launcher_win32::{dialog, disk, display, http, wine, xinput};

    struct App {
        window: Window,
    }

    impl eframe::App for App {
        fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
            self.window.pad_input(ctx, raw);
        }

        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            if ui.ctx().input(|input| input.viewport().close_requested())
                && !self.window.close_requested()
            {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
            self.window.draw(ui);
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
    }

    pub fn show(
        settings_path: &Path,
        directx_root: &Path,
        saved: Option<&str>,
        linux: bool,
    ) -> Result<Outcome, String> {
        let wine = wine::is_wine();
        // Under Wine the window is created at its final size (XWayland can
        // lose a later resize). Wine then scales the window by its own DPI
        // setting, so that is divided out; the DPI scale is read first, as it
        // also makes the process DPI aware before the screen size is taken.
        // On Windows the window keeps its 1x creation size (no native fit).
        let native = display::wine_dpi_scale();
        let screen = display::screen_size();
        let fit = WindowFit::new(SIZE, wine, screen, native);
        let zoom = fit.zoom();
        let options = fit.native_options(srr_launcher::instance::WINDOW_TITLE);
        let host = Host {
            screen,
            linux,
            dxvk: linux || wine,
            pads: xinput::connected(),
            pick_folder: Box::new(|| dialog::pick_folder("Choose the folder with SEGA Rally.exe")),
            test_rumble: Box::new(|| xinput::test_rumble(60, Duration::from_millis(500))),
            buttons: Box::new(xinput::buttons),
            directx_root: directx_root.to_path_buf(),
            start_download: {
                let root = directx_root.to_path_buf();
                Box::new(move |job| {
                    let root = root.clone();
                    std::thread::spawn(move || {
                        let fetch =
                            http::WinHttp::new(concat!("SRRLauncher/", env!("CARGO_PKG_VERSION")));
                        let _ = directx::install(
                            &directx::pin(),
                            &root,
                            &disk::free_space,
                            &fetch,
                            &job,
                        );
                    });
                })
            },
        };
        let window = Window::new(settings_path.to_path_buf(), saved.map(str::to_string), host);
        let outcome = window.outcome.clone();
        eframe::run_native(
            srr_launcher::instance::WINDOW_TITLE,
            options,
            Box::new(move |cc| {
                theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
                cc.egui_ctx.set_zoom_factor(zoom);
                Ok(Box::new(App { window }))
            }),
        )
        .map_err(|error| error.to_string())?;
        Ok(*outcome.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

#[cfg(test)]
mod tests;
