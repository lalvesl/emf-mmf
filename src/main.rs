use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .run();
}

#[derive(Component)]
struct Rotator;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 3D Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.8, 0.4))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        Rotator,
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 1500.0,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // 3D Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Dir3::Y),
    ));
}

fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Rotator>>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs());
    }
}
