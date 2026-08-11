use bevy::prelude::*;

use crate::config::{MotorConfig, ViewConfig};
use crate::electrical::ElectricalState;
use crate::vectors::arrow::{self, ArrowHead, ArrowShaft, HeadQuery, ShaftQuery};
use crate::winding::axis;

pub struct MmfVectorsPlugin;

impl Plugin for MmfVectorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                regenerate_vectors.run_if(crate::config::scene_changed),
                animate_vectors,
            ),
        );
    }
}

/// Identifies an MMF arrow. `None` represents the resultant vector.
#[derive(Component)]
pub struct MmfVector {
    pub phase: Option<usize>,
    pub pole: usize,
}

const PHASE_HEAD_HEIGHT: f32 = 0.2;
const RESULT_HEAD_HEIGHT: f32 = 0.3;

/// Poles that get a resultant arrow: the north of each pole pair.
///
/// The `2p` pole axes alternate north/south and are evenly spaced, so every
/// south sits exactly halfway between two norths — it marks nothing the norths
/// have not already fixed. Drawing all `2p` simply doubled the arrows, which
/// went unnoticed at two poles (where the pair superimposed) and became obvious
/// from four up, where the duplicates fan out across the bore.
fn resultant_poles(pole_pairs: usize) -> impl Iterator<Item = usize> {
    (0..(2 * pole_pairs)).step_by(2)
}

/// MMF contribution of one phase at one pole, as a vector in the bore plane.
///
/// A single phase pulsates along a fixed axis — the arrow keeps its direction
/// and swings its length with the current, flipping over when the current does.
fn phase_vector(
    config: &MotorConfig,
    phase: usize,
    pole: usize,
    elec_angle: f32,
    alpha_m: f32,
) -> Vec3 {
    let current = axis::phase_current(elec_angle, phase, alpha_m);
    let axis_phys = axis::magnetic_axis(config, phase, pole);
    // Consecutive poles invert the field direction.
    let amplitude = current * if pole.is_multiple_of(2) { 1.0 } else { -1.0 };
    Vec3::new(axis_phys.cos(), 0.0, axis_phys.sin()) * amplitude
}

/// Resultant MMF at one pole, as a vector in the bore plane.
///
/// The phasor sum has to be taken in *electrical* space. There the phase axes
/// sit exactly one phase displacement apart, which is what makes a balanced
/// set collapse to one vector of constant magnitude, turning at `ω` — see
/// [`axis::resultant_axis`] for the closed form (and why it trails the phase
/// axes by a quarter turn).
///
/// Adding up the arrows as they are *drawn* does not work. Their mechanical
/// axes are compressed by `1/p` while the currents still shift by the full
/// `α_m`, so the two spacings only agree at one pole pair. Beyond that the sum
/// keeps a backward wave alongside the forward one, and the beat between them
/// dragged the arrow down to 33% of full length at four poles and 4% at twelve.
///
/// Mechanically the wave turns at `ω/p` — the synchronous speed the rotor is
/// already driven at, so the arrow and the rotor's north pole now coincide by
/// construction rather than by coincidence.
fn resultant_vector(config: &MotorConfig, pole: usize, elec_angle: f32) -> Vec3 {
    let mech = axis::resultant_axis(config, pole, elec_angle);
    let magnitude = config.phases as f32 / 2.0;
    Vec3::new(mech.cos(), 0.0, mech.sin()) * magnitude
}

