use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

use crate::config::{MotorConfig, MotorConfigChanged};
use crate::eletrical::ElectricalState;

pub struct VectorsPlugin;

impl Plugin for VectorsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (regenerate_vectors, animate_vectors));
    }
}

/// Identifies an MMF arrow. `None` represents the resultant vector.
#[derive(Component)]
pub struct MmfVector {
    pub phase: Option<usize>,
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

    let m = config.phases;
    if m == 0 {
        return;
    }

    let shaft_mesh = meshes.add(Cylinder::new(0.04, 1.0));
    let head_mesh = meshes.add(Cone {
        radius: 0.1,
        height: 0.3,
    });

    // Spawn Phase Vectors
    for phase in 0..m {
        let color = crate::colors::phase_color(phase);
        let mat = materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            ..default()
        });

        commands
            .spawn((
                Transform::default(),
                Visibility::default(),
                MmfVector { phase: Some(phase) },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(shaft_mesh.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                ));
                parent.spawn((
                    Mesh3d(head_mesh.clone()),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 1.0, 0.0),
                ));
            });
    }

    let res_shaft = meshes.add(Cylinder::new(0.06, 1.0));
    let res_head = meshes.add(Cone {
        radius: 0.15,
        height: 0.4,
    });

    // Spawn Resultant Vector (White)
    let res_color = Color::WHITE;
    let res_mat = materials.add(StandardMaterial {
        base_color: res_color,
        emissive: res_color.into(),
        ..default()
    });

    commands
        .spawn((
            Transform::default(),
            Visibility::default(),
            MmfVector { phase: None },
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(res_shaft),
                MeshMaterial3d(res_mat.clone()),
                Transform::from_xyz(0.0, 0.5, 0.0),
            ));
            parent.spawn((
                Mesh3d(res_head),
                MeshMaterial3d(res_mat),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ));
        });
}

fn animate_vectors(
    config: Res<MotorConfig>,
    state: Res<ElectricalState>,
    mut query: Query<(&MmfVector, &mut Transform)>,
) {
    let m = config.phases;
    if m == 0 {
        return;
    }

    // Since we are creating a theoretical vector phasor diagram centrally on the stator core (visualising MMF amplitudes),
    // mapping it purely to electrical degrees prevents the squashing illusion typical when physically superposing components over p>1 configurations!
    // The resultant vector perfectly forms a synchronous constant-magnitude rotating field.
    let p = config.pole_pairs as f32;
    let elec_angle = state.angle;
    let max_radius = crate::config::STATOR_BORE_RADIUS * 0.9;

    let mut sum_vec = Vec3::ZERO;
    let mut phase_vecs = vec![];
    for phase in 0..m {
        // Shift formula matches motor specifications (symmetry odd vs even)
        let phase_shift_elec = if m % 2 != 0 {
            phase as f32 * TAU / (m as f32)
        } else {
            phase as f32 * PI / (m as f32)
        };

        let current = (elec_angle - phase_shift_elec).cos();

        let axis_rad = phase_shift_elec / p;

        // Form spatial vector for the phase (magnitude modulated by exact current)
        let dir = Vec3::new(axis_rad.cos(), 0.0, axis_rad.sin());
        let scaled_vec = dir * current;

        phase_vecs.push(scaled_vec);
        sum_vec += scaled_vec;
    }

    for (vector, mut transform) in &mut query {
        let (target_vec, max_ideal) = if let Some(phase) = vector.phase {
            (phase_vecs[phase], 1.0)
        } else {
            // MMF magnitude max amplitude is conventionally `m / 2` times peak phase amplitude mathematically.
            (sum_vec, m as f32 / 2.0)
        };

        let length = target_vec.length();
        let scale_length = (length * max_radius) / max_ideal.max(1.0);

        if length > 0.001 {
            let normalized = target_vec / length;
            // The default Arrow template aligns with +Y Axis! Rotate target along the spatial vector direction.
            let rot = Quat::from_rotation_arc(Vec3::Y, normalized);
            transform.rotation = rot;

            // Limit scale so the arrow doesn't become totally invisible or wildly distorted on minimal shifts.
            transform.scale = Vec3::new(1.0, scale_length.max(0.01), 1.0);
        } else {
            transform.scale = Vec3::ZERO;
        }
    }
}
