use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Key, Mesh,
    Rect, Sense, Shape, Stroke, StrokeKind, TextureHandle, Vec2, pos2, vec2,
};
use srr_launcher::config::{self, FrameLimit, LANGUAGES, Quality};
use srr_launcher::edition;
use srr_launcher::form::{self, Form, Look, PadAction, Picture};

use crate::win32;

const BLUE: Color32 = Color32::from_rgb(0x0a, 0x1b, 0x33); // SEGA navy panel fill
const PANEL: Color32 = Color32::from_rgb(0x10, 0x2a, 0x4d); // card fill
const LINE: Color32 = Color32::from_rgb(0x1f, 0x4a, 0x84); // widget borders
const ORANGE: Color32 = Color32::from_rgb(0xff, 0x7a, 0x00); // the one accent (Revo orange)
const INK: Color32 = Color32::from_rgb(0xed, 0xf3, 0xfc); // text
const MUTED: Color32 = Color32::from_rgb(0x9f, 0xb5, 0xd8); // secondary text
const ON_ORANGE: Color32 = Color32::from_rgb(0x0a, 0x1b, 0x33); // text on orange
const ERROR: Color32 = Color32::from_rgb(0xff, 0x8a, 0x80);
const SIZE: Vec2 = vec2(form::WINDOW.0, form::WINDOW.1);
const PHOTO_W: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Play,
    Closed,
}

pub fn show(settings_path: &Path, saved: Option<&str>, linux: bool) -> Result<Outcome, String> {
    let mut outcome = Outcome::Closed;
    let wine = win32::is_wine();
    // Under Wine the window is created at its final size (XWayland can
    // lose a later resize). Wine then scales the window by its own DPI
    // setting, so that is divided out; the DPI scale is read first, as it
    // also makes the process DPI aware before the screen size is taken.
    let native = if wine { win32::wine_dpi_scale() } else { 1.0 };
    let zoom = form::creation_zoom(wine, win32::screen_size(), native);
    let size = SIZE * zoom;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Sega Rally Revo")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false)
            .with_maximize_button(false),
        centered: !wine,
        ..Default::default()
    };
    let path = settings_path.to_path_buf();
    let saved = saved.map(str::to_string);
    eframe::run_native(
        "Sega Rally Revo",
        options,
        Box::new(|cc| {
            set_style(&cc.egui_ctx);
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

fn set_style(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let font = |bytes: &'static [u8]| std::sync::Arc::new(FontData::from_static(bytes));
    fonts.font_data.insert(
        "barlow".into(),
        font(include_bytes!("../assets/fonts/Barlow-Regular.ttf")),
    );
    fonts.font_data.insert(
        "barlow-semibold".into(),
        font(include_bytes!("../assets/fonts/Barlow-SemiBold.ttf")),
    );
    fonts.font_data.insert(
        "condensed".into(),
        font(include_bytes!(
            "../assets/fonts/BarlowCondensed-SemiBold.ttf"
        )),
    );
    fonts.font_data.insert(
        "decal".into(),
        font(include_bytes!(
            "../assets/fonts/BarlowCondensed-BlackItalic.ttf"
        )),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.insert(family, vec!["barlow".into()]);
    }
    for name in ["barlow-semibold", "condensed", "decal"] {
        fonts.families.insert(
            FontFamily::Name(name.into()),
            vec![name.into(), "barlow".into()],
        );
    }
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        let visuals = &mut style.visuals;
        *visuals = egui::Visuals::dark();
        visuals.panel_fill = BLUE;
        visuals.extreme_bg_color = PANEL;
        visuals.text_edit_bg_color = Some(PANEL);
        visuals.override_text_color = Some(INK);
        visuals.selection.bg_fill = ORANGE.gamma_multiply(0.5);
        visuals.selection.stroke = Stroke::new(1.0, ORANGE);
        visuals.text_cursor.stroke = Stroke::new(2.0, ORANGE);
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.open,
        ] {
            widget.bg_stroke = Stroke::new(1.0, LINE);
            widget.corner_radius = CornerRadius::same(6);
        }
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, MUTED);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, ORANGE);
        style.spacing.item_spacing = vec2(8.0, 8.0);
    });
}

fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

fn named(name: &str, size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(name.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Tab {
    Play,
    Picture,
    Controls,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Play, Tab::Picture, Tab::Controls];

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
    pad_held: u16,
    pad_actions: Vec<PadAction>,
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
        let screen = win32::screen_size();
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
            pads: win32::xinput_pads().len(),
            linux,
            dxvk: linux || win32::is_wine(),
            portrait: load_portrait(ctx),
            path,
            text,
            error: None,
            rumble_test: None,
            pad_held: 0,
            pad_actions: Vec::new(),
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
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&self.path, saved) {
            Ok(()) => {
                *self.outcome = Outcome::Play;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => self.error = Some(format!("Can't save the settings: {error}")),
        }
    }
}

fn load_portrait(ctx: &egui::Context) -> Option<TextureHandle> {
    let mut decoder = jpeg_decoder::Decoder::new(&include_bytes!("../assets/portrait.jpg")[..]);
    let rgb = decoder.decode().ok()?;
    let info = decoder.info()?;
    let size = [usize::from(info.width), usize::from(info.height)];
    (rgb.len() == size[0] * size[1] * 3).then(|| {
        ctx.load_texture(
            "portrait",
            egui::ColorImage::from_rgb(size, &rgb),
            egui::TextureOptions::LINEAR,
        )
    })
}

