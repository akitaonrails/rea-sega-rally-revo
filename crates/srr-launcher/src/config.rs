use std::path::PathBuf;

use rea_launcher::cfg::{self, ArgError, Rejected};

/// Display aspect ratios. `16:9` and `16:10` exist in the game's own
/// tables; the ultra-wide ratios are patched into the runtime copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    Ratio169,
    Ratio1610,
    Ratio219,
    Ratio329,
}

impl Aspect {
    pub const ALL: [Aspect; 4] = [
        Aspect::Ratio169,
        Aspect::Ratio1610,
        Aspect::Ratio219,
        Aspect::Ratio329,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Aspect::Ratio169 => "16:9",
            Aspect::Ratio1610 => "16:10",
            Aspect::Ratio219 => "21:9",
            Aspect::Ratio329 => "32:9",
        }
    }

    /// The value the renderer should use.
    pub fn ratio(self) -> f32 {
        match self {
            Aspect::Ratio169 => 16.0 / 9.0,
            Aspect::Ratio1610 => 1.6,
            Aspect::Ratio219 => 64.0 / 27.0,
            Aspect::Ratio329 => 32.0 / 9.0,
        }
    }

    fn parse(value: &str) -> Option<Aspect> {
        Aspect::ALL
            .into_iter()
            .find(|aspect| aspect.key() == value.trim())
    }

    pub fn for_resolution(width: u32, height: u32) -> Aspect {
        let ratio = width as f64 / height.max(1) as f64;
        if (ratio - 1.6).abs() < 0.06 {
            Aspect::Ratio1610
        } else if (ratio - 64.0 / 27.0).abs() < 0.12 {
            Aspect::Ratio219
        } else if (ratio - 32.0 / 9.0).abs() < 0.2 {
            Aspect::Ratio329
        } else {
            Aspect::Ratio169
        }
    }
}

/// The game's own graphics preset names. LOWEST and LOW run the 2007
/// hardware profile and are not exposed; Normal is era-typical, High is
/// the configuration proven on the reference 4K setup, Ultra maxes every
/// slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Normal,
    High,
    Ultra,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Normal, Quality::High, Quality::Ultra];

    pub fn key(self) -> &'static str {
        match self {
            Quality::Normal => "normal",
            Quality::High => "high",
            Quality::Ultra => "ultra",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Quality::Normal => "Normal",
            Quality::High => "High",
            Quality::Ultra => "Ultra",
        }
    }

    fn parse(value: &str) -> Option<Quality> {
        Quality::ALL
            .into_iter()
            .find(|quality| quality.key() == value.trim().to_ascii_lowercase())
    }
}

/// DXVK-side frame limiter. The community Lutris setup ships
/// `d3d9.maxFrameRate = 60`; with the pacer fix applied the limit can be
/// lifted or raised on high-refresh displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameLimit {
    Fps60,
    Fps120,
    Off,
}

impl FrameLimit {
    pub const ALL: [FrameLimit; 3] = [FrameLimit::Fps60, FrameLimit::Fps120, FrameLimit::Off];

    pub fn key(self) -> &'static str {
        match self {
            FrameLimit::Fps60 => "60",
            FrameLimit::Fps120 => "120",
            FrameLimit::Off => "off",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FrameLimit::Fps60 => "60 fps",
            FrameLimit::Fps120 => "120 fps",
            FrameLimit::Off => "Off",
        }
    }

    pub fn fps(self) -> Option<u32> {
        match self {
            FrameLimit::Fps60 => Some(60),
            FrameLimit::Fps120 => Some(120),
            FrameLimit::Off => None,
        }
    }

    fn parse(value: &str) -> Option<FrameLimit> {
        FrameLimit::ALL
            .into_iter()
            .find(|limit| limit.key() == value.trim().to_ascii_lowercase())
    }
}

pub const LANGUAGES: [&str; 6] = [
    "English", "French", "German", "Spanish", "Italian", "Polish",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub resolution: Option<(u32, u32)>,
    pub aspect: Option<Aspect>,
    pub quality: Option<Quality>,
    pub vsync: Option<bool>,
    pub frame_limit: Option<FrameLimit>,
    pub stutter_fix: Option<bool>,
    pub language: Option<String>,
}

pub const FILE_NAME: &str = "launcher.cfg";

pub const TEMPLATE: &str = r"# Sega Rally Revo launcher settings.
# key = value; lines starting with # are comments.

