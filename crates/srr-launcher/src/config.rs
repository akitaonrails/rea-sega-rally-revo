//! `launcher.cfg` and the command line, from one option table
//! ([`OPTIONS`], the SDK's `cfg::spec`): `--help`, the template, file and
//! flag parsing, and the window's labels and descriptions all come from it.

use std::path::PathBuf;

use rea_launcher::cfg::{
    self, ArgError, KeyStyle, Rejected, Resolution,
    spec::{self, Kind, OptionSpec, Value},
};

/// The aspect the renderer gets. It follows the resolution: 16:9 and
/// 16:10 are in the game's own tables; ultra-wide sizes get the aspect
/// fix in the private copy (`patches::aspect_set`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    /// 16:9, 16:10 and anything narrower: the game's own tables.
    Native,
    /// 21:9 (64:27) screens such as 2560x1080 and 3440x1440.
    Ultra21,
    /// 32:9 screens such as 3840x1080 and 5120x1440.
    Ultra32,
}

impl Aspect {
    pub fn for_resolution((width, height): (u32, u32)) -> Aspect {
        let ratio = width as f64 / height.max(1) as f64;
        if (ratio - 32.0 / 9.0).abs() < 0.2 {
            Aspect::Ultra32
        } else if (ratio - 64.0 / 27.0).abs() < 0.12 {
            Aspect::Ultra21
        } else {
            Aspect::Native
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Aspect::Native => "native",
            Aspect::Ultra21 => "21:9",
            Aspect::Ultra32 => "32:9",
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

    pub fn parse(value: &str) -> Option<Quality> {
        Quality::ALL
            .into_iter()
            .find(|quality| quality.key().eq_ignore_ascii_case(value.trim()))
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

    pub fn parse(value: &str) -> Option<FrameLimit> {
        FrameLimit::ALL
            .into_iter()
            .find(|limit| limit.key().eq_ignore_ascii_case(value.trim()))
    }
}

/// A look: a named set of picture values (resolution, quality, vsync).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    Modern,
    SteamDeck,
    Original,
    Custom,
}

/// A look's picture values; `resolution: None` is the screen's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    pub resolution: Option<(u32, u32)>,
    pub quality: Quality,
    pub vsync: bool,
}

impl Look {
    pub const ALL: [Look; 4] = [Look::Modern, Look::SteamDeck, Look::Original, Look::Custom];

    pub fn key(self) -> &'static str {
        match self {
            Look::Modern => "modern",
            Look::SteamDeck => "steam-deck",
            Look::Original => "original",
            Look::Custom => "custom",
        }
    }

    pub fn parse(value: &str) -> Option<Look> {
        Look::ALL
            .into_iter()
            .find(|look| look.key().eq_ignore_ascii_case(value.trim()))
    }

    /// The look's values, `None` for Custom.
    pub fn preset(self) -> Option<Preset> {
        match self {
            Look::Modern => Some(Preset {
                resolution: None,
                quality: Quality::High,
                vsync: true,
            }),
            Look::SteamDeck => Some(Preset {
                resolution: Some(rea_launcher::display::STEAM_DECK_SCREEN),
                quality: Quality::High,
                vsync: true,
            }),
            Look::Original => Some(Preset {
                resolution: Some((1280, 720)),
                quality: Quality::Normal,
                vsync: true,
            }),
            Look::Custom => None,
        }
    }
}

pub const LANGUAGES: [&str; 6] = [
    "English", "French", "German", "Spanish", "Italian", "Polish",
];

const LOOKS: &[(&str, &str)] = &[
    ("modern", "Modern"),
    ("steam-deck", "Steam Deck"),
    ("original", "Original 2007"),
    ("custom", "Custom"),
];
const QUALITIES: &[(&str, &str)] = &[("normal", "Normal"), ("high", "High"), ("ultra", "Ultra")];
const FRAME_LIMITS: &[(&str, &str)] = &[("60", "60 fps"), ("120", "120 fps"), ("off", "Off")];
const LANGUAGE_CHOICES: &[(&str, &str)] = &[
    ("English", "English"),
    ("French", "French"),
    ("German", "German"),
    ("Spanish", "Spanish"),
    ("Italian", "Italian"),
    ("Polish", "Polish"),
];
/// `auto`: the screen's own size (the SDK's "whole screen" word).
const AUTO: &cfg::Aliases = &[("auto", Resolution::Fullscreen)];

