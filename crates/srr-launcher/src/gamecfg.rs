//! The game's own configuration, written the way the original .NET
//! launcher would write it — so the unreliable-under-Wine settings UI never
//! has to run.
//!
//! `config.ini` lives in `Documents\SEGA Rally\Saved Games\config.ini`.
//! Only the `[Graphics]` keys this launcher manages are touched; existing
//! `[Audio]` and `[GameController]` sections (which hold the player's
//! controller GUID and mapping) are preserved verbatim. The real file is
//! replaced atomically ([`write_atomic`]).

use crate::config::Quality;

/// The per-feature values of each preset. The game's own quality scale is
/// LOWEST=0, LOW=1, NORMAL=2, HIGH=3 (from the original settings
/// launcher's enum). `High` is the exact configuration proven on the
/// reference 4K/DXVK setup; `Ultra` sets every slider to HIGH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualityValues(pub [i32; 10]);

pub fn quality_values(quality: Quality) -> QualityValues {
    // GraphicQuality, Shadow, Reflection, Track, Texture, Shader,
    // DrawDistance, Particle, RearViewMirror, Mud
    match quality {
        Quality::Normal => QualityValues([2, 2, 2, 2, 2, 2, 2, 2, 2, 2]),
        Quality::High => QualityValues([3, 2, 2, 3, 2, 1, 2, 2, 2, 1]),
        Quality::Ultra => QualityValues([3, 3, 3, 3, 3, 3, 3, 3, 3, 3]),
    }
}

