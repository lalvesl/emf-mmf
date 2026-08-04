use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::{PI, TAU};

use super::WindingPart;
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
    /// Radius at the start of the arc — the deep coil side.
    r_from: f32,
    /// Radius at the end of the arc — the shallow coil side it returns into.
    r_to: f32,
    y_base: f32,
    y_offset: f32,
    wire_size: f32,
    arc_segments: usize,
    cross_sides: usize,
}

fn build_arc_tube_mesh(params: ArcTubeParams) -> Mesh {
    let a_from = params.a_from;
    let a_diff = params.a_diff;
    let r_from = params.r_from;
    let r_to = params.r_to;
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
        // Sweep radially as well, so the arc meets the shallow conductor it
        // actually connects to rather than floating at a fixed radius.
        let r = r_from + (r_to - r_from) * t;
        centers.push(Vec3::new(r * a.cos(), y, r * a.sin()));
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
    let layout = data.layout;
    let wire_height = STATOR_HEIGHT * 0.95;

    // Round conductors: a cylinder whose axis already runs along Y, which is
    // the axial direction of the machine. All of them are identical, so the
    // mesh is built once and instanced.
    let wire = meshes.add(Cylinder::new(layout.wire_radius, wire_height));

    for conductor in data.conductors {
        let mat = phase_mats[conductor.phase % phase_mats.len()].clone();
        let position = layout.position(conductor.index, data.slot_center(conductor.slot), 0.0);

        commands.spawn((
            Mesh3d(wire.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(position),
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
    let layout = data.layout;
    let half_h = data.half_h;
    let pitch = data.pitch;

    // Arc geometry constants (shared across all arcs)
    let arc_segments = 24; // was 120 separate entities; now 24 segments per tube mesh
    let cross_sides = 6; // hexagonal cross-section approximation

    // Same gauge as the slot conductors, so a coil reads as one continuous wire.
    let wire_size = layout.wire_radius * 2.0;

    // One arc per conductor that starts a coil; it returns into the shallow
    // half of the slot `pitch` steps away.
    for conductor in data.conductors {
        if !super::starts_coil(conductor, data.config.layers) {
            continue;
        }

        let return_slot = (conductor.slot + pitch) % n;
        let partner = layout.coil_partner(conductor.index);
        let mat = phase_mats[conductor.phase % phase_mats.len()].clone();

        let (r_from, offset_from) = layout.placement(conductor.index);
        let (r_to, offset_to) = layout.placement(partner);

        let a_from = data.slot_center(conductor.slot) + offset_from / r_from;
        let a_to = data.slot_center(return_slot) + offset_to / r_to;

        // Handle wrap-around
        let mut a_diff = a_to - a_from;
        if a_diff < 0.0 {
            a_diff += TAU;
        }

        // Stagger by phase so overlapping coil heads stay readable, and by
        // conductor so the wires of one coil do not fuse into a single blob.
        //
        // All three terms are measured in wire gauges rather than world units:
        // with few conductors the wire is fat and the arcs need to climb higher
        // to stay apart, while with many thin wires the per-conductor term
        // already provides the spread.
        let lift = wire_size * 1.6
            + conductor.phase as f32 * wire_size * 1.1
            + conductor.index as f32 * wire_size * 0.8;
        let y_base_top = half_h + 0.05;
        let y_base_bot = -half_h - 0.05;

        for (y_base, y_offset) in [(y_base_top, lift), (y_base_bot, -lift)] {
            let mesh = build_arc_tube_mesh(ArcTubeParams {
                a_from,
                a_diff,
                r_from,
                r_to,
                y_base,
                y_offset,
                wire_size,
                arc_segments,
                cross_sides,
            });
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(mat.clone()),
                Transform::default(),
                WindingPart,
            ));
        }
    }
}
