use bevy::prelude::*;
use std::f32::consts::{PI, TAU};

use crate::config::*;

/// Direction of current flow in a slot conductor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

/// Assignment of a conductor to a slot.
#[derive(Clone, Debug)]
pub struct SlotAssignment {
    pub phase: usize,
    pub direction: Direction,
}

/// Computes the winding distribution: which phase goes in which slot.
pub fn compute_winding(config: &MotorConfig) -> Vec<Option<SlotAssignment>> {
    let n = config.groove_count;
    let m = config.phases;
    let p = config.pole_pairs;

    // Configuration must be valid: n divisible by (2 * p * m)
    if m == 0 || p == 0 || n < 2 * p * m || !n.is_multiple_of(2 * p * m) {
        return vec![None; n];
    }

    let q = n / (2 * p * m); // slots per pole per phase
    let slots_per_pole = n / (2 * p);

    let mut assignments: Vec<Option<SlotAssignment>> = vec![None; n];

    for pole in 0..(2 * p) {
        let direction = if pole % 2 == 0 {
            Direction::In
        } else {
            Direction::Out
        };

        for phase in 0..m {
            for j in 0..q {
                let slot_idx = (pole * slots_per_pole + phase * q + j) % n;
                assignments[slot_idx] = Some(SlotAssignment { phase, direction });
            }
        }
    }

    assignments
}

/// Coil pitch in number of slots.
pub fn coil_pitch(config: &MotorConfig) -> usize {
    let slots_per_pole = config.groove_count / (2 * config.pole_pairs);
    if config.short_pitched {
        slots_per_pole.saturating_sub(1).max(1)
    } else {
        slots_per_pole
    }
}

/// Spawn an endwinding arc connecting two slots above or below the stator.
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

/// System: generate winding conductors and endwindings when config changes.
pub fn regenerate_winding(
    mut commands: Commands,
    config: Res<MotorConfig>,
    query: Query<Entity, With<WindingPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !config.is_changed() {
        return;
    }

    // Despawn old winding geometry
    for entity in &query {
        commands.entity(entity).despawn();
    }

    let assignments = compute_winding(&config);
    let n = config.groove_count;
    let segment_angle = TAU / n as f32;
    let tooth_angle = segment_angle * 0.5;
    let half_h = STATOR_HEIGHT / 2.0;
    let r_bore = STATOR_BORE_RADIUS;
    let r_slot_bot = slot_bottom_radius();
    let pitch = coil_pitch(&config);

    // Pre-create phase materials (emissive for visibility)
    let phase_mats: Vec<_> = (0..config.phases)
        .map(|p| {
            let color = phase_color(p);
            materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })
        })
        .collect();

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
    for (i, assignment) in assignments.iter().enumerate() {
        let Some(assign) = assignment else { continue };
        if assign.direction == Direction::Out {
            continue; // Only draw endwindings from the "In" side
        }
        let return_slot = (i + pitch) % n;
        let mat = phase_mats[assign.phase % phase_mats.len()].clone();

        // Top endwinding arc
        spawn_endwinding_arc!(
            &mut commands,
            &mut meshes,
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
            &mut commands,
            &mut meshes,
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

/// Marker for winding entities.
#[derive(Component)]
pub struct WindingPart;
