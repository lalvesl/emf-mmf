use crate::config::{MotorConfig, ROTOR_RADIUS, RotorConfig, STATOR_HEIGHT, ViewConfig};
use crate::electrical::ElectricalState;
use crate::rotor::torque::RotorEquilibrium;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::TAU;

pub struct RotorPlugin;

impl Plugin for RotorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RotorEquilibrium>().add_systems(
            Update,
            (
                regenerate_rotor.run_if(crate::config::rotor_shape_changed),
                (super::torque::solve_equilibrium, animate_rotor).chain(),
            ),
        );
    }
}

/// Marker for the root rotor entity that rotates.
#[derive(Component)]
pub struct RotorRoot;

/// Marker for all rotor geometry entities (for cleanup).
#[derive(Component)]
pub struct RotorPart;

fn regenerate_rotor(
    mut commands: Commands,
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    rotor: Res<RotorConfig>,
    query: Query<Entity, With<RotorPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Despawn old rotor. `RotorRoot` also carries `RotorPart`, and despawning
    // it recursively removes its children (which carry `RotorPart` too), so
    // `try_despawn` is used to skip entities already gone by the time this
    // command runs.
    for entity in &query {
        commands.entity(entity).try_despawn();
    }

    if !view.show_rotor {
        return;
    }

    let p = rotor.effective_pole_pairs(&config);
    let h = STATOR_HEIGHT;
    let r = ROTOR_RADIUS * 0.8;

    // Materials
    let iron_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.7, 0.75),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });
    let north_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1), // North - Red
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });
    let south_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.8), // South - Blue
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });

    // Rotor root
    commands
        .spawn((
            RotorRoot,
            RotorPart,
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // Central solid iron core (shaft/body)
            let r_core = r * 0.75;
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(r_core, h))),
                MeshMaterial3d(iron_mat),
                Transform::default(),
                RotorPart,
            ));

            // Poles (2 * p) - each pole is a surface segment
            let p_count = 2 * p;
            let pole_angle = TAU / (p_count as f32);

            for i in 0..p_count {
                let a_start = i as f32 * pole_angle;
                let mat = if i % 2 == 1 {
                    north_mat.clone()
                } else {
                    south_mat.clone()
                };

                parent.spawn((
                    Mesh3d(meshes.add(build_rotor_sector_mesh(
                        r_core, r, h, a_start, pole_angle, 16,
                    ))),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    RotorPart,
                ));
            }
        });
}