impl eframe::App for Window<'_> {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        let now = win32::xinput_buttons();
        self.pad_actions = form::pad_actions(self.pad_held, now);
        self.pad_held = now;
        for action in &self.pad_actions {
            let (key, modifiers) = match action {
                PadAction::Next => (Key::Tab, egui::Modifiers::NONE),
                PadAction::Previous => (Key::Tab, egui::Modifiers::SHIFT),
                PadAction::Press => (Key::Space, egui::Modifiers::NONE),
                _ => continue,
            };
            for pressed in [true, false] {
                raw.events.push(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                });
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BLUE);
        self.draw_photo(ui, full);
        draw_decal(ui, full);
        let content = Rect::from_min_max(
            pos2(full.left() + PHOTO_W + 12.0, full.top() + 24.0),
            pos2(full.right() - 32.0, full.bottom() - 24.0),
        );
        let footer_h = 52.0;
        let body = Rect::from_min_max(
            content.min,
            pos2(content.right(), content.bottom() - footer_h - 16.0),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            ui.label(
                egui::RichText::new("Sega Rally Revo")
                    .font(named("decal", 43.0))
                    .color(INK),
            );
            ui.add_space(-6.0);
            self.status(ui);
            ui.add_space(6.0);
            self.tabs(ui);
            ui.add_space(14.0);
            // Tall tabs scroll inside the body instead of running into the footer.
            egui::ScrollArea::vertical()
                .id_salt(self.tab)
                .auto_shrink(false)
                .show(ui, |ui| match self.tab {
                    Tab::Play => self.play_tab(ui),
                    Tab::Picture => self.picture_tab(ui),
                    Tab::Controls => self.controls_tab(ui),
                });
        });
        let footer = Rect::from_min_max(
            pos2(content.left(), content.bottom() - footer_h),
            content.max,
        );
        self.footer(ui, footer, &ctx);
        if ctx.input(|input| input.key_pressed(Key::Enter)) && !ctx.egui_wants_keyboard_input() {
            self.play(&ctx);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
        for action in std::mem::take(&mut self.pad_actions) {
            match action {
                PadAction::Play => self.play(&ctx),
                PadAction::NextTab => self.tab = self.tab.step(1),
                PadAction::PreviousTab => self.tab = self.tab.step(-1),
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
    fn draw_photo(&self, ui: &egui::Ui, full: Rect) {
        let rect = Rect::from_min_size(full.min, vec2(PHOTO_W, full.height()));
        let painter = ui.painter();
        if let Some(texture) = &self.portrait {
            let [width, height] = texture.size();
            let shown_w = rect.height() * width as f32 / height as f32;
            let u = (rect.width() / shown_w).min(1.0);
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(u, 1.0)),
                Color32::WHITE,
            );
        }
        gradient(
            painter,
            Rect::from_min_max(
                pos2(rect.left() + rect.width() * 0.55, rect.top()),
                rect.max,
            ),
            true,
        );
        gradient(
            painter,
            Rect::from_min_max(pos2(rect.left(), rect.bottom() - 130.0), rect.max),
            false,
        );
        let x = rect.left() + 20.0;
        let y = rect.bottom() - 20.0;
        painter.text(
            pos2(x, y),
            Align2::LEFT_BOTTOM,
            "Original artwork made for this launcher",
            font(11.0),
            MUTED,
        );
        painter.text(
            pos2(x, y - 16.0),
            Align2::LEFT_BOTTOM,
            "Sega Racing Studio, 2007",
            font(13.0),
            MUTED,
        );
        painter.text(
            pos2(x, y - 34.0),
            Align2::LEFT_BOTTOM,
            "SEGA RALLY REVO",
            named("barlow-semibold", 19.0),
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
        ui.label(egui::RichText::new(text).font(font(14.0)).color(color));
    }

    fn tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 26.0;
            for (tab, name) in [
                (Tab::Play, "Play"),
                (Tab::Picture, "Picture"),
                (Tab::Controls, "Controls"),
            ] {
                let on = self.tab == tab;
                let galley = ui.painter().layout_no_wrap(
                    name.to_string(),
                    named("condensed", 22.0),
                    if on { INK } else { MUTED },
                );
                let (rect, response) =
                    ui.allocate_exact_size(galley.size() + vec2(0.0, 8.0), Sense::click());
                let color = if on || response.hovered() { INK } else { MUTED };
                ui.painter().galley(rect.min, galley, color);
                if on {
                    let bar = Rect::from_min_size(
                        pos2(rect.left(), rect.bottom() - 3.0),
                        vec2(rect.width(), 3.0),
                    );
                    ui.painter().rect_filled(bar, 0.0, ORANGE);
                }
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.tab = tab;
                }
            }
        });
        let y = ui.cursor().top() - 4.0;
        let rect = ui.max_rect();
        ui.painter()
            .hline(rect.left()..=rect.right(), y, Stroke::new(1.0, LINE));
    }

    fn play_tab(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Game folder (PC v4.0.6.0)");
        let hint = if self.linux {
            "The folder with the game, e.g. Z:\\home\\you\\Games\\Sega Rally"
        } else {
            "The folder with your installed SEGA Rally.exe"
        };
        if folder_field(
            ui,
            &mut self.form.game,
            hint,
            "Choose the folder with SEGA Rally.exe",
        ) {
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
        ui.label(egui::RichText::new(note).font(font(13.0)).color(color));
        ui.add_space(10.0);

        heading(ui, "Look");
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
                if look_card(
                    ui,
                    vec2(width, 116.0),
                    self.form.look == look,
                    title,
                    &note,
                    look == form::recommended(self.screen),
                ) {
                    self.form.set_look(look);
                    if look == Look::Custom {
                        self.tab = Tab::Picture;
                    }
                }
            }
        });
        ui.add_space(14.0);

        heading(ui, "Timing");
        let stutter_fix_disabled = form::stutter_fix_disabled(self.folder_has_pacer());
        ui.horizontal_top(|ui| {
            field(ui, "Stutter fix");
            chips(
                ui,
                &[(true, "On (recommended)"), (false, "Off")],
                &mut self.form.stutter_fix,
                stutter_fix_disabled,
            );
        });
        let folder_lacks_pacer = matches!(self.check, Check::Found { pacer: false, .. });
        if !folder_lacks_pacer {
            ui.label(
                egui::RichText::new(
                    "The community pacer fix is applied to the private copy: it removes the \
                     whole-game microstutter and the stage-2+ championship judder (two stacked \
                     timing bugs in the 2007 build).",
                )
                .font(font(13.0))
                .color(MUTED),
            );
        } else {
            ui.label(
                egui::RichText::new(
                    "The stutter fix needs the clean SEGA Rally_SSE1.exe, which this folder \
                     does not have, so the game runs unpatched.",
                )
                .font(font(13.0))
                .color(ERROR),
            );
        }
        ui.horizontal_top(|ui| {
            field(ui, "Frame limit");
            chips(
                ui,
                &FrameLimit::ALL.map(|limit| (limit, limit.label())),
                &mut self.form.frame_limit,
                form::frame_limit_disabled(self.dxvk),
            );
        });
        ui.label(
            egui::RichText::new(if self.dxvk {
                "The limit runs through DXVK's frame limiter, the community-standard pacing \
                 guard. With vsync on, the display's refresh caps the game too."
            } else {
                "The frame limit needs DXVK (Linux, Wine or Proton). On Windows the game \
                 follows vsync instead."
            })
            .font(font(13.0))
            .color(MUTED),
        );
        ui.add_space(8.0);

        heading(ui, "Language");
        ui.horizontal_top(|ui| {
            field(ui, "Menus");
            let options: Vec<(String, &str)> = LANGUAGES
                .iter()
                .map(|language| ((*language).to_string(), *language))
                .collect();
            chips(ui, &options, &mut self.form.language, None);
        });
    }

    fn picture_tab(&mut self, ui: &mut egui::Ui) {
        let before = self.form.picture;
        let picture = &mut self.form.picture;
        let resolutions = &self.resolutions[..];
        let row = |ui: &mut egui::Ui, name: &str, add: &mut dyn FnMut(&mut egui::Ui)| {
            ui.horizontal_top(|ui| {
                field(ui, name);
                ui.vertical(|ui| add(ui));
            });
        };
        row(ui, "Resolution", &mut |ui| {
            chips(ui, resolutions, &mut picture.resolution, None)
        });
        row(ui, "Aspect", &mut |ui| {
            chips(ui, &form::ASPECTS, &mut picture.aspect, None)
        });
        ui.label(
            egui::RichText::new(
                "21:9 and 32:9 patch the aspect tables of the private copy. 16:9 and 16:10 \
                 need no patch: the game picks between them from the resolution.",
            )
            .font(font(13.0))
            .color(MUTED),
        );
        row(ui, "Quality", &mut |ui| {
            chips(
                ui,
                &Quality::ALL.map(|quality| (quality, quality.label())),
                &mut picture.quality,
                None,
            )
        });
        row(ui, "", &mut |ui| {
            toggle(ui, &mut picture.vsync, "Vsync (60 Hz)");
        });
        if self.form.picture != before {
            self.form.look = Form::look_of(&self.form.picture, self.screen);
        }
    }

    fn controls_tab(&mut self, ui: &mut egui::Ui) {
        heading(ui, "Xbox-style pads");
        let pads = match self.pads {
            0 => "No Xbox-style pad is connected now. Connect one before starting the game."
                .to_string(),
            1 => "One Xbox-style pad connected.".to_string(),
            count => format!("{count} Xbox-style pads connected."),
        };
        ui.label(egui::RichText::new(pads).font(font(15.0)).color(INK));
        ui.label(
            egui::RichText::new(
                "On Linux, srr-play preconfigures Wine's SDL controller bridge for this game \
                 (Map Controllers=1). Use the pad's X-input mode and bind it in the game's \
                 Controls menu.",
            )
            .font(font(13.0))
            .color(MUTED),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "Leave force feedback off in the game's controller options: it is a known \
                 crash (reported on Windows 10) and is not needed to play.",
            )
            .font(font(13.0))
            .color(ORANGE),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if outline_button(ui, "Test rumble").clicked() {
                let took = win32::test_rumble(60);
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
            ui.label(egui::RichText::new(result).font(font(13.0)).color(ORANGE));
        }
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(
                "The launcher window itself supports d-pad navigation, A to change a setting, \
                 LB/RB for tabs, and Start to play.",
            )
            .font(font(13.0))
            .color(MUTED),
        );
    }

    fn footer(&mut self, ui: &mut egui::Ui, rect: Rect, ctx: &egui::Context) {
        let button = Rect::from_min_size(
            pos2(rect.right() - 170.0, rect.top()),
            vec2(170.0, rect.height()),
        );
        let hint = "The source game is never modified. Settings are saved when you press Play.";
        let galley = ui.painter().layout(
            hint.to_string(),
            font(13.0),
            MUTED,
            button.left() - rect.left() - 20.0,
        );
        let y = rect.center().y - galley.size().y / 2.0;
        ui.painter().galley(pos2(rect.left(), y), galley, MUTED);
        let response = ui.interact(button, ui.scope_id().with("play"), Sense::click());
        let ready = self.ready();
        let fill = match (ready, response.hovered()) {
            (false, _) => LINE,
            (true, true) => Color32::from_rgb(0xff, 0xad, 0x33),
            (true, false) => ORANGE,
        };
        let slant = 10.0;
        let plate = vec![
            pos2(button.left() + slant, button.top()),
            button.right_top(),
            pos2(button.right() - slant, button.bottom()),
            button.left_bottom(),
        ];
        ui.painter()
            .add(Shape::convex_polygon(plate, fill, Stroke::NONE));
        ui.painter().text(
            button.center(),
            Align2::CENTER_CENTER,
            "Play",
            named("decal", 30.0),
            if ready { ON_ORANGE } else { MUTED },
        );
        if response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            self.play(ctx);
        }
    }
}

