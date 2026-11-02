//! Headless window tests (egui_kittest through the AccessKit tree): every
//! tab renders, every option has a labelled control that changes what is
//! saved, disabled controls say why, and Play writes `launcher.cfg`.

use std::cell::Cell;
use std::rc::Rc;

use rea_launcher::cfg::spec;
use rea_launcher_ui::egui::accesskit::Role;
use rea_launcher_ui::spec::rendered_keys;
use rea_launcher_ui::testing::{self, Harness, Queryable};

use super::*;
use crate::palette::{METRICS, PALETTE};
use srr_launcher::config::{OPTIONS, Settings};

fn temp(name: &str) -> PathBuf {
    // A hostile path: spaces, an apostrophe, non-ASCII letters.
    let dir = std::env::temp_dir()
        .join(format!("srr-ui-{}-{name}", std::process::id()))
        .join("José")
        .join("Jogos d'Ação");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Fake {
    downloads: Rc<Cell<usize>>,
    rumbles: Rc<Cell<usize>>,
}

fn host(dxvk: bool, fake: &Fake, root: PathBuf) -> Host {
    let downloads = fake.downloads.clone();
    let rumbles = fake.rumbles.clone();
    Host {
        screen: Some((3840, 2160)),
        linux: dxvk,
        dxvk,
        pads: 1,
        pick_folder: Box::new(|| None),
        test_rumble: Box::new(move || {
            rumbles.set(rumbles.get() + 1);
            1
        }),
        buttons: Box::new(|| 0),
        directx_root: root,
        start_download: Box::new(move |_| downloads.set(downloads.get() + 1)),
    }
}

fn open(saved: Option<&str>, dxvk: bool, name: &str) -> (Harness<'static, Window>, Fake, PathBuf) {
    let dir = temp(name);
    let fake = Fake {
        downloads: Rc::new(Cell::new(0)),
        rumbles: Rc::new(Cell::new(0)),
    };
    let window = Window::new(
        dir.join("launcher.cfg"),
        saved.map(str::to_string),
        host(dxvk, &fake, dir.join("directx")),
    );
    let ui = testing::harness(&PALETTE, &METRICS, window, |ui, window: &mut Window| {
        window.draw(ui)
    });
    (ui, fake, dir)
}

/// Scrolls the control with `label` into view, then clicks it.
fn click(ui: &mut Harness<'static, Window>, label: &str) {
    ui.get_by_label(label).scroll_to_me();
    ui.run();
    ui.get_by_label(label).click();
    ui.run();
}

fn tab(ui: &mut Harness<'static, Window>, name: &str) {
    ui.get_by_role_and_label(Role::Tab, name).click();
    ui.run();
}

/// Marks the folder as checked and playable, as a real game folder would.
fn ready(ui: &mut Harness<'static, Window>) {
    let window = ui.state_mut();
    window.form.game = r"D:\Games\Sega Rally".into();
    window.checked = window.form.game.clone();
    window.check = Check::Found {
        names: vec!["PC v4.0.6.0 (SSE1, clean)"],
        pacer: true,
    };
    ui.run();
}

#[test]
fn every_tab_renders_and_the_window_matches_the_option_table() {
    let (mut ui, _, _) = open(None, true, "parity");
    for name in ["Picture", "Controls", "Play"] {
        tab(&mut ui, name);
        assert!(testing::checked(&ui.get_by_role_and_label(Role::Tab, name)));
    }
    spec::assert_parity(OPTIONS, &rendered_keys(&ui.ctx));
}

#[test]
fn every_option_control_is_labelled_and_described() {
    let (mut ui, _, _) = open(None, true, "labels");
    for key in ["stutter_fix", "frame_limit", "language"] {
        let option = config::option(key);
        let row = ui.get_by_role_and_label(Role::RadioGroup, option.label);
        assert_eq!(
            testing::description(&row).as_deref(),
            Some(option.description)
        );
    }
    assert!(ui.query_by_label("Modern").is_some());
    assert!(
        ui.query_by_role_and_label(Role::TextInput, "Game folder: the one with SEGA Rally.exe")
            .is_some()
            || ui
                .query_by_role_and_label(
                    Role::TextInput,
                    "Game folder, e.g. Z:\\home\\you\\Games\\Sega Rally"
                )
                .is_some()
    );
    tab(&mut ui, "Picture");
    for key in ["resolution", "quality"] {
        let option = config::option(key);
        let row = ui.get_by_role_and_label(Role::RadioGroup, option.label);
        assert_eq!(
            testing::description(&row).as_deref(),
            Some(option.description)
        );
    }
    let vsync = ui.get_by_role_and_label(Role::Switch, "Vsync");
    assert_eq!(
        testing::description(&vsync).as_deref(),
        Some(config::option("vsync").description)
    );
    // The removed aspect choice is gone for good.
    assert!(ui.query_by_label("16:10").is_none());
}

#[test]
fn picture_controls_change_the_form_and_make_the_look_custom() {
    let (mut ui, _, _) = open(None, true, "picture");
    tab(&mut ui, "Picture");
    ui.get_by_label("Ultra").click();
    ui.run();
    assert_eq!(ui.state().form.picture.quality, Quality::Ultra);
    assert_eq!(ui.state().form.look, Look::Custom);
    ui.get_by_label("1440p").click();
    ui.run();
    assert_eq!(ui.state().form.picture.resolution, Some((2560, 1440)));
    ui.get_by_role_and_label(Role::Switch, "Vsync").click();
    ui.run();
    assert!(!ui.state().form.picture.vsync);
    assert!(
        ui.query_by_label_contains("Runs at 2560×1440, in the game's own aspect")
            .is_some()
    );
}

#[test]
fn an_ultra_wide_size_from_the_file_is_shown_with_the_fix() {
    let (mut ui, _, _) = open(Some("resolution = 3440x1440\n"), true, "ultra");
    tab(&mut ui, "Picture");
    assert!(testing::checked(&ui.get_by_label("3440×1440")));
    assert!(
        ui.query_by_label_contains("21:9, so the private copy")
            .is_some()
    );
}

#[test]
fn play_tab_controls_change_the_form() {
    let (mut ui, _, _) = open(None, true, "play");
    click(&mut ui, "120 fps");
    assert_eq!(ui.state().form.frame_limit, FrameLimit::Fps120);
    click(&mut ui, "Polish");
    assert_eq!(ui.state().form.language, "Polish");
    click(&mut ui, "Original 2007");
    assert_eq!(ui.state().form.look, Look::Original);
    assert_eq!(ui.state().form.picture.resolution, Some((1280, 720)));
    click(&mut ui, "Custom");
    assert_eq!(ui.state().tab, Tab::Picture);
}

#[test]
fn disabled_controls_say_why() {
    let (ui, _, _) = open(None, false, "disabled");
    let fps = ui.get_by_label("120 fps");
    assert!(testing::disabled(&fps));
    assert!(testing::description(&fps).unwrap().contains("Needs DXVK"));
    let fix = ui.get_by_label("On (recommended)");
    assert!(testing::disabled(&fix));
    assert!(testing::description(&fix).unwrap().contains("game folder"));
    // Play waits for a game folder.
    assert!(testing::disabled(
        &ui.get_by_role_and_label(Role::Button, "Play")
    ));
}

#[test]
fn play_saves_every_value_to_launcher_cfg() {
    let (mut ui, _, dir) = open(None, true, "save");
    ready(&mut ui);
    // The stutter fix's Off (the frame limit has an Off too).
    ui.get_all_by_label("Off").next().unwrap().scroll_to_me();
    ui.run();
    ui.get_all_by_label("Off").next().unwrap().click();
    ui.run();
    ui.get_by_role_and_label(Role::Button, "Play").click();
    ui.run();
    assert_eq!(*ui.state().outcome.lock().unwrap(), Outcome::Play);
    let text = std::fs::read_to_string(dir.join("launcher.cfg")).unwrap();
    let (settings, warnings) = Settings::parse_file(&text);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(settings.game, Some(PathBuf::from(r"D:\Games\Sega Rally")));
    assert_eq!(settings.resolution, Some(Some((3840, 2160))));
    assert_eq!(settings.stutter_fix, Some(false));
    assert_eq!(settings.vsync, Some(true));
    assert_eq!(settings.look, Some(Look::Modern));
    assert!(text.contains("stutter_fix = off"));
}

#[test]
fn the_directx_row_downloads_on_request_only() {
    let (mut ui, fake, _) = open(None, true, "directx");
    assert_eq!(fake.downloads.get(), 0, "nothing without the button");
    assert!(
        ui.query_by_label_contains("Microsoft DirectX 9 files, 100 MB")
            .is_some()
    );
    ui.get_by_role_and_label(Role::Button, "Download")
        .scroll_to_me();
    ui.run();
    ui.get_by_role_and_label(Role::Button, "Download").click();
    ui.run();
    assert_eq!(fake.downloads.get(), 1);
    // The worker would now be connecting: the row offers Cancel.
    assert!(ui.query_by_role_and_label(Role::Button, "Cancel").is_some());
}

#[test]
fn test_rumble_reports_the_result() {
    let (mut ui, fake, _) = open(None, true, "rumble");
    tab(&mut ui, "Controls");
    ui.get_by_role_and_label(Role::Button, "Test rumble")
        .click();
    ui.run();
    assert_eq!(fake.rumbles.get(), 1);
    assert!(
        ui.query_by_label_contains("the pad accepts rumble")
            .is_some()
    );
}

#[test]
fn the_pad_switches_tabs_and_plays() {
    let (mut ui, _, _) = open(None, true, "pad");
    ready(&mut ui);
    let press = |ui: &mut Harness<'static, Window>, buttons: u16| {
        ui.state_mut().host.buttons = Box::new(move || buttons);
        let ctx = ui.ctx.clone();
        let mut raw = rea_launcher_ui::egui::RawInput::default();
        ui.state_mut().pad_input(&ctx, &mut raw);
        ui.run();
        ui.state_mut().host.buttons = Box::new(|| 0);
        let mut raw = rea_launcher_ui::egui::RawInput::default();
        ui.state_mut().pad_input(&ctx, &mut raw);
        ui.run();
    };
    press(&mut ui, rea_launcher::pad::RIGHT_SHOULDER);
    assert_eq!(ui.state().tab, Tab::Picture);
    press(&mut ui, rea_launcher::pad::LEFT_SHOULDER);
    assert_eq!(ui.state().tab, Tab::Play);
    press(&mut ui, rea_launcher::pad::START);
    assert_eq!(*ui.state().outcome.lock().unwrap(), Outcome::Play);
}

#[test]
fn a_wrong_folder_says_how_to_fix_it() {
    let (mut ui, _, dir) = open(None, true, "wrong-folder");
    ui.state_mut().form.game = dir.join("nowhere").display().to_string();
    ui.run();
    ui.run();
    assert!(
        ui.query_by_label_contains("choose the folder with Browse")
            .is_some()
    );
    // An empty folder names both executables and where to report.
    ui.state_mut().form.game = dir.display().to_string();
    ui.run();
    ui.run();
    assert!(
        ui.query_by_label_contains("Neither SEGA Rally.exe")
            .is_some()
    );
    assert!(testing::disabled(
        &ui.get_by_role_and_label(Role::Button, "Play")
    ));
}
