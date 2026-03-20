use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::TAU;

use crate::config::*;

/// Marker for all stator geometry entities (for cleanup on regeneration).
#[derive(Component)]
pub struct StatorPart;

/// System: regenerate stator mesh when config changes.
pub fn regenerate_stator(
    mut commands: Commands,
    config: Res<MotorConfig>,
    query: Query<Entity, With<StatorPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !config.is_changed() {
        return;
    }

    // Despawn old geometry
    for entity in &query {
        commands.entity(entity).despawn();
    }

    let n = config.groove_count;
    let r_outer = STATOR_OUTER_RADIUS;
    let r_bore = STATOR_BORE_RADIUS;
    let r_slot_bot = slot_bottom_radius();
    let half_h = STATOR_HEIGHT / 2.0;

    let segment_angle = TAU / n as f32;
    let tooth_angle = segment_angle * 0.5;

    // Stator iron material
    let iron_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.36, 0.40),
        metallic: 0.85,
        perceptual_roughness: 0.25,
        ..default()
    });

    // Yoke: continuous ring from slot_bottom to outer
    let yoke = generate_ring_mesh(r_slot_bot, r_outer, -half_h, half_h, n * 2);
    commands.spawn((
        Mesh3d(meshes.add(yoke)),
        MeshMaterial3d(iron_mat.clone()),
        Transform::default(),
        StatorPart,
    ));

    // Teeth: one sector per groove
    for i in 0..n {
        let a_start = i as f32 * segment_angle;
        let a_end = a_start + tooth_angle;
        let tooth = generate_sector_mesh(r_bore, r_slot_bot, a_start, a_end, -half_h, half_h, 4);
        commands.spawn((
            Mesh3d(meshes.add(tooth)),
            MeshMaterial3d(iron_mat.clone()),
            Transform::default(),
            StatorPart,
        ));
    }
}

// ---------------------------------------------------------------------------
// Mesh generation helpers
// ---------------------------------------------------------------------------

/// Generate a full ring (annulus) mesh extruded along Y.
fn generate_ring_mesh(r_inner: f32, r_outer: f32, y_bot: f32, y_top: f32, segments: usize) -> Mesh {
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    // -- Outer wall --
    add_cylinder_wall(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_outer, y_bot, y_top, 0.0, TAU, segments, true,
    );
    // -- Inner wall --
    add_cylinder_wall(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, y_bot, y_top, 0.0, TAU, segments, false,
    );
    // -- Top cap --
    add_annulus_cap(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, r_outer, y_top, 0.0, TAU, segments, true,
    );
    // -- Bottom cap --
    add_annulus_cap(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, r_outer, y_bot, 0.0, TAU, segments, false,
    );

    build_mesh(pos, nor, uvs, idx)
}

/// Generate a sector of a hollow cylinder (for teeth).
/// Includes outer wall, inner wall, top/bottom caps, and two side walls.
fn generate_sector_mesh(
    r_inner: f32,
    r_outer: f32,
    a_start: f32,
    a_end: f32,
    y_bot: f32,
    y_top: f32,
    segments: usize,
) -> Mesh {
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    // Curved walls
    add_cylinder_wall(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_outer, y_bot, y_top, a_start, a_end, segments,
        true,
    );
    add_cylinder_wall(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, y_bot, y_top, a_start, a_end, segments,
        false,
    );
    // Top/bottom caps
    add_annulus_cap(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, r_outer, y_top, a_start, a_end, segments,
        true,
    );
    add_annulus_cap(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, r_outer, y_bot, a_start, a_end, segments,
        false,
    );
    // Side walls (slot walls)
    add_radial_wall(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, r_outer, y_bot, y_top, a_start, true,
    );
    add_radial_wall(
        &mut pos, &mut nor, &mut uvs, &mut idx, r_inner, r_outer, y_bot, y_top, a_end, false,
    );

    build_mesh(pos, nor, uvs, idx)
}