fn gradient(painter: &egui::Painter, rect: Rect, horizontal: bool) {
    let mut mesh = Mesh::default();
    let clear = Color32::from_rgba_unmultiplied(BLUE.r(), BLUE.g(), BLUE.b(), 0);
    let (a, b, c, d) = if horizontal {
        (clear, BLUE, BLUE, clear)
    } else {
        (clear, clear, BLUE, BLUE)
    };
    mesh.colored_vertex(rect.left_top(), a);
    mesh.colored_vertex(rect.right_top(), b);
    mesh.colored_vertex(rect.right_bottom(), c);
    mesh.colored_vertex(rect.left_bottom(), d);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn draw_decal(ui: &mut egui::Ui, full: Rect) {
    let painter = ui.painter().with_clip_rect(full);
    for (index, (width, alpha)) in [(30.0, 1.0), (16.0, 0.65)].into_iter().enumerate() {
        let x = full.right() - 120.0 + index as f32 * 40.0;
        let slant = 60.0;
        let points = vec![
            pos2(x, full.top()),
            pos2(x + width, full.top()),
            pos2(x + width - slant, full.top() + 112.0),
            pos2(x - slant, full.top() + 112.0),
        ];
        painter.add(Shape::convex_polygon(
            points,
            ORANGE.gamma_multiply(alpha),
            Stroke::NONE,
        ));
    }
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(named("barlow-semibold", 14.0))
            .color(MUTED),
    );
}

