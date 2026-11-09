//! Display policy proven on the reference mixed-GPU, mixed-monitor host.
//! The game always runs fullscreen (it ignores `RunWindowed=1`), so the only
//! choice is where that fullscreen lands:
//!
//! - Under plain Wine, the game's exclusive-fullscreen request becomes a
//!   real XRandR mode switch that multi-monitor hosts reject — an isolated
//!   Wine desktop at the chosen resolution renders regardless.
//! - Inside gamescope the mode change is contained, so the game goes
//!   fullscreen directly.
//! - On native Windows, fullscreen needs a real landscape mode; when there
//!   is none the launcher can only warn.
//! - Borderless (a patch, `patches::BORDERLESS_*`) needs no mode at all: a
//!   screen-sized window; under plain Wine it still runs in the isolated
//!   desktop, which then covers the screen.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// The game's own exclusive fullscreen.
    Fullscreen,
    /// `explorer /desktop=…` at the chosen resolution (plain Wine).
    WineDesktop,
    /// Native Windows with a portrait monitor: expect a failed mode switch.
    WarnPortrait,
    /// Native Windows without that display mode: expect a failed switch.
    WarnUnsupportedMode,
    /// The borderless patch: a screen-sized window, no mode switch (native
    /// Windows, or inside gamescope).
    Borderless,
}

pub fn plan(
    wine: bool,
    gamescope: bool,
    portrait: bool,
    mode_supported: bool,
    borderless: bool,
) -> Plan {
    if wine && !gamescope {
        return Plan::WineDesktop;
    }
    if borderless {
        return Plan::Borderless;
    }
    if wine {
        return Plan::Fullscreen;
    }
    if portrait {
        Plan::WarnPortrait
    } else if !mode_supported {
        Plan::WarnUnsupportedMode
    } else {
        Plan::Fullscreen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_wine_uses_the_isolated_desktop() {
        assert_eq!(plan(true, false, false, true, false), Plan::WineDesktop);
        // a portrait host monitor does not matter inside the Wine desktop
        assert_eq!(plan(true, false, true, false, false), Plan::WineDesktop);
        assert_eq!(plan(true, true, false, true, false), Plan::Fullscreen);
        // borderless keeps the isolated desktop under plain Wine
        assert_eq!(plan(true, false, false, true, true), Plan::WineDesktop);
        assert_eq!(plan(true, true, false, true, true), Plan::Borderless);
    }

    #[test]
    fn windows_needs_a_supported_landscape_mode() {
        assert_eq!(plan(false, false, false, true, false), Plan::Fullscreen);
        assert_eq!(plan(false, false, true, true, false), Plan::WarnPortrait);
        assert_eq!(
            plan(false, false, false, false, false),
            Plan::WarnUnsupportedMode
        );
    }

    #[test]
    fn borderless_on_windows_needs_no_display_mode() {
        assert_eq!(plan(false, false, true, false, true), Plan::Borderless);
        assert_eq!(plan(false, false, false, true, true), Plan::Borderless);
    }
}