/// Add a curved cylinder wall (outward or inward facing).
fn add_cylinder_wall(
    pos: &mut Vec<[f32; 3]>,
    nor: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    idx: &mut Vec<u32>,
    radius: f32,
    y_bot: f32,
    y_top: f32,
    a_start: f32,
    a_end: f32,
    segments: usize,
    outward: bool,
) {
    let base = pos.len() as u32;
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = a_start + t * (a_end - a_start);
        let (c, s) = (a.cos(), a.sin());
        let n_dir = if outward { 1.0 } else { -1.0 };
        pos.push([radius * c, y_bot, radius * s]);
        nor.push([n_dir * c, 0.0, n_dir * s]);
        uvs.push([t, 0.0]);
        pos.push([radius * c, y_top, radius * s]);
        nor.push([n_dir * c, 0.0, n_dir * s]);
        uvs.push([t, 1.0]);
    }
    for i in 0..segments {
        let b = base + (i as u32) * 2;
        if outward {
            idx.extend_from_slice(&[b, b + 1, b + 3, b, b + 3, b + 2]);
        } else {
            idx.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
        }
    }
}

/// Add an annulus cap (top or bottom face).
fn add_annulus_cap(
    pos: &mut Vec<[f32; 3]>,
    nor: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    idx: &mut Vec<u32>,
    r_inner: f32,
    r_outer: f32,
    y: f32,
    a_start: f32,
    a_end: f32,
    segments: usize,
    top: bool,
) {
    let base = pos.len() as u32;
    let ny = if top { 1.0 } else { -1.0 };
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = a_start + t * (a_end - a_start);
        let (c, s) = (a.cos(), a.sin());
        pos.push([r_outer * c, y, r_outer * s]);
        nor.push([0.0, ny, 0.0]);
        uvs.push([t, 1.0]);
        pos.push([r_inner * c, y, r_inner * s]);
        nor.push([0.0, ny, 0.0]);
        uvs.push([t, 0.0]);
    }
    for i in 0..segments {
        let b = base + (i as u32) * 2;
        if top {
            idx.extend_from_slice(&[b, b + 1, b + 3, b, b + 3, b + 2]);
        } else {
            idx.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
        }
    }
}

/// Add a flat radial wall (slot wall) at a fixed angle.
fn add_radial_wall(
    pos: &mut Vec<[f32; 3]>,
    nor: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    idx: &mut Vec<u32>,
    r_inner: f32,
    r_outer: f32,
    y_bot: f32,
    y_top: f32,
    angle: f32,
    left_side: bool,
) {
    let base = pos.len() as u32;
    let (c, s) = (angle.cos(), angle.sin());
    // Normal perpendicular to the radial direction (tangent)
    let n_dir = if left_side { -1.0 } else { 1.0 };
    let nx = n_dir * (-s);
    let nz = n_dir * c;

    // 4 corners: inner-bottom, inner-top, outer-bottom, outer-top
    pos.push([r_inner * c, y_bot, r_inner * s]);
    nor.push([nx, 0.0, nz]);
    uvs.push([0.0, 0.0]);

    pos.push([r_inner * c, y_top, r_inner * s]);
    nor.push([nx, 0.0, nz]);
    uvs.push([0.0, 1.0]);

    pos.push([r_outer * c, y_bot, r_outer * s]);
    nor.push([nx, 0.0, nz]);
    uvs.push([1.0, 0.0]);

    pos.push([r_outer * c, y_top, r_outer * s]);
    nor.push([nx, 0.0, nz]);
    uvs.push([1.0, 1.0]);

    if left_side {
        idx.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
    } else {
        idx.extend_from_slice(&[base, base + 3, base + 1, base, base + 2, base + 3]);
    }
}

fn build_mesh(pos: Vec<[f32; 3]>, nor: Vec<[f32; 3]>, uvs: Vec<[f32; 2]>, idx: Vec<u32>) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, nor);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(idx));
    mesh
}
