mod camera;
mod colors;
mod config;
mod eletrical;
mod i18n;
mod stator;
mod ui;
mod winding;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "EMF-MMF — Stator Winding Simulator".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ui::UiPlugin)
        .init_resource::<config::MotorConfig>()
        .add_message::<config::MotorConfigChanged>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera::orbit_camera,
                stator::regenerate_stator,
                winding::regenerate_winding,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
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
