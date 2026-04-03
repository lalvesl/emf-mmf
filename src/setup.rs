use crate::camera;
use bevy::prelude::*;

pub fn setup(mut commands: Commands) {
    // Ambient light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
        ..default()
    });

    // Directional light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.6, 0.4, 0.0)),
    ));

    // Point light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 4_000_000.0,
            range: 30.0,
            ..default()
        },
        Transform::from_xyz(5.0, 8.0, 5.0),
    ));

    // Camera with orbit controller
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 8.0).looking_at(Vec3::ZERO, Dir3::Y),
        camera::OrbitCamera::default(),
    ));
}
