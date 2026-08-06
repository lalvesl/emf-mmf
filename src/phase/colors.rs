use bevy::prelude::*;
use bevy_egui::egui;

use crate::config::MAX_PHASES;

/// Half-width of the hue wedge kept clear around each polarity colour.
pub const POLARITY_GUARD: f32 = 30.0;

/// Hues reserved for magnetic polarity: red for north, blue for south.
pub const POLARITY_HUES: [f32; 2] = [0.0, 240.0];

/// The phase palette, in fixed slot order.
///
/// These are chosen, not generated. Spreading hue evenly around the wheel
/// cannot work here: red and blue belong to magnetic polarity, and an evenly
/// spread palette lands on both. Each slot below sits at least
/// [`POLARITY_GUARD`] degrees off red and off blue.
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
    (27, 167, 132), // teal
    (178, 98, 218), // violet
    (193, 121, 21), // amber
    (25, 138, 179), // cyan
    (113, 156, 28), // lime
    (212, 84, 180), // magenta
    (169, 147, 4),  // gold
    (33, 131, 49),  // green
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

    /// Shortest distance between two hues, in degrees.
    fn hue_distance(a: f32, b: f32) -> f32 {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    }

    fn hue_of(color: Color) -> f32 {
        let hsla: bevy::color::Hsla = color.into();
        hsla.hue
    }

    /// The guarantee the palette was chosen for: no phase may be mistaken for
    /// the colours the field overlay paints polarity with.
    #[test]
    fn no_phase_colour_lands_near_a_polarity_colour() {
        for total in 1..=MAX_PHASES {
            for phase in 0..total {
                let hue = hue_of(phase_color(phase, total));
                for reserved in POLARITY_HUES {
                    let distance = hue_distance(hue, reserved);
                    assert!(
                        distance >= POLARITY_GUARD,
                        "m={total} phase {phase} sits at {hue:.1}°, only {distance:.1}° \
                         from the reserved {reserved}° — guard is {POLARITY_GUARD}°"
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
