//! The game's own configuration, written the way the original .NET
//! launcher would write it — so the unreliable-under-Wine settings UI never
//! has to run.
//!
//! `config.ini` lives in `Documents\SEGA Rally\Saved Games\config.ini`.
//! A missing file is seeded once, complete. After that each launch writes
//! only the `[Graphics]` keys of the options set in `launcher.cfg`
//! (resolution, quality, vsync), and records them (`ini::Owned`): when an
//! option goes back to unset, a key is restored only while it still holds
//! the launcher's value, so a choice made in the game's own menu stays.
//! `[Audio]`, comments and unknown keys are kept byte for byte, in any
//! encoding (the SDK's `ini` module). The `rumble` option owns exactly
//! one field of `[GameController] Mapping` — the pad-vibration switch the
//! game's own settings tool also writes — and leaves every other
//! character of the player's mapping (pad GUID, layout, the eleven button
//! bindings) untouched, with the same `Owned` release semantics. The
//! caller replaces the real file atomically.

use rea_launcher::ini::{self, Ini, Owned};
use rea_launcher::wine;

use crate::config::Quality;

/// The per-feature values of each preset. The game's own quality scale is
/// LOWEST=0, LOW=1, NORMAL=2, HIGH=3 (from the original settings
/// launcher's enum). `High` is the exact configuration proven on the
/// reference 4K/DXVK setup; `Ultra` sets every slider to HIGH.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualityValues(pub [i32; 10]);

pub fn quality_values(quality: Quality) -> QualityValues {
    match quality {
        Quality::Normal => QualityValues([2, 2, 2, 2, 2, 2, 2, 2, 2, 2]),
        Quality::High => QualityValues([3, 2, 2, 3, 2, 1, 2, 2, 2, 1]),
        Quality::Ultra => QualityValues([3, 3, 3, 3, 3, 3, 3, 3, 3, 3]),
    }
}

/// The ten `[Graphics]` keys of a quality preset, in the game's order.
pub const QUALITY_KEYS: [&str; 10] = [
    "GraphicQuality",
    "ShadowQuality",
    "ReflectionQuality",
    "TrackQuality",
    "TextureQuality",
    "ShaderQuality",
    "DrawDistanceQuality",
    "ParticleQuality",
    "RearViewMirrorQuality",
    "MudQuality",
];

pub const SECTION: &str = "Graphics";

/// The complete `[Graphics]` section of a first run, in the game's order.
pub fn graphics_keys(
    resolution: (u32, u32),
    quality: Quality,
    vsync: bool,
) -> Vec<(&'static str, String)> {
    let values = quality_values(quality);
    let mut keys = vec![
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
    ];
    keys.extend(
        QUALITY_KEYS
            .into_iter()
            .zip(values.0.map(|v| v.to_string())),
    );
    keys.extend([
        ("VSync", vsync_value(vsync).to_string()),
        ("RefreshRate", "60".to_string()),
        ("ShaderVersion", "3".to_string()),
        ("ResolutionFilter", "1".to_string()),
        ("RunWindowed", "0".to_string()),
    ]);
    keys
}

fn vsync_value(vsync: bool) -> &'static str {
    if vsync { "1" } else { "0" }
}

/// Full config.ini for a first run (no file exists yet): `[Audio]` and
/// `[GameController]` as the original tool wrote them, then `[Graphics]`.
pub fn seed_config(resolution: (u32, u32), quality: Quality, vsync: bool) -> String {
    let graphics: Vec<String> = graphics_keys(resolution, quality, vsync)
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    format!(
        "[Audio]\n\
         DriverGUID=00000000-0000-0000-0000-000000000000\n\
         Quality=3\n\
         SoftwareEmulation=0\n\
         \n\
         [GameController]\n\
         GUID=\n\
         Mapping={SEED_MAPPING}\n\
         \n\
         [Graphics]\n{}\n",
        graphics.join("\n")
    )
}

/// The section that holds the controller the game plays with: `GUID` (the
/// pad's DirectInput instance) and `Mapping`.
pub const CONTROLLER_SECTION: &str = "GameController";

/// The `Mapping` key inside `[GameController]`.
pub const MAPPING_KEY: &str = "Mapping";

