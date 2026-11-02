use rea_launcher::cfg;
pub use rea_launcher::display::STEAM_DECK_SCREEN;

use crate::config::{self, Aspect, FrameLimit, Quality, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picture {
    pub resolution: Option<(u32, u32)>,
    pub aspect: Option<Aspect>,
    pub quality: Quality,
    pub vsync: bool,
}

impl Picture {
    pub const MODERN: Picture = Picture {
        resolution: None,
        aspect: None,
        quality: Quality::High,
        vsync: true,
    };

    pub const STEAM_DECK: Picture = Picture {
        resolution: Some(STEAM_DECK_SCREEN),
        aspect: Some(Aspect::Ratio1610),
        quality: Quality::High,
        vsync: true,
    };

    pub const ORIGINAL: Picture = Picture {
        resolution: Some((1280, 720)),
        aspect: Some(Aspect::Ratio169),
        quality: Quality::Normal,
        vsync: true,
    };

    pub fn resolved(self, screen: Option<(u32, u32)>) -> Picture {
        let settings = Settings {
            resolution: self.resolution,
            aspect: self.aspect,
            ..Settings::default()
        };
        Picture {
            resolution: Some(settings.effective_resolution(screen)),
            aspect: Some(settings.effective_aspect(screen)),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Modern,
    SteamDeck,
    Original,
    Custom,
}

impl Look {
    /// The `look` key the window saves in launcher.cfg (window memory
    /// only; the game and the command line never read it).
    pub fn key(self) -> &'static str {
        match self {
            Look::Modern => "modern",
            Look::SteamDeck => "steam-deck",
            Look::Original => "original",
            Look::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Look> {
        [Look::Modern, Look::SteamDeck, Look::Original, Look::Custom]
            .into_iter()
            .find(|look| look.key().eq_ignore_ascii_case(value.trim()))
    }

    /// The preset's picture, `None` for Custom.
    fn picture(self) -> Option<Picture> {
        match self {
            Look::Modern => Some(Picture::MODERN),
            Look::SteamDeck => Some(Picture::STEAM_DECK),
            Look::Original => Some(Picture::ORIGINAL),
            Look::Custom => None,
        }
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
    pub language: String,
}

impl Form {
    pub fn new(settings: &Settings, screen: Option<(u32, u32)>) -> Form {
        let picture = Picture {
            resolution: settings.resolution,
            aspect: settings.aspect,
            quality: settings.effective_quality(),
            vsync: settings.vsync.unwrap_or(true),
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
            language: settings.effective_language(),
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
        let mut form = Form::new(&Settings::parse_file(text).0, screen);
        let remembered = cfg::entries(text)
            .filter(|entry| entry.key == "look")
            .filter_map(|entry| Look::parse(entry.value))
            .last();
        if let Some(look) = remembered {
            let fits = look
                .picture()
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
        if picture == Picture::STEAM_DECK {
            Look::SteamDeck
        } else if picture == Picture::MODERN.resolved(screen) {
            Look::Modern
        } else if picture == Picture::ORIGINAL {
            Look::Original
        } else {
            Look::Custom
        }
    }

    /// Picks a look; Modern, Steam Deck and Original set the picture,
    /// Custom keeps the current values to edit. The chosen look is kept as
    /// is, never worked out again from the values.
    pub fn set_look(&mut self, look: Look) {
        self.look = look;
        match look {
            Look::Modern => self.picture = Picture::MODERN,
            Look::SteamDeck => self.picture = Picture::STEAM_DECK,
            Look::Original => self.picture = Picture::ORIGINAL,
            Look::Custom => {}
        }
    }

    pub fn save(&self, text: &str, screen: Option<(u32, u32)>) -> String {
        let picture = self.picture.resolved(screen);
        let mut out = cfg::set_key(text, "game", self.game.trim());
        out = cfg::set_key(
            &out,
            "resolution",
            &format!(
                "{}x{}",
                picture.resolution.unwrap().0,
                picture.resolution.unwrap().1
            ),
        );
        out = cfg::set_key(&out, "aspect", picture.aspect.unwrap().key());
        out = cfg::set_key(&out, "quality", picture.quality.key());
        out = cfg::set_key(&out, "vsync", &picture.vsync.to_string());
        out = cfg::set_key(&out, "frame-limit", self.frame_limit.key());
        out = cfg::set_key(&out, "stutter-fix", &self.stutter_fix.to_string());
        out = cfg::set_key(&out, "language", &self.language);
        cfg::set_key(&out, "look", self.look.key())
    }
}

/// Why the stutter-fix chips are disabled, if they are: `folder` is
/// `None` before a usable game folder is chosen, else whether it holds the
/// clean SSE1 exe the pacer patch needs.
pub fn stutter_fix_disabled(folder: Option<bool>) -> Option<&'static str> {
    match folder {
        None => Some("Choose the game folder first."),
        Some(false) => Some("Needs the clean SEGA Rally_SSE1.exe in the game folder."),
        Some(true) => None,
    }
}

/// Why the frame-limit chips are disabled, if they are.
pub fn frame_limit_disabled(dxvk: bool) -> Option<&'static str> {
    (!dxvk).then_some("Needs DXVK: Linux, Wine or Proton. On Windows the game follows vsync.")
}

pub fn resolutions(screen: Option<(u32, u32)>) -> Vec<(Option<(u32, u32)>, String)> {
    let common = [(1920, 1080), (2560, 1440), (3840, 2160)];
    let mut out = vec![(None, "Auto".to_string())];
    if let Some(size) = screen.and_then(config::landscape)
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
    out
}

pub const ASPECTS: [(Option<Aspect>, &str); 5] = [
    (None, "Auto"),
    (Some(Aspect::Ratio169), "16:9"),
    (Some(Aspect::Ratio1610), "16:10"),
    (Some(Aspect::Ratio219), "21:9"),
    (Some(Aspect::Ratio329), "32:9"),
];

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Option<(u32, u32)> = Some((3840, 2160));

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
        let text = form.save(config::TEMPLATE, SCREEN);
        let (settings, _) = Settings::parse_file(&text);
        assert_eq!(Form::new(&settings, SCREEN).look, Look::Modern);
        assert!(text.contains("resolution = 3840x2160"));
        assert!(text.contains("aspect = 16:9"));
        assert!(text.contains("frame-limit = 60"));
        assert!(text.contains("stutter-fix = true"));
    }

    #[test]
    fn original_and_deck_are_distinct() {
        let mut form = Form::first_run(SCREEN);
        form.set_look(Look::Original);
        assert_eq!(form.picture.quality, Quality::Normal);
        assert_eq!(form.picture.resolution, Some((1280, 720)));
        form.set_look(Look::SteamDeck);
        assert_eq!(form.picture.aspect, Some(Aspect::Ratio1610));
        assert_eq!(form.picture.resolution, Some(STEAM_DECK_SCREEN));
    }

    #[test]
    fn every_field_survives_save_and_load() {
        let mut form = Form::first_run(SCREEN);
        form.game = r"D:\Games\Sega Rally".into();
        form.set_look(Look::SteamDeck);
        form.picture.vsync = false;
        form.frame_limit = FrameLimit::Off;
        form.stutter_fix = false;
        form.language = "Polish".into();
        let (settings, warnings) = Settings::parse_file(&form.save(config::TEMPLATE, SCREEN));
        assert!(warnings.is_empty(), "{warnings:?}");
        let loaded = Form::new(&settings, SCREEN);
        assert_eq!(loaded.game, form.game);
        assert_eq!(loaded.picture, form.picture.resolved(SCREEN));
        assert_eq!(loaded.frame_limit, FrameLimit::Off);
        assert!(!loaded.stutter_fix);
        assert_eq!(loaded.language, "Polish");
        // vsync off makes it a custom look, not the Steam Deck preset
        assert_eq!(loaded.look, Look::Custom);
    }

    const DECK: Option<(u32, u32)> = Some(STEAM_DECK_SCREEN);

    #[test]
    fn only_a_missing_or_blank_file_is_a_first_run() {
        let deck = Some(STEAM_DECK_SCREEN);
        let first = Form::first_run(deck);
        assert_eq!(Form::from_file(None, deck), first);
        assert_eq!(Form::from_file(Some(""), deck), first);
        assert_eq!(Form::from_file(Some(" \r\n"), deck), first);
        assert_eq!(first.look, Look::SteamDeck);
        // The template saved word for word is the player's file: its values
        // decide, so the Deck preset is not forced onto it.
        let kept = Form::from_file(Some(config::TEMPLATE), deck);
        assert_eq!(
            kept,
            Form::new(&Settings::parse_file(config::TEMPLATE).0, deck)
        );
        // A file holding only the game folder (written by --game) too.
        let with_game = cfg::set_key(config::TEMPLATE, "game", r"D:\Sega Rally");
        assert_eq!(
            Form::from_file(Some(&with_game), deck).game,
            r"D:\Sega Rally"
        );
    }

    #[test]
    fn a_saved_look_reopens_as_itself_while_its_values_fit() {
        let deck = Some(STEAM_DECK_SCREEN);
        for look in [Look::Modern, Look::SteamDeck, Look::Original, Look::Custom] {
            let mut form = Form::first_run(deck);
            form.set_look(look);
            let text = form.save(config::TEMPLATE, deck);
            assert!(text.contains(&format!("look = {}", look.key())));
            assert_eq!(Form::from_file(Some(&text), deck).look, look, "{look:?}");
            assert_eq!(Look::parse(look.key()), Some(look));
        }
        // A hand edit that breaks the preset falls back to the values.
        let mut form = Form::first_run(deck);
        form.set_look(Look::Modern);
        let edited = cfg::set_key(&form.save(config::TEMPLATE, deck), "quality", "ultra");
        assert_eq!(Form::from_file(Some(&edited), deck).look, Look::Custom);
        // An unknown look name is ignored, and is not a settings warning.
        let odd = cfg::set_key(&form.save(config::TEMPLATE, deck), "look", "fancy");
        assert_eq!(Form::from_file(Some(&odd), deck).look, Look::SteamDeck);
        assert!(Settings::parse_file(&odd).1.is_empty());
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
        assert_eq!(form.picture, Picture::STEAM_DECK);

        // Picking Steam Deck on a Deck stays Steam Deck (its values equal
        // Modern there) and so does reloading what it saved.
        let mut form = Form::first_run(DECK);
        form.set_look(Look::Modern);
        form.set_look(Look::SteamDeck);
        assert_eq!(form.look, Look::SteamDeck);
        let (settings, _) = Settings::parse_file(&form.save(config::TEMPLATE, DECK));
        assert_eq!(Form::new(&settings, DECK).look, Look::SteamDeck);
    }

    #[test]
    fn a_picked_look_is_remembered_even_when_values_match_another() {
        let mut form = Form::first_run(DECK);
        form.set_look(Look::Modern);
        assert_eq!(form.look, Look::Modern);
    }

    #[test]
    fn one_changed_value_is_custom() {
        let mut form = Form::first_run(SCREEN);
        form.picture.quality = Quality::Ultra;
        assert_eq!(Form::look_of(&form.picture, SCREEN), Look::Custom);
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
}