# Folder containing SEGA Rally.exe / SEGA Rally_SSE1.exe. Never changed.
game =

# Private writable copy. Default: %LOCALAPPDATA%\SRRLauncher\SegaRally
#runtime =

# Resolution and aspect are filled in by the settings window. Command-line
# runs without these use the current landscape screen, or 1920x1080.
#resolution = 3840x2160
#aspect = 16:9

# The game's own preset: normal, high (default) or ultra.
#quality = high

vsync = true

# Frame limit through DXVK: 60 (default), 120 or off.
#frame-limit = 60

# Community stutter fix for the clean SSE1 build: on (default) or off.
#stutter-fix = on

# Menu language written to the registry: English, French, German, Spanish,
# Italian or Polish.
#language = English
";

pub const USAGE: &str = concat!(
    "SRRLauncher ",
    env!("CARGO_PKG_VERSION"),
    "\n\nSRRLauncher [options]\n\n",
    "  --game <folder>       folder with SEGA Rally.exe or SEGA Rally_SSE1.exe (remembered)\n",
    "  --runtime <folder>    private writable game copy\n",
    "  --resolution <WxH>    e.g. 3840x2160\n",
    "  --aspect <ratio>      16:9, 16:10, 21:9 or 32:9\n",
    "  --quality <preset>    normal, high or ultra\n",
    "  --vsync on|off        wait for the display refresh\n",
    "  --frame-limit <fps>   60, 120 or off\n",
    "  --stutter-fix on|off  community pacer fix on the private copy\n",
    "  --language <name>     English, French, German, Spanish, Italian or Polish\n",
    "  --check               validate the game and exit\n",
    "  --setup               prepare the runtime and settings without playing\n",
    "  --no-window           play with saved settings, no launcher window\n",
    "  --settings            only show the settings window\n",
    "  --linux               running under srr-play/AppImage: install DXVK into the copy\n",
    "  --help                show this help\n\n",
    "Options override launcher.cfg for one run; only --game is remembered."
);

/// The registry spelling of a supported menu language, any case.
fn canonical_language(value: &str) -> Option<&'static str> {
    LANGUAGES
        .into_iter()
        .find(|language| language.eq_ignore_ascii_case(value.trim()))
}

/// `WIDTHxHEIGHT` within 640x480..=7680x4320 (sizes only, no words).
fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    cfg::parse_size(value, (640, 480), (7680, 4320))
}

/// The settings keys, as the command line spells them (`--frame-limit`
/// arrives as `frame_limit`); the file keeps its hyphenated spelling.
const FLAG_KEYS: [&str; 9] = [
    "game",
    "runtime",
    "resolution",
    "aspect",
    "quality",
    "vsync",
    "frame_limit",
    "stutter_fix",
    "language",
];

/// SRR's own messages for a refused command line (shown in a message box).
fn arg_message(error: ArgError) -> String {
    match error {
        ArgError::Help => USAGE.to_string(),
        ArgError::Unknown(flag) => format!("unknown option {flag}\n\n{USAGE}"),
        ArgError::MissingValue(flag) => format!("{flag} needs a value\n\n{USAGE}"),
        ArgError::Bad { flag, value, .. } => match flag.as_str() {
            "--vsync" => format!("invalid vsync value {value}"),
            "--stutter-fix" => format!("invalid stutter-fix value {value}"),
            "--frame-limit" => format!("invalid frame limit {value}\n\n{USAGE}"),
            other => format!(
                "invalid {} {value}\n\n{USAGE}",
                other.trim_start_matches('-')
            ),
        },
    }
}