/// Every option of the launcher, once: `launcher.cfg` key, flag, values,
/// label and the one-line description the window and the template show.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec::new(
        "game",
        Kind::Path,
        "Game folder",
        "The folder with SEGA Rally.exe or SEGA Rally_SSE1.exe. It is only read, never changed.",
    ),
    OptionSpec::new(
        "runtime",
        Kind::Path,
        "Private copy",
        r"Where the launcher keeps its fixed-up copy of the game. Default: %LOCALAPPDATA%\SRRLauncher\SegaRally.",
    )
    .launcher_only(),
    OptionSpec::new(
        "look",
        Kind::Choice(LOOKS),
        "Look",
        "A ready-made picture: Modern (your screen, High), Steam Deck (1280x800, High), Original 2007 (1280x720, Normal). On the command line it sets those values for one run.",
    ),
    OptionSpec::new(
        "resolution",
        Kind::Size {
            min: (640, 480),
            max: (7680, 4320),
            aliases: AUTO,
        },
        "Resolution",
        "The game's screen size; auto uses your screen. Ultra-wide sizes (21:9, 32:9) get the aspect fix, so the picture is not stretched.",
    )
    .with_example("3840x2160"),
    OptionSpec::new(
        "quality",
        Kind::Choice(QUALITIES),
        "Quality",
        "The game's detail preset: Normal is the 2007 look, High sharper shadows and track detail, Ultra every slider at maximum.",
    ),
    OptionSpec::new(
        "vsync",
        Kind::Toggle,
        "Vsync",
        "Waits for the screen's refresh: no tearing. Off can tear on screens above 60 Hz.",
    ),
    OptionSpec::new(
        "frame_limit",
        Kind::Choice(FRAME_LIMITS),
        "Frame limit",
        "The highest frame rate under Linux, Wine or Proton (through DXVK). 60 is the steady default.",
    )
    .with_aliases(&["frame-limit"]),
    OptionSpec::new(
        "stutter_fix",
        Kind::Toggle,
        "Stutter fix",
        "The community fix for the microstutter in every race and the judder from Championship stage 2. Needs the clean SEGA Rally_SSE1.exe.",
    )
    .with_aliases(&["stutter-fix"]),
    OptionSpec::new(
        "language",
        Kind::Choice(LANGUAGE_CHOICES),
        "Language",
        "The language of the game's menus.",
    ),
    // Keys of earlier releases that no longer do anything: read and
    // ignored silently (file and flag), commented out when the window
    // saves.
    OptionSpec::removed(
        "aspect",
        "the aspect follows the resolution (ultra-wide sizes get the aspect fix)",
    ),
    OptionSpec::removed("fullscreen", "the game always runs full screen (v0.1.0)"),
];

/// The launcher's value-less switches and their `--help` lines.
pub const SWITCH_HELP: [(&str, &str); 7] = [
    (
        "--check",
        "check the game folder and exit (exit code 1 if it cannot play)",
    ),
    (
        "--setup",
        "prepare the private copy and the game's settings, without playing",
    ),
    (
        "--download-directx",
        "download Microsoft's DirectX 9 files into the private copy, then exit",
    ),
    (
        "--no-window",
        "play with the saved settings, no launcher window",
    ),
    ("--settings", "only show the settings window"),
    (
        "--linux",
        "running under srr-play or the AppImage: install DXVK into the copy",
    ),
    ("--help", "show this help"),
];

/// The launcher's value-less switches.
pub const SWITCHES: [&str; 7] = {
    let mut flags = [""; 7];
    let mut i = 0;
    while i < flags.len() {
        flags[i] = SWITCH_HELP[i].0;
        i += 1;
    }
    flags
};

/// The option row for `key`.
pub fn option(key: &str) -> &'static OptionSpec {
    spec::find(OPTIONS, key).expect("a key of OPTIONS")
}

pub const FILE_NAME: &str = "launcher.cfg";

/// The first-run `launcher.cfg`: every option commented out.
pub fn template() -> String {
    spec::template(
        "Sega Rally Revo launcher settings.\n\
         key = value; lines starting with # are comments. An unset key keeps the \
         game's own value (or the launcher's default where the game has none). \
         The settings window fills these in when you press Play.",
        OPTIONS,
    )
}

