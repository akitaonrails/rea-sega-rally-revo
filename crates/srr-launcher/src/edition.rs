//! Identification of the known Sega Rally Revo executables by SHA-256.
//!
//! The 2007 PC release ships two game executables and a .NET settings
//! launcher:
//!
//! - `SEGA Rally.exe` — the SSE2 build. The widely mirrored full rip
//!   replaces it with a NoDVD build (supported by the owner's decision of
//!   2026-11-02, see `AGENTS.md`), which is playable but has no clean
//!   reference for the community stutter fix.
//! - `SEGA Rally_SSE1.exe` — the SSE1 fallback build. In the same rip this
//!   file is byte-identical to the clean original, and it is the build the
//!   launcher's (clean-room) stutter fix applies to.
//!
//! Unknown executables are refused with their SHA-256 and the issue link,
//! never patched.

use rea_launcher::identify::{self, Known};

use crate::patches::{MAIN_NODVD, SSE1_CLEAN, SSE1_PACER};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edition {
    pub name: &'static str,
    pub sha256: [u8; 32],
    /// The stutter fix runs on this build: it can be applied
    /// (`patches::PACER`), or it already is.
    pub pacer_fixable: bool,
    /// The build already carries the stutter fix.
    pub pacer_applied: bool,
}

pub const SSE1_EXE: &str = "SEGA Rally_SSE1.exe";
pub const MAIN_EXE: &str = "SEGA Rally.exe";
pub const LAUNCHER_EXE: &str = "SEGARallyLauncher.exe";

/// Where players report an executable the launcher does not know.
pub const REPORT_URL: &str =
    "https://github.com/akitaonrails/rea-sega-rally-revo/issues/new?template=unknown-edition.yml";

/// The clean `SEGA Rally_SSE1.exe` (file version 4.0.6.0, 3,727,360
/// bytes). Matches the reference build of the community stutter fix (MD5
/// 4e71704338c6d7f3ef57ea81fe5b015d).
pub static SSE1_CLEAN_BUILD: Edition = Edition {
    name: "PC v4.0.6.0 (SSE1, clean)",
    sha256: SSE1_CLEAN,
    pacer_fixable: true,
    pacer_applied: false,
};

/// The same executable with the community stutter fix (stuken's patch.py)
/// already applied by the player: run as it is, never patched.
pub static SSE1_PACER_BUILD: Edition = Edition {
    name: "PC v4.0.6.0 (SSE1, stutter fix applied)",
    sha256: SSE1_PACER,
    pacer_fixable: true,
    pacer_applied: true,
};

/// The NoDVD `SEGA Rally.exe` that ships in the same full rip (3,715,072
/// bytes).
pub static MAIN_NODVD_BUILD: Edition = Edition {
    name: "PC v4.0.6.0 (NoDVD SEGA Rally.exe)",
    sha256: MAIN_NODVD,
    pacer_fixable: false,
    pacer_applied: false,
};

/// Every known build by SHA-256 (references to the statics above).
pub static EDITIONS: [Known<&Edition>; 3] = [
    Known {
        sha256: SSE1_CLEAN,
        value: &SSE1_CLEAN_BUILD,
    },
    Known {
        sha256: SSE1_PACER,
        value: &SSE1_PACER_BUILD,
    },
    Known {
        sha256: MAIN_NODVD,
        value: &MAIN_NODVD_BUILD,
    },
];

/// What one executable of the folder is: missing, known, or unknown (its
/// SHA-256).
pub type Found = Option<Result<&'static Edition, [u8; 32]>>;

pub fn identify(exe: &[u8]) -> Result<&'static Edition, [u8; 32]> {
    identify::identify(exe, &EDITIONS).copied()
}

/// Both game executables as found in a game folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameEditions {
    pub main: Found,
    pub sse1: Found,
}

impl GameEditions {
    /// From the executables' bytes (`None`: the file is not there).
    pub fn from_bytes(main: Option<&[u8]>, sse1: Option<&[u8]>) -> GameEditions {
        GameEditions {
            main: main.map(identify),
            sse1: sse1.map(identify),
        }
    }

