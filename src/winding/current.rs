use bevy::prelude::*;
use std::f32::consts::PI;

use super::{Direction, SlotAssignment, WindingPart};
use crate::config::*;

pub fn render_current_directions(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    config: &MotorConfig,
    assignments: &[Option<SlotAssignment>],
    phase_mats_opp: &[Handle<StandardMaterial>],
    segment_angle: f32,
    tooth_angle: f32,
    r_bore: f32,
    r_slot_bot: f32,
) {
    if config.show_endwindings {
        return;
    }

    // --- Show current directions (crosses for In, dots for Out) over the coils ---
    for (i, assignment) in assignments.iter().enumerate() {
        let Some(assign) = assignment else { continue };
        let mat = phase_mats_opp[assign.phase % phase_mats_opp.len()].clone();

        let slot_center = i as f32 * segment_angle + tooth_angle + segment_angle * 0.25;
        let r_mid = (r_bore + r_slot_bot) / 2.0;

        let wire_height = STATOR_HEIGHT * 0.95;
        let wire_radial = (r_slot_bot - r_bore) * 0.55;
        let wire_tangential = segment_angle * 0.35 * r_mid;

        let x = r_mid * slot_center.cos();
        let z = r_mid * slot_center.sin();
        let top_y = wire_height / 2.0 + 0.002;
        let bottom_y = -wire_height / 2.0 - 0.002;

        let symbol_radius = wire_tangential.min(wire_radial) * 0.45;
        let line_thickness = symbol_radius * 0.25;

        if assign.direction == Direction::In {
            // Cross (X)
            let length = symbol_radius * 2.0;
            let bar = Cuboid::new(length, line_thickness, line_thickness);
            let mesh_bar = meshes.add(bar);

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
            let dot = Cylinder::new(symbol_radius * 0.6, line_thickness);
            let mesh_dot = meshes.add(dot);

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
