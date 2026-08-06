use bevy::prelude::*;
use bevy_egui::egui;

const PHASE_SATURATION: f32 = 0.72;
const PHASE_LIGHTNESS: f32 = 0.50;

/// Half-width of the hue wedge kept clear around each polarity colour.
pub const POLARITY_GUARD: f32 = 20.0;

/// Hues reserved for magnetic polarity: red for north, blue for south.
pub const POLARITY_HUES: [f32; 2] = [0.0, 240.0];

/// The colour wheel minus a `POLARITY_GUARD` wedge either side of red (0°) and
/// blue (240°).
///
/// Merely *offsetting* the palette cannot solve this. Ten phases sit 36° apart
/// and cover the whole wheel, so whatever the offset, one of them lands on red
/// and another on blue. Removing the reserved wedges from the range instead
/// makes the separation a property of the construction, at any phase count —
/// the cost being that the phases crowd a little closer to each other.
const PHASE_HUE_ARCS: [(f32, f32); 2] = [
    (
        POLARITY_HUES[0] + POLARITY_GUARD,
        POLARITY_HUES[1] - POLARITY_GUARD,
    ),
    (POLARITY_HUES[1] + POLARITY_GUARD, 360.0 - POLARITY_GUARD),
];

/// Hue of `phase`, spread evenly across the arcs left free by polarity.
///
/// Each phase is centred in its share rather than placed at the start of it,
/// so the palette never sits flush against a reserved wedge.
pub fn phase_hue(phase: usize, total_phases: usize) -> f32 {
    let total = total_phases.max(1);
    let usable: f32 = PHASE_HUE_ARCS.iter().map(|(start, end)| end - start).sum();

    let share = usable / total as f32;
    let mut position = (phase % total) as f32 * share + share * 0.5;

    for (start, end) in PHASE_HUE_ARCS {
        let span = end - start;
        if position < span {
            return start + position;
        }
        position -= span;
    }

    // Rounding can leave a hair past the end; the last arc's end is the limit.
    PHASE_HUE_ARCS[PHASE_HUE_ARCS.len() - 1].1
}

/// Phase color from chromatic circle division.
///
/// Hue is spread over the arcs polarity does not claim — see [`phase_hue`].
#[inline]
pub fn phase_color(phase: usize, total_phases: usize) -> Color {
    let hue = phase_hue(phase, total_phases);
    Color::from(bevy::color::Hsla::new(
        hue,
        PHASE_SATURATION,
        PHASE_LIGHTNESS,
        1.0,
    ))
}

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

    /// The guarantee the reserved arcs exist for: no phase may land on the
    /// colours the field overlay uses for magnetic polarity.
    ///
    /// Offsetting the palette could not deliver this. Ten phases at 36° spacing
    /// cover the whole wheel, so some phase always fell on red and another on
    /// blue whatever the offset.
    #[test]
    fn no_phase_hue_lands_near_a_polarity_colour() {
        for total in 2..=10_usize {
            for phase in 0..total {
                let hue = phase_hue(phase, total);
                for reserved in POLARITY_HUES {
                    let distance = hue_distance(hue, reserved);
                    assert!(
                        distance >= POLARITY_GUARD - 1e-3,
                        "m={total} phase {phase} sits at {hue:.1}°, only {distance:.1}° \
                         from the reserved {reserved}° — guard is {POLARITY_GUARD}°"
                    );
                }
            }
        }
    }

    /// Every hue must stay inside one of the usable arcs.
    #[test]
    fn every_hue_falls_inside_a_usable_arc() {
        for total in 1..=10_usize {
            for phase in 0..total {
                let hue = phase_hue(phase, total);
                assert!(
                    PHASE_HUE_ARCS
                        .iter()
                        .any(|(start, end)| hue >= *start - 1e-3 && hue <= *end + 1e-3),
                    "m={total} phase {phase} at {hue:.1}° escaped the usable arcs"
                );
            }
        }
    }

    /// Reserving hue costs phase-to-phase separation, so check what is left is
    /// still workable. Ten phases is the worst case the sliders allow.
    #[test]
    fn phases_stay_distinguishable_from_each_other() {
        for total in 2..=10_usize {
            let hues: Vec<f32> = (0..total).map(|p| phase_hue(p, total)).collect();
            for (i, &a) in hues.iter().enumerate() {
                for &b in &hues[i + 1..] {
                    let distance = hue_distance(a, b);
                    assert!(
                        distance > 20.0,
                        "m={total}: hues {a:.1}° and {b:.1}° are only {distance:.1}° apart"
                    );
                }
            }
        }
    }

    /// Phases must not repeat, and the index must wrap rather than run off.
    #[test]
    fn the_phase_index_wraps() {
        assert_eq!(phase_hue(0, 3), phase_hue(3, 3));
        assert_eq!(phase_hue(1, 3), phase_hue(4, 3));
    }
}
