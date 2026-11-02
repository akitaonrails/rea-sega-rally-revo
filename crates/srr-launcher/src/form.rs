use crate::config::{self, Aspect, FrameLimit, Quality, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picture {
    pub resolution: Option<(u32, u32)>,
    pub aspect: Option<Aspect>,
    pub quality: Quality,
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Picture {
    pub const MODERN: Picture = Picture {
        resolution: None,
        aspect: None,
        quality: Quality::High,
        fullscreen: true,
        vsync: true,
    };

    pub const STEAM_DECK: Picture = Picture {
        resolution: Some(STEAM_DECK_SCREEN),
        aspect: Some(Aspect::Ratio1610),
        quality: Quality::High,
        fullscreen: true,
        vsync: true,
    };

    pub const ORIGINAL: Picture = Picture {
        resolution: Some((1280, 720)),
        aspect: Some(Aspect::Ratio169),
        quality: Quality::Normal,
        fullscreen: true,
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

pub const STEAM_DECK_SCREEN: (u32, u32) = (1280, 800);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Modern,
    SteamDeck,
    Original,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form {
    pub game: String,
    pub picture: Picture,
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
            fullscreen: settings.fullscreen.unwrap_or(true),
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

    pub fn first_run(screen: Option<(u32, u32)>) -> Form {
        let mut form = Form::new(&Settings::default(), screen);
        form.set_look(Look::Modern, screen);
        form
    }

    pub fn look_of(picture: &Picture, screen: Option<(u32, u32)>) -> Look {
        let picture = picture.resolved(screen);
        if picture == Picture::MODERN.resolved(screen) {
            Look::Modern
        } else if picture == Picture::STEAM_DECK {
            Look::SteamDeck
        } else if picture == Picture::ORIGINAL {
            Look::Original
        } else {
            Look::Custom
        }
    }

    pub fn set_look(&mut self, look: Look, screen: Option<(u32, u32)>) {
        self.look = look;
        match look {
            Look::Modern => self.picture = Picture::MODERN,
            Look::SteamDeck => self.picture = Picture::STEAM_DECK,
            Look::Original => self.picture = Picture::ORIGINAL,
            Look::Custom => {}
        }
        self.look = Form::look_of(&self.picture, screen);
    }

    pub fn save(&self, text: &str, screen: Option<(u32, u32)>) -> String {
        let picture = self.picture.resolved(screen);
        let mut out = config::set_key(text, "game", self.game.trim());
        out = config::set_key(
            &out,
            "resolution",
            &format!(
                "{}x{}",
                picture.resolution.unwrap().0,
                picture.resolution.unwrap().1
            ),
        );
        out = config::set_key(&out, "aspect", picture.aspect.unwrap().key());
        out = config::set_key(&out, "quality", picture.quality.key());
        out = config::set_key(&out, "fullscreen", &picture.fullscreen.to_string());
        out = config::set_key(&out, "vsync", &picture.vsync.to_string());
        out = config::set_key(&out, "frame-limit", self.frame_limit.key());
        out = config::set_key(&out, "stutter-fix", &self.stutter_fix.to_string());
        config::set_key(&out, "language", &self.language)
    }
}

pub const WINDOW: (f32, f32) = (880.0, 640.0);

pub fn window_zoom(screen: Option<(u32, u32)>, native: f32) -> f32 {
    let native = if native > 0.0 { native } else { 1.0 };
    let Some((width, height)) = screen else {
        return 1.0;
    };
    let (width, height) = (width as f32, height as f32);
    let fit = 0.9 * (width / WINDOW.0).min(height / WINDOW.1);
    let target = (width.min(height) / 1440.0).max(1.0).min(fit);
    let zoom = (target / native).max(1.0);
    (zoom * 20.0).round() / 20.0
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadAction {
    Play,
    Next,
    Previous,
    Press,
    NextTab,
    PreviousTab,
}

pub fn pad_actions(previous: u16, current: u16) -> Vec<PadAction> {
    let pressed = current & !previous;
    let mut actions = Vec::new();
    for (mask, action) in [
        (0x1000, PadAction::Play),
        (0x0010, PadAction::PreviousTab),
        (0x0020, PadAction::NextTab),
        (0x0001, PadAction::Next),
        (0x0002, PadAction::Previous),
        (0x0004, PadAction::Press),
        (0x0008, PadAction::Press),
    ] {
        if pressed & mask != 0 {
            actions.push(action);
        }
    }
    actions
}

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
        form.set_look(Look::Original, SCREEN);
        assert_eq!(form.picture.quality, Quality::Normal);
        assert_eq!(form.picture.resolution, Some((1280, 720)));
        form.set_look(Look::SteamDeck, SCREEN);
        assert_eq!(form.picture.aspect, Some(Aspect::Ratio1610));
        assert_eq!(form.picture.resolution, Some(STEAM_DECK_SCREEN));
    }

    #[test]
    fn one_changed_value_is_custom() {
        let mut form = Form::first_run(SCREEN);
        form.picture.quality = Quality::Ultra;
        assert_eq!(Form::look_of(&form.picture, SCREEN), Look::Custom);
    }

    #[test]
    fn pad_buttons_are_edge_triggered() {
        assert_eq!(pad_actions(0, 0x1000), [PadAction::Play]);
        assert!(pad_actions(0x1000, 0x1000).is_empty());
    }
}