/// Render the `[Graphics]` section this launcher manages.
pub fn graphics_section(resolution: (u32, u32), quality: Quality, vsync: bool) -> String {
    let values = quality_values(quality);
    let keys = [
        ("AdapterOrdinal", "0".to_string()),
        ("RenderDevice", "Hardware".to_string()),
        ("AdapterFormat", "X8R8G8B8".to_string()),
        ("HorizontalResolution", resolution.0.to_string()),
        ("VerticalResolution", resolution.1.to_string()),
        ("BackBufferFormat", "A8R8G8B8".to_string()),
        ("DepthStencilFormat", "D24S8".to_string()),
        ("AntiAliasingLevel", "0".to_string()),
        ("MultisampleType", "0".to_string()),
        ("MultisampleQuality", "0".to_string()),
        ("GraphicQuality", values.0[0].to_string()),
        ("ShadowQuality", values.0[1].to_string()),
        ("ReflectionQuality", values.0[2].to_string()),
        ("TrackQuality", values.0[3].to_string()),
        ("TextureQuality", values.0[4].to_string()),
        ("ShaderQuality", values.0[5].to_string()),
        ("DrawDistanceQuality", values.0[6].to_string()),
        ("ParticleQuality", values.0[7].to_string()),
        ("RearViewMirrorQuality", values.0[8].to_string()),
        ("MudQuality", values.0[9].to_string()),
        ("VSync", if vsync { "1" } else { "0" }.to_string()),
        ("RefreshRate", "60".to_string()),
        ("ShaderVersion", "3".to_string()),
        ("ResolutionFilter", "1".to_string()),
        ("RunWindowed", "0".to_string()),
    ];
    keys.iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `[Audio]` and `[GameController]` sections seeded on first run, when
/// no config.ini exists yet. Once the file exists these sections are the
/// player's (they hold the controller GUID and mapping) and are never
/// rewritten.
pub fn seed_sections() -> String {
    concat!(
        "[Audio]\n",
        "DriverGUID=00000000-0000-0000-0000-000000000000\n",
        "Quality=3\n",
        "SoftwareEmulation=0\n",
        "\n",
        "[GameController]\n",
        "GUID=\n",
        "Mapping=000100010000000100000000-0000-0000-0000-0000000000000000000E000F001200130014000200030017001800010000\n"
    )
    .to_string()
}

/// Merge our managed `[Graphics]` keys into an existing config.ini text.
/// Only `[Graphics]` is edited (see [`crate::ini`]): keys are updated in
/// place whatever their case or spacing, missing keys are added to the
/// section, a missing section is appended. `[Audio]`, `[GameController]`
/// and every comment or unknown key stay exactly as they were.
pub fn update_config(
    existing: &str,
    resolution: (u32, u32),
    quality: Quality,
    vsync: bool,
) -> String {
    graphics_section(resolution, quality, vsync)
        .lines()
        .map(|line| line.split_once('=').expect("key=value"))
        .fold(existing.to_string(), |text, (key, value)| {
            crate::ini::set_section_key(&text, "Graphics", key, value)
        })
}

/// Replace `path` with `text` atomically: a crash or a full disk mid-write
/// leaves the player's previous file, never a truncated one.
pub fn write_atomic(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let mut temp = path.as_os_str().to_owned();
    temp.push(".srr-tmp");
    let temp = std::path::PathBuf::from(temp);
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

/// Full config.ini for a first run (no file exists yet).
pub fn seed_config(resolution: (u32, u32), quality: Quality, vsync: bool) -> String {
    format!(
        "{}\n[Graphics]\n{}\n",
        seed_sections(),
        graphics_section(resolution, quality, vsync)
    )
}

/// The two bootup logo videos, renamed out of the way in the runtime copy:
/// Wine's Quartz path stalls on them and they add nothing (verified on the
/// reference setup).
pub const BOOTUP_VIDEOS: [&str; 2] = [
    "Bootup/video/Rally/SEGA_Logo720p.wmv",
    "Bootup/video/Rally/SRS_Logo_1.wmv",
];

pub const DISABLED_SUFFIX: &str = ".srr-disabled";

/// Wine's input bus key (`HKLM`), read by `winebus.sys` when the bus starts.
pub const WINEBUS_KEY: &str = r"System\CurrentControlSet\Services\winebus";

/// The controller bridge this game needs under Wine: Wine's SDL-backed
/// XInput device (`Map Controllers=1`), with the raw hidraw/evdev duplicates
/// kept out. The value names are exactly what `winebus.sys` reads —
/// `DisableHidraw`/`DisableInput` without a space, `Enable SDL`/`Map
/// Controllers` with one. Seeded only when absent, so a player's own value
/// always wins.
pub const WINEBUS_VALUES: [(&str, u32); 4] = [
    ("DisableHidraw", 1),
    ("DisableInput", 1),
    ("Enable SDL", 1),
    ("Map Controllers", 1),
];

/// DXVK-side frame limiter, the community-standard pacing guard for this
/// game. `None` writes a neutral file.
pub fn dxvk_conf(limit: Option<u32>) -> String {
    match limit {
        Some(limit) => format!("d3d9.maxFrameRate = {limit}\n"),
        None => "# no frame limit (managed by SRRLauncher)\n".to_string(),
    }
}

/// Registry values the game reads before it will start. `installdir` is
/// the private runtime copy, with the trailing backslash of the original
/// `SEGA Rally.reg`.
pub fn registry_values(runtime_windows_path: &str, language: &str) -> [(&'static str, String); 3] {
    [
        ("installdir", format!("{runtime_windows_path}\\")),
        ("sku", "Europe".to_string()),
        ("language", language.to_string()),
    ]
}

/// The Documents layout the game expects, seeded when missing. Without
/// these the game can reach the main menu and then die with an access
/// violation.
pub struct DocumentsPaths {
    pub saved_games: std::path::PathBuf,
    pub rank_table: std::path::PathBuf,
    pub ghosts: std::path::PathBuf,
}

pub fn documents_paths(profile: &std::path::Path, public: &std::path::Path) -> DocumentsPaths {
    let saved_games = profile
        .join("Documents")
        .join("SEGA Rally")
        .join("Saved Games");
    let shared = public
        .join("Documents")
        .join("SEGA Rally")
        .join("Shared Data");
    DocumentsPaths {
        rank_table: shared.join("Rank Table"),
        ghosts: shared.join("Ghosts"),
        saved_games,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_matches_the_proven_seed() {
        let text = graphics_section((3840, 2160), Quality::High, true);
        for (key, value) in [
            ("HorizontalResolution", "3840"),
            ("VerticalResolution", "2160"),
            ("GraphicQuality", "3"),
            ("ShadowQuality", "2"),
            ("TrackQuality", "3"),
            ("TextureQuality", "2"),
            ("ShaderQuality", "1"),
            ("DrawDistanceQuality", "2"),
            ("MudQuality", "1"),
            ("VSync", "1"),
            ("RunWindowed", "0"),
            ("RefreshRate", "60"),
        ] {
            assert!(
                text.contains(&format!("{key}={value}")),
                "missing {key}={value}"
            );
        }
    }

    #[test]
    fn seed_contains_the_seed_sections_and_graphics() {
        let text = seed_config((1920, 1080), Quality::Normal, false);
        assert!(text.contains("[Audio]"));
        assert!(text.contains("Quality=3"));
        assert!(text.contains("[GameController]\nGUID="));
        assert!(text.contains("[Graphics]\nAdapterOrdinal=0"));
        assert!(text.contains("HorizontalResolution=1920"));
        assert!(text.contains("VSync=0"));
    }

    #[test]
    fn update_preserves_unrelated_sections_and_comments() {
        let existing = "# player file\n[Audio]\nQuality=2\n\n[GameController]\nGUID={1234}\nMapping=ABC\n\n[Graphics]\nHorizontalResolution=800\nVSync=0\nCustomKey=keep\n";
        let out = update_config(existing, (2560, 1440), Quality::High, true);
        assert!(out.contains("# player file"));
        assert!(out.contains("[Audio]\nQuality=2"));
        assert!(out.contains("GUID={1234}"));
        assert!(out.contains("Mapping=ABC"));
        assert!(out.contains("HorizontalResolution=2560"));
        assert!(out.contains("VerticalResolution=1440"));
        assert!(out.contains("VSync=1"));
        assert!(out.contains("CustomKey=keep"));
        assert!(out.contains("GraphicQuality=3"));
    }

    #[test]
    fn update_adds_a_missing_graphics_section() {
        let out = update_config("[Audio]\nQuality=3\n", (1920, 1080), Quality::Normal, true);
        assert!(out.contains("[Graphics]\nAdapterOrdinal=0"));
        assert!(out.contains("[Audio]\nQuality=3"));
    }

    #[test]
    fn update_adds_missing_keys_into_an_existing_section() {
        let out = update_config("[Graphics]\nVSync=0\n", (1920, 1080), Quality::Normal, true);
        // the new keys land inside the section, after the keys already there
        let graphics = out.split_once("[Graphics]\n").unwrap().1;
        assert!(graphics.starts_with("VSync=1\n"));
        assert!(graphics.contains("AdapterOrdinal=0"));
        assert!(graphics.contains("MudQuality=2"));
        assert!(out.ends_with("RunWindowed=0\n"));
    }

    #[test]
    fn winebus_value_names_are_the_ones_wine_reads() {
        let names: Vec<&str> = WINEBUS_VALUES.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "DisableHidraw",
                "DisableInput",
                "Enable SDL",
                "Map Controllers"
            ]
        );
        assert!(WINEBUS_VALUES.iter().all(|(_, value)| *value == 1));
        assert_eq!(WINEBUS_KEY, r"System\CurrentControlSet\Services\winebus");
    }

    #[test]
    fn update_recognises_spaced_and_differently_cased_keys() {
        let existing = "[Audio]\r\nQuality = 2\r\n\r\n[GameController]\r\nGUID = {1234}\r\n\r\n[Graphics]\r\nHorizontalResolution = 1280\r\nverticalresolution=720\r\n";
        let out = update_config(existing, (3840, 2160), Quality::High, true);
        assert_eq!(out.matches("HorizontalResolution").count(), 1);
        assert_eq!(
            out.to_ascii_lowercase()
                .matches("verticalresolution")
                .count(),
            1
        );
        assert!(out.contains("HorizontalResolution=3840\r\n"));
        assert!(out.contains("VerticalResolution=2160\r\n"));
        // the player's sections, spacing included, are untouched
        assert!(
            out.starts_with("[Audio]\r\nQuality = 2\r\n\r\n[GameController]\r\nGUID = {1234}\r\n")
        );
        assert!(!out.replace("\r\n", "").contains('\n'), "line endings kept");
        // and running it again changes nothing
        assert_eq!(update_config(&out, (3840, 2160), Quality::High, true), out);
    }

    #[test]
    fn write_atomic_replaces_the_file_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("srr-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.ini");
        std::fs::write(&path, "old").unwrap();
        write_atomic(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn dxvk_conf_limits_or_stays_neutral() {
        assert_eq!(dxvk_conf(Some(60)), "d3d9.maxFrameRate = 60\n");
        assert_eq!(
            dxvk_conf(None),
            "# no frame limit (managed by SRRLauncher)\n"
        );
    }

    #[test]
    fn registry_values_carry_the_runtime_path_and_language() {
        let values = registry_values(r"C:\users\me\AppData\Local\SRRLauncher\SegaRally", "French");
        assert_eq!(values[0].0, "installdir");
        assert_eq!(
            values[0].1,
            r"C:\users\me\AppData\Local\SRRLauncher\SegaRally\"
        );
        assert_eq!(values[1], ("sku", "Europe".to_string()));
        assert_eq!(values[2], ("language", "French".to_string()));
    }

    #[test]
    fn documents_layout_is_the_expected_shape() {
        let profile = std::path::Path::new(r"C:\users\akita");
        let public = std::path::Path::new(r"C:\users\Public");
        let paths = documents_paths(profile, public);
        assert_eq!(
            paths.saved_games,
            profile
                .join("Documents")
                .join("SEGA Rally")
                .join("Saved Games")
        );
        assert_eq!(
            paths.rank_table,
            public
                .join("Documents")
                .join("SEGA Rally")
                .join("Shared Data")
                .join("Rank Table")
        );
        assert_eq!(
            paths.ghosts,
            public
                .join("Documents")
                .join("SEGA Rally")
                .join("Shared Data")
                .join("Ghosts")
        );
    }
}
