//! Identification of the known Sega Rally Revo executables by SHA-256.
//!
//! The 2007 PC release ships two game executables and a .NET settings
//! launcher:
//!
//! - `SEGA Rally.exe` — the SSE2 build. The widely mirrored full-rip
//!   replaces it with a NoDVD build, which is playable but has no known
//!   clean reference for community binary patches.
//! - `SEGA Rally_SSE1.exe` — the SSE1 fallback build. In the same rip this
//!   file is byte-identical to the clean original, and it is the build the
//!   community stutter fix targets.
//!
//! Unknown executables are rejected, never patched.

use rea_launcher::identify::{self, Known};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition {
    pub name: &'static str,
    /// The community pacer fix (see `patches::pacer`) applies to this build.
    pub pacer_fixable: bool,
}

pub const SSE1_EXE: &str = "SEGA Rally_SSE1.exe";
pub const MAIN_EXE: &str = "SEGA Rally.exe";
pub const LAUNCHER_EXE: &str = "SEGARallyLauncher.exe";

/// The clean `SEGA Rally_SSE1.exe` from the EN-FR-DE-ES-IT-PL full-rip
/// (file version 4.0.6.0, 3,727,360 bytes). Matches the reference build of
/// the community stutter fix (MD5 4e71704338c6d7f3ef57ea81fe5b015d).
pub static SSE1_CLEAN: Edition = Edition {
    name: "PC v4.0.6.0 (SSE1, clean)",
    pacer_fixable: true,
};

/// The same executable after `patches::pacer` was applied.
pub static SSE1_PACER: Edition = Edition {
    name: "PC v4.0.6.0 (SSE1, stutter fix applied)",
    pacer_fixable: true,
};

/// The NoDVD `SEGA Rally.exe` that ships in the same full-rip. Playable,
/// but no clean reference exists for the pacer fix.
pub static MAIN_NODVD: Edition = Edition {
    name: "PC v4.0.6.0 (NoDVD rip)",
    pacer_fixable: false,
};

/// SHA-256 of [`SSE1_PACER`]: a patched copy is accepted only when it
/// hashes to this.
pub const SSE1_PACER_SHA256: [u8; 32] =
    identify::hex32("4c08c1deae28a329b6588138198556ac8eaded8444d0eb94a1fe2f1ad08f2520");

/// Every known build by SHA-256. The values are references to the statics
/// above (not copies), so callers can compare an identified edition
/// against them by address.
pub static EDITIONS: [Known<&Edition>; 3] = [
    Known::new(
        "22655a54bad6932239ade58325703bd4b5a2020376737748026443fa5aadf76e",
        &SSE1_CLEAN,
    ),
    Known {
        sha256: SSE1_PACER_SHA256,
        value: &SSE1_PACER,
    },
    Known::new(
        "ad84ecea6ac5e86e1f1a420f07595866429bba63e7c489b40661a8032ba391d7",
        &MAIN_NODVD,
    ),
];

pub fn identify(exe: &[u8]) -> Result<&'static Edition, [u8; 32]> {
    identify::identify(exe, &EDITIONS).copied()
}

/// Both game executables as found in a game folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEditions {
    pub main: Result<&'static Edition, [u8; 32]>,
    pub sse1: Result<&'static Edition, [u8; 32]>,
}

impl GameEditions {
    /// True when at least one executable is a known, playable build.
    pub fn playable(&self) -> bool {
        self.main.is_ok() || self.sse1.is_ok()
    }

    /// True when the pacer fix can be applied (clean SSE1 present) or has
    /// already been applied in this folder.
    pub fn pacer_available(&self) -> bool {
        matches!(&self.sse1, Ok(edition) if edition.pacer_fixable)
    }

    /// Unknown-executable detail for error messages.
    pub fn unknown_note(&self) -> Option<String> {
        let mut notes = Vec::new();
        for (name, result) in [("SEGA Rally.exe", &self.main), (SSE1_EXE, &self.sse1)] {
            if let Err(digest) = result {
                notes.push(format!("{name}: {}", identify::hex(digest)));
            }
        }
        (!notes.is_empty()).then(|| notes.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identified_editions_are_the_statics() {
        for (known, edition) in EDITIONS.iter().zip([&SSE1_CLEAN, &SSE1_PACER, &MAIN_NODVD]) {
            let found = identify::identify_digest(known.sha256, &EDITIONS).copied();
            assert!(std::ptr::eq(found.unwrap(), edition), "{}", edition.name);
        }
        assert!(identify::identify_digest([0; 32], &EDITIONS).is_err());
    }

    #[test]
    fn hashes_are_distinct() {
        let mut seen = Vec::new();
        for known in &EDITIONS {
            assert!(!seen.contains(&known.sha256));
            seen.push(known.sha256);
        }
    }

    #[test]
    fn unknown_exe_reports_its_hash() {
        assert_eq!(
            identify(b"not a game"),
            Err(identify::sha256(b"not a game"))
        );
    }

    #[test]
    fn availability_follows_the_clean_sse1_build() {
        let both = GameEditions {
            main: Ok(&MAIN_NODVD),
            sse1: Ok(&SSE1_CLEAN),
        };
        assert!(both.playable());
        assert!(both.pacer_available());
        let patched = GameEditions {
            main: Ok(&MAIN_NODVD),
            sse1: Ok(&SSE1_PACER),
        };
        assert!(patched.pacer_available());
        let nodvd_only = GameEditions {
            main: Ok(&MAIN_NODVD),
            sse1: Err([0; 32]),
        };
        assert!(nodvd_only.playable());
        assert!(!nodvd_only.pacer_available());
        assert!(nodvd_only.unknown_note().is_some());
    }
}