fn field(ui: &mut egui::Ui, text: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(118.0, 30.0), Sense::hover());
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        text,
        font(15.0),
        INK,
    );
}

fn folder_field(ui: &mut egui::Ui, value: &mut String, hint: &str, title: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let width = ui.available_width() - 100.0;
        let edit = egui::TextEdit::singleline(value)
            .hint_text(egui::RichText::new(hint).color(MUTED))
            .font(font(15.0))
            .margin(vec2(10.0, 8.0))
            .desired_width(width);
        changed |= ui.add(edit).changed();
        if outline_button(ui, "Browse…").clicked()
            && let Some(dir) = win32::pick_folder(title)
        {
            *value = dir.display().to_string();
            changed = true;
        }
    });
    changed
}

fn outline_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(92.0, 34.0), Sense::click());
    let hover = response.hovered();
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        if hover { LINE } else { Color32::TRANSPARENT },
        Stroke::new(1.5, ORANGE),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        named("barlow-semibold", 15.0),
        ORANGE,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A row of choice chips. `disabled` holds the reason when the whole row
/// cannot be changed: the chips then ignore clicks, show the reason on
/// hover, and still outline the saved value so it stays visible.
fn chips<T: PartialEq + Clone, S: AsRef<str>>(
    ui: &mut egui::Ui,
    options: &[(T, S)],
    current: &mut T,
    disabled: Option<&str>,
) {
    let enabled = disabled.is_none();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (value, label) in options {
            let on = *current == *value;
            let text_color = if !enabled {
                MUTED
            } else if on {
                ON_ORANGE
            } else {
                INK
            };
            let galley = ui.painter().layout_no_wrap(
                label.as_ref().to_string(),
                if on {
                    named("barlow-semibold", 14.0)
                } else {
                    font(14.0)
                },
                text_color,
            );
            let size = vec2(galley.size().x + 24.0, 30.0);
            let sense = if enabled {
                Sense::click()
            } else {
                Sense::hover()
            };
            let (rect, response) = ui.allocate_exact_size(size, sense);
            let fill = if !enabled {
                Color32::TRANSPARENT
            } else if on {
                ORANGE
            } else if response.hovered() {
                LINE
            } else {
                Color32::TRANSPARENT
            };
            let stroke = match (enabled, on) {
                (false, true) => ORANGE.gamma_multiply(0.5),
                (false, false) => LINE.gamma_multiply(0.5),
                (true, true) => ORANGE,
                (true, false) => LINE,
            };
            ui.painter().rect(
                rect,
                CornerRadius::same(6),
                fill,
                Stroke::new(1.0, stroke),
                StrokeKind::Inside,
            );
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, INK);
            if let Some(reason) = disabled {
                response.on_hover_text(reason);
            } else if response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                *current = value.clone();
            }
        }
    });
}

