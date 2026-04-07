use crate::config::{MotorConfig, MotorConfigChanged, ROTOR_RADIUS, STATOR_HEIGHT};
use crate::electrical::ElectricalState;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::TAU;

pub struct RotorPlugin;

impl Plugin for RotorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (regenerate_rotor, animate_rotor));
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
    mut ev_config: MessageReader<MotorConfigChanged>,
    query: Query<Entity, With<RotorPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if ev_config.read().next().is_none() {
        return;
    }

    // Despawn old rotor
    for entity in &query {
        commands.entity(entity).despawn();
    }

    if !config.show_rotor {
        return;
    }

    let p = config.pole_pairs;
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
                let mat = if i % 2 == 0 {
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

fn animate_rotor(
    config: Res<MotorConfig>,
    state: Res<ElectricalState>,
    mut query: Query<&mut Transform, With<RotorRoot>>,
) {
    if !config.show_rotor {
        return;
    }

    let p = config.pole_pairs as f32;
    if p == 0.0 {
        return;
    }

    // --- Synchronization with MMF Vectors ---
    // The Peak of the Resultant MMF field rotates at state.angle / p.
    // We calculate the mechanical offset required to align the Rotor North pole
    // with that peak.

    let n = config.groove_count as f32;
    let m = config.phases as f32;
    let q = n / (2.0 * p * m);
    let pitch = crate::winding::coil_pitch(&config) as f32;
    let alpha = (p * TAU) / n;

    // Magnetic axis of Phase A at state.angle = 0 (same logic as mmf.rs)
    let offset_elec = (q - 1.0 + pitch) / 2.0 * alpha;
    let offset_mech = (TAU / n) * 0.75;
    let mmf_peak_axis_0 = (offset_elec / p) + offset_mech;

    // The center of the rotor's first North pole in its local coordinate system
    let rotor_pole_width = TAU / (2.0 * p);
    let rotor_pole_0_center_local = rotor_pole_width / 2.0; // i.e. PI / (2.0 * p)

    // We want: rotor_pole_0_center_local + rotation_offset = mmf_peak_axis_0
    let rotation_offset = mmf_peak_axis_0 - rotor_pole_0_center_local;

    // Apply rotation (CCW direction to match vectors)
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_y(-(state.angle / p + rotation_offset));
    }
}
