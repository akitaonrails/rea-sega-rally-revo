//! The settings window's state, independent of egui: what each control
//! holds, the looks, and how it is saved to `launcher.cfg`.

use rea_launcher::cfg::{self, spec};
pub use rea_launcher::display::STEAM_DECK_SCREEN;

pub use crate::config::Look;
use crate::config::{self, DisplayMode, FrameLimit, Quality, Settings};

/// The picture values a look sets; `resolution: None` is Auto (the
/// screen's size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picture {
    pub resolution: Option<(u32, u32)>,
    pub quality: Quality,
    pub vsync: bool,
}

impl Picture {
    fn of(look: Look) -> Option<Picture> {
        look.preset().map(|preset| Picture {
            resolution: preset.resolution,
            quality: preset.quality,
            vsync: preset.vsync,
        })
    }

    /// The values with Auto worked out for `screen`.
    pub fn resolved(self, screen: Option<(u32, u32)>) -> Picture {
        Picture {
            resolution: Some(config::resolve_resolution(self.resolution, screen)),
            ..self
        }
    }
}

/// The look to recommend on a screen: Steam Deck on a Deck, else Modern.
pub fn recommended(screen: Option<(u32, u32)>) -> Look {
    if screen.and_then(config::landscape) == Some(STEAM_DECK_SCREEN) {
        Look::SteamDeck
    } else {
        Look::Modern
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub picture: Picture,
    /// Remembered so a look picked by name stays picked even when its
    /// values match another (Modern on a Deck is the Steam Deck picture),
    /// and Custom stays selected while the player edits.
    pub look: Look,
    pub frame_limit: FrameLimit,
    pub stutter_fix: bool,
    pub rumble: bool,
    pub display_mode: DisplayMode,
    /// The `keys` text as typed (checked by `keys::KeyMap::parse`).
    pub keys: String,
    pub language: String,
}

impl Form {
    pub fn new(settings: &Settings, screen: Option<(u32, u32)>) -> Form {
        let picture = Picture {
            resolution: settings.resolution.flatten(),
            quality: settings.effective_quality(),
            vsync: settings.effective_vsync(),
        };
        Form {
            game: settings
                .game
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            look: Form::look_of(&picture, screen),
            picture,
            frame_limit: settings.effective_frame_limit(),
            stutter_fix: settings.effective_stutter_fix(),
            rumble: settings.effective_rumble(),
            display_mode: settings.effective_display_mode(),
            keys: settings.effective_keys().text(),
            language: settings.effective_language().to_string(),
        }
    }

    /// The form for a first run (no settings file yet): the recommended
    /// look for this screen.
    pub fn first_run(screen: Option<(u32, u32)>) -> Form {
        let mut form = Form::new(&Settings::default(), screen);
        form.set_look(recommended(screen));
        form
    }

    /// The form for a settings file's text: the recommended look when there
    /// is no file yet (or it is blank), otherwise what the file says. A file
    /// that happens to match the template word for word is the player's,
    /// not a first run.
    ///
    /// The look the window saved wins while the values still make it
    /// (Custom always does): on a Deck, Modern and Steam Deck have the same
    /// values and only the saved name tells them apart. Without one, or
    /// after a hand edit, the look is worked out from the values.
    pub fn from_file(saved: Option<&str>, screen: Option<(u32, u32)>) -> Form {
        let Some(text) = saved.filter(|text| !text.trim().is_empty()) else {
            return Form::first_run(screen);
        };
        let settings = Settings::parse_file(text).0;
        let mut form = Form::new(&settings, screen);
        if let Some(look) = settings.look {
            let fits = Picture::of(look)
                .is_none_or(|preset| preset.resolved(screen) == form.picture.resolved(screen));
            if fits {
                form.look = look;
            }
        }
        form
    }

    /// The look these picture values make on a screen. The Steam Deck
    /// preset is checked first: on a 1280×800 screen Modern resolves to the
    /// same picture, and there the Deck is the one to show.
    pub fn look_of(picture: &Picture, screen: Option<(u32, u32)>) -> Look {
        let picture = picture.resolved(screen);
        [Look::SteamDeck, Look::Modern, Look::Original]
            .into_iter()
            .find(|look| Picture::of(*look).map(|p| p.resolved(screen)) == Some(picture))
            .unwrap_or(Look::Custom)
    }

    /// Picks a look; Modern, Steam Deck and Original set the picture,
    /// Custom keeps the current values to edit. The chosen look is kept as
    /// is, never worked out again from the values.
    pub fn set_look(&mut self, look: Look) {
        self.look = look;
        if let Some(picture) = Picture::of(look) {
            self.picture = picture;
        }
    }

    /// Changes the picture values and works out the look again.
    pub fn edit_picture(&mut self, picture: Picture, screen: Option<(u32, u32)>) {
        if picture != self.picture {
            self.picture = picture;
            self.look = Form::look_of(&picture, screen);
        }
    }

    /// The settings file with every window value written (Auto saved as
    /// the size it stands for on this screen).
    pub fn save(&self, text: &str, screen: Option<(u32, u32)>) -> String {
        let picture = self.picture.resolved(screen);
        let (width, height) = picture.resolution.expect("resolved");
        let values = [
            ("game", self.game.trim().to_string()),
            ("look", self.look.key().to_string()),
            ("resolution", format!("{width}x{height}")),
            ("quality", picture.quality.key().to_string()),
            ("vsync", cfg::on_off(picture.vsync).to_string()),
            ("frame_limit", self.frame_limit.key().to_string()),
            ("stutter_fix", cfg::on_off(self.stutter_fix).to_string()),
            ("rumble", cfg::on_off(self.rumble).to_string()),
            ("display_mode", self.display_mode.key().to_string()),
            ("keys", self.keys.trim().to_string()),
            ("language", self.language.clone()),
        ];
        let text = spec::retire_removed(text, config::OPTIONS);
        values
            .iter()
            .fold(text, |out, (key, value)| config::set_key(&out, key, value))
    }
}

/// Why the stutter-fix chips are disabled, if they are: `folder` is
/// `None` before a usable game folder is chosen, else whether it holds the
/// clean SSE1 exe the pacer patch needs. The SDK chips write it under the
/// row (a pad player never hovers); the row's description already says
/// what the fix needs, so this says what it means for this folder.
pub fn stutter_fix_disabled(folder: Option<bool>) -> Option<&'static str> {
    match folder {
        None => Some("Choose the game folder first."),
        Some(false) => Some(
            "This folder has no clean SEGA Rally_SSE1.exe, so the game runs without the \
             stutter fix.",
        ),
        Some(true) => None,
    }
}

