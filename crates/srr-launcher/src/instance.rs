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

/// What a second `--download-directx` says when the settings window is
/// open: that window may be downloading the same files right now.
pub const WINDOW_OPEN: &str = "The launcher window is open; use its Download button. \
     (--download-directx works when the window is closed.)";

/// Whether a run with these switches is an action that does its work
/// without the window (SDK v0.3.2 "action flags"): a second start must
/// not bring the window forward and exit 0 as if the work were done.
pub fn is_action(switches: &[String]) -> bool {
    switches.iter().any(|switch| switch == "--download-directx")
}

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
    /// An action flag found the settings window open: say to use the
    /// window, leave it where it is, exit with [`ALREADY_RUNNING`].
    WindowOpen,
}

impl Second {
    /// `bring_to_front` is the SDK's `AlreadyRunning::bring_to_front` for
    /// [`WINDOW_TITLE`] (a fake in tests).
    /// For an action flag (`action`), `window_found` is the SDK's
    /// `AlreadyRunning::is_window_found` (a fake in tests): the window is
    /// never brought forward and the run always fails.
    pub fn of(
        action: bool,
        window_found: impl FnOnce() -> bool,
        bring_to_front: impl FnOnce() -> bool,
    ) -> Second {
        if action {
            if window_found() {
                Second::WindowOpen
            } else {
                Second::Busy
            }
        } else if bring_to_front() {
            Second::Focused
        } else {
            Second::Busy
        }
    }

    pub fn exit_code(self) -> u32 {
        match self {
            Second::Focused => 0,
            Second::Busy | Second::WindowOpen => ALREADY_RUNNING,
        }
    }

    /// The message for the player, if any.
    pub fn message(self) -> Option<&'static str> {
        match self {
            Second::Focused => None,
            Second::Busy => Some(BUSY),
            Second::WindowOpen => Some(WINDOW_OPEN),
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
        let second = Second::of(
            false,
            || unreachable!(),
            || {
                asked.set(asked.get() + 1);
                true
            },
        );
        assert_eq!(asked.get(), 1);
        assert_eq!(second, Second::Focused);
        assert_eq!((second.exit_code(), second.message()), (0, None));
        // The first launcher is waiting for the game: Play is refused.
        let second = Second::of(false, || unreachable!(), || false);
        assert_eq!(second, Second::Busy);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        assert!(second.message().unwrap().contains("Quit the game first"));
    }

    #[test]
    fn download_directx_with_the_window_open_says_to_use_it_and_fails() {
        let action = switches(&["--download-directx"]);
        assert!(is_action(&action));
        assert!(!is_action(&switches(&["--no-window"])));
        assert!(!is_action(&[]));
        // The window is open: it is not brought forward, the run fails.
        let second = Second::of(is_action(&action), || true, || unreachable!());
        assert_eq!(second, Second::WindowOpen);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        assert!(
            second
                .message()
                .unwrap()
                .contains("use its Download button")
        );
        // The game (or another action) holds the instance: busy, failed.
        let second = Second::of(true, || false, || unreachable!());
        assert_eq!(second, Second::Busy);
        assert_ne!(second.exit_code(), 0);
    }

    #[test]
    fn the_exit_code_is_not_one_the_launcher_already_uses() {
        // 1 error/--check refusal, 2 not Windows, 3 window closed, 4 no window.
        assert!(!(0..=4).contains(&ALREADY_RUNNING));
    }
}