    pub fn main_known(&self) -> Option<&'static Edition> {
        self.main.and_then(Result::ok)
    }

    pub fn sse1_known(&self) -> Option<&'static Edition> {
        self.sse1.and_then(Result::ok)
    }

    /// True when at least one executable is a known, playable build.
    pub fn playable(&self) -> bool {
        self.main_known().is_some() || self.sse1_known().is_some()
    }

    /// True when the stutter fix can run (the clean SSE1 build, or the
    /// SSE1 build with the fix already applied).
    pub fn pacer_available(&self) -> bool {
        self.sse1_known()
            .is_some_and(|edition| edition.pacer_fixable)
    }

    /// The known builds' names.
    pub fn names(&self) -> Vec<&'static str> {
        [self.main_known(), self.sse1_known()]
            .into_iter()
            .flatten()
            .map(|edition| edition.name)
            .collect()
    }

    /// Each executable that is there but unknown, with its SHA-256.
    pub fn unknown_note(&self) -> Option<String> {
        let notes: Vec<String> = [(MAIN_EXE, self.main), (SSE1_EXE, self.sse1)]
            .into_iter()
            .filter_map(|(name, found)| match found {
                Some(Err(digest)) => Some(format!("{name}: SHA-256 {}", identify::hex(&digest))),
                _ => None,
            })
            .collect();
        (!notes.is_empty()).then(|| notes.join("\n"))
    }

    /// Why this folder cannot be played, for the window and the error
    /// dialog: what is supported, each unknown executable's SHA-256, and
    /// where to report it.
    pub fn refusal(&self) -> String {
        let found = match self.unknown_note() {
            Some(note) => format!("\n\n{note}"),
            None => format!("\n\nNeither {MAIN_EXE} nor {SSE1_EXE} is in this folder."),
        };
        format!(
            "This launcher supports the Sega Rally Revo PC v4.0.6.0 build (the clean \
             {SSE1_EXE} or the NoDVD {MAIN_EXE} of the full rip).{found}\n\n\
             If your copy is another edition, please report it with launcher.log \
             attached: {REPORT_URL}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identified_editions_are_the_statics() {
        for (known, edition) in
            EDITIONS
                .iter()
                .zip([&SSE1_CLEAN_BUILD, &SSE1_PACER_BUILD, &MAIN_NODVD_BUILD])
        {
            let found = identify::identify_digest(known.sha256, &EDITIONS).copied();
            assert!(std::ptr::eq(found.unwrap(), edition), "{}", edition.name);
            assert_eq!(edition.sha256, known.sha256);
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
    fn unknown_exe_reports_its_hash_and_where_to_report() {
        assert_eq!(
            identify(b"not a game"),
            Err(identify::sha256(b"not a game"))
        );
        let folder = GameEditions::from_bytes(Some(b"not a game"), None);
        assert!(!folder.playable());
        let refusal = folder.refusal();
        assert!(refusal.contains(&identify::hex(&identify::sha256(b"not a game"))));
        assert!(refusal.contains(REPORT_URL));
        assert!(refusal.contains("launcher.log"));
        let empty = GameEditions::from_bytes(None, None);
        assert!(empty.refusal().contains("Neither"));
        assert_eq!(empty.unknown_note(), None);
    }

    #[test]
    fn availability_follows_the_clean_sse1_build() {
        let both = GameEditions {
            main: Some(Ok(&MAIN_NODVD_BUILD)),
            sse1: Some(Ok(&SSE1_CLEAN_BUILD)),
        };
        assert!(both.playable());
        assert!(both.pacer_available());
        assert_eq!(both.names().len(), 2);
        let patched = GameEditions {
            main: None,
            sse1: Some(Ok(&SSE1_PACER_BUILD)),
        };
        assert!(patched.pacer_available());
        let nodvd_only = GameEditions {
            main: Some(Ok(&MAIN_NODVD_BUILD)),
            sse1: Some(Err([1; 32])),
        };
        assert!(nodvd_only.playable());
        assert!(!nodvd_only.pacer_available());
        assert!(nodvd_only.unknown_note().is_some());
    }
}
