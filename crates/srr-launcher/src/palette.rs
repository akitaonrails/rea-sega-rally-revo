//! The settings window's colours and widget metrics: SEGA navy with the one
//! Revo-orange accent, in the SDK's palette roles (`rea_launcher_ui::theme`).

use rea_launcher_ui::egui::Color32;
use rea_launcher_ui::theme::{CardMetrics, Metrics, Palette, tint};

pub const BLUE: Color32 = Color32::from_rgb(0x0a, 0x1b, 0x33); // SEGA navy panel fill
pub const PANEL: Color32 = Color32::from_rgb(0x10, 0x2a, 0x4d); // card fill
/// Control borders: the least lightening of the earlier `#1f4a84` that
/// reaches 3:1 against both fills (WCAG non-text contrast).
pub const LINE: Color32 = Color32::from_rgb(0x54, 0x75, 0xa1);
pub const ORANGE: Color32 = Color32::from_rgb(0xff, 0x7a, 0x00); // the one accent (Revo orange)
pub const INK: Color32 = Color32::from_rgb(0xed, 0xf3, 0xfc); // text
pub const MUTED: Color32 = Color32::from_rgb(0x9f, 0xb5, 0xd8); // secondary text
pub const ON_ORANGE: Color32 = Color32::from_rgb(0x0a, 0x1b, 0x33); // text on orange
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x8a, 0x80);

/// One accent in both the action and the selection roles. Disabled chips
/// keep the window's earlier look: muted text, the old border colour at
/// half strength.
pub const PALETTE: Palette = Palette {
    base: BLUE,
    panel: PANEL,
    line: LINE,
    ink: INK,
    muted: MUTED,
    action: ORANGE,
    action_hover: Color32::from_rgb(0xff, 0xad, 0x33),
    on_action: ON_ORANGE,
    select: ORANGE,
    on_select: ON_ORANGE,
    error: ERROR,
    disabled_ink: MUTED,
    // #1f4a84 at half strength, premultiplied.
    disabled_line: Color32::from_rgba_premultiplied(16, 37, 66, 128),
    focus: INK,
    selection: tint(ORANGE, 128),
};

/// The sizes this window's own widgets drew before the SDK had them:
/// roomier look cards (CMR3's) and toggle labels on one line.
pub const METRICS: Metrics = Metrics {
    toggle_wrap: false,
    card: CardMetrics {
        pad: 14.0,
        note: 13.5,
        note_top: 32.0,
        ..Metrics::DEFAULT.card
    },
    ..Metrics::DEFAULT
};

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher_ui::theme;

    #[test]
    fn the_palette_meets_the_contrast_and_size_rules() {
        assert_eq!(
            theme::contrast_problems(&PALETTE, &METRICS),
            Vec::<String>::new()
        );
        assert_eq!(theme::text_size_problems(&METRICS), Vec::<String>::new());
        assert_eq!(
            PALETTE.disabled_line,
            Color32::from_rgb(0x1f, 0x4a, 0x84).gamma_multiply(0.5)
        );
    }
}
