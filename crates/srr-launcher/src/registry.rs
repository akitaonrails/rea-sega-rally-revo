//! The game's registry key, given to it from `HKCU`.
//!
//! Both executables open `HKLM\SOFTWARE\SEGA\SEGA Rally` with
//! `RegOpenKeyExA` (`KEY_READ`) and read `language` (0x57C980) and `sku`
//! (0x666170); nothing reads `installdir`. An unelevated launcher can't
//! write `HKLM`, and the VirtualStore copy Windows makes instead never
//! reaches this game: its `asInvoker` manifest turns UAC registry
//! virtualization off for it. So the launcher writes the values to
//! `HKCU\Software\Sega\SEGA Rally`, and the SDK's close hook DLL, in the
//! runtime copies' import table, sends the game's open of its `HKLM` key
//! there (`REA_REGISTRY`; without `REA_CLOSE_HOOK_ATTACH` the DLL does
//! nothing else). Same path on Windows and under Wine (akitaonrails/
//! rea-colin-mcrae#6 is the same bug class).

use rea_close_hook::registry::{self, Config};
use rea_launcher_win32::reg::{Data, Mismatch, Value, When};

/// Where the game (and the launcher before v0.6.0) kept the values.
pub const MACHINE_KEY: &str = r"SOFTWARE\Sega\SEGA Rally";
/// Where the launcher writes them now (`HKCU`).
pub const USER_KEY: &str = r"Software\Sega\SEGA Rally";
/// The close hook's rule: the game's `HKLM` key, answered from [`USER_KEY`].
pub const RULE: &str = r"HKLM\SOFTWARE\Sega\SEGA Rally -> HKCU\Software\Sega\SEGA Rally";

/// The close hook DLL the runtime exes import.
pub const HOOK_DLL: &str = rea_close_hook::DLL;

/// The variables the game process gets: the registry redirect.
pub fn game_env() -> Vec<(&'static str, String)> {
    let rules = Config::parse(RULE).expect("RULE parses (tested)");
    vec![(registry::ENV, rules.env_value())]
}

/// The game's own values from before (the VirtualStore copy or `HKLM`,
/// `Key::read_machine`), carried over once into `HKCU`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Earlier {
    pub language: Option<String>,
    pub sku: Option<String>,
}

/// The values for [`USER_KEY`]. `language` is written when the option is
/// set; unset, the game's earlier value (else English) only fills a missing
/// one. `sku` follows the language: the Polish menus need `polish` (the
/// full rip's readme; with `Europe` the game shows English), every other
/// language `Europe`.
pub fn values(language: Option<&str>, earlier: &Earlier) -> Vec<Value> {
    let string = |text: &str| Data::String(text.to_string());
    match language {
        Some(language) => {
            let sku = if language.eq_ignore_ascii_case("Polish") {
                "polish"
            } else {
                "Europe"
            };
            vec![
                Value::new("sku", string(sku), When::Always),
                Value::new("language", string(language), When::Always),
            ]
        }
        None => vec![
            Value::new(
                "sku",
                string(earlier.sku.as_deref().unwrap_or("Europe")),
                When::IfMissing,
            ),
            Value::new(
                "language",
                string(earlier.language.as_deref().unwrap_or("English")),
                When::IfMissing,
            ),
        ],
    }
}

