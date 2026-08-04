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
    mut ev_config: MessageReader<MotorConfigChanged>,
    query: Query<Entity, With<StatorPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Visibility toggles never affect the iron, so they must not pay for a
    // full rebuild. `count()` (not `any()`) so the reader is always drained.
    if ev_config.read().filter(|e| e.geometry).count() == 0 {
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

    // Teeth: every tooth is the same sector at a different angle, so one mesh
    // is built and instanced `n` times rather than generating `n` meshes.
    let tooth = meshes.add(generate_sector_mesh(
        r_bore,
        r_slot_bot,
        0.0,
        tooth_angle,
        -half_h,
        half_h,
        4,
    ));
    for i in 0..n {
        commands.spawn((
            Mesh3d(tooth.clone()),
            MeshMaterial3d(iron_mat.clone()),
            Transform::from_rotation(tooth_rotation(i, segment_angle)),
            StatorPart,
        ));
    }
}

/// Rotation placing the tooth mesh (built at angle 0) at groove `i`.
///
/// A `Quat::from_rotation_y(θ)` maps a point at parametric angle `a` to
/// `a - θ`, so reaching `i · segment_angle` takes a negative rotation.
#[inline]
fn tooth_rotation(index: usize, segment_angle: f32) -> Quat {
    Quat::from_rotation_y(-(index as f32) * segment_angle)
}

// ---------------------------------------------------------------------------
// Mesh generation helpers
// ---------------------------------------------------------------------------

macro_rules! add_cylinder_wall {
    ($pos:expr, $nor:expr, $uvs:expr, $idx:expr, $radius:expr, $y_bot:expr, $y_top:expr, $a_start:expr, $a_end:expr, $segments:expr, $outward:expr) => {{
        let base = $pos.len() as u32;
        for i in 0..=$segments {
            let t = i as f32 / $segments as f32;
            let a = $a_start + t * ($a_end - $a_start);
            let (c, s) = (a.cos(), a.sin());
            let n_dir = if $outward { 1.0 } else { -1.0 };
            $pos.push([$radius * c, $y_bot, $radius * s]);
            $nor.push([n_dir * c, 0.0, n_dir * s]);
            $uvs.push([t, 0.0]);
            $pos.push([$radius * c, $y_top, $radius * s]);
            $nor.push([n_dir * c, 0.0, n_dir * s]);
            $uvs.push([t, 1.0]);
        }
        for i in 0..$segments {
            let b = base + (i as u32) * 2;
            if $outward {
                $idx.extend_from_slice(&[b, b + 1, b + 3, b, b + 3, b + 2]);
            } else {
                $idx.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
            }
        }
    }};
}

macro_rules! add_annulus_cap {
    ($pos:expr, $nor:expr, $uvs:expr, $idx:expr, $r_inner:expr, $r_outer:expr, $y:expr, $a_start:expr, $a_end:expr, $segments:expr, $top:expr) => {{
        let base = $pos.len() as u32;
        let ny = if $top { 1.0 } else { -1.0 };
        for i in 0..=$segments {
            let t = i as f32 / $segments as f32;
            let a = $a_start + t * ($a_end - $a_start);
            let (c, s) = (a.cos(), a.sin());
            $pos.push([$r_outer * c, $y, $r_outer * s]);
            $nor.push([0.0, ny, 0.0]);
            $uvs.push([t, 1.0]);
            $pos.push([$r_inner * c, $y, $r_inner * s]);
            $nor.push([0.0, ny, 0.0]);
            $uvs.push([t, 0.0]);
        }
        for i in 0..$segments {
            let b = base + (i as u32) * 2;
            if $top {
                $idx.extend_from_slice(&[b, b + 1, b + 3, b, b + 3, b + 2]);
            } else {
                $idx.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
            }
        }
    }};
}