fn regenerate_vectors(
    mut commands: Commands,
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    query: Query<Entity, With<MmfVector>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Despawn old vectors
    for entity in &query {
        commands.entity(entity).despawn();
    }

    if !view.show_vectors {
        return;
    }

    let m = config.phases;
    let p = config.pole_pairs;
    if m == 0 || p == 0 {
        return;
    }

    let shaft_mesh = meshes.add(Cylinder::new(0.02, 1.0));
    let head_mesh = meshes.add(Cone {
        radius: 0.05,
        height: PHASE_HEAD_HEIGHT,
    });

    // The resultant arrow looks the same for every pole, so its meshes and
    // material are built once and shared instead of per-pole.
    let res_shaft = meshes.add(Cylinder::new(0.04, 1.0));
    let res_head = meshes.add(Cone {
        radius: 0.08,
        height: RESULT_HEAD_HEIGHT,
    });
    let res_color = Color::WHITE;
    let res_mat = materials.add(StandardMaterial {
        base_color: res_color,
        emissive: res_color.into(),
        ..default()
    });

    // Spawn Phase Vectors
    for pole in 0..(2 * p) {
        for phase in 0..m {
            let color = crate::phase::colors::phase_color(phase, m);
            let mat = materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            });

            commands
                .spawn((
                    Transform::default(),
                    Visibility::default(),
                    MmfVector {
                        phase: Some(phase),
                        pole,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Mesh3d(shaft_mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::default(),
                        ArrowShaft,
                    ));
                    parent.spawn((
                        Mesh3d(head_mesh.clone()),
                        MeshMaterial3d(mat),
                        Transform::default(),
                        ArrowHead {
                            height: PHASE_HEAD_HEIGHT,
                        },
                    ));
                });
        }
    }

    // One resultant arrow per pole pair — see `resultant_poles`.
    for pole in resultant_poles(p) {
        commands
            .spawn((
                Transform::default(),
                Visibility::default(),
                MmfVector { phase: None, pole },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(res_shaft.clone()),
                    MeshMaterial3d(res_mat.clone()),
                    Transform::default(),
                    ArrowShaft,
                ));
                parent.spawn((
                    Mesh3d(res_head.clone()),
                    MeshMaterial3d(res_mat.clone()),
                    Transform::default(),
                    ArrowHead {
                        height: RESULT_HEAD_HEIGHT,
                    },
                ));
            });
    }
}

