//! The winding's angular reference frame, derived once.
//!
//! Everything that has to point at a magnetic axis — the 3D MMF arrows, the
//! field overlay, the rotor — used to re-derive these formulas locally, each
//! copy carrying its own `// matches winding.rs slot offset` comment. They
//! agreed by inspection rather than by construction, which is precisely how the
//! `starts_coil` bug got in. Deriving them here once means a change to the slot
//! geometry or to the phase convention can no longer desynchronise the views.

use std::f32::consts::{PI, TAU};

use crate::config::MotorConfig;

/// Where a slot's centre sits within its angular segment.
///
/// Each of the `S` segments is half tooth then half slot, so the slot spans the
/// second half and its centre falls three quarters of the way along.
pub const SLOT_CENTER_FRACTION: f32 = 0.75;

/// Mechanical angle of one tooth-plus-slot segment.
#[inline]
pub fn segment_angle(groove_count: usize) -> f32 {
    TAU / groove_count.max(1) as f32
}

/// Mechanical angle of the centre of `slot`.
///
/// This is the anchor every other view aligns to: `slot_center(0, S)` is the
/// mechanical offset that converts an electrical belt position into a physical
/// angle.
#[inline]
pub fn slot_center(slot: usize, groove_count: usize) -> f32 {
    (slot as f32 + SLOT_CENTER_FRACTION) * segment_angle(groove_count)
}

/// Electrical displacement between consecutive phases, in radians.
///
/// An odd phase count spreads the phases over a full electrical turn. An even
/// count folds them into half a turn, because the second half would only repeat
/// the first with reversed polarity.
#[inline]
pub fn phase_displacement(phases: usize) -> f32 {
    if phases == 0 {
        return TAU;
    }
    if phases.is_multiple_of(2) {
        PI / phases as f32
    } else {
        TAU / phases as f32
    }
}

/// Instantaneous per-unit current of `phase` at electrical angle `elec_angle`.
///
/// `alpha_m` is [`phase_displacement`], taken as an argument so callers driving
/// a loop can hoist it out.
#[inline]
pub fn phase_current(elec_angle: f32, phase: usize, alpha_m: f32) -> f32 {
    (elec_angle - phase as f32 * alpha_m).cos()
}

/// Electrical angle spanned by one slot.
#[inline]
pub fn slot_angle_elec(config: &MotorConfig) -> f32 {
    config.pole_pairs as f32 * TAU / config.groove_count.max(1) as f32
}

/// Slots per pole per phase (`q`), as a real number.
#[inline]
pub fn slots_per_pole_per_phase(config: &MotorConfig) -> f32 {
    let divisor = 2 * config.pole_pairs * config.phases;
    if divisor == 0 {
        return 0.0;
    }
    config.groove_count as f32 / divisor as f32
}

/// Electrical angle from the start of a coil group to its magnetic axis.
///
/// The axis lies midway between the centroid of the outgoing coil sides — `q`
/// consecutive slots — and the centroid of the return sides, `coil_pitch` slots
/// further along. Chording shortens that span and so pulls the axis back.
#[inline]
pub fn axis_offset_elec(config: &MotorConfig) -> f32 {
    let q = slots_per_pole_per_phase(config);
    let pitch = super::coil_pitch(config) as f32;
    (q - 1.0 + pitch) / 2.0 * slot_angle_elec(config)
}

/// Mechanical angle of the magnetic axis of one phase-pole group.
///
/// Consecutive poles of a phase sit half an electrical turn apart; dividing by
/// `p` converts electrical back to mechanical, and the slot centre anchors the
/// result to the physical geometry.
pub fn magnetic_axis(config: &MotorConfig, phase: usize, pole: usize) -> f32 {
    let p = config.pole_pairs.max(1) as f32;
    let start_elec = phase as f32 * phase_displacement(config.phases) + pole as f32 * PI;
    (start_elec + axis_offset_elec(config)) / p + slot_center(0, config.groove_count)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::default;

    const EPS: f32 = 1e-5;

    fn config(groove_count: usize, phases: usize, pole_pairs: usize) -> MotorConfig {
        MotorConfig {
            groove_count,
            phases,
            pole_pairs,
            layers: 2,
            ..default()
        }
    }

    /// The mechanical anchor must be the slot centre the conductors actually
    /// use, not a constant that merely happens to match today.
    #[test]
    fn the_axis_anchor_is_the_real_slot_centre() {
        for n in [12_usize, 24, 144] {
            let seg = segment_angle(n);
            // The value every view used to hard-code.
            assert!((slot_center(0, n) - (TAU / n as f32) * 0.75).abs() < EPS);
            // And it must agree with how the winding places slot `s`.
            for slot in [0_usize, 1, 7] {
                let tooth_angle = seg * 0.5;
                let from_winding = slot as f32 * seg + tooth_angle + seg * 0.25;
                assert!(
                    (slot_center(slot, n) - from_winding).abs() < EPS,
                    "n={n} slot={slot}"
                );
            }
        }
    }

    /// Hand-checked: `S=24, p=2, m=3` gives `q=2` and a pole pitch of 6 slots.
    /// The go-side centroid sits at slot 0.5, the return side at 6.5, so the
    /// axis lands at 3.5 slots. Chording to 5 pulls it back to 3.0.
    #[test]
    fn the_magnetic_axis_sits_between_the_two_coil_sides() {
        let mut full = config(24, 3, 2);
        full.short_pitched = false;
        let alpha = slot_angle_elec(&full);

        assert!((axis_offset_elec(&full) - 3.5 * alpha).abs() < EPS);

        let mut chorded = full.clone();
        chorded.short_pitched = true;
        assert!((axis_offset_elec(&chorded) - 3.0 * alpha).abs() < EPS);
    }

    /// Pole `k` of a phase must sit exactly one pole pitch from pole `k-1`,
    /// and phase `j` one phase displacement from phase `j-1`.
    #[test]
    fn axes_are_evenly_spaced_in_both_directions() {
        let cfg = config(24, 3, 2);
        let p = cfg.pole_pairs as f32;

        for phase in 0..cfg.phases {
            for pole in 1..(2 * cfg.pole_pairs) {
                let step = magnetic_axis(&cfg, phase, pole) - magnetic_axis(&cfg, phase, pole - 1);
                assert!((step - PI / p).abs() < EPS, "phase={phase} pole={pole}");
            }
        }

        let expected = phase_displacement(cfg.phases) / p;
        for phase in 1..cfg.phases {
            let step = magnetic_axis(&cfg, phase, 0) - magnetic_axis(&cfg, phase - 1, 0);
            assert!((step - expected).abs() < EPS, "phase={phase}");
        }
    }

    /// The convention that odd counts span a full turn and even counts a half.
    #[test]
    fn phase_displacement_follows_the_parity_convention() {
        assert!((phase_displacement(3) - TAU / 3.0).abs() < EPS);
        assert!((phase_displacement(5) - TAU / 5.0).abs() < EPS);
        assert!((phase_displacement(2) - PI / 2.0).abs() < EPS);
        assert!((phase_displacement(6) - PI / 6.0).abs() < EPS);
    }

    /// A balanced polyphase set sums to zero at every instant.
    #[test]
    fn odd_phase_currents_are_balanced() {
        for m in [3_usize, 5, 7] {
            let alpha_m = phase_displacement(m);
            for step in 0..8 {
                let t = step as f32 * 0.7;
                let sum: f32 = (0..m).map(|k| phase_current(t, k, alpha_m)).sum();
                assert!(sum.abs() < 1e-4, "m={m} sums to {sum} at t={t}");
            }
        }
    }
}