macro_rules! add_radial_wall {
    ($pos:expr, $nor:expr, $uvs:expr, $idx:expr, $r_inner:expr, $r_outer:expr, $y_bot:expr, $y_top:expr, $angle:expr, $left_side:expr) => {{
        let base = $pos.len() as u32;
        let (c, s) = ($angle.cos(), $angle.sin());
        let n_dir = if $left_side { -1.0 } else { 1.0 };
        let nx = n_dir * (-s);
        let nz = n_dir * c;

        $pos.push([$r_inner * c, $y_bot, $r_inner * s]);
        $nor.push([nx, 0.0, nz]);
        $uvs.push([0.0, 0.0]);
        $pos.push([$r_inner * c, $y_top, $r_inner * s]);
        $nor.push([nx, 0.0, nz]);
        $uvs.push([0.0, 1.0]);
        $pos.push([$r_outer * c, $y_bot, $r_outer * s]);
        $nor.push([nx, 0.0, nz]);
        $uvs.push([1.0, 0.0]);
        $pos.push([$r_outer * c, $y_top, $r_outer * s]);
        $nor.push([nx, 0.0, nz]);
        $uvs.push([1.0, 1.0]);

        if $left_side {
            $idx.extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
        } else {
            $idx.extend_from_slice(&[base, base + 3, base + 1, base, base + 2, base + 3]);
        }
    }};
}

/// Generate a full ring (annulus) mesh extruded along Y.
fn generate_ring_mesh(r_inner: f32, r_outer: f32, y_bot: f32, y_top: f32, segments: usize) -> Mesh {
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();

    // -- Outer wall --
    add_cylinder_wall!(
        pos, nor, uvs, idx, r_outer, y_bot, y_top, 0.0, TAU, segments, true
    );
    // -- Inner wall --
    add_cylinder_wall!(
        pos, nor, uvs, idx, r_inner, y_bot, y_top, 0.0, TAU, segments, false
    );
    // -- Top cap --
    add_annulus_cap!(
        pos, nor, uvs, idx, r_inner, r_outer, y_top, 0.0, TAU, segments, true
    );
    // -- Bottom cap --
    add_annulus_cap!(
        pos, nor, uvs, idx, r_inner, r_outer, y_bot, 0.0, TAU, segments, false
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
    add_cylinder_wall!(
        pos, nor, uvs, idx, r_outer, y_bot, y_top, a_start, a_end, segments, true
    );
    add_cylinder_wall!(
        pos, nor, uvs, idx, r_inner, y_bot, y_top, a_start, a_end, segments, false
    );
    // Top/bottom caps
    add_annulus_cap!(
        pos, nor, uvs, idx, r_inner, r_outer, y_top, a_start, a_end, segments, true
    );
    add_annulus_cap!(
        pos, nor, uvs, idx, r_inner, r_outer, y_bot, a_start, a_end, segments, false
    );
    // Side walls (slot walls)
    add_radial_wall!(
        pos, nor, uvs, idx, r_inner, r_outer, y_bot, y_top, a_start, true
    );
    add_radial_wall!(
        pos, nor, uvs, idx, r_inner, r_outer, y_bot, y_top, a_end, false
    );

    build_mesh(pos, nor, uvs, idx)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The instanced tooth must land exactly where the old per-tooth mesh did.
    /// Guards the sign of `tooth_rotation`, which is easy to get backwards.
    #[test]
    fn instanced_teeth_land_on_their_grooves() {
        let n = 24_usize;
        let segment_angle = TAU / n as f32;
        let tooth_angle = segment_angle * 0.5;

        let base = generate_sector_mesh(2.0, 2.6, 0.0, tooth_angle, -1.0, 1.0, 4);
        let base_pos = base
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .expect("base mesh has positions");

        for i in [0_usize, 1, 7, 23] {
            let a_start = i as f32 * segment_angle;
            let expected = generate_sector_mesh(
                2.0,
                2.6,
                a_start,
                a_start + tooth_angle,
                -1.0,
                1.0,
                4,
            );
            let expected_pos = expected
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .and_then(|a| a.as_float3())
                .expect("expected mesh has positions");

            assert_eq!(base_pos.len(), expected_pos.len());
            let rotation = tooth_rotation(i, segment_angle);
            for (b, e) in base_pos.iter().zip(expected_pos) {
                let got = rotation * Vec3::from_array(*b);
                let want = Vec3::from_array(*e);
                assert!(got.distance(want) < 1e-3, "groove {i}: {got:?} != {want:?}");
            }
        }
    }
}