/// The `[GameController] Mapping` value the game's own settings tool
/// seeds: version 0001, layout 1, pad vibration on, an empty GUID and
/// the tool's default button bindings.
pub const SEED_MAPPING: &str = "000100010000000100000000-0000-0000-0000-0000000000000000000E000F001200130014000200030017001800010000";

/// `Mapping` is 100 characters: `version(4) const(4) layout(4)
/// allow-vibration(4) GUID(36) dinput-feedback(4)` then eleven 4-hex
/// button bindings — the format of the game's own settings tool
/// (`c_controllerMapStringLength`; the game parses the same string at
/// `0x589cd0` and gates its rumble on field 4, `0x66e2f8`).
pub const MAPPING_LEN: usize = 100;

/// Chars `[12..16]` of `Mapping`: the pad-vibration switch (`0001` on,
/// `0000` off), the field the game's own XInput page writes from its
/// "Allow vibration" checkbox.
const VIBRATION_RANGE: std::ops::Range<usize> = 12..16;

/// What a rumble write did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RumbleEdit {
    /// The vibration field was set in the player's existing mapping.
    Written,
    /// There was no usable mapping, so the default one was seeded with
    /// the vibration field.
    Seeded,
    /// The mapping is not the known 100-character v0001 format; it was
    /// left exactly as the player has it.
    Foreign,
}

/// The `Mapping` value with the pad-vibration field set to `on`, from the
/// value currently in `config.ini`. Every other character — the pad GUID,
/// the layout, the eleven button bindings — is kept exactly as it is.
pub fn with_rumble(current: Option<&str>, on: bool) -> (String, RumbleEdit) {
    let flag = if on { "0001" } else { "0000" };
    let seed = || {
        let mut out = SEED_MAPPING.to_string();
        out.replace_range(VIBRATION_RANGE, flag);
        out
    };
    let known = |value: &str| {
        value.len() == MAPPING_LEN
            && value[..4].eq_ignore_ascii_case("0001")
            && value.bytes().enumerate().all(|(i, b)| {
                // the GUID field is the tool's GUID ToString(): hex with dashes
                b.is_ascii_hexdigit() || (VIBRATION_RANGE.end..56).contains(&i) && b == b'-'
            })
    };
    match current.map(str::trim) {
        Some(value) if known(value) => {
            let mut out = value.to_string();
            out.replace_range(VIBRATION_RANGE, flag);
            (out, RumbleEdit::Written)
        }
        Some("") | Some("NA") | None => (seed(), RumbleEdit::Seeded),
        Some(value) => (value.to_string(), RumbleEdit::Foreign),
    }
}

/// The picture options as `launcher.cfg` has them: `None` is unset (the
/// game's value stays).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Graphics {
    pub resolution: Option<(u32, u32)>,
    pub quality: Option<Quality>,
    pub vsync: Option<bool>,
}

/// Each option's `[Graphics]` keys and values: `Some` writes them, `None`
/// releases them.
fn owned_keys(graphics: &Graphics) -> Vec<(&'static str, Option<String>)> {
    let mut keys = Vec::new();
    let resolution = graphics.resolution;
    keys.push((
        "HorizontalResolution",
        resolution.map(|(width, _)| width.to_string()),
    ));
    keys.push((
        "VerticalResolution",
        resolution.map(|(_, height)| height.to_string()),
    ));
    let values = graphics.quality.map(quality_values);
    for (i, key) in QUALITY_KEYS.into_iter().enumerate() {
        keys.push((key, values.map(|values| values.0[i].to_string())));
    }
    keys.push((
        "VSync",
        graphics.vsync.map(|on| vsync_value(on).to_string()),
    ));
    keys
}

