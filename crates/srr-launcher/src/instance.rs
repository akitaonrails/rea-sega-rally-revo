//! One launcher at a time (SDK `docs/STANDARDS.md`, "Clean exit, single
//! instance"). The launcher holds `rea_launcher_win32::instance::single`
//! for its whole run, including while it waits for the game, so a second
//! start can neither open a second window nor start the game again from
//! the same private copy. This module is the host-testable part: which
//! runs take the instance and what a second start does.

/// The instance name (`Local\rea-launcher-rea-sega-rally-revo`).
pub const NAME: &str = "rea-sega-rally-revo";

/// The settings window's title, which a second start brings forward.
pub const WINDOW_TITLE: &str = "Sega Rally Revo";

/// Exit code of a second start that found no window to bring forward
/// (the game, or a run without a window, is in progress).
pub const ALREADY_RUNNING: u32 = 5;

/// What a second start says when there is no window to bring forward.
pub const BUSY: &str = "Sega Rally Revo is already running from this launcher. \
     Quit the game first, then start the launcher again.";

/// Whether a run with these switches takes the instance. `--check` only
/// reads the game folder, so it may run next to a game; every other run
/// (window, play, `--setup`, `--download-directx`) writes the private
/// copy or the launcher's own files.
pub fn needs_instance(switches: &[String]) -> bool {
    !switches.iter().any(|switch| switch == "--check")
}

/// What a second start does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Second {
    /// The running launcher's window came forward; exit quietly (0).
    Focused,
    /// No window (the game is running): say so and exit with
    /// [`ALREADY_RUNNING`].
    Busy,
}

impl Second {
    /// `bring_to_front` is the SDK's `AlreadyRunning::bring_to_front` for
    /// [`WINDOW_TITLE`] (a fake in tests).
    pub fn of(bring_to_front: impl FnOnce() -> bool) -> Second {
        if bring_to_front() {
            Second::Focused
        } else {
            Second::Busy
        }
    }

    pub fn exit_code(self) -> u32 {
        match self {
            Second::Focused => 0,
            Second::Busy => ALREADY_RUNNING,
        }
    }

    /// The message for the player, if any.
    pub fn message(self) -> Option<&'static str> {
        match self {
            Second::Focused => None,
            Second::Busy => Some(BUSY),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn switches(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn every_run_but_check_takes_the_instance() {
        assert!(needs_instance(&[]));
        assert!(needs_instance(&switches(&["--no-window"])));
        assert!(needs_instance(&switches(&["--settings"])));
        assert!(needs_instance(&switches(&["--setup"])));
        assert!(needs_instance(&switches(&["--download-directx"])));
        assert!(needs_instance(&switches(&["--linux", "--no-window"])));
        assert!(!needs_instance(&switches(&["--check"])));
    }

    #[test]
    fn a_second_start_brings_the_window_forward_or_refuses_to_play() {
        // The first launcher shows its window: it comes forward, no message.
        let asked = Cell::new(0);
        let second = Second::of(|| {
            asked.set(asked.get() + 1);
            true
        });
        assert_eq!(asked.get(), 1);
        assert_eq!(second, Second::Focused);
        assert_eq!((second.exit_code(), second.message()), (0, None));
        // The first launcher is waiting for the game: Play is refused.
        let second = Second::of(|| false);
        assert_eq!(second, Second::Busy);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        assert!(second.message().unwrap().contains("Quit the game first"));
    }

    #[test]
    fn the_exit_code_is_not_one_the_launcher_already_uses() {
        // 1 error/--check refusal, 2 not Windows, 3 window closed, 4 no window.
        assert!(!(0..=4).contains(&ALREADY_RUNNING));
    }
}