/// `values` for the log: `sku = "Europe", language = "German"`, each
/// seeded-only one marked.
pub fn describe(values: &[Value]) -> String {
    values
        .iter()
        .map(|value| match value.when {
            When::Always => format!("{} = {}", value.name, value.data),
            When::IfMissing => format!("{} = {} (if missing)", value.name, value.data),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The runtime copy of a game exe (`bytes`: the player's, patched) with the
/// close hook DLL in its import table, so Windows loads it with the game.
/// Unchanged when the import is already there. Both game exes mark every
/// import descriptor bound with no bound import directory; the SDK clears
/// that stale binding (time stamp and forwarder chain) as it adds the import.
pub fn with_hook(bytes: &[u8]) -> Result<Vec<u8>, String> {
    rea_close_hook::import_into(bytes)
        .map_err(|error| format!("{HOOK_DLL} can't be added to its import table ({error})"))
}

/// Why the game would not see its key, for `--check`: the DLL isn't next to
/// the launcher (`dll_present`), an exe can't take the import, or the rule
/// doesn't parse. Empty when everything is in place.
pub fn hook_problems(dll_present: bool, exes: &[(&str, &[u8])]) -> Vec<String> {
    let mut problems = Vec::new();
    if !dll_present {
        problems.push(format!(
            "{HOOK_DLL} is missing: keep it next to SRRLauncher.exe (it comes in the same zip)."
        ));
    }
    for (name, bytes) in exes {
        if let Err(error) = with_hook(bytes) {
            problems.push(format!("{name}: {error}."));
        }
    }
    if let Err(error) = Config::parse(RULE) {
        problems.push(format!("{error}."));
    }
    problems
}

/// What a player reads when the values didn't land in `HKCU`.
pub fn not_saved(mismatches: &[Mismatch]) -> String {
    let list: Vec<String> = mismatches.iter().map(|m| format!("- {m}")).collect();
    format!(
        "The game's language could not be saved in HKCU\\{USER_KEY}, so the game would not \
         read it:\n{}\n\nThe launcher needs no administrator rights for this: it writes only \
         your own part of the registry. Check that nothing (a security program, a policy) \
         blocks writing HKEY_CURRENT_USER, then try again. If it keeps failing, please \
         report it: https://github.com/akitaonrails/rea-sega-rally-revo/issues",
        list.join("\n")
    )
}

/// The game's values from before, read-only: the VirtualStore copy first,
/// then `HKLM` (where the game itself looked). Unreadable counts as absent.
#[cfg(windows)]
pub fn earlier() -> Earlier {
    use rea_launcher_win32::reg::Key;
    let Ok(old) = Key::read_machine(MACHINE_KEY) else {
        return Earlier::default();
    };
    let read = |name: &str| old.string(name).ok().flatten();
    Earlier {
        language: read("language"),
        sku: read("sku"),
    }
}

/// Writes `values` to [`USER_KEY`] and reads them back; the error is the
/// player's message ([`not_saved`]).
#[cfg(windows)]
pub fn write(values: &[Value]) -> Result<(), String> {
    use rea_launcher_win32::reg::{Hive, write_verified};
    write_verified(Hive::CurrentUser, USER_KEY, values).map_err(|wrong| not_saved(&wrong))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_close_hook::registry::{Key, Root, Target};
    use rea_launcher_win32::reg::{Problem, verify};

    fn data(values: &[Value], name: &str) -> (String, When) {
        let value = values.iter().find(|v| v.name == name).unwrap();
        match &value.data {
            Data::String(text) => (text.clone(), value.when),
            Data::Dword(_) => panic!("{name} is a string"),
        }
    }

    #[test]
    fn the_rule_sends_the_games_key_and_subkeys_to_hkcu() {
        let rules = Config::parse(RULE).unwrap();
        // The game spells it SEGA; the rule matches any case.
        let game = Key::new(Root::LocalMachine, r"SOFTWARE\SEGA\SEGA Rally");
        let Some(Target { key, .. }) = rules.map(&game) else {
            panic!("the game's key is redirected");
        };
        assert_eq!(key, Key::new(Root::CurrentUser, USER_KEY));
        assert_eq!(
            rules.map(&Key::new(Root::LocalMachine, r"SOFTWARE\Sega")),
            None
        );
        assert_eq!(
            rules.map(&Key::new(Root::LocalMachine, r"SOFTWARE\Microsoft\DirectX")),
            None
        );
        assert_eq!(
            game_env(),
            [(
                "REA_REGISTRY",
                r"HKLM\SOFTWARE\Sega\SEGA Rally -> HKCU\Software\Sega\SEGA Rally".to_string()
            )]
        );
        assert_eq!(Config::parse(&game_env()[0].1).unwrap(), rules);
    }

    #[test]
    fn a_chosen_language_is_written_with_its_sku() {
        let earlier = Earlier {
            language: Some("Italian".into()),
            sku: Some("polish".into()),
        };
        let german = values(Some("German"), &earlier);
        assert_eq!(data(&german, "language"), ("German".into(), When::Always));
        assert_eq!(data(&german, "sku"), ("Europe".into(), When::Always));
        // The full rip's readme: Polish shows only with sku "polish"; with
        // "Europe" the game falls back to English (seen in the game,
        // 2026-11-02).
        let polish = values(Some("Polish"), &Earlier::default());
        assert_eq!(data(&polish, "sku"), ("polish".into(), When::Always));
        // Nothing reads installdir: it is no longer written.
        assert!(german.iter().all(|v| v.name != "installdir"));
        assert_eq!(german.len(), 2);
    }

    #[test]
    fn unset_the_players_earlier_values_carry_over() {
        let fresh = values(None, &Earlier::default());
        assert_eq!(
            data(&fresh, "language"),
            ("English".into(), When::IfMissing)
        );
        assert_eq!(data(&fresh, "sku"), ("Europe".into(), When::IfMissing));
        let earlier = Earlier {
            language: Some("Polish".into()),
            sku: Some("polish".into()),
        };
        let kept = values(None, &earlier);
        assert_eq!(data(&kept, "language"), ("Polish".into(), When::IfMissing));
        assert_eq!(data(&kept, "sku"), ("polish".into(), When::IfMissing));
        assert_eq!(
            describe(&kept),
            r#"sku = "polish" (if missing), language = "Polish" (if missing)"#
        );
    }

    #[test]
    fn a_value_that_did_not_land_is_named_for_the_player() {
        let wanted = values(Some("French"), &Earlier::default());
        // Read back: language holds something else, sku is gone.
        let wrong = verify(&wanted, |value| {
            Ok(match value.name.as_str() {
                "language" => Some(Data::String("English".into())),
                _ => None,
            })
        });
        assert_eq!(wrong.len(), 2);
        assert!(matches!(wrong[0].problem, Problem::Missing));
        let text = not_saved(&wrong);
        assert!(text.contains(r"HKCU\Software\Sega\SEGA Rally"), "{text}");
        assert!(text.contains("- sku is missing"), "{text}");
        assert!(
            text.contains(r#"- language is "English", not "French""#),
            "{text}"
        );
        assert!(text.contains("no administrator rights"), "{text}");
        // Read back as written: nothing to report.
        assert!(verify(&wanted, |value| Ok(Some(value.data.clone()))).is_empty());
    }

    /// A PE32 like the game's: one import descriptor stamped bound
    /// (`0xFFFFFFFF`), no name table, no bound import directory; its
    /// address table holds `thunk`.
    fn stale_bound_image(thunk: u32) -> Vec<u8> {
        let mut image = vec![0u8; 0x600];
        let put = |image: &mut Vec<u8>, at: usize, value: u32| {
            image[at..at + 4].copy_from_slice(&value.to_le_bytes())
        };
        image[0..2].copy_from_slice(b"MZ");
        put(&mut image, 0x3C, 0x80);
        image[0x80..0x84].copy_from_slice(b"PE\0\0");
        image[0x84..0x86].copy_from_slice(&0x14Cu16.to_le_bytes());
        image[0x86..0x88].copy_from_slice(&1u16.to_le_bytes());
        image[0x94..0x96].copy_from_slice(&224u16.to_le_bytes());
        image[0x96..0x98].copy_from_slice(&0x0102u16.to_le_bytes());
        let optional = 0x98;
        image[optional..optional + 2].copy_from_slice(&0x10Bu16.to_le_bytes());
        put(&mut image, optional + 16, 0x1000);
        put(&mut image, optional + 28, 0x40_0000);
        put(&mut image, optional + 32, 0x1000);
        put(&mut image, optional + 36, 0x200);
        image[optional + 40..optional + 42].copy_from_slice(&4u16.to_le_bytes());
        put(&mut image, optional + 56, 0x2000);
        put(&mut image, optional + 60, 0x400);
        image[optional + 68..optional + 70].copy_from_slice(&2u16.to_le_bytes());
        put(&mut image, optional + 92, 16);
        put(&mut image, optional + 96 + 8, 0x1000); // imports
        put(&mut image, optional + 96 + 12, 40);
        let section = optional + 224;
        image[section..section + 6].copy_from_slice(b".idata");
        put(&mut image, section + 8, 0x200);
        put(&mut image, section + 12, 0x1000);
        put(&mut image, section + 16, 0x200);
        put(&mut image, section + 20, 0x400);
        put(&mut image, section + 36, 0xC000_0040);
        let data = 0x400; // RVA 0x1000
        put(&mut image, data + 4, 0xFFFF_FFFF);
        put(&mut image, data + 8, 0xFFFF_FFFF);
        put(&mut image, data + 12, 0x1080);
        put(&mut image, data + 16, 0x1040);
        put(&mut image, data + 0x40, thunk);
        image[data + 0x62..data + 0x70].copy_from_slice(b"RegOpenKeyExA\0");
        image[data + 0x80..data + 0x8D].copy_from_slice(b"ADVAPI32.dll\0");
        image
    }

    #[test]
    fn a_stale_binding_like_the_games_takes_the_import() {
        let image = stale_bound_image(0x1060);
        let hooked = with_hook(&image).unwrap();
        let imports = rea_launcher::pe::edit::imports(&hooked).unwrap();
        let dlls: Vec<&str> = imports.iter().map(|i| i.dll.as_str()).collect();
        assert_eq!(dlls, ["ADVAPI32.dll", HOOK_DLL]);
        assert_eq!(imports[0].functions.len(), 1);
        // A table of bound addresses is not ours to unbind: refused.
        let bound = stale_bound_image(0x7700_1234);
        assert!(with_hook(&bound).is_err());
    }

    #[test]
    fn check_names_a_missing_dll_and_an_exe_that_cannot_take_the_import() {
        assert!(hook_problems(true, &[]).is_empty());
        let problems = hook_problems(false, &[("SEGA Rally.exe", b"not a PE")]);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].starts_with("rea_close_hook.dll is missing"));
        assert!(problems[1].starts_with("SEGA Rally.exe: rea_close_hook.dll can't be added"));
    }

    /// Under Wine (`sdk/scripts/wine-test`), in the test prefix: an earlier
    /// release's HKLM value is carried over into HKCU, a chosen language
    /// is written over it, and nothing is written to HKLM.
    #[cfg(windows)]
    #[test]
    fn values_land_in_hkcu_and_earlier_ones_carry_over() {
        use rea_launcher_win32::reg::{Hive, Key};
        let read = |hive: Hive, path: &str, name: &str| {
            Key::open_read(hive, path)
                .unwrap()
                .and_then(|key| key.string(name).unwrap())
        };
        let user = |name: &str| read(Hive::CurrentUser, USER_KEY, name);
        let machine = |name: &str| read(Hive::LocalMachine, MACHINE_KEY, name);
        let clear = |hive: Hive, path: &str| {
            if let Ok(Some(key)) = Key::open_read(hive, path) {
                drop(key);
                let key = Key::create(hive, path).unwrap();
                key.delete_value("language").unwrap();
                key.delete_value("sku").unwrap();
            }
        };
        clear(Hive::CurrentUser, USER_KEY);
        clear(Hive::LocalMachine, MACHINE_KEY);
        assert_eq!((user("language"), machine("language")), (None, None));

        // No value anywhere: the defaults, in HKCU only.
        write(&values(None, &earlier())).unwrap();
        assert_eq!(user("language").as_deref(), Some("English"));
        assert_eq!(user("sku").as_deref(), Some("Europe"));
        assert_eq!(machine("language"), None, "nothing is written to HKLM");

        // A player of an earlier release: Polish in HKLM, nothing in HKCU.
        clear(Hive::CurrentUser, USER_KEY);
        let old = Key::machine(MACHINE_KEY).unwrap();
        old.set_string("language", "Polish").unwrap();
        old.set_string("sku", "polish").unwrap();
        write(&values(None, &earlier())).unwrap();
        assert_eq!(user("language").as_deref(), Some("Polish"));
        assert_eq!(user("sku").as_deref(), Some("polish"));

        // Chosen in the launcher: written over the carried-over value; the
        // old key is only read.
        write(&values(Some("German"), &earlier())).unwrap();
        assert_eq!(user("language").as_deref(), Some("German"));
        assert_eq!(user("sku").as_deref(), Some("Europe"));
        assert_eq!(machine("language").as_deref(), Some("Polish"));
        clear(Hive::LocalMachine, MACHINE_KEY);
    }
}