/// `--help`: the options from the table, then the switches.
pub fn usage() -> String {
    format!(
        "SRRLauncher {}\n\nSRRLauncher [options]\n\n{}\n\
         Options override launcher.cfg for one run; only --game is remembered.\n",
        env!("CARGO_PKG_VERSION"),
        spec::usage_with(OPTIONS, KeyStyle::Underscore, &SWITCH_HELP)
    )
}

/// The message for a refused command line.
fn arg_message(error: ArgError) -> String {
    match error {
        ArgError::Help(_) => usage(),
        ArgError::Unknown(flag) => format!("unknown option {flag}\n\n{}", usage()),
        ArgError::MissingValue(flag) => format!("{flag} needs a value\n\n{}", usage()),
        ArgError::Bad { flag, value, needs } => {
            format!("{flag} needs {needs}, not {value:?}\n\n{}", usage())
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub game: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub look: Option<Look>,
    /// `Some(None)` is `auto` said out loud; `None` is unset.
    pub resolution: Option<Option<(u32, u32)>>,
    pub quality: Option<Quality>,
    pub vsync: Option<bool>,
    pub frame_limit: Option<FrameLimit>,
    pub stutter_fix: Option<bool>,
    pub language: Option<String>,
}

/// A parsed command line: the one-run settings and the switches given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cli {
    pub settings: Settings,
    pub switches: Vec<String>,
}

impl Cli {
    pub fn has(&self, switch: &str) -> bool {
        self.switches.iter().any(|s| s == switch)
    }

    /// True when only switches (no options) were given.
    pub fn options_given(&self) -> bool {
        self.settings != Settings::default()
    }
}

impl Settings {
    /// Takes one checked value (keys are the table's, aliases resolved).
    pub fn apply(&mut self, key: &str, value: Value) -> Result<(), Rejected> {
        match (key, value) {
            ("game", Value::Text(path)) => self.game = Some(path.into()),
            ("runtime", Value::Text(path)) => self.runtime = Some(path.into()),
            ("look", Value::Choice(look)) => self.look = Look::parse(look),
            ("resolution", Value::Size(Resolution::Size(width, height))) => {
                self.resolution = Some(Some((width, height)))
            }
            ("resolution", Value::Size(_)) => self.resolution = Some(None),
            ("quality", Value::Choice(quality)) => self.quality = Quality::parse(quality),
            ("vsync", Value::Toggle(on)) => self.vsync = Some(on),
            ("frame_limit", Value::Choice(limit)) => self.frame_limit = FrameLimit::parse(limit),
            ("stutter_fix", Value::Toggle(on)) => self.stutter_fix = Some(on),
            ("language", Value::Choice(language)) => self.language = Some(language.to_string()),
            _ => return Err(Rejected::Unknown),
        }
        Ok(())
    }

    /// The settings in a `launcher.cfg` text and a warning per bad line.
    /// Removed keys (`aspect`, `fullscreen`) are skipped without a warning.
    pub fn parse_file(text: &str) -> (Settings, Vec<String>) {
        let mut settings = Settings::default();
        let warnings = spec::parse_file(text, FILE_NAME, OPTIONS, |key, value| {
            settings.apply(key, value)
        });
        (settings, warnings)
    }

    /// The command line; the error is the message to show.
    pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
        let mut settings = Settings::default();
        let switches = spec::parse_args(
            args,
            KeyStyle::Underscore,
            OPTIONS,
            &SWITCHES,
            |key, value| settings.apply(key, value),
        )
        .map_err(arg_message)?;
        Ok(Cli { settings, switches })
    }

    /// `over` (the command line) on top of `self` (the file). A look given
    /// in `over` sets all of its values first; explicit options in `over`
    /// still win over the look.
    pub fn merged(self, over: Settings) -> Settings {
        let mut base = self;
        if let Some(preset) = over.look.and_then(Look::preset) {
            base.resolution = Some(preset.resolution);
            base.quality = Some(preset.quality);
            base.vsync = Some(preset.vsync);
        }
        Settings {
            game: over.game.or(base.game),
            runtime: over.runtime.or(base.runtime),
            look: over.look.or(base.look),
            resolution: over.resolution.or(base.resolution),
            quality: over.quality.or(base.quality),
            vsync: over.vsync.or(base.vsync),
            frame_limit: over.frame_limit.or(base.frame_limit),
            stutter_fix: over.stutter_fix.or(base.stutter_fix),
            language: over.language.or(base.language),
        }
    }

    /// The size the game runs at: the setting, or the landscape screen, or
    /// 1920x1080.
    pub fn effective_resolution(&self, screen: Option<(u32, u32)>) -> (u32, u32) {
        resolve_resolution(self.resolution.flatten(), screen)
    }

    pub fn effective_quality(&self) -> Quality {
        self.quality.unwrap_or(Quality::High)
    }

    pub fn effective_vsync(&self) -> bool {
        self.vsync.unwrap_or(true)
    }

    pub fn effective_frame_limit(&self) -> FrameLimit {
        self.frame_limit.unwrap_or(FrameLimit::Fps60)
    }

    pub fn effective_stutter_fix(&self) -> bool {
        self.stutter_fix.unwrap_or(true)
    }

    pub fn effective_language(&self) -> &str {
        self.language.as_deref().unwrap_or("English")
    }
}

