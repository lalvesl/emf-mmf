use crate::config::{MotorConfig, MotorConfigChanged, ROTOR_RADIUS, STATOR_HEIGHT};
use crate::eletrical::ElectricalState;
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
    let r = ROTOR_RADIUS;

    // Materials
    let north_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1), // North - Red
        emissive: LinearRgba::new(0.4, 0.0, 0.0, 1.0),
        ..default()
    });
    let south_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.8), // South - Blue
        emissive: LinearRgba::new(0.0, 0.0, 0.4, 1.0),
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
            // Poles (2 * p) - each pole is a Cylinder Sector
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
                    Mesh3d(meshes.add(build_rotor_sector_mesh(r, h, a_start, pole_angle, 16))),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    RotorPart,
                ));
            }
        });
}

/// Creates a 3D sector (wedge) of a cylinder.
fn build_rotor_sector_mesh(
    radius: f32,
    height: f32,
    start_angle: f32,
    sweep_angle: f32,
    segments: u32,
) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let half_h = height / 2.0;

    // Vertices:
    // 0: top center
    // 1: bot center
    positions.push([0.0, half_h, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    positions.push([0.0, -half_h, 0.0]);
    normals.push([0.0, -1.0, 0.0]);

    // Arc vertices
    for i in 0..=segments {
        let frac = i as f32 / segments as f32;
        let angle = start_angle + frac * sweep_angle;
        let c = angle.cos();
        let s = angle.sin();

        // Top arc
        positions.push([radius * c, half_h, radius * s]);
        normals.push([0.0, 1.0, 0.0]);

        // Bot arc
        positions.push([radius * c, -half_h, radius * s]);
        normals.push([0.0, -1.0, 0.0]);
    }

    // Indices for Top cap
    let top_center = 0;
    for i in 0..segments {
        let v0 = 2 + i * 2;
        let v1 = 2 + (i + 1) * 2;
        indices.extend_from_slice(&[top_center, v0, v1]);
    }

    // Indices for Bottom cap
    let bot_center = 1;
    for i in 0..segments {
        let v0 = 2 + i * 2 + 1;
        let v1 = 2 + (i + 1) * 2 + 1;
        indices.extend_from_slice(&[bot_center, v1, v0]);
    }

    // Side walls (the curved part)
    // We need new vertices for the side walls to have radial normals
    let side_start_idx = positions.len() as u32;
    for i in 0..=segments {
        let frac = i as f32 / segments as f32;
        let angle = start_angle + frac * sweep_angle;
        let c = angle.cos();
        let s = angle.sin();

        // Top arc wall
        positions.push([radius * c, half_h, radius * s]);
        normals.push([c, 0.0, s]);

        // Bot arc wall
        positions.push([radius * c, -half_h, radius * s]);
        normals.push([c, 0.0, s]);
    }

    for i in 0..segments {
        let v0 = side_start_idx + i * 2;
        let v1 = side_start_idx + (i + 1) * 2;
        let v2 = v0 + 1;
        let v3 = v1 + 1;
        // Two triangles for each segment quad
        indices.extend_from_slice(&[v0, v1, v3, v0, v3, v2]);
    }

    // Radial faces (flat sides of the wedge)
    // Start face
    {
        let angle = start_angle;
        let c = angle.cos();
        let s = angle.sin();
        let n = [s, 0.0, -c]; // Normal perpendicular to radial vector

        let base = positions.len() as u32;
        positions.push([0.0, half_h, 0.0]);
        positions.push([0.0, -half_h, 0.0]);
        positions.push([radius * c, half_h, radius * s]);
        positions.push([radius * c, -half_h, radius * s]);
        for _ in 0..4 {
            normals.push(n);
        }
        indices.extend_from_slice(&[base, base + 2, base + 3, base, base + 3, base + 1]);
    }

    // End face
    {
        let angle = start_angle + sweep_angle;
        let c = angle.cos();
        let s = angle.sin();
        let n = [-s, 0.0, c];

        let base = positions.len() as u32;
        positions.push([0.0, half_h, 0.0]);
        positions.push([0.0, -half_h, 0.0]);
        positions.push([radius * c, half_h, radius * s]);
        positions.push([radius * c, -half_h, radius * s]);
        for _ in 0..4 {
            normals.push(n);
        }
        indices.extend_from_slice(&[base, base + 3, base + 2, base, base + 1, base + 3]);
    }

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

    // Synchronous speed: mech_angle = elec_angle / p
    // We update all rotor instances
    for mut transform in &mut query {
        transform.rotation = Quat::from_rotation_y(state.angle / p);
    }
}
