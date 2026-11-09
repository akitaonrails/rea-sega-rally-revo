//! The `keys` option: the player's own keyboard keys for the game's eleven
//! keyboard actions (issue #1).
//!
//! The game reads the keyboard through DirectInput into a 256-byte state
//! array and asks four small accessors of its input object "is key K down
//! (now, last frame, just pressed, just released)?", K being a fixed
//! DirectInput key code. The key set (`patches::KEYS_*`) puts a 256-byte
//! table in front of those accessors: the game asks for key K and gets the
//! state of key `table[K]`. Every key the player doesn't move stays itself,
//! so the menus (Enter, Escape, arrows) work as before. `docs/RESEARCH.md`
//! ("Keyboard keys") has the evidence.

use std::fmt::Write as _;

/// The game's keyboard actions, their `keys` names and the manual's keys
/// (DirectInput codes).
pub const ACTIONS: [(&str, &str, u8); 11] = [
    ("accelerate", "Accelerate", 0x1E),            // A
    ("brake", "Brake / reverse", 0x2C),            // Z
    ("steer_left", "Steer left", 0x33),            // ,
    ("steer_right", "Steer right", 0x34),          // .
    ("handbrake", "Handbrake", 0x39),              // Space
    ("shift_up", "Shift up", 0x1F),                // S
    ("shift_down", "Shift down", 0x2D),            // X
    ("camera", "Change camera", 0x1D),             // Left Ctrl
    ("driver_names", "Driver names", 0x0F),        // Tab
    ("class_up", "Time Trial class up", 0x18),     // O
    ("class_down", "Time Trial class down", 0x19), // P
];

/// The keys a player can choose, by their US-keyboard position (DirectInput
/// reads positions, not letters). Escape, Enter and the F keys are left
/// out: the game's menus and pause need them.
pub const KEYS: &[(&str, u8)] = &[
    ("a", 0x1E),
    ("b", 0x30),
    ("c", 0x2E),
    ("d", 0x20),
    ("e", 0x12),
    ("f", 0x21),
    ("g", 0x22),
    ("h", 0x23),
    ("i", 0x17),
    ("j", 0x24),
    ("k", 0x25),
    ("l", 0x26),
    ("m", 0x32),
    ("n", 0x31),
    ("o", 0x18),
    ("p", 0x19),
    ("q", 0x10),
    ("r", 0x13),
    ("s", 0x1F),
    ("t", 0x14),
    ("u", 0x16),
    ("v", 0x2F),
    ("w", 0x11),
    ("x", 0x2D),
    ("y", 0x15),
    ("z", 0x2C),
    ("1", 0x02),
    ("2", 0x03),
    ("3", 0x04),
    ("4", 0x05),
    ("5", 0x06),
    ("6", 0x07),
    ("7", 0x08),
    ("8", 0x09),
    ("9", 0x0A),
    ("0", 0x0B),
    ("up", 0xC8),
    ("down", 0xD0),
    ("left", 0xCB),
    ("right", 0xCD),
    ("space", 0x39),
    ("tab", 0x0F),
    ("backspace", 0x0E),
    ("left_shift", 0x2A),
    ("right_shift", 0x36),
    ("left_ctrl", 0x1D),
    ("right_ctrl", 0x9D),
    ("left_alt", 0x38),
    ("right_alt", 0xB8),
    ("comma", 0x33),
    ("period", 0x34),
    ("slash", 0x35),
    ("semicolon", 0x27),
    ("apostrophe", 0x28),
    ("grave", 0x29),
    ("minus", 0x0C),
    ("equals", 0x0D),
    ("left_bracket", 0x1A),
    ("right_bracket", 0x1B),
    ("backslash", 0x2B),
    ("insert", 0xD2),
    ("delete", 0xD3),
    ("home", 0xC7),
    ("end", 0xCF),
    ("page_up", 0xC9),
    ("page_down", 0xD1),
    ("numpad_0", 0x52),
    ("numpad_1", 0x4F),
    ("numpad_2", 0x50),
    ("numpad_3", 0x51),
    ("numpad_4", 0x4B),
    ("numpad_5", 0x4C),
    ("numpad_6", 0x4D),
    ("numpad_7", 0x47),
    ("numpad_8", 0x48),
    ("numpad_9", 0x49),
    ("numpad_plus", 0x4E),
    ("numpad_minus", 0x4A),
    ("numpad_multiply", 0x37),
    ("numpad_divide", 0xB5),
    ("numpad_period", 0x53),
    ("numpad_enter", 0x9C),
];

/// The keys the player moved: for each action (index into [`ACTIONS`]) the
/// key it now uses. Every other action keeps the manual's key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyMap {
    moved: Vec<(usize, u8)>,
}

fn key_code(name: &str) -> Option<u8> {
    KEYS.iter().find(|(k, _)| *k == name).map(|(_, code)| *code)
}

fn key_name(code: u8) -> &'static str {
    KEYS.iter()
        .find(|(_, c)| *c == code)
        .map(|(k, _)| *k)
        .expect("every action's key is in KEYS")
}