/// Creates a 3D sector (annular) of a cylinder.
fn build_rotor_sector_mesh(
    r_inner: f32,
    r_outer: f32,
    height: f32,
    start_angle: f32,
    sweep_angle: f32,
    segments: u32,
) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let half_h = height / 2.0;

    // Normals:
    // Top = +Y
    // Bot = -Y
    // Outer = radial
    // Inner = -radial (inward)
    // Left side = orthogonal to radius
    // Right side = orthogonal to radius

    let add_wall = |pos: &mut Vec<[f32; 3]>,
                    nor: &mut Vec<[f32; 3]>,
                    idx: &mut Vec<u32>,
                    radius: f32,
                    outward: bool| {
        let base = pos.len() as u32;
        let n_dir = if outward { 1.0 } else { -1.0 };
        for i in 0..=segments {
            let frac = i as f32 / segments as f32;
            let angle = start_angle + frac * sweep_angle;
            let (c, s) = (angle.cos(), angle.sin());
            pos.push([radius * c, half_h, radius * s]);
            nor.push([c * n_dir, 0.0, s * n_dir]);
            pos.push([radius * c, -half_h, radius * s]);
            nor.push([c * n_dir, 0.0, s * n_dir]);
        }
        for i in 0..segments {
            let b = base + i * 2;
            if outward {
                idx.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
            } else {
                idx.extend_from_slice(&[b, b + 1, b + 3, b, b + 3, b + 2]);
            }
        }
    };

    let add_cap = |pos: &mut Vec<[f32; 3]>,
                   nor: &mut Vec<[f32; 3]>,
                   idx: &mut Vec<u32>,
                   y: f32,
                   top: bool| {
        let base = pos.len() as u32;
        let ny = if top { 1.0 } else { -1.0 };
        for i in 0..=segments {
            let frac = i as f32 / segments as f32;
            let angle = start_angle + frac * sweep_angle;
            let (c, s) = (angle.cos(), angle.sin());
            pos.push([r_outer * c, y, r_outer * s]);
            nor.push([0.0, ny, 0.0]);
            pos.push([r_inner * c, y, r_inner * s]);
            nor.push([0.0, ny, 0.0]);
        }
        for i in 0..segments {
            let b = base + i * 2;
            if top {
                idx.extend_from_slice(&[b, b + 1, b + 3, b, b + 3, b + 2]);
            } else {
                idx.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
            }
        }
    };

    let add_radial = |pos: &mut Vec<[f32; 3]>,
                      nor: &mut Vec<[f32; 3]>,
                      idx: &mut Vec<u32>,
                      angle: f32,
                      start_side: bool| {
        let base = pos.len() as u32;
        let (c, s) = (angle.cos(), angle.sin());
        let n_dir = if start_side { 1.0 } else { -1.0 };
        let nx = n_dir * s;
        let nz = n_dir * (-c);
        let n = [nx, 0.0, nz];

        pos.push([r_inner * c, half_h, r_inner * s]);
        pos.push([r_inner * c, -half_h, r_inner * s]);
        pos.push([r_outer * c, half_h, r_outer * s]);
        pos.push([r_outer * c, -half_h, r_outer * s]);
        for _ in 0..4 {
            nor.push(n);
        }
        if start_side {
            idx.extend_from_slice(&[base, base + 3, base + 2, base, base + 1, base + 3]);
        } else {
            idx.extend_from_slice(&[base, base + 2, base + 3, base, base + 3, base + 1]);
        }
    };

    add_wall(&mut positions, &mut normals, &mut indices, r_outer, true);
    add_wall(&mut positions, &mut normals, &mut indices, r_inner, false);
    add_cap(&mut positions, &mut normals, &mut indices, half_h, true);
    add_cap(&mut positions, &mut normals, &mut indices, -half_h, false);
    add_radial(
        &mut positions,
        &mut normals,
        &mut indices,
        start_angle,
        true,
    );
    add_radial(
        &mut positions,
        &mut normals,
        &mut indices,
        start_angle + sweep_angle,
        false,
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
/// Mechanical angle the rotor root is turned to, before Bevy's sign flip.
///
/// The equilibrium is solved against the field of this very frame, so the
/// synchronous rotation is *already inside* `resting`. Adding `state.angle /
/// p_s` on top of it — as the earlier extrapolating version had to — spins the
/// rotor at twice the field's speed. Pulled out of the system so that stays a
/// testable claim rather than a comment.
fn rotor_rotation(
    equilibrium: &RotorEquilibrium,
    config: &MotorConfig,
    rotor_pole_pairs: usize,
    load_angle: f32,
    elec_angle: f32,
) -> f32 {
    // The fallback is the old geometric alignment, and it is only ever reached
    // when the field has no grip on this rotor at all. Nothing is holding it
    // anywhere then, so one position is as defensible as another, and this one
    // at least lines a pole up with the resultant arrow — which carries the
    // synchronous rotation of its own accord.
    let resting = equilibrium.resting().unwrap_or_else(|| {
        crate::winding::axis::resultant_axis(config, 0, elec_angle)
            - TAU / (4.0 * rotor_pole_pairs.max(1) as f32)
    });

    // How far the rotor is held from that rest. Subtracted, so a positive angle
    // lags the field — the direction a load pulls a motor. `settle` is what
    // folds a command past half a well into the next one, which is the slip.
    resting - equilibrium.settle(load_angle)
}

fn animate_rotor(
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    rotor: Res<RotorConfig>,
    equilibrium: Res<RotorEquilibrium>,
    state: Res<ElectricalState>,
    mut query: Query<&mut Transform, With<RotorRoot>>,
) {
    if !view.show_rotor {
        return;
    }

    let rotation = rotor_rotation(
        &equilibrium,
        &config,
        rotor.effective_pole_pairs(&config),
        rotor.load_angle,
        state.angle,
    );

    // Apply rotation (CCW direction to match vectors)
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_y(-rotation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rotor::torque;
    use bevy::prelude::default;
    use std::f32::consts::PI;

    const GRADIENT: f32 = 2.0;

    fn config(pole_pairs: usize) -> MotorConfig {
        MotorConfig {
            pole_pairs,
            groove_count: 72,
            phases: 3,
            ..default()
        }
    }

    /// Regression: one full electrical turn must advance the rotor by exactly
    /// one pole-pair pitch. An earlier version added the synchronous rotation
    /// on top of an equilibrium that already carried it, and the rotor ran at
    /// double speed — which no test of the solver alone could have caught.
    #[test]
    fn a_full_electrical_turn_advances_exactly_one_pole_pair_pitch() {
        const FRAMES: usize = 180;

        // Both paths: a pair the field grips, and a pair it ignores, which
        // falls back on geometry and has to track the field just the same.
        for (p_s, p_r) in [(1_usize, 1_usize), (2, 2), (3, 3), (3, 1), (2, 3)] {
            let config = config(p_s);
            let expected = TAU / p_s as f32;

            let mut chosen: Option<f32> = None;
            let mut start = 0.0;
            let mut total = 0.0;
            let mut last = 0.0;

            for frame in 0..=FRAMES {
                let elec = frame as f32 * TAU / FRAMES as f32;
                let mut solved = torque::solve(&config, p_r, GRADIENT, elec);
                solved.chosen = torque::pick_nearest(&solved, chosen);
                chosen = solved.chosen;

                let now = rotor_rotation(&solved, &config, p_r, 0.0, elec);
                if frame == 0 {
                    start = now;
                } else {
                    // Unwrapped, so a walk that crosses the seam still counts.
                    let step = (now - last + PI).rem_euclid(TAU) - PI;
                    total += step;
                }
                last = now;
            }

            let _ = start;
            assert!(
                (total - expected).abs() < 0.05,
                "p_s={p_s} p_r={p_r}: turned {total} over an electrical turn, expected {expected}"
            );
        }
    }
}
