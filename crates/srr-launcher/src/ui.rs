use std::path::{Path, PathBuf};
use std::time::Duration;

use rea_launcher::fsx;
use rea_launcher_ui::eframe;
use rea_launcher_ui::egui::{self, Align2, Key, Rect, TextureHandle, pos2, vec2};
use rea_launcher_ui::pad::{self, Nav, PadNav};
use rea_launcher_ui::theme::{self, body, decal, semibold};
use rea_launcher_ui::widgets::{self, Crop, LookCard};
use rea_launcher_ui::window::{Layout, SIZE, WindowFit};
use rea_launcher_win32::{dialog, display, wine, xinput};
use srr_launcher::config::{self, FrameLimit, LANGUAGES, Quality};
use srr_launcher::edition;
use srr_launcher::form::{self, Form, Look, Picture};

use crate::palette::{BLUE, ERROR, INK, METRICS, MUTED, ORANGE, PALETTE};

const PHOTO_W: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Play,
    Closed,
}

pub fn show(settings_path: &Path, saved: Option<&str>, linux: bool) -> Result<Outcome, String> {
    let mut outcome = Outcome::Closed;
    let wine = wine::is_wine();
    // Under Wine the window is created at its final size (XWayland can
    // lose a later resize). Wine then scales the window by its own DPI
    // setting, so that is divided out; the DPI scale is read first, as it
    // also makes the process DPI aware before the screen size is taken.
    // On Windows the window keeps its 1x creation size.
    let native = display::wine_dpi_scale();
    let fit = WindowFit::new(SIZE, wine, display::screen_size(), native);
    let zoom = fit.zoom();
    let options = fit.native_options("Sega Rally Revo");
    let path = settings_path.to_path_buf();
    let saved = saved.map(str::to_string);
    eframe::run_native(
        "Sega Rally Revo",
        options,
        Box::new(|cc| {
            theme::install_with(&cc.egui_ctx, &PALETTE, &METRICS);
            cc.egui_ctx.set_zoom_factor(zoom);
            Ok(Box::new(Window::new(
                &cc.egui_ctx,
                path,
                saved,
                linux,
                &mut outcome,
            )))
        }),
    )
    .map_err(|error| error.to_string())?;
    Ok(outcome)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tab {
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

enum Check {
    Empty,
    Found {
        names: Vec<&'static str>,
        pacer: bool,
    },
    Problem(String),
}

fn check_game(folder: &str) -> Check {
    let folder = folder.trim();
    if folder.is_empty() {
        return Check::Empty;
    }
    let dir = Path::new(folder);
    if !dir.is_dir() {
        return Check::Problem("No such folder.".into());
    }
    let read = |name: &str| std::fs::read(dir.join(name)).ok();
    let main = read(edition::MAIN_EXE);
    let sse1 = read(edition::SSE1_EXE);
    if main.is_none() && sse1.is_none() {
        return Check::Problem(
            "There is no SEGA Rally.exe or SEGA Rally_SSE1.exe in this folder.".into(),
        );
    }
    let mut editions = edition::GameEditions {
        main: Err([0; 32]),
        sse1: Err([0; 32]),
    };
    if let Some(bytes) = &main {
        editions.main = edition::identify(bytes);
    }
    if let Some(bytes) = &sse1 {
        editions.sse1 = edition::identify(bytes);
    }
    if editions.playable() {
        let mut names = Vec::new();
        if let Ok(edition) = editions.main {
            names.push(edition.name);
        }
        if let Ok(edition) = editions.sse1 {
            names.push(edition.name);
        }
        Check::Found {
            names,
            pacer: editions.pacer_available(),
        }
    } else {
        let unknown = editions
            .unknown_note()
            .unwrap_or_else(|| "unrecognised executables".into());
        Check::Problem(format!(
            "Unsupported game executables.\nThis launcher supports the v4.0.6.0 PC build \
             (clean SSE1 or the full-rip NoDVD main exe).\n\n{unknown}"
        ))
    }
}

struct Window<'a> {
    form: Form,
    tab: Tab,
    check: Check,
    checked: String,
    screen: Option<(u32, u32)>,
    resolutions: Vec<(Option<(u32, u32)>, String)>,
    pads: usize,
    linux: bool,
    /// DXVK will run the game (Linux helper, Wine or Proton).
    dxvk: bool,
    portrait: Option<TextureHandle>,
    path: PathBuf,
    text: String,
    error: Option<String>,
    rumble_test: Option<String>,
    pad: PadNav,
    pending: Vec<Nav>,
    outcome: &'a mut Outcome,
}

impl<'a> Window<'a> {
    fn new(
        ctx: &egui::Context,
        path: PathBuf,
        saved: Option<String>,
        linux: bool,
        outcome: &'a mut Outcome,
    ) -> Window<'a> {
        let screen = display::screen_size();
        let form = Form::from_file(saved.as_deref(), screen);
        let text = saved.unwrap_or_else(|| config::TEMPLATE.to_string());
        let check = check_game(&form.game);
        Window {
            checked: form.game.clone(),
            check,
            form,
            tab: Tab::Play,
            screen,
            resolutions: form::resolutions(screen),
            pads: xinput::connected(),
            linux,
            dxvk: linux || wine::is_wine(),
            portrait: widgets::load_jpeg(ctx, "portrait", include_bytes!("../assets/portrait.jpg")),
            path,
            text,
            error: None,
            rumble_test: None,
            pad: PadNav::default(),
            pending: Vec::new(),
            outcome,
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

    fn play(&mut self, ctx: &egui::Context) {
        if !self.ready() {
            self.tab = Tab::Play;
            return;
        }
        // A folder without the clean SSE1 exe only skips the fix at launch;
        // the saved choice stays for when the clean exe is there.
        let saved = self.form.save(&self.text, self.screen);
        match fsx::write_atomic(&self.path, saved) {
            Ok(()) => {
                *self.outcome = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => self.error = Some(format!("Can't save the settings: {error}")),
        }
    }
}

impl eframe::App for Window<'_> {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        let navs = self.pad.update(xinput::buttons());
        pad::inject(ctx, raw, &navs);
        self.pending.extend(navs);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
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
        let hint = "The source game is never modified. Settings are saved when you press Play.";
        if widgets::footer(ui, layout.footer, hint, self.ready()).clicked() {
            self.play(&ctx);
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play(&ctx);
        }
        ctx.request_repaint_after(Duration::from_millis(50));
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
}

impl Window<'_> {
    fn draw_photo(&self, ui: &egui::Ui, rect: Rect) {
        let painter = ui.painter();
        widgets::hero_image(painter, rect, self.portrait.as_ref(), Crop::Start, BLUE);
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
            (None, Check::Found { .. }) => ("Ready. Press Play or Enter.".to_string(), MUTED),
            (None, _) => (
                "First, show the launcher where the game is.".to_string(),
                MUTED,
            ),
        };
        ui.label(egui::RichText::new(text).font(body(14.0)).color(color));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, "Game folder (PC v4.0.6.0)");
        let hint = if self.linux {
            "The folder with the game, e.g. Z:\\home\\you\\Games\\Sega Rally"
        } else {
            "The folder with your installed SEGA Rally.exe"
        };
        if widgets::folder_field(ui, &mut self.form.game, hint, || {
            dialog::pick_folder("Choose the folder with SEGA Rally.exe")
        }) {
            self.error = None;
        }
        let (note, color) = match &self.check {
            Check::Empty => (
                "The source folder is only read, never changed.".to_string(),
                MUTED,
            ),
            Check::Found { names, .. } => (format!("Found: {}", names.join(" · ")), ORANGE),
            Check::Problem(problem) => (problem.clone(), ERROR),
        };
        ui.label(egui::RichText::new(note).font(body(13.0)).color(color));
        ui.add_space(10.0);

        widgets::heading(ui, "Look");
        ui.add_space(6.0);
        let modern_note = match Picture::MODERN.resolved(self.screen).resolution {
            Some((width, height)) => format!("{width}×{height}, High."),
            None => "High graphics at your screen's size.".to_string(),
        };
        let cards = [
            (Look::Modern, "Modern", modern_note),
            (
                Look::SteamDeck,
                "Steam Deck",
                "1280×800, High, 16:10.".to_string(),
            ),
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
                    badge: (look == form::recommended(self.screen)).then_some("Recommended"),
                    ..LookCard::default()
                };
                if widgets::look_card(ui, vec2(width, 116.0), &card).clicked() {
                    self.form.set_look(look);
                    if look == Look::Custom {
                        self.tab = Tab::Picture;
                    }
                }
            }
        });
        ui.add_space(14.0);

        widgets::heading(ui, "Timing");
        let stutter_fix_disabled = form::stutter_fix_disabled(self.folder_has_pacer());
        ui.horizontal_top(|ui| {
            widgets::field(ui, "Stutter fix");
            widgets::chips(
                ui,
                &[(true, "On (recommended)"), (false, "Off")],
                &mut self.form.stutter_fix,
                stutter_fix_disabled,
            );
        });
        let folder_lacks_pacer = matches!(self.check, Check::Found { pacer: false, .. });
        if !folder_lacks_pacer {
            widgets::note(
                ui,
                "The community pacer fix is applied to the private copy: it removes the \
                 whole-game microstutter and the stage-2+ championship judder (two stacked \
                 timing bugs in the 2007 build).",
            );
        } else {
            widgets::error_note(
                ui,
                "The stutter fix needs the clean SEGA Rally_SSE1.exe, which this folder \
                 does not have, so the game runs unpatched.",
            );
        }
        ui.horizontal_top(|ui| {
            widgets::field(ui, "Frame limit");
            widgets::chips(
                ui,
                &FrameLimit::ALL.map(|limit| (limit, limit.label())),
                &mut self.form.frame_limit,
                form::frame_limit_disabled(self.dxvk),
            );
        });
        widgets::note(
            ui,
            if self.dxvk {
                "The limit runs through DXVK's frame limiter, the community-standard pacing \
                 guard. With vsync on, the display's refresh caps the game too."
            } else {
                "The frame limit needs DXVK (Linux, Wine or Proton). On Windows the game \
                 follows vsync instead."
            },
        );
        ui.add_space(8.0);

        widgets::heading(ui, "Language");
        ui.horizontal_top(|ui| {
            widgets::field(ui, "Menus");
            let options: Vec<(String, &str)> = LANGUAGES
                .iter()
                .map(|language| ((*language).to_string(), *language))
                .collect();
            widgets::chips(ui, &options, &mut self.form.language, None);
        });
    }

    fn picture_tab(&mut self, ui: &mut egui::Ui) {
        let before = self.form.picture;
        let picture = &mut self.form.picture;
        let resolutions = &self.resolutions[..];
        let row = |ui: &mut egui::Ui, name: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
            ui.horizontal_top(|ui| {
                widgets::field(ui, name);
                ui.vertical(|ui| add(ui));
            });
        };
        row(ui, "Resolution", &mut |ui| {
            widgets::chips(ui, resolutions, &mut picture.resolution, None);
        });
        row(ui, "Aspect", &mut |ui| {
            widgets::chips(ui, &form::ASPECTS, &mut picture.aspect, None);
        });
        widgets::note(
            ui,
            "21:9 and 32:9 patch the aspect tables of the private copy. 16:9 and 16:10 \
             need no patch: the game picks between them from the resolution.",
        );
        row(ui, "Quality", &mut |ui| {
            widgets::chips(
                ui,
                &Quality::ALL.map(|quality| (quality, quality.label())),
                &mut picture.quality,
                None,
            );
        });
        row(ui, "", &mut |ui| {
            widgets::toggle(ui, &mut picture.vsync, "Vsync (60 Hz)", None);
        });
        if self.form.picture != before {
            self.form.look = Form::look_of(&self.form.picture, self.screen);
        }
    }

    fn controls_tab(&mut self, ui: &mut egui::Ui) {
        widgets::heading(ui, "Xbox-style pads");
        let pads = match self.pads {
            0 => "No Xbox-style pad is connected now. Connect one before starting the game."
                .to_string(),
            1 => "One Xbox-style pad connected.".to_string(),
            count => format!("{count} Xbox-style pads connected."),
        };
        ui.label(egui::RichText::new(pads).font(body(15.0)).color(INK));
        widgets::note(
            ui,
            "On Linux, srr-play preconfigures Wine's SDL controller bridge for this game \
             (Map Controllers=1). Use the pad's X-input mode and bind it in the game's \
             Controls menu.",
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
        ui.horizontal(|ui| {
            if widgets::outline_button(ui, "Test rumble").clicked() {
                let took = xinput::test_rumble(60, Duration::from_millis(500));
                self.rumble_test = Some(if took == 0 {
                    "No pad accepted rumble. Check the pad connection and X-input mode.".to_string()
                } else {
                    "Sent for half a second: the pad accepts rumble. This only tests the \
                     pad; keep in-game force feedback off."
                        .to_string()
                });
            }
        });
        if let Some(result) = &self.rumble_test {
            ui.label(egui::RichText::new(result).font(body(13.0)).color(ORANGE));
        }
        ui.add_space(10.0);
        widgets::note(
            ui,
            "The launcher window itself supports d-pad navigation, A to change a setting, \
             LB/RB for tabs, and Start to play.",
        );
    }
}
