//! One launcher at a time (SDK `docs/STANDARDS.md`, "Clean exit, single
//! instance" and "Action flags and one instance"). Every run holds
//! `rea_launcher_win32::instance::single` for its whole run, including
//! while it waits for the game and for `--check`, so a second start can
//! neither open a second window nor start the game again from the same
//! private copy. What a second start does is the SDK's
//! `instance::SecondStart` (`AlreadyRunning::second_start` with
//! `Window::Own(WINDOW_TITLE)`, the action from `instance::is_action`);
//! this module is the host-testable part that is ours: what the player is
//! told.

use rea_launcher_win32::instance::{self, SecondStart};

/// The instance name (`Local\rea-launcher-rea-sega-rally-revo`).
pub const NAME: &str = "rea-sega-rally-revo";

/// The settings window's title, which a second start brings forward.
pub const WINDOW_TITLE: &str = "Sega Rally Revo";

/// The game's name in the messages.
pub const GAME: &str = "Sega Rally Revo";

/// What a second `--download-directx` says when the settings window is
/// open: that window may be downloading the same files right now.
pub const WINDOW_OPEN: &str = "The launcher window is open; use its Download button. \
     (--download-directx works when the window is closed.)";

/// Whether a run with these switches is an action that does its work
/// without the window (`--check`, `--setup`, `--download-directx`): a
/// second start must not bring the window forward and exit 0 as if the
/// work were done. The SDK's list; SRR has no actions of its own.
pub fn is_action(switches: &[String]) -> bool {
    instance::is_action(switches, &[])
}

/// What a second start tells the player, if anything: the SDK's words,
/// except for `--download-directx` finding the window open, which says to
/// use the window's Download button.
pub fn message(second: SecondStart, switches: &[String]) -> Option<String> {
    let download = switches.iter().any(|switch| switch == "--download-directx");
    match second {
        SecondStart::WindowOpen if download => Some(WINDOW_OPEN.to_string()),
        other => other.message(GAME),
    }
}

/// Before Play: the game from this runtime still runs (the ids
/// `process::find_running` found for its executables), so Play is refused
/// as for a second start without a window. A launcher that was killed
/// while its game ran released the instance, but not the game.
pub fn before_play(running: &[u32]) -> Option<SecondStart> {
    (!running.is_empty()).then_some(SecondStart::Busy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_win32::instance::ALREADY_RUNNING;
    use std::cell::Cell;

    fn switches(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_second_start_brings_the_window_forward_or_refuses_to_play() {
        // The first launcher shows its window: it comes forward, no message.
        let asked = Cell::new(0);
        let second = SecondStart::decide(
            false,
            || unreachable!(),
            || {
                asked.set(asked.get() + 1);
                true
            },
        );
        assert_eq!(asked.get(), 1);
        assert_eq!(second, SecondStart::Focused);
        assert_eq!((second.exit_code(), message(second, &[])), (0, None));
        // The first launcher is waiting for the game: Play is refused.
        let second = SecondStart::decide(false, || unreachable!(), || false);
        assert_eq!(second, SecondStart::Busy);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        let text = message(second, &[]).unwrap();
        assert!(
            text.starts_with("Sega Rally Revo is already running"),
            "{text}"
        );
        assert!(text.contains("Quit the game first"), "{text}");
    }

    #[test]
    fn download_directx_with_the_window_open_says_to_use_it_and_fails() {
        let action = switches(&["--download-directx"]);
        assert!(is_action(&action));
        assert!(!is_action(&switches(&["--no-window"])));
        assert!(!is_action(&[]));
        // The window is open: it is not brought forward, the run fails.
        let second = SecondStart::decide(is_action(&action), || true, || unreachable!());
        assert_eq!(second, SecondStart::WindowOpen);
        assert_eq!(second.exit_code(), ALREADY_RUNNING);
        assert!(
            message(second, &action)
                .unwrap()
                .contains("use its Download button")
        );
        // The game (or another action) holds the instance: busy, failed.
        let second = SecondStart::decide(true, || false, || unreachable!());
        assert_eq!(second, SecondStart::Busy);
        assert_ne!(second.exit_code(), 0);
    }

    /// rea-launcher-sdk#1 (Windows 10): `--setup` with the window open
    /// brought it forward and exited 0 silently; `--check` ran alongside.
    /// Both are actions: refused with exit 5 and the window message.
    #[test]
    fn setup_and_check_with_the_window_open_are_refused_with_exit_5() {
        for line in [
            &["--setup"][..],
            &["--check"],
            &["--setup", "--no-window"],
            &["--check", "--game", r"D:\Games\SEGA Rally"],
            &["--check", "--linux"],
        ] {
            let line = switches(line);
            assert!(is_action(&line), "{line:?}");
            let second = SecondStart::decide(is_action(&line), || true, || unreachable!());
            assert_eq!(second, SecondStart::WindowOpen, "{line:?}");
            assert_eq!(second.exit_code(), ALREADY_RUNNING, "{line:?}");
            assert_eq!(
                message(second, &line).as_deref(),
                Some(
                    "The Sega Rally Revo launcher window is open. Use it there, \
                     or close it and try again."
                ),
                "{line:?}"
            );
            // The game runs instead: refused too, never a silent 0.
            let second = SecondStart::decide(is_action(&line), || false, || unreachable!());
            assert_eq!(second.exit_code(), ALREADY_RUNNING, "{line:?}");
        }
        // A plain start or a play run still brings the window forward.
        for line in [&[][..], &["--no-window"], &["--settings"]] {
            assert!(!is_action(&switches(line)), "{line:?}");
        }
    }

    #[test]
    fn play_is_refused_while_the_runtimes_game_runs() {
        assert_eq!(before_play(&[]), None);
        let refused = before_play(&[4242]).unwrap();
        assert_eq!(refused, SecondStart::Busy);
        assert_eq!(refused.exit_code(), ALREADY_RUNNING);
        assert!(
            message(refused, &[])
                .unwrap()
                .contains("Quit the game first")
        );
    }

    #[test]
    fn the_exit_code_is_not_one_the_launcher_already_uses() {
        // 1 error/--check refusal, 2 bad command line (or not Windows),
        // 3 window closed, 4 no window.
        assert_eq!(ALREADY_RUNNING, 5);
    }
}
