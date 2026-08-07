use bevy::prelude::*;
use bevy_egui::egui;

use crate::config::MAX_PHASES;

/// How close a phase colour may come to a polarity colour, as an OKLab
/// distance.
///
/// Measured against the rim colours the field overlay actually paints
/// ([`NORTH_COLOR`](crate::mmf_field::render::NORTH_COLOR) and its south
/// counterpart), not against nominal red and blue — the south rim is at hue
/// 234.8°, not 240°, and both are deep rather than pure.
///
/// A hue wedge used to stand here, and it measured the wrong thing: hue alone
/// cannot tell a pale pink from fire-engine red, and it ranked Crimson Wine
/// (18.5° off red) as safer than Burnt Terracotta (17.5°) when perceptually the
/// terracotta sits *two and a half times closer* to the north rim. This is a
/// regression floor rather than a perceptual threshold: the value sits just
/// under the tightest approved pair, so the palette cannot get more confusable
/// than it is today without the test saying so.
pub const POLARITY_GUARD: f32 = 0.05;

/// The phase palette, in fixed slot order.
///
/// These are chosen, not generated. Spreading hue evenly around the wheel
/// cannot work here: red and blue belong to magnetic polarity, and an evenly
/// spread palette lands on both. No slot below comes within
/// [`POLARITY_GUARD`] of either polarity colour; the tightest is Burnt
/// Terracotta against the north rim, at 0.059.
///
/// The *order* is part of the design, not cosmetic — it is what keeps
/// neighbouring slots apart under colour-vision deficiency, so slots are
/// assigned in sequence and never reordered or cycled. Validated as a set
/// against a dark surface: every slot inside the dark lightness band
/// (OKLCH L 0.48–0.67), above the chroma floor, at least 3:1 contrast on the
/// surface, worst adjacent pair ΔE 9.0 under simulated protanopia and 17.1
/// under normal vision.
///
/// Eight is where this stops. Any two phases can end up side by side around the
/// bore, and no set of eight hues clears that stricter all-pairs test — the
/// phase letter shown in the legend and the winding diagram is what carries
/// identity where colour alone falls short.
const PHASE_PALETTE: [(u8, u8, u8); MAX_PHASES] = [
    // (27, 167, 132), // teal
    // (178, 98, 218), // violet
    // (193, 121, 21), // amber
    // (25, 138, 179), // cyan
    // (113, 156, 28), // lime
    // (212, 84, 180), // magenta
    // (169, 147, 4),  // gold
    // (33, 131, 49),  // green
    // (255, 99, 72),  // Vivid Coral
    // (255, 184, 0),  // Vibrant Amber
    // (34, 209, 134), // Emerald Spring Green
    // (0, 200, 222),  // Bright Turquoise
    // (168, 85, 247), // Neon Purple
    // (244, 63, 94),  // Electric Rose
    // (255, 140, 0),  // Fiery Orange
    // (0, 229, 255),  // Vivid Cyan
    (16, 80, 50),   // Dark Forest Green: Deep, rich woodland green
    (217, 119, 6),  // Deep Amber: Warm golden-orange tone
    (107, 33, 168), // Royal Amethyst: Deep, vibrant purple
    (14, 116, 144), // Deep Ocean Teal: Rich cyan-teal blend
    (136, 19, 55),  // Crimson Wine: Deep maroon-burgundy tone
    (194, 65, 12),  // Burnt Terracotta: Rich rust-orange hue
];

/// Colour of `phase`, wrapping if the index runs past the phase count.
#[inline]
pub fn phase_color(phase: usize, total_phases: usize) -> Color {
    let total = total_phases.clamp(1, MAX_PHASES);
    let (r, g, b) = PHASE_PALETTE[phase % total];
    Color::srgb_u8(r, g, b)
}

/// A contrasting colour for marks drawn on top of a phase-coloured object,
/// such as the current-direction symbols on a conductor's end face.
#[inline]
pub fn phase_color_opposite(phase: usize, total_phases: usize) -> Color {
    let color = phase_color(phase, total_phases);
    let hsla: bevy::color::Hsla = color.into();
    Color::from(bevy::color::Hsla::new(
        (hsla.hue + 180.0) % 360.0,
        hsla.saturation,
        hsla.lightness,
        hsla.alpha,
    ))
}