/// How the window shows a key: `Up`, `Left shift`, `Numpad 8`, `A`.
pub fn label(code: u8) -> String {
    let name = key_name(code).replace('_', " ");
    let mut chars = name.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

impl KeyMap {
    /// `accelerate=up, brake=down` (any case, `-` for `_`, spaces allowed);
    /// empty is the game's keys. The error says what is wrong.
    pub fn parse(text: &str) -> Result<KeyMap, String> {
        let mut moved: Vec<(usize, u8)> = Vec::new();
        for pair in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let normal = pair.to_ascii_lowercase().replace('-', "_");
            let Some((action, key)) = normal.split_once('=') else {
                return Err(format!("action=key pairs, not {pair:?}"));
            };
            let (action, key) = (action.trim(), key.trim());
            let Some(index) = ACTIONS.iter().position(|(name, _, _)| *name == action) else {
                return Err(format!(
                    "one of the actions {}, not {action:?}",
                    action_list()
                ));
            };
            let Some(code) = key_code(key) else {
                return Err(format!(
                    "a key name such as up, left_shift or numpad_8, not {key:?}"
                ));
            };
            if moved.iter().any(|(i, _)| *i == index) {
                return Err(format!("each action once ({action} is given twice)"));
            }
            moved.push((index, code));
        }
        moved.sort_unstable();
        let map = KeyMap { moved };
        let layout = map.layout();
        for (i, code) in layout.iter().enumerate() {
            if let Some(j) = layout[..i].iter().position(|c| c == code) {
                return Err(format!(
                    "a different key per action ({} is both {} and {})",
                    key_name(*code),
                    ACTIONS[j].0,
                    ACTIONS[i].0
                ));
            }
        }
        Ok(map)
    }

    /// The key every action uses, in [`ACTIONS`] order.
    pub fn layout(&self) -> [u8; 11] {
        let mut keys = ACTIONS.map(|(_, _, code)| code);
        for (index, code) in &self.moved {
            keys[*index] = *code;
        }
        keys
    }

    /// True when every action has the manual's key (no patch needed).
    pub fn is_default(&self) -> bool {
        self.layout() == ACTIONS.map(|(_, _, code)| code)
    }

    /// The table the game's key accessors read through: entry K is the key
    /// whose state the game sees when it asks for key K. An action's key
    /// answers for that action's original key; every other key is itself.
    pub fn table(&self) -> [u8; 256] {
        let mut table = [0u8; 256];
        for (k, entry) in table.iter_mut().enumerate() {
            *entry = k as u8;
        }
        for (index, code) in &self.moved {
            table[ACTIONS[*index].2 as usize] = *code;
        }
        table
    }

    /// The canonical text (`accelerate=up, brake=down`), empty for none.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (index, code) in &self.moved {
            if !out.is_empty() {
                out.push_str(", ");
            }
            let _ = write!(out, "{}={}", ACTIONS[*index].0, key_name(*code));
        }
        out
    }
}

fn action_list() -> String {
    ACTIONS.map(|(name, _, _)| name).join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_manual_keys() {
        let keys = KeyMap::default();
        assert!(keys.is_default());
        let table = keys.table();
        assert!(table.iter().enumerate().all(|(k, v)| k as u8 == *v));
        assert_eq!(keys.text(), "");
        assert_eq!(label(0x2A), "Left shift");
        assert_eq!(label(0x1E), "A");
        // Every manual key has a name, so the window can show it.
        for (_, _, code) in ACTIONS {
            assert!(KEYS.iter().any(|(_, c)| *c == code));
        }
    }

    #[test]
    fn a_moved_action_reads_its_new_key_and_the_rest_stay() {
        let keys = KeyMap::parse("Accelerate = UP, steer-left=left").unwrap();
        assert_eq!(keys.text(), "accelerate=up, steer_left=left");
        let table = keys.table();
        assert_eq!(table[0x1E], 0xC8, "A (accelerate) reads Up");
        assert_eq!(table[0x33], 0xCB, "comma (steer left) reads Left");
        assert_eq!(table[0xC8], 0xC8, "Up itself still answers for Up (menus)");
        assert_eq!(table[0x2C], 0x2C, "Z (brake) unchanged");
        assert!(!keys.is_default());
        assert_eq!(keys.layout()[0], 0xC8);
        // Naming the manual key is the default again.
        assert!(KeyMap::parse("accelerate=a").unwrap().is_default());
        assert_eq!(KeyMap::parse("  ").unwrap(), KeyMap::default());
    }

    #[test]
    fn two_actions_on_one_key_are_refused() {
        let error = KeyMap::parse("accelerate=z").unwrap_err();
        assert!(error.contains("z is both accelerate and brake"), "{error}");
        // Moving brake too frees Z.
        assert!(KeyMap::parse("accelerate=z, brake=a").is_ok());
        let error = KeyMap::parse("accelerate=up, brake=up").unwrap_err();
        assert!(error.contains("up is both"), "{error}");
    }

    #[test]
    fn bad_text_says_what_is_wrong() {
        assert!(
            KeyMap::parse("fly=up")
                .unwrap_err()
                .contains("accelerate, brake")
        );
        assert!(
            KeyMap::parse("accelerate=escape")
                .unwrap_err()
                .contains("key name")
        );
        assert!(KeyMap::parse("accelerate=enter").is_err());
        assert!(
            KeyMap::parse("accelerate up")
                .unwrap_err()
                .contains("action=key")
        );
        assert!(
            KeyMap::parse("brake=down, brake=s")
                .unwrap_err()
                .contains("twice")
        );
    }
}
