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
        height: 0.2,
    });

    // The resultant arrow looks the same for every pole, so its meshes and
    // material are built once and shared instead of per-pole.
    let res_shaft = meshes.add(Cylinder::new(0.04, 1.0));
    let res_head = meshes.add(Cone {
        radius: 0.08,
        height: 0.3,
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
                        Transform::from_xyz(0.0, 0.5, 0.0),
                    ));
                    parent.spawn((
                        Mesh3d(head_mesh.clone()),
                        MeshMaterial3d(mat),
                        Transform::from_xyz(0.0, 1.0, 0.0),
                    ));
                });
        }

        // Spawn Resultant Vector (White)
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
                    Transform::from_xyz(0.0, 0.5, 0.0),
                ));
                parent.spawn((
                    Mesh3d(res_head.clone()),
                    MeshMaterial3d(res_mat.clone()),
                    Transform::from_xyz(0.0, 1.0, 0.0),
                ));
            });
    }
}

fn animate_vectors(
    config: Res<MotorConfig>,
    state: Res<ElectricalState>,
    mut query: Query<(&MmfVector, &mut Transform)>,
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
            let current = axis::phase_current(elec_angle, phase, alpha_m);
            let axis_phys = axis::magnetic_axis(&config, phase, pole);

            let mmf_amplitude = current * if pole % 2 == 0 { 1.0 } else { -1.0 };

            let dir = Vec3::new(axis_phys.cos(), 0.0, axis_phys.sin());
            let scaled_vec = dir * mmf_amplitude;

            *phase_vec_entry = scaled_vec;
            resultant_vecs[pole] += scaled_vec;
        }
    }

    for (vector, mut transform) in &mut query {
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
        let scale_length = (length * max_radius) / max_ideal.max(1.0);

        if length > 0.001 {
            let normalized = target_vec / length;
            let rot = Quat::from_rotation_arc(Vec3::Y, normalized);
            transform.rotation = rot;

            transform.scale = Vec3::new(1.0, scale_length.max(0.01), 1.0);
        } else {
            transform.scale = Vec3::ZERO;
        }
    }
}
