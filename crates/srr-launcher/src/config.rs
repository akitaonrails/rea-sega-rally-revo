use std::path::PathBuf;

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

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    let value = value.trim().to_ascii_lowercase();
    let (width, height) = value.split_once('x')?;
    let width = width.trim().parse().ok()?;
    let height = height.trim().parse().ok()?;
    ((640..=7680).contains(&width) && (480..=4320).contains(&height)).then_some((width, height))
}

fn entry(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (key, value) = line.split_once('=')?;
    let value = value.trim();
    (!value.is_empty()).then_some((key.trim(), value))
}

impl Settings {
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let mut warnings = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let Some((key, value)) = entry(line) else {
                continue;
            };
            let mut bad = || {
                warnings.push(format!(
                    "{FILE_NAME} line {}: invalid {key} {value:?}",
                    index + 1
                ))
            };
            match key {
                "game" => settings.game = Some(value.into()),
                "runtime" => settings.runtime = Some(value.into()),
                "resolution" => match parse_resolution(value) {
                    Some(value) => settings.resolution = Some(value),
                    None => bad(),
                },
                "aspect" => match Aspect::parse(value) {
                    Some(value) => settings.aspect = Some(value),
                    None => bad(),
                },
                "quality" => match Quality::parse(value) {
                    Some(value) => settings.quality = Some(value),
                    None => bad(),
                },
                "vsync" => match parse_bool(value) {
                    Some(value) => settings.vsync = Some(value),
                    None => bad(),
                },
                "frame-limit" => match FrameLimit::parse(value) {
                    Some(value) => settings.frame_limit = Some(value),
                    None => bad(),
                },
                "stutter-fix" => match parse_bool(value) {
                    Some(value) => settings.stutter_fix = Some(value),
                    None => bad(),
                },
                "language" => match canonical_language(value) {
                    Some(language) => settings.language = Some(language.to_string()),
                    None => bad(),
                },
                _ => {}
            }
        }
        (settings, warnings)
    }

    pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Settings, String> {
        let mut settings = Settings::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let mut value = |name: &str| {
                args.next()
                    .ok_or_else(|| format!("{name} needs a value\n\n{USAGE}"))
            };
            match arg.as_str() {
                "--game" => settings.game = Some(value("--game")?.into()),
                "--runtime" => settings.runtime = Some(value("--runtime")?.into()),
                "--resolution" => {
                    let text = value("--resolution")?;
                    settings.resolution = Some(
                        parse_resolution(&text)
                            .ok_or_else(|| format!("invalid resolution {text}\n\n{USAGE}"))?,
                    );
                }
                "--aspect" => {
                    let text = value("--aspect")?;
                    settings.aspect = Some(
                        Aspect::parse(&text)
                            .ok_or_else(|| format!("invalid aspect {text}\n\n{USAGE}"))?,
                    );
                }
                "--quality" => {
                    let text = value("--quality")?;
                    settings.quality = Some(
                        Quality::parse(&text)
                            .ok_or_else(|| format!("invalid quality {text}\n\n{USAGE}"))?,
                    );
                }
                "--vsync" => {
                    let text = value("--vsync")?;
                    settings.vsync = Some(
                        parse_bool(&text).ok_or_else(|| format!("invalid vsync value {text}"))?,
                    );
                }
                "--frame-limit" => {
                    let text = value("--frame-limit")?;
                    settings.frame_limit = Some(
                        FrameLimit::parse(&text)
                            .ok_or_else(|| format!("invalid frame limit {text}\n\n{USAGE}"))?,
                    );
                }
                "--stutter-fix" => {
                    let text = value("--stutter-fix")?;
                    settings.stutter_fix = Some(
                        parse_bool(&text)
                            .ok_or_else(|| format!("invalid stutter-fix value {text}"))?,
                    );
                }
                "--language" => {
                    let text = value("--language")?;
                    let language = canonical_language(&text)
                        .ok_or_else(|| format!("invalid language {text}\n\n{USAGE}"))?;
                    settings.language = Some(language.to_string());
                }
                _ => return Err(format!("unknown option {arg}\n\n{USAGE}")),
            }
        }
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

pub fn landscape((a, b): (u32, u32)) -> Option<(u32, u32)> {
    let (width, height) = (a.max(b) & !1, a.min(b) & !1);
    (width >= 640 && height >= 480 && width <= 7680 && height <= 4320).then_some((width, height))
}

pub fn set_key(text: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            if entry(line).is_some_and(|(candidate, _)| candidate == key) {
                found = true;
                format!("{key} = {value}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(format!("{key} = {value}"));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
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
        let text = set_key(text, "quality", "high");
        assert!(text.contains("# hello\nquality = high\n"));
    }

    #[test]
    fn rotated_screens_become_landscape() {
        assert_eq!(landscape((2160, 3840)), Some((3840, 2160)));
        assert_eq!(Aspect::for_resolution(1280, 800), Aspect::Ratio1610);
        assert_eq!(Aspect::for_resolution(3440, 1440), Aspect::Ratio219);
        assert_eq!(Aspect::for_resolution(3840, 1080), Aspect::Ratio329);
    }

    #[test]
    fn frame_limits_map_to_d3d9_values() {
        assert_eq!(FrameLimit::Fps60.fps(), Some(60));
        assert_eq!(FrameLimit::Fps120.fps(), Some(120));
        assert_eq!(FrameLimit::Off.fps(), None);
    }
}
