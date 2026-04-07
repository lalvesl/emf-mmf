use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::{PI, TAU};

use super::{Direction, WindingPart};
use crate::config::*;

// ─── Arc tube mesh builder ─────────────────────────────────────────────────────

/// Builds a single tube mesh that follows the arc of an endwinding.
///
/// The arc sweeps from `a_from` to `a_to` (handling wrap-around), parabola-
/// lifted by `y_offset`, at radius `r_mid`.  The cross-section is a square with
/// side `wire_size` approximated by `cross_sides` triangles.
struct ArcTubeParams {
    a_from: f32,
    a_diff: f32,
    r_mid: f32,
    y_base: f32,
    y_offset: f32,
    wire_size: f32,
    arc_segments: usize,
    cross_sides: usize,
}

fn build_arc_tube_mesh(params: ArcTubeParams) -> Mesh {
    let a_from = params.a_from;
    let a_diff = params.a_diff;
    let r_mid = params.r_mid;
    let y_base = params.y_base;
    let y_offset = params.y_offset;
    let wire_size = params.wire_size;
    let arc_segments = params.arc_segments;
    let cross_sides = params.cross_sides;
    // Centre-line points + Frenet frames
    let n = arc_segments + 1;
    let mut centers: Vec<Vec3> = Vec::with_capacity(n);
    let mut tangents: Vec<Vec3> = Vec::with_capacity(n);

    for seg in 0..n {
        let t = seg as f32 / arc_segments as f32;
        let a = a_from + a_diff * t;
        let y = y_base + y_offset * (PI * t).sin();
        centers.push(Vec3::new(r_mid * a.cos(), y, r_mid * a.sin()));
    }

    // Finite-difference tangents
    for i in 0..n {
        let t = if i == 0 {
            centers[1] - centers[0]
        } else if i == n - 1 {
            centers[n - 1] - centers[n - 2]
        } else {
            centers[i + 1] - centers[i - 1]
        };
        tangents.push(t.normalize_or_zero());
    }

    // Build a consistent "up" reference perpendicular to the first tangent
    let world_up = Vec3::Y;

    let ring_verts = cross_sides;
    let total_verts = n * ring_verts;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_verts);

    // Parallel-transport frame
    let mut prev_up = {
        let t0 = tangents[0];
        // pick an initial "up" that isn't collinear with t0
        let candidate = if t0.dot(world_up).abs() < 0.9 {
            world_up
        } else {
            Vec3::X
        };
        t0.cross(candidate).cross(t0).normalize_or_zero()
    };

    for (i, (&center, &tang)) in centers.iter().zip(tangents.iter()).enumerate() {
        // Transport the up vector along the tangent changes
        if i > 0 {
            let prev_tang = tangents[i - 1];
            let rot_axis = prev_tang.cross(tang);
            let sin_a = rot_axis.length();
            if sin_a > 1e-6 {
                let rotation = Quat::from_axis_angle(rot_axis.normalize(), sin_a.asin());
                prev_up = rotation * prev_up;
            }
            let rejected = prev_up.reject_from(tang).normalize_or_zero();
            if rejected.is_finite() {
                prev_up = rejected;
            }
        }
        let right = tang.cross(prev_up).normalize_or_zero();
        let up = right.cross(tang).normalize_or_zero();

        let half = wire_size * 0.5;
        let ring_u = i as f32 / arc_segments as f32;

        for j in 0..ring_verts {
            let angle = j as f32 / ring_verts as f32 * TAU;
            let (s, c) = angle.sin_cos();
            let offset = right * (c * half) + up * (s * half);
            let pos = center + offset;
            let norm = offset.normalize_or_zero();
            positions.push(pos.into());
            normals.push(norm.into());
            uvs.push([ring_u, j as f32 / ring_verts as f32]);
        }
    }

    // Indices — connect rings into quads
    let mut indices: Vec<u32> = Vec::new();
    let rv = ring_verts as u32;
    for i in 0..(arc_segments as u32) {
        for j in 0..rv {
            let a = i * rv + j;
            let b = i * rv + (j + 1) % rv;
            let c = (i + 1) * rv + j;
            let d = (i + 1) * rv + (j + 1) % rv;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

// ─── Public render functions ──────────────────────────────────────────────────

pub fn render_conductors(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    data: &super::WindingData,
    phase_mats: &[Handle<StandardMaterial>],
) {
    let assignments = data.assignments;
    let segment_angle = data.segment_angle;
    let tooth_angle = data.tooth_angle;
    let r_bore = data.r_bore;
    let r_slot_bot = data.r_slot_bot;

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
}

pub fn render_header_coils(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    data: &super::WindingData,
    phase_mats: &[Handle<StandardMaterial>],
) {
    if !data.config.show_endwindings {
        return;
    }

    let n = data.config.groove_count;
    let assignments = data.assignments;
    let segment_angle = data.segment_angle;
    let tooth_angle = data.tooth_angle;
    let half_h = data.half_h;
    let pitch = data.pitch;

    // Arc geometry constants (shared across all arcs)
    let arc_segments = 24; // was 120 separate entities; now 24 segments per tube mesh
    let cross_sides = 6;   // hexagonal cross-section approximation
    let r_mid = (STATOR_BORE_RADIUS + slot_bottom_radius()) / 2.0;
    let wire_size = 0.04;

    // --- Endwindings (arcs connecting coil sides) ---
    for (i, assignment) in assignments.iter().enumerate() {
        let Some(assign) = assignment else { continue };
        if assign.direction == Direction::Out {
            continue; // Only draw endwindings from the "In" side
        }
        let return_slot = (i + pitch) % n;
        let mat = phase_mats[assign.phase % phase_mats.len()].clone();

        let a_from = i as f32 * segment_angle + tooth_angle + segment_angle * 0.25;
        let a_to = return_slot as f32 * segment_angle + tooth_angle + segment_angle * 0.25;

        // Handle wrap-around
        let mut a_diff = a_to - a_from;
        if a_diff < 0.0 {
            a_diff += TAU;
        }

        let y_offset_top = 0.15 + (assign.phase as f32 * 0.08);
        let y_base_top = half_h + 0.05;
        let y_offset_bot = -(0.15 + (assign.phase as f32 * 0.08));
        let y_base_bot = -half_h - 0.05;

        // Top endwinding — one entity with one merged tube mesh
        let mesh_top = build_arc_tube_mesh(ArcTubeParams {
            a_from,
            a_diff,
            r_mid,
            y_base: y_base_top,
            y_offset: y_offset_top,
            wire_size,
            arc_segments,
            cross_sides,
        });
        commands.spawn((
            Mesh3d(meshes.add(mesh_top)),
            MeshMaterial3d(mat.clone()),
            Transform::default(),
            WindingPart,
        ));

        // Bottom endwinding — one entity with one merged tube mesh
        let mesh_bot = build_arc_tube_mesh(ArcTubeParams {
            a_from,
            a_diff,
            r_mid,
            y_base: y_base_bot,
            y_offset: y_offset_bot,
            wire_size,
            arc_segments,
            cross_sides,
        });
        commands.spawn((
            Mesh3d(meshes.add(mesh_bot)),
            MeshMaterial3d(mat),
            Transform::default(),
            WindingPart,
        ));
    }
}