/// `resolution` (`None` is auto) on a screen: the landscape screen size,
/// else 1920x1080.
pub fn resolve_resolution(
    resolution: Option<(u32, u32)>,
    screen: Option<(u32, u32)>,
) -> (u32, u32) {
    resolution
        .or_else(|| screen.and_then(landscape))
        .unwrap_or((1920, 1080))
}

/// A screen size as a landscape resolution worth offering: 640x480 and up
/// (SRR offers 640x480 itself, unlike the SDK's plain `landscape`).
pub fn landscape(screen: (u32, u32)) -> Option<(u32, u32)> {
    rea_launcher::display::landscape_from(screen, (640, 480))
}

/// Writes `value` for `key` in a `launcher.cfg` text: the current key name,
/// with any old spelling (`frame-limit`) turned into a comment.
pub fn set_key(text: &str, key: &str, value: &str) -> String {
    spec::set_key(text, option(key), value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Cli, String> {
        Settings::parse_args(list.iter().map(|a| a.to_string()))
    }

    #[test]
    fn defaults_are_the_recommended_setup() {
        let settings = Settings::default();
        assert_eq!(
            settings.effective_resolution(Some((3840, 2160))),
            (3840, 2160)
        );
        assert_eq!(settings.effective_resolution(None), (1920, 1080));
        assert_eq!(settings.effective_quality(), Quality::High);
        assert!(settings.effective_vsync());
        assert_eq!(settings.effective_frame_limit(), FrameLimit::Fps60);
        assert!(settings.effective_stutter_fix());
        assert_eq!(settings.effective_language(), "English");
    }

    #[test]
    fn file_and_cli_merge() {
        let (saved, warnings) =
            Settings::parse_file("game = C:\\Sega Rally\nquality = normal\nvsync = false\n");
        assert!(warnings.is_empty());
        let cli = args(&["--quality", "high", "--resolution", "2560x1440"]).unwrap();
        let merged = saved.merged(cli.settings);
        assert_eq!(merged.quality, Some(Quality::High));
        assert_eq!(merged.resolution, Some(Some((2560, 1440))));
        assert_eq!(merged.vsync, Some(false));
        assert_eq!(merged.game, Some(PathBuf::from("C:\\Sega Rally")));
    }

    #[test]
    fn a_look_on_the_command_line_sets_its_values_and_options_still_win() {
        let (saved, _) =
            Settings::parse_file("resolution = 3840x2160\nquality = ultra\nvsync = off\n");
        let cli = args(&["--look", "original"]).unwrap();
        let merged = saved.clone().merged(cli.settings);
        assert_eq!(merged.resolution, Some(Some((1280, 720))));
        assert_eq!(merged.quality, Some(Quality::Normal));
        assert_eq!(merged.vsync, Some(true));
        let cli = args(&["--look", "steam-deck", "--quality", "ultra"]).unwrap();
        let merged = saved.clone().merged(cli.settings);
        assert_eq!(merged.resolution, Some(Some((1280, 800))));
        assert_eq!(merged.quality, Some(Quality::Ultra));
        // Modern is the screen's size.
        let merged = saved.merged(args(&["--look", "modern"]).unwrap().settings);
        assert_eq!(merged.resolution, Some(None));
        assert_eq!(
            merged.effective_resolution(Some((2560, 1440))),
            (2560, 1440)
        );
        // Custom keeps the file's values.
        let (saved, _) = Settings::parse_file("quality = ultra\n");
        let merged = saved.merged(args(&["--look", "custom"]).unwrap().settings);
        assert_eq!(merged.quality, Some(Quality::Ultra));
    }

    #[test]
    fn auto_is_a_resolution_word() {
        let cli = args(&["--resolution", "auto"]).unwrap();
        assert_eq!(cli.settings.resolution, Some(None));
        assert_eq!(
            cli.settings.effective_resolution(Some((2160, 3840))),
            (3840, 2160)
        );
        let (file, warnings) = Settings::parse_file("resolution = AUTO\n");
        assert!(warnings.is_empty());
        assert_eq!(file.resolution, Some(None));
    }

    #[test]
    fn invalid_values_warn_or_fail() {
        let (settings, warnings) = Settings::parse_file(
            "resolution = huge\nframe_limit = 7\nlanguage = Klingon\nvsync = maybe\nlook = fancy\n",
        );
        assert_eq!(warnings.len(), 5, "{warnings:?}");
        assert_eq!(settings, Settings::default());
        for bad in [
            &["--frame-limit", "7"][..],
            &["--language", "Klingon"],
            &["--vsync", "maybe"],
            &["--stutter-fix", "sometimes"],
            &["--quality", "insane"],
            &["--resolution", "100x100"],
            &["--look", "fancy"],
            &["--bogus"],
            &["--game"],
        ] {
            assert!(args(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn refused_options_say_what_they_need() {
        let error = |list: &[&str]| args(list).unwrap_err();
        let usage = usage();
        assert_eq!(
            error(&["--bogus"]),
            format!("unknown option --bogus\n\n{usage}")
        );
        assert_eq!(
            error(&["--game"]),
            format!("--game needs a value\n\n{usage}")
        );
        assert_eq!(
            error(&["--vsync", "maybe"]),
            format!("--vsync needs on or off, not \"maybe\"\n\n{usage}")
        );
        assert_eq!(
            error(&["--frame-limit", "7"]),
            format!("--frame-limit needs 60, 120 or off, not \"7\"\n\n{usage}")
        );
        assert_eq!(
            error(&["--resolution", "huge"]),
            format!("--resolution needs WIDTHxHEIGHT or auto, not \"huge\"\n\n{usage}")
        );
        assert!(error(&["--language", "Klingon"]).starts_with(
            "--language needs English, French, German, Spanish, Italian or Polish, not"
        ));
        for help in ["--help", "-h", "/?"] {
            assert_eq!(error(&[help]), usage);
        }
    }

    #[test]
    fn values_are_case_insensitive_and_canonical() {
        let (settings, warnings) =
            Settings::parse_file("language = french\nquality = HIGH\nstutter_fix = off\n");
        assert!(warnings.is_empty());
        assert_eq!(settings.language.as_deref(), Some("French"));
        assert_eq!(settings.quality, Some(Quality::High));
        assert_eq!(settings.stutter_fix, Some(false));
        let cli = args(&["--language", "POLISH"]).unwrap();
        assert_eq!(cli.settings.language.as_deref(), Some("Polish"));
    }

    #[test]
    fn switches_come_back_in_order() {
        let cli = args(&["--no-window", "--quality", "ultra", "--linux"]).unwrap();
        assert_eq!(cli.switches, ["--no-window", "--linux"]);
        assert!(cli.has("--linux") && !cli.has("--check"));
        assert!(cli.options_given());
        assert!(!args(&["--check"]).unwrap().options_given());
    }

    /// Files written by v0.1.x: hyphenated keys, `true`/`false`, the
    /// retired `aspect` and `fullscreen`, a `look` the window saved.
    #[test]
    fn an_earlier_release_file_keeps_working() {
        let old = "game = D:\\Games\\Sega Rally\nresolution = 3440x1440\naspect = 16:9\n\
                   quality = high\nvsync = true\nframe-limit = off\nstutter-fix = false\n\
                   language = German\nlook = steam-deck\nfullscreen = false\n";
        let (settings, warnings) = Settings::parse_file(old);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(settings.frame_limit, Some(FrameLimit::Off));
        assert_eq!(settings.stutter_fix, Some(false));
        assert_eq!(settings.vsync, Some(true));
        assert_eq!(settings.look, Some(Look::SteamDeck));
        assert_eq!(settings.resolution, Some(Some((3440, 1440))));
        // The old flags too.
        let cli = args(&["--frame-limit", "120", "--stutter-fix", "off"]).unwrap();
        assert_eq!(cli.settings.frame_limit, Some(FrameLimit::Fps120));
        assert_eq!(cli.settings.stutter_fix, Some(false));
        // Saving writes the new names and comments the old lines out.
        let saved = set_key(old, "frame_limit", "60");
        let saved = set_key(&saved, "stutter_fix", "on");
        assert!(saved.contains("frame_limit = 60\n"));
        assert!(saved.contains("stutter_fix = on\n"));
        assert!(!saved.contains("\nframe-limit = "));
        assert!(!saved.contains("\nstutter-fix = "));
        let (reread, warnings) = Settings::parse_file(&saved);
        assert!(warnings.is_empty());
        assert_eq!(reread.frame_limit, Some(FrameLimit::Fps60));
        assert_eq!(reread.stutter_fix, Some(true));
        // The removed flags are accepted and ignored.
        let cli = args(&["--aspect", "21:9", "--fullscreen", "off"]).unwrap();
        assert!(!cli.options_given());
        // Saving comments the removed lines out.
        let retired = spec::retire_removed(&saved, OPTIONS);
        assert!(!retired.contains("\naspect = ") && !retired.contains("\nfullscreen = "));
        assert!(retired.contains("\n#aspect =\n") && retired.contains("\n#fullscreen =\n"));
        assert_eq!(Settings::parse_file(&retired).0, reread);
    }

    #[test]
    fn the_template_parses_to_nothing_and_lists_every_key() {
        let template = template();
        let (settings, warnings) = Settings::parse_file(&template);
        assert!(warnings.is_empty());
        assert_eq!(settings, Settings::default());
        for option in spec::active(OPTIONS) {
            assert!(
                template.contains(&format!("#{} =", option.key)),
                "{}",
                option.key
            );
            assert!(!option.description.is_empty(), "{}", option.key);
        }
        // The window fills a placeholder in place.
        let filled = set_key(&template, "vsync", "off");
        assert_eq!(Settings::parse_file(&filled).0.vsync, Some(false));
    }

    #[test]
    fn help_lists_every_flag() {
        let usage = usage();
        for option in spec::active(OPTIONS) {
            assert!(usage.contains(&option.flag_name(KeyStyle::Underscore)));
        }
        assert!(!usage.contains("--aspect") && !usage.contains("--fullscreen"));
        assert!(!template().contains("aspect =") && !template().contains("fullscreen"));
        for switch in SWITCHES {
            assert!(usage.contains(switch), "{switch}");
        }
        assert!(usage.contains("--resolution <WxH>"));
        assert!(usage.contains("--frame-limit 60|120|off"));
        assert!(usage.contains("--look modern|steam-deck|original|custom"));
    }

    #[test]
    fn ultra_wide_sizes_pick_the_aspect_fix() {
        assert_eq!(Aspect::for_resolution((1920, 1080)), Aspect::Native);
        assert_eq!(Aspect::for_resolution((1280, 800)), Aspect::Native);
        assert_eq!(Aspect::for_resolution((1280, 720)), Aspect::Native);
        assert_eq!(Aspect::for_resolution((2560, 1080)), Aspect::Ultra21);
        assert_eq!(Aspect::for_resolution((3440, 1440)), Aspect::Ultra21);
        assert_eq!(Aspect::for_resolution((3840, 1080)), Aspect::Ultra32);
        assert_eq!(Aspect::for_resolution((5120, 1440)), Aspect::Ultra32);
    }

    #[test]
    fn rotated_screens_become_landscape() {
        assert_eq!(landscape((2160, 3840)), Some((3840, 2160)));
        // 640x480 itself is offered (the SDK's rule starts above it).
        assert_eq!(landscape((480, 640)), Some((640, 480)));
        assert_eq!(landscape((639, 480)), None);
    }

    /// A `launcher.cfg` saved by Notepad in a Windows code page (cp1252)
    /// and paths with spaces, apostrophes and accents.
    #[test]
    fn a_cp1252_file_with_a_hostile_path_is_read() {
        let dir = std::env::temp_dir().join(format!("srr-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        std::fs::write(
            &path,
            b"game = C:\\Users\\Jos\xe9\\Jogos d'A\xe7\xe3o\r\nquality = high\r\n",
        )
        .unwrap();
        let text = cfg::read_file(&path).unwrap().unwrap();
        let (settings, warnings) = Settings::parse_file(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            settings.game,
            Some(PathBuf::from("C:\\Users\\José\\Jogos d'Ação"))
        );
        // Saving keeps the CRLF lines.
        let saved = set_key(&text, "vsync", "on");
        assert!(saved.contains("Jogos d'Ação\r\nquality = high\r\nvsync = on\r\n"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