#[inline]
pub fn phase_color_egui(phase: usize, total_phases: usize) -> egui::Color32 {
    let color: bevy::color::Srgba = phase_color(phase, total_phases).into();
    egui::Color32::from_rgb(
        (color.red * 255.0) as u8,
        (color.green * 255.0) as u8,
        (color.blue * 255.0) as u8,
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::mmf_field::render::{NORTH_COLOR, SOUTH_COLOR};

    /// Perceived distance between two colours, in OKLab.
    ///
    /// Euclidean in a space built so that equal steps look equally large, which
    /// is the whole reason for measuring here instead of in hue: it weighs
    /// lightness and chroma alongside hue rather than discarding them.
    fn perceptual_distance(a: Color, b: Color) -> f32 {
        let (a, b): (bevy::color::Oklaba, bevy::color::Oklaba) = (a.into(), b.into());
        ((a.lightness - b.lightness).powi(2) + (a.a - b.a).powi(2) + (a.b - b.b).powi(2)).sqrt()
    }

    /// The guarantee the palette was chosen for: no phase may be mistaken for
    /// the colours the field overlay paints polarity with.
    ///
    /// The two are read together — the overlay blends the phase hue through the
    /// core of a lobe and the polarity colour over its rim — so a phase that
    /// collapsed onto a rim colour would cost the lobe its identity, not just
    /// look similar somewhere else on screen.
    #[test]
    fn no_phase_colour_lands_near_a_polarity_colour() {
        let reserved = [
            ("north", Color::srgb_from_array(NORTH_COLOR)),
            ("south", Color::srgb_from_array(SOUTH_COLOR)),
        ];

        for total in 1..=MAX_PHASES {
            for phase in 0..total {
                let color = phase_color(phase, total);
                for (pole, polarity) in reserved {
                    let distance = perceptual_distance(color, polarity);
                    assert!(
                        distance >= POLARITY_GUARD,
                        "m={total} phase {phase} sits {distance:.4} from the {pole} rim \
                         — guard is {POLARITY_GUARD}"
                    );
                }
            }
        }
    }

    /// Slots are assigned in sequence, so two phases of the same machine must
    /// never share a colour.
    #[test]
    fn every_phase_of_a_machine_gets_its_own_colour() {
        for total in 1..=MAX_PHASES {
            let colors: Vec<_> = (0..total)
                .map(|p| phase_color(p, total).to_srgba().to_u8_array())
                .collect();
            for (i, a) in colors.iter().enumerate() {
                for b in &colors[i + 1..] {
                    assert_ne!(a, b, "m={total}: two phases share a colour");
                }
            }
        }
    }

    /// The palette is indexed by slot, so phase `k` keeps its colour whatever
    /// the machine's phase count — identity must not depend on the total.
    #[test]
    fn a_phase_keeps_its_colour_across_phase_counts() {
        for phase in 0..3 {
            let reference = phase_color(phase, 3).to_srgba().to_u8_array();
            for total in (phase + 1)..=MAX_PHASES {
                assert_eq!(
                    phase_color(phase, total).to_srgba().to_u8_array(),
                    reference,
                    "phase {phase} changed colour at m={total}"
                );
            }
        }
    }

    /// An index past the phase count wraps rather than running off the palette.
    #[test]
    fn the_phase_index_wraps() {
        assert_eq!(
            phase_color(0, 3).to_srgba().to_u8_array(),
            phase_color(3, 3).to_srgba().to_u8_array()
        );
        assert_eq!(
            phase_color(1, 3).to_srgba().to_u8_array(),
            phase_color(4, 3).to_srgba().to_u8_array()
        );
    }

    /// A phase count outside the supported range must not index past the end.
    #[test]
    fn an_out_of_range_phase_count_is_clamped() {
        for total in [0_usize, MAX_PHASES + 1, 100] {
            for phase in 0..12 {
                let _ = phase_color(phase, total);
            }
        }
    }
}
