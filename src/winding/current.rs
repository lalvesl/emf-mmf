use bevy::prelude::*;
use std::f32::consts::PI;

use super::{Direction, WindingPart};

pub fn render_current_directions(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    data: &super::WindingData,
    phase_mats_opp: &[Handle<StandardMaterial>],
) {
    if data.config.show_endwindings {
        return;
    }

    let layout = data.layout;
    let wire_height = data.conductor_height();

    let top_y = wire_height / 2.0 + 0.002;
    let bottom_y = -wire_height / 2.0 - 0.002;

    // Symbols sit on the end face of each conductor, so they scale with it.
    let symbol_radius = layout.wire_radius * 0.8;
    let line_thickness = (symbol_radius * 0.28).max(0.004);

    // The cross bar and the dot are identical in every slot; build one of each
    // up front rather than a fresh asset per slot.
    let mesh_bar = meshes.add(Cuboid::new(
        symbol_radius * 2.0,
        line_thickness,
        line_thickness,
    ));
    let mesh_dot = meshes.add(Cylinder::new(symbol_radius * 0.6, line_thickness));

    // --- Show current directions (crosses for In, dots for Out) over the coils ---
    for conductor in data.conductors {
        let mat = phase_mats_opp[conductor.phase % phase_mats_opp.len()].clone();

        let slot_center = data.slot_center(conductor.slot);
        let position = layout.position(conductor.index, slot_center, 0.0);
        let (x, z) = (position.x, position.z);

        if conductor.direction == Direction::In {
            // Cross (X)
            let rot1 =
                Quat::from_rotation_y(-slot_center + PI / 2.0) * Quat::from_rotation_y(PI / 4.0);
            let rot2 =
                Quat::from_rotation_y(-slot_center + PI / 2.0) * Quat::from_rotation_y(-PI / 4.0);

            // Top
            commands.spawn((
                Mesh3d(mesh_bar.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, top_y, z).with_rotation(rot1),
                WindingPart,
            ));
            commands.spawn((
                Mesh3d(mesh_bar.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, top_y, z).with_rotation(rot2),
                WindingPart,
            ));
            // Bottom
            commands.spawn((
                Mesh3d(mesh_bar.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, bottom_y, z).with_rotation(rot1),
                WindingPart,
            ));
            commands.spawn((
                Mesh3d(mesh_bar.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, bottom_y, z).with_rotation(rot2),
                WindingPart,
            ));
        } else {
            // Dot (cylinder)
            // Top
            commands.spawn((
                Mesh3d(mesh_dot.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, top_y, z),
                WindingPart,
            ));
            // Bottom
            commands.spawn((
                Mesh3d(mesh_dot.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, bottom_y, z),
                WindingPart,
            ));
        }
    }
}
