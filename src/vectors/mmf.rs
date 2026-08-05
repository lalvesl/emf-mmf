use bevy::prelude::*;

use crate::config::{MotorConfig, MotorConfigChanged};
use crate::electrical::ElectricalState;
use crate::winding::axis;

pub struct MmfVectorsPlugin;

impl Plugin for MmfVectorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (regenerate_vectors, animate_vectors));
    }
}

/// Identifies an MMF arrow. `None` represents the resultant vector.
#[derive(Component)]
pub struct MmfVector {
    pub phase: Option<usize>,
    pub pole: usize,
}

/// The stem of an arrow. Stretched along Y to the vector's length.
#[derive(Component)]
struct ArrowShaft;

/// The cone of an arrow. Moved to the tip, but kept at its built size.
#[derive(Component)]
struct ArrowHead {
    height: f32,
}

const PHASE_HEAD_HEIGHT: f32 = 0.2;
const RESULT_HEAD_HEIGHT: f32 = 0.3;

/// Shafts, disjoint from the heads and from the arrows that own them.
type ShaftQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Transform,
    (With<ArrowShaft>, Without<ArrowHead>, Without<MmfVector>),
>;

/// Heads, disjoint from the shafts and from the arrows that own them.
type HeadQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ArrowHead, &'static mut Transform),
    (Without<ArrowShaft>, Without<MmfVector>),
>;

/// Poles that get a resultant arrow: the north of each pole pair.
///
/// The `2p` pole axes alternate north/south, and a south arrow's direction is
/// fully determined by the norths around it — it lands exactly between them.
/// At one pole pair the south even coincides with the north, which is why two
/// poles always looked like a single arrow while four or more fanned the
/// duplicates out across the bore.
fn resultant_poles(pole_pairs: usize) -> impl Iterator<Item = usize> {
    (0..(2 * pole_pairs)).step_by(2)
}

/// MMF contribution of one phase at one pole, as a vector in the bore plane.
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

fn regenerate_vectors(
    mut commands: Commands,
    mut ev_config: MessageReader<MotorConfigChanged>,
    config: Res<MotorConfig>,
    query: Query<Entity, With<MmfVector>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if ev_config.read().next().is_none() {
        return;
    }

    // Despawn old vectors
    for entity in &query {
        commands.entity(entity).despawn();
    }

    if !config.show_vectors {
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
    state: Res<ElectricalState>,
    mut vectors: Query<(&MmfVector, &Children, &mut Transform)>,
    mut shafts: ShaftQuery,
    mut heads: HeadQuery,
) {
    if !config.show_vectors {
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
    let mut resultant_vecs: Vec<Vec3> = vec![Vec3::ZERO; 2 * p];

    for pole in 0..(2 * p) {
        for (phase, phase_vec_entry) in phase_vecs[pole].iter_mut().enumerate().take(m) {
            let vec = phase_vector(&config, phase, pole, elec_angle, alpha_m);
            *phase_vec_entry = vec;
            resultant_vecs[pole] += vec;
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
            // MMF magnitude max expected mathematically is roughly `m / 2`
            (resultant_vecs[pole], m as f32 / 2.0)
        };

        let length = target_vec.length();
        if length <= 0.001 {
            transform.scale = Vec3::ZERO;
            continue;
        }

        transform.scale = Vec3::ONE;
        transform.rotation = Quat::from_rotation_arc(Vec3::Y, target_vec / length);

        let world_length = (length * max_radius) / max_ideal.max(1.0);

        // Lay the shaft and the head out along the arrow rather than scaling
        // the whole thing in Y: that stretched the cone into a needle on long
        // vectors and squashed it flat on short ones. The head keeps its built
        // size, shrinking only when the arrow is too short to hold it — and
        // then uniformly, so it stays a cone.
        let head_height = children
            .into_iter()
            .find_map(|&child| heads.get(child).ok().map(|(head, _)| head.height))
            .unwrap_or(0.0);
        let head_length = head_height.min(world_length * 0.5);
        let shaft_length = world_length - head_length;

        for &child in children {
            if let Ok(mut shaft) = shafts.get_mut(child) {
                shaft.scale.y = shaft_length.max(1e-4);
                shaft.translation.y = shaft_length * 0.5;
            } else if let Ok((head, mut head_transform)) = heads.get_mut(child) {
                head_transform.scale = Vec3::splat(head_length / head.height);
                head_transform.translation.y = shaft_length + head_length * 0.5;
            }
        }
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

    fn resultant(config: &MotorConfig, pole: usize, elec_angle: f32) -> Vec3 {
        let alpha_m = axis::phase_displacement(config.phases);
        (0..config.phases)
            .map(|phase| phase_vector(config, phase, pole, elec_angle, alpha_m))
            .sum()
    }

    fn angle_between(a: f32, b: f32) -> f32 {
        let d = (a - b).rem_euclid(TAU);
        d.min(TAU - d)
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

    /// Why two poles always looked like a single arrow: at one pole pair the
    /// south resultant points exactly where the north does, so the second arrow
    /// sat invisibly on top of the first.
    #[test]
    fn a_single_pole_pair_superimposes_its_two_arrows() {
        let cfg = config(24, 3, 1);
        for step in 0..6 {
            let t = step as f32 * 0.9;
            let north = resultant(&cfg, 0, t);
            let south = resultant(&cfg, 1, t);
            assert!(
                angle_between(heading(north), heading(south)) < EPS,
                "t={t}: the two arrows are not superimposed"
            );
        }
    }

    /// From four poles up they no longer overlap — which is the clutter the
    /// north-only rule removes. The souths sit exactly halfway between the
    /// norths, so they add no information.
    #[test]
    fn souths_fan_out_from_four_poles_up_but_stay_between_the_norths() {
        for pole_pairs in 2..=6_usize {
            let cfg = config(24 * pole_pairs.div_ceil(2), 3, pole_pairs);
            let t = 0.7;

            let north = heading(resultant(&cfg, 0, t));
            let south = heading(resultant(&cfg, 1, t));
            assert!(
                angle_between(north, south) > EPS,
                "p={pole_pairs}: the south still overlaps the north"
            );

            // Consecutive norths are one pole-pair pitch apart around the bore.
            if pole_pairs > 1 {
                let next_north = heading(resultant(&cfg, 2, t));
                assert!(
                    (angle_between(north, next_north) - TAU / pole_pairs as f32).abs() < EPS
                        || (angle_between(north, next_north) - (TAU - TAU / pole_pairs as f32))
                            .abs()
                            < EPS,
                    "p={pole_pairs}: norths are not evenly spread"
                );
            }

            // And the south bisects them: it trails the north by exactly half
            // a pole-pair pitch, plus the half turn that makes it a south.
            let expected = (north + PI / pole_pairs as f32 + PI).rem_euclid(TAU);
            assert!(
                angle_between(south, expected) < EPS,
                "p={pole_pairs}: the south is not determined by the norths"
            );
        }
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