impl Settings {
    /// Takes one `key = value` (file spelling: `frame-limit`).
    pub fn apply(&mut self, key: &str, value: &str) -> Result<(), Rejected> {
        match key {
            "game" => self.game = Some(value.into()),
            "runtime" => self.runtime = Some(value.into()),
            "resolution" => {
                self.resolution = Some(cfg::need(parse_resolution(value), "WIDTHxHEIGHT")?)
            }
            "aspect" => {
                self.aspect = Some(cfg::need(
                    Aspect::parse(value),
                    "16:9, 16:10, 21:9 or 32:9",
                )?)
            }
            "quality" => {
                self.quality = Some(cfg::need(Quality::parse(value), "normal, high or ultra")?)
            }
            "vsync" => self.vsync = Some(cfg::need(cfg::parse_bool(value), cfg::ON_OFF)?),
            "frame-limit" => {
                self.frame_limit = Some(cfg::need(FrameLimit::parse(value), "60, 120 or off")?)
            }
            "stutter-fix" => {
                self.stutter_fix = Some(cfg::need(cfg::parse_bool(value), cfg::ON_OFF)?)
            }
            "language" => {
                let language = cfg::need(canonical_language(value), "a supported language")?;
                self.language = Some(language.to_string());
            }
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// The settings in a `launcher.cfg` text and a warning per bad line.
    /// Keys this launcher does not know (`look`, removed ones such as
    /// `fullscreen`) are skipped without a warning.
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let warnings = cfg::parse(text, FILE_NAME, |key, value| {
            match settings.apply(key, value) {
                Err(Rejected::Unknown) => Ok(()),
                other => other,
            }
        });
        (settings, warnings)
    }

    /// `--key value` options; the error is the message to show.
    pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Settings, String> {
        let mut settings = Settings::default();
        cfg::parse_args(args, &FLAG_KEYS, &[], |key, value| {
            settings.apply(&key.replace('_', "-"), value)
        })
        .map_err(arg_message)?;
        Ok(settings)
    }

    pub fn merged(self, over: Settings) -> Settings {
        Settings {
            game: over.game.or(self.game),
            runtime: over.runtime.or(self.runtime),
            resolution: over.resolution.or(self.resolution),
            aspect: over.aspect.or(self.aspect),
            quality: over.quality.or(self.quality),
            vsync: over.vsync.or(self.vsync),
            frame_limit: over.frame_limit.or(self.frame_limit),
            stutter_fix: over.stutter_fix.or(self.stutter_fix),
            language: over.language.or(self.language),
        }
    }

    pub fn effective_resolution(&self, screen: Option<(u32, u32)>) -> (u32, u32) {
        self.resolution
            .or_else(|| screen.and_then(landscape))
            .unwrap_or((1920, 1080))
    }

    pub fn effective_aspect(&self, screen: Option<(u32, u32)>) -> Aspect {
        let (width, height) = self.effective_resolution(screen);
        self.aspect
            .unwrap_or_else(|| Aspect::for_resolution(width, height))
    }

    pub fn effective_quality(&self) -> Quality {
        self.quality.unwrap_or(Quality::High)
    }

    pub fn effective_frame_limit(&self) -> FrameLimit {
        self.frame_limit.unwrap_or(FrameLimit::Fps60)
    }

    pub fn effective_stutter_fix(&self) -> bool {
        self.stutter_fix.unwrap_or(true)
    }

    pub fn effective_language(&self) -> String {
        self.language
            .clone()
            .unwrap_or_else(|| "English".to_string())
    }
}

/// A screen size as a landscape resolution worth offering. SRR's rule
/// takes 640x480 itself, unlike the SDK's `display::landscape` (larger
/// than 640x480), so it stays here.
pub fn landscape((a, b): (u32, u32)) -> Option<(u32, u32)> {
    let (width, height) = (a.max(b) & !1, a.min(b) & !1);
    (width >= 640 && height >= 480 && width <= 7680 && height <= 4320).then_some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_recommended_setup() {
        let settings = Settings::default();
        assert_eq!(
            settings.effective_resolution(Some((3840, 2160))),
            (3840, 2160)
        );
        assert_eq!(
            settings.effective_aspect(Some((3840, 2160))),
            Aspect::Ratio169
        );
        assert_eq!(settings.effective_quality(), Quality::High);
        assert_eq!(settings.effective_frame_limit(), FrameLimit::Fps60);
        assert!(settings.effective_stutter_fix());
        assert_eq!(settings.effective_language(), "English");
    }

    #[test]
    fn file_and_cli_merge() {
        let (saved, warnings) =
            Settings::parse_file("game = C:\\Sega Rally\nquality = normal\nvsync = false\n");
        assert!(warnings.is_empty());
        let over = Settings::parse_args([
            "--quality".into(),
            "high".into(),
            "--resolution".into(),
            "2560x1440".into(),
        ])
        .unwrap();
        let merged = saved.merged(over);
        assert_eq!(merged.quality, Some(Quality::High));
        assert_eq!(merged.resolution, Some((2560, 1440)));
        assert_eq!(merged.vsync, Some(false));
    }

    #[test]
    fn invalid_values_warn_or_fail() {
        let (settings, warnings) = Settings::parse_file(
            "resolution = huge\nframe-limit = 7\nlanguage = Klingon\nvsync = maybe\naspect = 5:4\n",
        );
        assert_eq!(warnings.len(), 5);
        assert_eq!(settings.language, None);
        let args = |list: &[&str]| Settings::parse_args(list.iter().map(|a| a.to_string()));
        for bad in [
            &["--frame-limit", "7"][..],
            &["--language", "Klingon"],
            &["--vsync", "maybe"],
            &["--stutter-fix", "sometimes"],
            &["--aspect", "5:4"],
            &["--quality", "insane"],
            &["--resolution", "100x100"],
            &["--fullscreen", "off"],
            &["--bogus"],
            &["--game"],
        ] {
            assert!(args(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn values_are_case_insensitive_and_canonical() {
        let (settings, warnings) =
            Settings::parse_file("language = french\nquality = HIGH\nstutter-fix = off\n");
        assert!(warnings.is_empty());
        assert_eq!(settings.language.as_deref(), Some("French"));
        assert_eq!(settings.quality, Some(Quality::High));
        assert_eq!(settings.stutter_fix, Some(false));
        let cli = Settings::parse_args(["--language".into(), "POLISH".into()]).unwrap();
        assert_eq!(cli.language.as_deref(), Some("Polish"));
    }

    #[test]
    fn removed_keys_in_old_files_are_ignored() {
        let (_, warnings) = Settings::parse_file("fullscreen = false\n");
        assert!(warnings.is_empty());
    }

    #[test]
    fn keys_are_updated_without_losing_comments() {
        let text = "# hello\nquality = normal\nvsync = true\n";
        let text = cfg::set_key(text, "quality", "high");
        assert!(text.contains("# hello\nquality = high\n"));
    }

    #[test]
    fn rotated_screens_become_landscape() {
        assert_eq!(landscape((2160, 3840)), Some((3840, 2160)));
        // 640x480 itself is offered (the SDK's rule starts above it).
        assert_eq!(landscape((480, 640)), Some((640, 480)));
        assert_eq!(landscape((639, 480)), None);
        assert_eq!(Aspect::for_resolution(1280, 800), Aspect::Ratio1610);
        assert_eq!(Aspect::for_resolution(3440, 1440), Aspect::Ratio219);
        assert_eq!(Aspect::for_resolution(3840, 1080), Aspect::Ratio329);
    }

    #[test]
    fn hyphenated_keys_and_their_flags() {
        let (file, warnings) = Settings::parse_file("frame-limit = off\nstutter-fix = off\n");
        assert!(warnings.is_empty());
        assert_eq!(file.frame_limit, Some(FrameLimit::Off));
        assert_eq!(file.stutter_fix, Some(false));
        // the underscore spelling is not a key of this file
        let (other, _) = Settings::parse_file("frame_limit = off\n");
        assert_eq!(other.frame_limit, None);
        let cli = Settings::parse_args(
            ["--frame-limit", "120", "--stutter-fix", "off"].map(String::from),
        )
        .unwrap();
        assert_eq!(cli.frame_limit, Some(FrameLimit::Fps120));
        assert_eq!(cli.stutter_fix, Some(false));
    }

    #[test]
    fn refused_options_keep_their_messages() {
        let args =
            |list: &[&str]| Settings::parse_args(list.iter().map(|a| a.to_string())).unwrap_err();
        assert_eq!(
            args(&["--bogus"]),
            format!("unknown option --bogus\n\n{USAGE}")
        );
        assert_eq!(
            args(&["--game"]),
            format!("--game needs a value\n\n{USAGE}")
        );
        assert_eq!(args(&["--vsync", "maybe"]), "invalid vsync value maybe");
        assert_eq!(args(&["--stutter-fix", "x"]), "invalid stutter-fix value x");
        assert_eq!(
            args(&["--frame-limit", "7"]),
            format!("invalid frame limit 7\n\n{USAGE}")
        );
        assert_eq!(
            args(&["--resolution", "huge"]),
            format!("invalid resolution huge\n\n{USAGE}")
        );
        assert_eq!(
            args(&["--language", "Klingon"]),
            format!("invalid language Klingon\n\n{USAGE}")
        );
    }

    #[test]
    fn frame_limits_map_to_d3d9_values() {
        assert_eq!(FrameLimit::Fps60.fps(), Some(60));
        assert_eq!(FrameLimit::Fps120.fps(), Some(120));
        assert_eq!(FrameLimit::Off.fps(), None);
    }
}
