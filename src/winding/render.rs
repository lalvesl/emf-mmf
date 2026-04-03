use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

use super::{Direction, SlotAssignment, WindingPart};
use crate::config::*;

macro_rules! spawn_endwinding_arc {
    ($commands:expr, $meshes:expr, $material:expr, $from_slot:expr, $to_slot:expr, $total_slots:expr, $y_base:expr, $y_offset:expr, $tooth_angle:expr, $segment_angle:expr) => {{
        let material = $material;
        let from_slot = $from_slot;
        let to_slot = $to_slot;
        let y_base = $y_base;
        let y_offset = $y_offset;
        let tooth_angle = $tooth_angle;
        let segment_angle = $segment_angle;

        let arc_segments = 120;
        let r_mid = (STATOR_BORE_RADIUS + slot_bottom_radius()) / 2.0;
        let wire_size = 0.04;

        let a_from = from_slot as f32 * segment_angle + tooth_angle + segment_angle * 0.25;
        let a_to = to_slot as f32 * segment_angle + tooth_angle + segment_angle * 0.25;

        // Handle wrapping
        let mut a_diff = a_to - a_from;
        if a_diff < 0.0 {
            a_diff += TAU;
        }

        for seg in 0..arc_segments {
            let t0 = seg as f32 / arc_segments as f32;
            let t1 = (seg + 1) as f32 / arc_segments as f32;

            let a0 = a_from + a_diff * t0;
            let a1 = a_from + a_diff * t1;
            let y0 = y_base + y_offset * (PI * t0).sin();
            let y1 = y_base + y_offset * (PI * t1).sin();

            let p0 = Vec3::new(r_mid * a0.cos(), y0, r_mid * a0.sin());
            let p1 = Vec3::new(r_mid * a1.cos(), y1, r_mid * a1.sin());

            let mid = (p0 + p1) / 2.0;
            let dir = p1 - p0;
            let len = dir.length();
            if len >= 0.001 {
                let cube = Cuboid::new(wire_size, wire_size, len);
                let rotation = Quat::from_rotation_arc(Vec3::Z, dir.normalize());

                $commands.spawn((
                    Mesh3d($meshes.add(cube)),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(mid).with_rotation(rotation),
                    WindingPart,
                ));
            }
        }
    }};
}

pub fn render_coils(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    config: &MotorConfig,
    assignments: &[Option<SlotAssignment>],
    phase_mats: &[Handle<StandardMaterial>],
    segment_angle: f32,
    tooth_angle: f32,
    half_h: f32,
    r_bore: f32,
    r_slot_bot: f32,
    pitch: usize,
) {
    let n = config.groove_count;

    // --- Conductors inside slots ---
    for (i, assignment) in assignments.iter().enumerate() {
        let Some(assign) = assignment else { continue };
        let mat = phase_mats[assign.phase % phase_mats.len()].clone();

        // Slot center angle (slot sits after the tooth)
        let slot_center = i as f32 * segment_angle + tooth_angle + segment_angle * 0.25;
        let r_mid = (r_bore + r_slot_bot) / 2.0;

        let wire_height = STATOR_HEIGHT * 0.95;
        let wire_radial = (r_slot_bot - r_bore) * 0.55;
        let wire_tangential = segment_angle * 0.35 * r_mid;

        let x = r_mid * slot_center.cos();
        let z = r_mid * slot_center.sin();

        // Cuboid oriented radially
        let cube = Cuboid::new(wire_tangential, wire_height, wire_radial);
        commands.spawn((
            Mesh3d(meshes.add(cube)),
            MeshMaterial3d(mat),
            Transform::from_xyz(x, 0.0, z)
                .with_rotation(Quat::from_rotation_y(-slot_center + PI / 2.0)),
            WindingPart,
        ));
    }

    // --- Endwindings (arcs connecting coil sides) ---
    if config.show_endwindings {
        for (i, assignment) in assignments.iter().enumerate() {
            let Some(assign) = assignment else { continue };
            if assign.direction == Direction::Out {
                continue; // Only draw endwindings from the "In" side
            }
            let return_slot = (i + pitch) % n;
            let mat = phase_mats[assign.phase % phase_mats.len()].clone();

            // Top endwinding arc
            spawn_endwinding_arc!(
                commands,
                meshes,
                mat.clone(),
                i,
                return_slot,
                n,
                half_h + 0.05,
                0.15 + (assign.phase as f32 * 0.08),
                tooth_angle,
                segment_angle
            );

            // Bottom endwinding arc
            spawn_endwinding_arc!(
                commands,
                meshes,
                mat,
                i,
                return_slot,
                n,
                -half_h - 0.05,
                -(0.15 + (assign.phase as f32 * 0.08)),
                tooth_angle,
                segment_angle
            );
        }
    }
}