fn toggle(ui: &mut egui::Ui, on: &mut bool, text: &str) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font(15.0), INK);
    let size = vec2(46.0 + 12.0 + galley.size().x, 30.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let t = ui.ctx().animate_bool(response.id, *on);
    let track = Rect::from_min_size(pos2(rect.left(), rect.center().y - 13.0), vec2(46.0, 26.0));
    let painter = ui.painter();
    painter.rect(
        track,
        CornerRadius::same(13),
        if *on { ORANGE } else { Color32::TRANSPARENT },
        Stroke::new(1.5, if *on { ORANGE } else { MUTED }),
        StrokeKind::Inside,
    );
    let x = egui::lerp(track.left() + 13.0..=track.right() - 13.0, t);
    painter.circle_filled(
        pos2(x, track.center().y),
        9.0,
        if *on { ON_ORANGE } else { MUTED },
    );
    painter.galley(
        pos2(
            track.right() + 12.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        INK,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand);
}

fn look_card(
    ui: &mut egui::Ui,
    size: Vec2,
    on: bool,
    title: &str,
    note: &str,
    recommended: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    let width = size.x;
    let fill = if on {
        PANEL
    } else if response.hovered() {
        PANEL.gamma_multiply(0.6)
    } else {
        Color32::TRANSPARENT
    };
    painter.rect(
        rect,
        CornerRadius::same(8),
        fill,
        Stroke::new(if on { 2.0 } else { 1.0 }, if on { ORANGE } else { LINE }),
        StrokeKind::Inside,
    );
    let pad = 14.0;
    painter.text(
        rect.left_top() + vec2(pad, pad - 2.0),
        Align2::LEFT_TOP,
        title,
        named("condensed", 22.0),
        if on { ORANGE } else { INK },
    );
    if recommended {
        let galley = painter.layout_no_wrap(
            "Recommended".into(),
            named("barlow-semibold", 12.0),
            ON_ORANGE,
        );
        let tab = Rect::from_min_size(
            pos2(
                rect.right() - pad - galley.size().x - 12.0,
                rect.top() - 9.0,
            ),
            galley.size() + vec2(12.0, 2.0),
        );
        painter.rect_filled(tab, CornerRadius::same(4), ORANGE);
        painter.galley(tab.min + vec2(6.0, 1.0), galley, ON_ORANGE);
    }
    let galley = painter.layout(note.to_string(), font(13.5), MUTED, width - 2.0 * pad);
    painter.galley(rect.left_top() + vec2(pad, pad + 32.0), galley, MUTED);
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}
