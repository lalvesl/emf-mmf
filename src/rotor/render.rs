use crate::config::{MotorConfig, MotorConfigChanged, ROTOR_RADIUS, STATOR_HEIGHT};
use crate::eletrical::ElectricalState;
use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

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
    let iron_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.7, 0.7, 0.75),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });
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
            // Main body
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(r * 0.95, h))),
                MeshMaterial3d(iron_mat),
                Transform::default(),
            ));

            // Poles (2 * p)
            let pole_angle = TAU / (2.0 * p as f32);
            for i in 0..(2 * p) {
                let a = i as f32 * pole_angle;
                let mat = if i % 2 == 0 {
                    north_mat.clone()
                } else {
                    south_mat.clone()
                };

                // Marker for pole geometry
                parent.spawn((
                    Mesh3d(meshes.add(Cylinder::new(r, h * 0.98))), // Slightly shorter poles
                    MeshMaterial3d(mat),
                    Transform {
                        translation: Vec3::new((r * 0.9) * a.cos(), 0.0, (r * 0.9) * a.sin()),
                        rotation: Quat::from_rotation_y(-a),
                        scale: Vec3::new(0.2, 1.0, 0.1), // Squashed cylinders as pole faces
                    },
                ));
            }
        });
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