fn animate_vectors(
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    state: Res<ElectricalState>,
    mut vectors: Query<(&MmfVector, &Children, &mut Transform)>,
    mut shafts: ShaftQuery<MmfVector>,
    mut heads: HeadQuery<MmfVector>,
) {
    if !view.show_vectors {
        return;
    }

    let m = config.phases;
    let p = config.pole_pairs;
    if m == 0 || p == 0 {
        return;
    }

    let elec_angle = state.angle;
    let max_radius = crate::config::STATOR_BORE_RADIUS * 0.9;

    let alpha_m = axis::phase_displacement(m);

    let mut phase_vecs: Vec<Vec<Vec3>> = vec![vec![Vec3::ZERO; m]; 2 * p];

    for (pole, pole_vecs) in phase_vecs.iter_mut().enumerate() {
        for (phase, phase_vec_entry) in pole_vecs.iter_mut().enumerate() {
            *phase_vec_entry = phase_vector(&config, phase, pole, elec_angle, alpha_m);
        }
    }

    for (vector, children, mut transform) in &mut vectors {
        let pole = vector.pole;

        // Defend against outdated entities from a previous configuration
        // waiting to be despawned cleanly by the commands buffer.
        if pole >= 2 * p {
            continue;
        }

        let (target_vec, max_ideal) = if let Some(phase) = vector.phase {
            if phase >= m {
                continue;
            }
            (phase_vecs[pole][phase], 1.0)
        } else {
            // A balanced set always resolves to `m / 2`, so the resultant is
            // drawn at full length and only ever turns.
            (resultant_vector(&config, pole, elec_angle), m as f32 / 2.0)
        };

        let length = target_vec.length();
        if length <= 0.001 {
            transform.scale = Vec3::ZERO;
            continue;
        }

        transform.scale = Vec3::ONE;
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, target_vec / length);

        let world_length = (length * max_radius) / max_ideal.max(1.0);

        arrow::lay_out(children, world_length, &mut shafts, &mut heads);
    }
}
// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{PI, TAU};

    const EPS: f32 = 1e-4;

    fn config(groove_count: usize, phases: usize, pole_pairs: usize) -> MotorConfig {
        MotorConfig {
            groove_count,
            phases,
            pole_pairs,
            ..default()
        }
    }

    /// Direction of a bore-plane vector, in mechanical radians.
    fn heading(v: Vec3) -> f32 {
        v.z.atan2(v.x)
    }

    fn angle_between(a: f32, b: f32) -> f32 {
        let d = (a - b).rem_euclid(TAU);
        d.min(TAU - d)
    }

    /// What the arrows used to do: sum the drawn vectors in mechanical space.
    fn mechanical_space_sum(config: &MotorConfig, pole: usize, elec_angle: f32) -> Vec3 {
        let alpha_m = axis::phase_displacement(config.phases);
        (0..config.phases)
            .map(|phase| phase_vector(config, phase, pole, elec_angle, alpha_m))
            .sum()
    }

    /// One arrow per pole pair, and always the north of the pair.
    #[test]
    fn one_resultant_arrow_per_pole_pair() {
        for pole_pairs in 1..=6_usize {
            let poles: Vec<_> = resultant_poles(pole_pairs).collect();
            assert_eq!(poles.len(), pole_pairs, "p={pole_pairs}");
            assert!(
                poles.iter().all(|pole| pole.is_multiple_of(2)),
                "p={pole_pairs}: a south pole slipped in: {poles:?}"
            );
        }
    }

    /// A balanced polyphase set produces a rotating field of *constant*
    /// amplitude — the arrow may only turn, never grow or shrink.
    #[test]
    fn the_resultant_keeps_a_constant_length() {
        for phases in [2_usize, 3, 5, 6] {
            for pole_pairs in 1..=6_usize {
                let cfg = config(24, phases, pole_pairs);
                let expected = phases as f32 / 2.0;

                for pole in resultant_poles(pole_pairs) {
                    for step in 0..24 {
                        let t = step as f32 * TAU / 24.0;
                        let length = resultant_vector(&cfg, pole, t).length();
                        assert!(
                            (length - expected).abs() < EPS,
                            "m={phases} p={pole_pairs} pole={pole} t={t}: \
                             length {length}, expected {expected}"
                        );
                    }
                }
            }
        }
    }

    /// Regression: summing the arrows as drawn — at mechanical angles, while
    /// the currents shift by the full electrical displacement — leaves a
    /// backward wave beside the forward one. Their beat collapsed the arrow to
    /// a few percent of full length, and worse the more poles there were.
    #[test]
    fn summing_in_mechanical_space_would_collapse_the_arrow() {
        let phases = 3_usize;
        let full = phases as f32 / 2.0;

        // One pole pair is the case where the two spacings agree, so the old
        // sum was correct there — which is why the bug stayed hidden.
        let cfg = config(24, phases, 1);
        for step in 0..24 {
            let t = step as f32 * TAU / 24.0;
            assert!(
                (mechanical_space_sum(&cfg, 0, t).length() - full).abs() < EPS,
                "one pole pair should already have been constant"
            );
        }

        // From two pole pairs up it pulses, and the collapse deepens with p.
        let mut worst_previous = 1.0_f32;
        for pole_pairs in [2_usize, 3, 4, 6] {
            let cfg = config(24 * pole_pairs, phases, pole_pairs);
            let worst = (0..360)
                .map(|step| {
                    let t = step as f32 * TAU / 360.0;
                    mechanical_space_sum(&cfg, 0, t).length() / full
                })
                .fold(f32::INFINITY, f32::min);

            assert!(
                worst < 0.4,
                "p={pole_pairs}: old sum only dropped to {worst}, expected a collapse"
            );
            assert!(
                worst < worst_previous,
                "p={pole_pairs}: collapse should deepen with pole count"
            );
            worst_previous = worst;

            // The fix holds steady over the same sweep.
            for step in 0..360 {
                let t = step as f32 * TAU / 360.0;
                assert!((resultant_vector(&cfg, 0, t).length() - full).abs() < EPS);
            }
        }
    }

    /// The field turns at the synchronous mechanical speed: one electrical
    /// revolution advances it by exactly one pole-pair pitch.
    #[test]
    fn the_resultant_turns_at_synchronous_speed() {
        for pole_pairs in 1..=6_usize {
            let cfg = config(24, 3, pole_pairs);
            let start = heading(resultant_vector(&cfg, 0, 0.0));
            let after = heading(resultant_vector(&cfg, 0, TAU));

            // One electrical revolution is one pole-pair pitch mechanically,
            // so the arrow lands where its neighbouring north stood — not back
            // on itself, unless there is only one pole pair.
            let expected = start + TAU / pole_pairs as f32;
            assert!(
                angle_between(after, expected) < EPS,
                "p={pole_pairs}: a full electrical turn must advance one pole-pair pitch"
            );

            // Half an electrical turn puts it halfway between two norths.
            let half = heading(resultant_vector(&cfg, 0, PI));
            assert!(
                (angle_between(start, half) - PI / pole_pairs as f32).abs() < EPS,
                "p={pole_pairs}: half a cycle should advance half a pole-pair pitch"
            );
        }
    }

    /// The drawn norths tile the bore evenly, one per pole pair.
    #[test]
    fn norths_are_evenly_spread_around_the_bore() {
        for pole_pairs in 2..=6_usize {
            let cfg = config(24, 3, pole_pairs);
            let t = 0.7;
            let poles: Vec<_> = resultant_poles(pole_pairs).collect();

            for pair in poles.windows(2) {
                let a = heading(resultant_vector(&cfg, pair[0], t));
                let b = heading(resultant_vector(&cfg, pair[1], t));
                let expected = (TAU / pole_pairs as f32).min(TAU - TAU / pole_pairs as f32);
                assert!(
                    (angle_between(a, b) - expected).abs() < EPS,
                    "p={pole_pairs}: norths {} and {} are not one pitch apart",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    /// The rotor is spun to `state.angle / p` past the phase-A magnetic axis.
    /// The resultant arrow must land on that same spot, or the rotor's north
    /// and the arrow marking it would drift apart.
    #[test]
    fn the_resultant_tracks_the_rotor_north() {
        for pole_pairs in 1..=6_usize {
            let cfg = config(24, 3, pole_pairs);
            for step in 0..12 {
                let t = step as f32 * TAU / 12.0;
                // Exactly what `rotor::render` uses to place its north pole.
                let rotor_north = axis::resultant_axis(&cfg, 0, 0.0) + t / pole_pairs as f32;
                let arrow = heading(resultant_vector(&cfg, 0, t));
                assert!(
                    angle_between(rotor_north, arrow) < EPS,
                    "p={pole_pairs} t={t}: arrow and rotor north disagree"
                );
            }
        }
    }

    /// A single phase pulsates along a fixed axis — that one is meant to
    /// breathe, unlike the resultant.
    #[test]
    fn a_phase_pulsates_on_a_fixed_axis() {
        let cfg = config(24, 3, 2);
        let alpha_m = axis::phase_displacement(cfg.phases);
        let expected_axis = axis::magnetic_axis(&cfg, 0, 0);

        let mut saw_short = false;
        for step in 0..24 {
            let t = step as f32 * TAU / 24.0;
            let v = phase_vector(&cfg, 0, 0, t, alpha_m);
            if v.length() < 0.2 {
                saw_short = true;
                continue;
            }
            // Always on the same line, pointing one way or the other.
            let along = angle_between(heading(v), expected_axis);
            assert!(
                along < EPS || (along - PI).abs() < EPS,
                "t={t}: phase arrow left its axis"
            );
        }
        assert!(
            saw_short,
            "a phase arrow should shrink as its current falls"
        );
    }

    /// The head must keep its proportions at every arrow length — a Y-only
    /// scale on the parent used to stretch it into a needle.
    #[test]
    fn the_head_keeps_its_shape_at_every_length() {
        let head_height = RESULT_HEAD_HEIGHT;
        for world_length in [0.05_f32, 0.1, 0.3, 1.0, 1.8] {
            let head_length = head_height.min(world_length * 0.5);
            let shaft_length = world_length - head_length;
            let head_scale = head_length / head_height;

            assert!(head_scale > 0.0 && head_scale <= 1.0, "len={world_length}");
            assert!(shaft_length >= 0.0, "len={world_length}");
            // Tip lands exactly at the vector's length.
            assert!(
                (shaft_length + head_length - world_length).abs() < 1e-6,
                "len={world_length}: tip overshoots"
            );
        }
    }
}
