//! The game's own configuration, written the way the original .NET
//! launcher would write it — so the unreliable-under-Wine settings UI never
//! has to run.
//!
//! `config.ini` lives in `Documents\SEGA Rally\Saved Games\config.ini`.
//! Only the `[Graphics]` keys this launcher manages are touched; existing
//! `[Audio]` and `[GameController]` sections (which hold the player's
//! controller GUID and mapping) are preserved verbatim, byte for byte in
//! any encoding (the SDK's `ini` module). The caller replaces the real file
//! atomically (`rea_launcher::fsx::write_atomic`).

use rea_launcher::ini::Ini;
use rea_launcher::wine;

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

/// The `[Graphics]` keys this launcher manages, in the game's order.
pub fn graphics_keys(
    resolution: (u32, u32),
    quality: Quality,
    vsync: bool,
) -> [(&'static str, String); 25] {
    let values = quality_values(quality);
    [
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
    ]
}

/// Render the `[Graphics]` section this launcher manages.
pub fn graphics_section(resolution: (u32, u32), quality: Quality, vsync: bool) -> String {
    graphics_keys(resolution, quality, vsync)
        .iter()
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

/// Sets our managed `[Graphics]` keys in `ini` (`key=value`, the game's
/// spelling): keys are updated in place whatever their case or spacing,
/// missing keys are added to the section, a missing section is appended.
/// `[Audio]`, `[GameController]` and every comment or unknown key stay
/// exactly as they were.
fn set_graphics(ini: &mut Ini, resolution: (u32, u32), quality: Quality, vsync: bool) {
    for (key, value) in graphics_keys(resolution, quality, vsync) {
        ini.set("Graphics", key, &value);
    }
}

/// [`update_config_bytes`] on text.
pub fn update_config(
    existing: &str,
    resolution: (u32, u32),
    quality: Quality,
    vsync: bool,
) -> String {
    let mut ini = Ini::parse(existing).tight();
    set_graphics(&mut ini, resolution, quality, vsync);
    ini.to_text().expect("parsed from text")
}

/// Our managed `[Graphics]` lines merged into the file's raw bytes,
/// changing nothing else. A file that is not UTF-8 is edited one byte per
/// character, so a controller name or path in any encoding is written back
/// byte for byte; a leading UTF-8 BOM is set aside, so `[Graphics]` on the
/// first line still matches, and kept.
pub fn update_config_bytes(
    existing: &[u8],
    resolution: (u32, u32),
    quality: Quality,
    vsync: bool,
) -> Vec<u8> {
    let mut ini = Ini::from_bytes(existing).tight();
    set_graphics(&mut ini, resolution, quality, vsync);
    ini.to_bytes()
}

/// What to write to config.ini given the result of reading it: the
/// updated file, a fresh seed only when it does not exist, and the read
/// error otherwise (a file that cannot be read is the player's and must not
/// be replaced by a seed that would lose `[Audio]`/`[GameController]`).
pub fn next_config(
    read: std::io::Result<Vec<u8>>,
    resolution: (u32, u32),
    quality: Quality,
    vsync: bool,
) -> std::io::Result<Vec<u8>> {
    match read {
        Ok(existing) => Ok(update_config_bytes(&existing, resolution, quality, vsync)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(seed_config(resolution, quality, vsync).into_bytes())
        }
        Err(error) => Err(error),
    }
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

/// Leftover temp files of earlier releases' `config.ini` writes
/// (`config.ini.srr-tmp`); removed once with `fsx::remove_stale_temps`.
pub const OLD_TEMP_SUFFIX: &str = ".srr-tmp";

/// The controller bridge this game needs under Wine (under the SDK's
/// `wine::WINEBUS_KEY`): Wine's SDL-backed XInput device (`Map
/// Controllers=1`, unlike the SDK's `SDL_ONLY` baseline), with the raw
/// hidraw/evdev duplicates kept out. The value names are exactly what
/// `winebus.sys` reads. Seeded only when absent, so a player's own value
/// always wins.
pub const WINEBUS_VALUES: [(&str, u32); 4] = [
    (wine::DISABLE_HIDRAW, 1),
    (wine::DISABLE_INPUT, 1),
    (wine::ENABLE_SDL, 1),
    (wine::MAP_CONTROLLERS, 1),
];

/// DXVK-side frame limiter, the community-standard pacing guard for this
/// game. `None` writes a neutral file.
pub fn dxvk_conf(limit: Option<u32>) -> String {
    match limit {
        Some(limit) => format!("d3d9.maxFrameRate = {limit}\n"),
        None => "# no frame limit (managed by SRRLauncher)\n".to_string(),
    }
}

/// The Wine DLL rule the game needs when the launcher installed DXVK's
/// `d3d9.dll` next to it: native first, so neither Wine's builtin d3d9 nor a
/// stray `d3d9=b` in the environment replaces DXVK.
pub const DXVK_OVERRIDE: &str = "d3d9=n,b";

/// `WINEDLLOVERRIDES` for the game: our `d3d9` rule first when DXVK is
/// installed, every unrelated rule the caller set kept (the AppImage's
/// `mscoree,mshtml=` among them), and only the `d3d9` names we own
/// replaced (`wine::merge_dll_overrides`). Without DXVK the caller's value
/// is passed through unchanged.
pub fn wine_overrides(existing: Option<&str>, dxvk: bool) -> Option<String> {
    if !dxvk {
        return existing.map(str::to_string);
    }
    Some(wine::merge_dll_overrides(DXVK_OVERRIDE, existing))
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
        assert_eq!(
            wine::WINEBUS_KEY,
            r"System\CurrentControlSet\Services\winebus"
        );
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
    fn update_bytes_keeps_non_utf8_controller_names_byte_for_byte() {
        let mut existing = b"[GameController]\r\nGUID={1234}\r\nName=Manette \xE9tendue\r\n\r\n[Graphics]\r\nVSync=0\r\n".to_vec();
        assert!(std::str::from_utf8(&existing).is_err());
        let out = update_config_bytes(&existing, (1920, 1080), Quality::High, true);
        assert!(out.starts_with(b"[GameController]\r\nGUID={1234}\r\nName=Manette \xE9tendue\r\n"));
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("VSync=1\r\n"));
        assert_eq!(text.matches("VSync").count(), 1);
        // valid UTF-8 (a pad name with an accent) survives too
        existing = "[GameController]\nName=Manette étendue\n"
            .as_bytes()
            .to_vec();
        let out = update_config_bytes(&existing, (1920, 1080), Quality::High, true);
        assert!(out.starts_with("[GameController]\nName=Manette étendue\n".as_bytes()));
    }

    #[test]
    fn only_a_missing_config_is_seeded() {
        use std::io::{Error, ErrorKind};
        let seeded = next_config(
            Err(Error::from(ErrorKind::NotFound)),
            (1920, 1080),
            Quality::High,
            true,
        )
        .unwrap();
        assert!(seeded.starts_with(b"[Audio]"));
        let denied = next_config(
            Err(Error::from(ErrorKind::PermissionDenied)),
            (1920, 1080),
            Quality::High,
            true,
        );
        assert_eq!(denied.unwrap_err().kind(), ErrorKind::PermissionDenied);
        let updated = next_config(
            Ok(b"[GameController]\nGUID={1}\n".to_vec()),
            (1920, 1080),
            Quality::High,
            true,
        )
        .unwrap();
        assert!(updated.starts_with(b"[GameController]\nGUID={1}\n"));
    }

    #[test]
    fn update_bytes_matches_graphics_after_a_bom_and_keeps_it() {
        let existing = b"\xEF\xBB\xBF[Graphics]\nVSync=0\n";
        let out = update_config_bytes(existing, (1920, 1080), Quality::High, true);
        assert!(out.starts_with(b"\xEF\xBB\xBF[Graphics]\nVSync=1\n"));
        assert_eq!(
            String::from_utf8_lossy(&out).matches("[Graphics]").count(),
            1
        );
    }

    #[test]
    fn wine_overrides_put_dxvk_first_and_keep_unrelated_rules() {
        assert_eq!(wine_overrides(None, true).as_deref(), Some("d3d9=n,b"));
        assert_eq!(
            wine_overrides(Some("mscoree,mshtml="), true).as_deref(),
            Some("d3d9=n,b;mscoree,mshtml=")
        );
        assert_eq!(
            wine_overrides(Some("d3d8,D3D9=b;dsound=n;;winemenubuilder.exe=d"), true).as_deref(),
            Some("d3d9=n,b;d3d8=b;dsound=n;winemenubuilder.exe=d")
        );
        // spelled with .dll, in any case
        assert_eq!(
            wine_overrides(Some("d3d9.dll=b;D3D9.DLL,xinput1_3.dll=n"), true).as_deref(),
            Some("d3d9=n,b;xinput1_3.dll=n")
        );
        assert_eq!(
            wine_overrides(Some("d3d9x.dll=b"), true).as_deref(),
            Some("d3d9=n,b;d3d9x.dll=b")
        );
        // no DXVK of ours: the caller's value is left exactly as it was
        assert_eq!(wine_overrides(None, false), None);
        assert_eq!(
            wine_overrides(Some("d3d9=b"), false).as_deref(),
            Some("d3d9=b")
        );
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