/// Why the frame-limit chips are disabled, if they are.
pub fn frame_limit_disabled(dxvk: bool) -> Option<&'static str> {
    (!dxvk).then_some("Needs DXVK: Linux, Wine or Proton. On Windows the game follows vsync.")
}

/// The resolution chips: Auto, the screen's own size, 1080p, 1440p, 4K,
/// and the current value when it is none of those (a size set in the
/// file or on the command line stays visible and selected).
pub fn resolutions(
    screen: Option<(u32, u32)>,
    current: Option<(u32, u32)>,
) -> Vec<(Option<(u32, u32)>, String)> {
    let common = [(1920, 1080), (2560, 1440), (3840, 2160)];
    let mut out = vec![(None, "Auto".to_string())];
    let screen = screen.and_then(config::landscape);
    if let Some(size) = screen
        && !common.contains(&size)
    {
        out.push((Some(size), format!("Screen {}×{}", size.0, size.1)));
    }
    for (width, height) in common {
        let label = match height {
            1080 => "1080p",
            1440 => "1440p",
            _ => "4K",
        };
        out.push((Some((width, height)), label.to_string()));
    }
    if let Some(size) = current
        && !out.iter().any(|(value, _)| *value == Some(size))
    {
        out.push((Some(size), format!("{}×{}", size.0, size.1)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Option<(u32, u32)> = Some((3840, 2160));
    const DECK: Option<(u32, u32)> = Some(STEAM_DECK_SCREEN);

    fn template() -> String {
        config::template()
    }

    #[test]
    fn first_run_is_modern_4k() {
        let form = Form::first_run(SCREEN);
        assert_eq!(form.look, Look::Modern);
        assert_eq!(form.picture.resolved(SCREEN).resolution, Some((3840, 2160)));
        assert_eq!(form.picture.quality, Quality::High);
        assert_eq!(form.frame_limit, FrameLimit::Fps60);
        assert!(form.stutter_fix);
    }

    #[test]
    fn saved_modern_settings_remain_modern() {
        let form = Form::first_run(SCREEN);
        let text = form.save(&template(), SCREEN);
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(Form::new(&settings, SCREEN).look, Look::Modern);
        for line in [
            "resolution = 3840x2160",
            "frame_limit = 60",
            "stutter_fix = on",
            "vsync = on",
            "look = modern",
        ] {
            assert!(text.contains(line), "{line}");
        }
        assert!(!text.contains("aspect ="));
    }

    #[test]
    fn original_and_deck_are_distinct() {
        let mut form = Form::first_run(SCREEN);
        form.set_look(Look::Original);
        assert_eq!(form.picture.quality, Quality::Normal);
        assert_eq!(form.picture.resolution, Some((1280, 720)));
        form.set_look(Look::SteamDeck);
        assert_eq!(form.picture.resolution, Some(STEAM_DECK_SCREEN));
    }

    #[test]
    fn every_field_survives_save_and_load() {
        let mut form = Form::first_run(SCREEN);
        form.game = r"D:\Games\Sega Rally".into();
        form.set_look(Look::SteamDeck);
        form.edit_picture(
            Picture {
                vsync: false,
                ..form.picture
            },
            SCREEN,
        );
        form.frame_limit = FrameLimit::Off;
        form.stutter_fix = false;
        form.language = "Polish".into();
        let (settings, warnings) = Settings::parse_file(&form.save(&template(), SCREEN));
        assert!(warnings.is_empty(), "{warnings:?}");
        let loaded = Form::new(&settings, SCREEN);
        assert_eq!(loaded.game, form.game);
        assert_eq!(loaded.picture, form.picture.resolved(SCREEN));
        assert_eq!(loaded.frame_limit, FrameLimit::Off);
        assert!(!loaded.stutter_fix);
        assert_eq!(loaded.language, "Polish");
        // vsync off makes it a custom look, not the Steam Deck preset
        assert_eq!(loaded.look, Look::Custom);
        assert_eq!(form.look, Look::Custom);
    }

    #[test]
    fn only_a_missing_or_blank_file_is_a_first_run() {
        let first = Form::first_run(DECK);
        assert_eq!(Form::from_file(None, DECK), first);
        assert_eq!(Form::from_file(Some(""), DECK), first);
        assert_eq!(Form::from_file(Some(" \r\n"), DECK), first);
        assert_eq!(first.look, Look::SteamDeck);
        // The template saved word for word is the player's file: its values
        // decide, so the Deck preset is not forced onto it.
        let kept = Form::from_file(Some(&template()), DECK);
        assert_eq!(kept, Form::new(&Settings::parse_file(&template()).0, DECK));
        // A file holding only the game folder (written by --game) too.
        let with_game = config::set_key(&template(), "game", r"D:\Sega Rally");
        assert_eq!(
            Form::from_file(Some(&with_game), DECK).game,
            r"D:\Sega Rally"
        );
    }

    #[test]
    fn a_saved_look_reopens_as_itself_while_its_values_fit() {
        for look in Look::ALL {
            let mut form = Form::first_run(DECK);
            form.set_look(look);
            let text = form.save(&template(), DECK);
            assert!(text.contains(&format!("look = {}", look.key())));
            assert_eq!(Form::from_file(Some(&text), DECK).look, look, "{look:?}");
            assert_eq!(Look::parse(look.key()), Some(look));
        }
        // A hand edit that breaks the preset falls back to the values.
        let mut form = Form::first_run(DECK);
        form.set_look(Look::Modern);
        let edited = config::set_key(&form.save(&template(), DECK), "quality", "ultra");
        assert_eq!(Form::from_file(Some(&edited), DECK).look, Look::Custom);
        // An unknown look name is ignored (with a warning in the log).
        let odd = config::set_key(&form.save(&template(), DECK), "look", "fancy");
        assert_eq!(Form::from_file(Some(&odd), DECK).look, Look::SteamDeck);
    }

    #[test]
    fn custom_stays_selected_when_picked() {
        let mut form = Form::first_run(SCREEN);
        form.set_look(Look::Custom);
        assert_eq!(form.look, Look::Custom);
        assert_eq!(form.picture, Form::first_run(SCREEN).picture);
    }

    #[test]
    fn a_deck_recommends_and_shows_the_steam_deck_look() {
        assert_eq!(recommended(DECK), Look::SteamDeck);
        // Gamescope can report the Deck's panel in portrait.
        assert_eq!(recommended(Some((800, 1280))), Look::SteamDeck);
        assert_eq!(recommended(SCREEN), Look::Modern);
        assert_eq!(recommended(None), Look::Modern);

        let form = Form::first_run(DECK);
        assert_eq!(form.look, Look::SteamDeck);

        // Picking Steam Deck on a Deck stays Steam Deck (its values equal
        // Modern there) and so does reloading what it saved.
        let mut form = Form::first_run(DECK);
        form.set_look(Look::Modern);
        assert_eq!(form.look, Look::Modern);
        form.set_look(Look::SteamDeck);
        assert_eq!(form.look, Look::SteamDeck);
        let (settings, _) = Settings::parse_file(&form.save(&template(), DECK));
        assert_eq!(Form::new(&settings, DECK).look, Look::SteamDeck);
    }

    #[test]
    fn one_changed_value_is_custom() {
        let mut form = Form::first_run(SCREEN);
        let picture = Picture {
            quality: Quality::Ultra,
            ..form.picture
        };
        form.edit_picture(picture, SCREEN);
        assert_eq!(form.look, Look::Custom);
    }

    #[test]
    fn disabled_chips_say_why() {
        assert!(stutter_fix_disabled(None).unwrap().contains("game folder"));
        assert!(
            stutter_fix_disabled(Some(false))
                .unwrap()
                .contains("SEGA Rally_SSE1.exe")
        );
        assert_eq!(stutter_fix_disabled(Some(true)), None);
        assert!(frame_limit_disabled(false).unwrap().contains("DXVK"));
        assert_eq!(frame_limit_disabled(true), None);
    }

    #[test]
    fn a_size_from_the_file_gets_its_own_chip() {
        let chips = resolutions(SCREEN, None);
        assert_eq!(
            chips
                .iter()
                .map(|(_, label)| label.as_str())
                .collect::<Vec<_>>(),
            ["Auto", "1080p", "1440p", "4K"]
        );
        let chips = resolutions(Some((3440, 1440)), Some((2560, 1080)));
        assert_eq!(
            chips
                .iter()
                .map(|(_, label)| label.as_str())
                .collect::<Vec<_>>(),
            [
                "Auto",
                "Screen 3440×1440",
                "1080p",
                "1440p",
                "4K",
                "2560×1080"
            ]
        );
        assert_eq!(resolutions(SCREEN, Some((1920, 1080))).len(), 4);
    }
}