/// The new config.ini given the result of reading it. A missing file is
/// seeded with `seed` (the effective values); an existing one gets only
/// the keys of the options in `graphics`, byte for byte otherwise; a file
/// that cannot be read is the player's and is not replaced (the error
/// comes back). `owned` is the record of what the launcher wrote. The
/// `rumble` option, when set, writes the vibration field of the
/// `[GameController] Mapping` string the same way; the edit it made comes
/// back with the bytes.
pub fn next_config(
    read: std::io::Result<Vec<u8>>,
    seed: ((u32, u32), Quality, bool),
    graphics: &Graphics,
    rumble: Option<bool>,
    owned: &mut Owned,
) -> std::io::Result<(Vec<u8>, Option<RumbleEdit>)> {
    let existing = match read {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            seed_config(seed.0, seed.1, seed.2).into_bytes()
        }
        Err(error) => return Err(error),
    };
    let mut ini = Ini::from_bytes(&existing).tight();
    for (key, value) in owned_keys(graphics) {
        match value {
            Some(value) => ini::set_owned(&mut ini, owned, SECTION, key, &value),
            None => {
                ini::release(&mut ini, owned, SECTION, key);
            }
        }
    }
    let rumble_edit = match rumble {
        Some(on) => {
            let current = ini.get(CONTROLLER_SECTION, MAPPING_KEY).map(str::to_owned);
            let (value, edit) = with_rumble(current.as_deref(), on);
            ini::set_owned(&mut ini, owned, CONTROLLER_SECTION, MAPPING_KEY, &value);
            Some(edit)
        }
        None => {
            ini::release(&mut ini, owned, CONTROLLER_SECTION, MAPPING_KEY);
            None
        }
    };
    Ok((ini.to_bytes(), rumble_edit))
}

/// The resolution a config.ini holds, if both keys are numbers.
pub fn resolution_in(config: &[u8]) -> Option<(u32, u32)> {
    let ini = Ini::from_bytes(config);
    let number = |key| ini.get(SECTION, key)?.trim().parse::<u32>().ok();
    Some((
        number("HorizontalResolution")?,
        number("VerticalResolution")?,
    ))
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

/// `WINEDLLOVERRIDES` for the game: `d3d9=n,b` when the launcher
/// installed DXVK, and `native` (the DirectX 9 files the player downloaded
/// into the private copy, without `.dll`) as `n,b` too, so Wine loads them
/// instead of its own. Every unrelated rule the caller set is kept (the
/// AppImage's `mscoree,mshtml=` among them) and only the names we own are
/// replaced (`wine::merge_dll_overrides`). With nothing of ours the
/// caller's value is passed through unchanged.
pub fn wine_overrides(existing: Option<&str>, dxvk: bool, native: &[String]) -> Option<String> {
    let mut names: Vec<&str> = Vec::new();
    if dxvk {
        names.push("d3d9");
    }
    names.extend(native.iter().map(String::as_str));
    if names.is_empty() {
        return existing.map(str::to_string);
    }
    Some(wine::merge_dll_overrides(
        &format!("{}=n,b", names.join(",")),
        existing,
    ))
}

/// The game's save file, in [`DocumentsPaths::saved_games`].
pub const SAVE_FILE: &str = "SEGA Rally.dat";
/// The game's local leaderboard, in [`DocumentsPaths::rank_table`].
pub const RANK_TABLE_FILE: &str = "SR_RANKTABLES.DAT";

/// The Documents layout the game expects. The folders are created when
/// missing; the save files inside are the game's own (owner decision
/// 2026-11-02: empty seeds made the game call them corrupt).
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

/// Create the Documents folders the game writes into. Never writes a save
/// file.
pub fn prepare_documents(documents: &DocumentsPaths) -> Result<(), String> {
    for dir in [
        &documents.saved_games,
        &documents.rank_table,
        &documents.ghosts,
    ] {
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    }
    Ok(())
}

/// Remove the save files earlier releases seeded: a zero-byte
/// `SEGA Rally.dat` or `SR_RANKTABLES.DAT` was never written by the game
/// (it called it corrupt), so the game recreates it. Any other file is the
/// player's and stays. Returns how many were removed.
pub fn remove_old_seeds(documents: &DocumentsPaths) -> usize {
    [
        documents.saved_games.join(SAVE_FILE),
        documents.rank_table.join(RANK_TABLE_FILE),
    ]
    .iter()
    .filter(|path| {
        std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file() && meta.len() == 0)
            && std::fs::remove_file(path).is_ok()
    })
    .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_SET: Graphics = Graphics {
        resolution: Some((2560, 1440)),
        quality: Some(Quality::High),
        vsync: Some(true),
    };

    fn update(existing: &str, graphics: &Graphics, owned: &mut Owned) -> String {
        let seed = ((1920, 1080), Quality::High, true);
        let (out, _) = next_config(
            Ok(existing.as_bytes().to_vec()),
            seed,
            graphics,
            None,
            owned,
        )
        .unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn high_matches_the_proven_seed() {
        let text = seed_config((3840, 2160), Quality::High, true);
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
                text.contains(&format!("\n{key}={value}\n")),
                "missing {key}={value}"
            );
        }
        assert_eq!(graphics_keys((1, 1), Quality::Normal, false).len(), 25);
    }

    #[test]
    fn a_missing_file_is_seeded_complete() {
        let mut owned = Owned::default();
        let (out, rumble) = next_config(
            Err(std::io::Error::from(std::io::ErrorKind::NotFound)),
            ((1920, 1080), Quality::Normal, false),
            &Graphics::default(),
            None,
            &mut owned,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(rumble.is_none());
        assert!(text.starts_with("[Audio]\nDriverGUID="));
        assert!(text.contains("[GameController]\nGUID=\n"));
        assert!(text.contains("[Graphics]\nAdapterOrdinal=0\n"));
        assert!(text.contains("HorizontalResolution=1920"));
        assert!(text.contains("VSync=0"));
        assert!(text.ends_with("RunWindowed=0\n"));
        assert!(owned.is_empty(), "nothing set in launcher.cfg");
    }

    #[test]
    fn only_the_set_options_keys_are_written() {
        let existing = "# player file\n[Audio]\nQuality=2\n\n[GameController]\nGUID={1234}\nMapping=ABC\n\n[Graphics]\nHorizontalResolution=800\nVerticalResolution=600\nVSync=0\nGraphicQuality=1\nRunWindowed=1\nCustomKey=keep\n";
        let mut owned = Owned::default();
        let only_vsync = Graphics {
            vsync: Some(true),
            ..Graphics::default()
        };
        let out = update(existing, &only_vsync, &mut owned);
        assert_eq!(
            out,
            existing.replace("VSync=0", "VSync=1"),
            "only VSync changes"
        );
        let out = update(existing, &ALL_SET, &mut owned);
        for kept in [
            "# player file",
            "[Audio]\nQuality=2",
            "GUID={1234}",
            "Mapping=ABC",
            "RunWindowed=1",
            "CustomKey=keep",
        ] {
            assert!(out.contains(kept), "{kept}");
        }
        for written in [
            "HorizontalResolution=2560",
            "VerticalResolution=1440",
            "VSync=1",
            "GraphicQuality=3",
            "MudQuality=1",
        ] {
            assert!(out.contains(written), "{written}");
        }
        assert!(!out.contains("AdapterOrdinal"), "unset keys are not added");
    }

    #[test]
    fn unsetting_restores_only_what_is_still_ours() {
        let existing = "[Graphics]\nHorizontalResolution=800\nVerticalResolution=600\nVSync=0\n";
        let mut owned = Owned::default();
        let written = update(existing, &ALL_SET, &mut owned);
        // The player changes vsync in the game's menu meanwhile.
        let played = written.replace("VSync=1", "VSync=0");
        let out = update(&played, &Graphics::default(), &mut owned);
        assert!(out.contains("HorizontalResolution=800\nVerticalResolution=600\n"));
        assert!(out.contains("VSync=0"));
        // The quality keys did not exist before us: they go again.
        assert!(!out.contains("GraphicQuality"));
        assert!(owned.is_empty());
    }

    #[test]
    fn spaced_and_differently_cased_keys_are_updated_in_place() {
        let existing = "[Audio]\r\nQuality = 2\r\n\r\n[GameController]\r\nGUID = {1234}\r\n\r\n[Graphics]\r\nHorizontalResolution = 1280\r\nverticalresolution=720\r\n";
        let mut owned = Owned::default();
        let graphics = Graphics {
            resolution: Some((3840, 2160)),
            ..Graphics::default()
        };
        let out = update(existing, &graphics, &mut owned);
        assert_eq!(out.matches("HorizontalResolution").count(), 1);
        assert_eq!(
            out.to_ascii_lowercase()
                .matches("verticalresolution")
                .count(),
            1
        );
        assert!(out.contains("HorizontalResolution=3840\r\n"));
        assert!(
            out.starts_with("[Audio]\r\nQuality = 2\r\n\r\n[GameController]\r\nGUID = {1234}\r\n")
        );
        assert!(!out.replace("\r\n", "").contains('\n'), "line endings kept");
        assert_eq!(update(&out, &graphics, &mut owned), out);
        assert_eq!(resolution_in(out.as_bytes()), Some((3840, 2160)));
    }

    #[test]
    fn non_utf8_controller_names_survive_byte_for_byte() {
        let existing = b"[GameController]\r\nGUID={1234}\r\nName=Manette \xE9tendue\r\n\r\n[Graphics]\r\nVSync=0\r\n".to_vec();
        let mut owned = Owned::default();
        let (out, _) = next_config(
            Ok(existing),
            ((1, 1), Quality::High, true),
            &ALL_SET,
            None,
            &mut owned,
        )
        .unwrap();
        assert!(out.starts_with(b"[GameController]\r\nGUID={1234}\r\nName=Manette \xE9tendue\r\n"));
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("VSync=1\r\n"));
        assert_eq!(text.matches("VSync").count(), 1);
    }

    #[test]
    fn a_bom_is_kept_and_graphics_still_matches() {
        let mut owned = Owned::default();
        let (out, _) = next_config(
            Ok(b"\xEF\xBB\xBF[Graphics]\nVSync=0\n".to_vec()),
            ((1, 1), Quality::High, true),
            &ALL_SET,
            None,
            &mut owned,
        )
        .unwrap();
        assert!(out.starts_with(b"\xEF\xBB\xBF[Graphics]\nVSync=1\n"));
        assert_eq!(
            String::from_utf8_lossy(&out).matches("[Graphics]").count(),
            1
        );
    }

    #[test]
    fn an_unreadable_config_is_left_alone() {
        use std::io::{Error, ErrorKind};
        let denied = next_config(
            Err(Error::from(ErrorKind::PermissionDenied)),
            ((1920, 1080), Quality::High, true),
            &ALL_SET,
            None,
            &mut Owned::default(),
        );
        assert_eq!(denied.unwrap_err().kind(), ErrorKind::PermissionDenied);
    }

    /// A mapping the way the game writes it once a pad is bound: the
    /// player's GUID in the middle field.
    const PAD_MAPPING: &str = "0001000200010001a1b2c3d4-e5f6-0000-0000-00a0b0c0d0e00000000E000F001200130014000200030017001800010000";

    #[test]
    fn the_seed_mapping_is_the_format_the_game_parses() {
        assert_eq!(SEED_MAPPING.len(), MAPPING_LEN);
        assert_eq!(&SEED_MAPPING[..4], "0001");
        // allow-vibration on, an empty GUID, dinput feedback off
        assert_eq!(&SEED_MAPPING[12..16], "0001");
        assert_eq!(
            &SEED_MAPPING[16..52],
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(&SEED_MAPPING[52..56], "0000");
        assert_eq!(PAD_MAPPING.len(), MAPPING_LEN);
    }

    #[test]
    fn rumble_edits_only_the_vibration_field() {
        let (value, edit) = with_rumble(Some(PAD_MAPPING), false);
        assert_eq!(edit, RumbleEdit::Written);
        assert_eq!(&value[..12], &PAD_MAPPING[..12]);
        assert_eq!(&value[12..16], "0000");
        assert_eq!(&value[16..], &PAD_MAPPING[16..]);

        let mut disabled = PAD_MAPPING.to_string();
        disabled.replace_range(12..16, "0000");
        let (value, edit) = with_rumble(Some(&disabled), true);
        assert_eq!(edit, RumbleEdit::Written);
        assert_eq!(&value[12..16], "0001");
        assert_eq!(value, PAD_MAPPING);
    }

    #[test]
    fn rumble_off_survives_the_player_remapping_their_pad() {
        // The player binds a new pad in the game while rumble is off:
        // the write is derived from the current string, so the new GUID
        // and bindings survive and only the vibration field is ours.
        let remapped = {
            let mut m = PAD_MAPPING.to_string();
            m.replace_range(16..52, "11223344-5566-0000-0000-001122334455");
            m.replace_range(60..64, "0200");
            m.replace_range(12..16, "0001"); // the game wrote vibration on
            m
        };
        let (value, edit) = with_rumble(Some(&remapped), false);
        assert_eq!(edit, RumbleEdit::Written);
        assert_eq!(&value[16..52], "11223344-5566-0000-0000-001122334455");
        assert_eq!(&value[60..64], "0200");
        assert_eq!(&value[12..16], "0000");
    }

    #[test]
    fn rumble_seeds_when_there_is_no_mapping() {
        for current in [None, Some(""), Some("NA")] {
            let (value, edit) = with_rumble(current, false);
            assert_eq!(edit, RumbleEdit::Seeded, "{current:?}");
            assert_eq!(value.len(), MAPPING_LEN);
            assert_eq!(&value[12..16], "0000");
        }
        let (value, edit) = with_rumble(None, true);
        assert_eq!(edit, RumbleEdit::Seeded);
        assert_eq!(value, SEED_MAPPING);
    }

    #[test]
    fn a_foreign_mapping_is_left_alone() {
        for current in [
            "0002FFFF".to_string() + &"0".repeat(MAPPING_LEN - 8), // another version
            "ABCD".to_string(),                                    // another length
        ] {
            let (value, edit) = with_rumble(Some(&current), false);
            assert_eq!(edit, RumbleEdit::Foreign, "{current}");
            assert_eq!(value, current);
        }
    }

    #[test]
    fn rumble_flows_through_next_config_and_back() {
        let seed = ((1920, 1080), Quality::High, true);
        let player = format!(
            "[GameController]\nGUID={{abcd}}\nMapping={PAD_MAPPING}\n\n[Graphics]\nVSync=0\n"
        );
        let mut owned = Owned::default();

        // off: four characters change, nothing else
        let (out, edit) = next_config(
            Ok(player.as_bytes().to_vec()),
            seed,
            &Graphics::default(),
            Some(false),
            &mut owned,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(edit, Some(RumbleEdit::Written));
        let expected = {
            let mut m = PAD_MAPPING.to_string();
            m.replace_range(12..16, "0000");
            m
        };
        assert_eq!(text, player.replace(PAD_MAPPING, &expected));
        assert!(owned.written(CONTROLLER_SECTION, MAPPING_KEY).is_some());

        // on again from the player's (edited) file: still only that field
        let (out, edit) = next_config(
            Ok(text.as_bytes().to_vec()),
            seed,
            &Graphics::default(),
            Some(true),
            &mut owned,
        )
        .unwrap();
        assert_eq!(edit, Some(RumbleEdit::Written));
        assert_eq!(String::from_utf8(out.clone()).unwrap(), player);

        // unset: the launcher's value is released, the player's is back
        let (out, edit) =
            next_config(Ok(out), seed, &Graphics::default(), None, &mut owned).unwrap();
        assert!(edit.is_none());
        assert_eq!(String::from_utf8(out).unwrap(), player);
        assert!(owned.written(CONTROLLER_SECTION, MAPPING_KEY).is_none());
    }

    #[test]
    fn rumble_unset_after_the_player_changed_the_mapping_leaves_it() {
        let seed = ((1920, 1080), Quality::High, true);
        let mut owned = Owned::default();
        let (off, _) = with_rumble(Some(PAD_MAPPING), false);
        let player_file = format!("[GameController]\nMapping={off}\n\n[Graphics]\nVSync=0\n");
        // the player rebinds their pad on top of our value
        let remapped = {
            let mut m = off.clone();
            m.replace_range(16..52, "11223344-5566-0000-0000-001122334455");
            m
        };
        let player_file = player_file.replace(&off, &remapped);
        let (out, _) = next_config(
            Ok(player_file.as_bytes().to_vec()),
            seed,
            &Graphics::default(),
            None,
            &mut owned,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text, player_file, "the player's mapping is kept as it is");
        // and setting rumble again edits the player's current string
        let (out, edit) = next_config(
            Ok(player_file.as_bytes().to_vec()),
            seed,
            &Graphics::default(),
            Some(false),
            &mut owned,
        )
        .unwrap();
        assert_eq!(edit, Some(RumbleEdit::Written));
        assert!(String::from_utf8(out).unwrap().contains(&{
            let mut m = remapped.clone();
            m.replace_range(12..16, "0000");
            m
        }));
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
    }

    #[test]
    fn wine_overrides_put_ours_first_and_keep_unrelated_rules() {
        let none: &[String] = &[];
        assert_eq!(
            wine_overrides(None, true, none).as_deref(),
            Some("d3d9=n,b")
        );
        assert_eq!(
            wine_overrides(Some("mscoree,mshtml="), true, none).as_deref(),
            Some("d3d9=n,b;mscoree,mshtml=")
        );
        assert_eq!(
            wine_overrides(
                Some("d3d8,D3D9=b;dsound=n;;winemenubuilder.exe=d"),
                true,
                none
            )
            .as_deref(),
            Some("d3d9=n,b;d3d8=b;dsound=n;winemenubuilder.exe=d")
        );
        assert_eq!(
            wine_overrides(Some("d3d9.dll=b;D3D9.DLL,xinput1_3.dll=n"), true, none).as_deref(),
            Some("d3d9=n,b;xinput1_3.dll=n")
        );
        let dx = ["d3dx9_35".to_string(), "d3dcompiler_43".to_string()];
        assert_eq!(
            wine_overrides(Some("d3dx9_35=b;x=n"), true, &dx).as_deref(),
            Some("d3d9,d3dx9_35,d3dcompiler_43=n,b;x=n")
        );
        assert_eq!(
            wine_overrides(None, false, &dx).as_deref(),
            Some("d3dx9_35,d3dcompiler_43=n,b")
        );
        // nothing of ours: the caller's value is left exactly as it was
        assert_eq!(wine_overrides(None, false, none), None);
        assert_eq!(
            wine_overrides(Some("d3d9=b"), false, none).as_deref(),
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

    fn temp(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "srr-launcher-gamecfg-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn documents_get_their_folders_and_no_save_files() {
        // Owner decision 2026-11-02: the game makes its own save files;
        // empty seeds made it report them as corrupt.
        let root = temp("folders");
        let documents = documents_paths(&root.join("me"), &root.join("Public"));
        prepare_documents(&documents).unwrap();
        for dir in [
            &documents.saved_games,
            &documents.rank_table,
            &documents.ghosts,
        ] {
            assert!(dir.is_dir(), "{}", dir.display());
        }
        assert!(!documents.saved_games.join(SAVE_FILE).exists());
        assert!(!documents.rank_table.join(RANK_TABLE_FILE).exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_old_empty_seeds_go_and_real_saves_stay() {
        let root = temp("upgrade");
        let documents = documents_paths(&root.join("me"), &root.join("Public"));
        prepare_documents(&documents).unwrap();
        let save = documents.saved_games.join(SAVE_FILE);
        let ranks = documents.rank_table.join(RANK_TABLE_FILE);
        // Earlier releases left zero-byte files the game never wrote.
        std::fs::write(&save, b"").unwrap();
        std::fs::write(&ranks, b"").unwrap();
        assert_eq!(remove_old_seeds(&documents), 2);
        assert!(!save.exists() && !ranks.exists());
        // Anything else is the player's: kept byte for byte.
        std::fs::write(&save, b"progress").unwrap();
        std::fs::write(&ranks, b"\0").unwrap();
        assert_eq!(remove_old_seeds(&documents), 0);
        assert_eq!(std::fs::read(&save).unwrap(), b"progress");
        assert_eq!(std::fs::read(&ranks).unwrap(), b"\0");
        // Nothing there: nothing to do.
        std::fs::remove_file(&save).unwrap();
        std::fs::remove_file(&ranks).unwrap();
        assert_eq!(remove_old_seeds(&documents), 0);
        std::fs::remove_dir_all(&root).unwrap();
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
        let shared = public
            .join("Documents")
            .join("SEGA Rally")
            .join("Shared Data");
        assert_eq!(paths.rank_table, shared.join("Rank Table"));
        assert_eq!(paths.ghosts, shared.join("Ghosts"));
    }
}
