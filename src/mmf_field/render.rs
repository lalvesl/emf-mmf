use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::{PI, TAU};

use crate::config::{MotorConfig, MotorConfigChanged, STATOR_BORE_RADIUS, STATOR_HEIGHT};
use crate::electrical::ElectricalState;

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct MmfFieldRenderPlugin;

impl Plugin for MmfFieldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (regenerate_field, animate_field));
    }
}

// ─── Component ───────────────────────────────────────────────────────────────

/// Marker for a single phase×pole MMF field sector mesh.
#[derive(Component)]
pub struct MmfFieldSector {
    pub phase: usize,
    pub pole: usize,
    /// Full-strength SRGBA components of the phase colour (alpha=1).
    pub base_color: [f32; 4],
    /// Angular half-width of this sector in mechanical radians.
    pub half_angular_span: f32,
    /// Mechanical angle (radians) of the magnetic axis of this group.
    pub axis_angle: f32,
    /// Number of ring segments used when building the mesh.
    pub segments: u32,
}

// ─── Regenerate (on config change) ───────────────────────────────────────────

fn regenerate_field(
    mut commands: Commands,
    mut ev_config: MessageReader<MotorConfigChanged>,
    config: Res<MotorConfig>,
    query: Query<Entity, With<MmfFieldSector>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if ev_config.read().next().is_none() {
        return;
    }

    // Despawn old field meshes
    for entity in &query {
        commands.entity(entity).despawn();
    }

    if !config.mmf_field.show {
        return;
    }

    let m = config.phases;
    let p = config.pole_pairs;
    if m == 0 || p == 0 {
        return;
    }

    let n = config.groove_count as f32;
    let m_f32 = m as f32;
    let p_f32 = p as f32;
    let q = n / (2.0 * p_f32 * m_f32);
    let pitch = crate::winding::coil_pitch(&config) as f32;

    let alpha = (p_f32 * TAU) / n; // electrical angle per slot
    let alpha_m = if !config.phases.is_multiple_of(2) {
        TAU / m_f32
    } else {
        PI / m_f32
    };

    let offset_mech = (TAU / n) * 0.75; // matches winding.rs slot offset

    // Angular half-width of one coil group in MECHANICAL radians.
    // One coil group spans `q` slots, so in electrical radians it is `q * alpha`.
    // In mechanical radians the span is `q * alpha / p`.
    let group_span_mech = (q * alpha) / p_f32; // full span
    let half_span = PI; //group_span_mech * 0.5;

    let r_inner = 0.05; // tiny inner hole to avoid degenerate tris
    let r_outer = STATOR_BORE_RADIUS * 0.97; // just inside the bore surface
    let y_bot = -STATOR_HEIGHT / 2.0 + 0.02; // slightly above stator floor
    let y_top = STATOR_HEIGHT / 2.0 - 0.02; // slightly below stator ceiling
    let segments: u32 = 48;

    let gradient_intensity = config.mmf_field.gradient_intensity;

    for pole in 0..(2 * p) {
        for phase in 0..m {
            if !config.mmf_field.phases_to_show[phase] {
                continue;
            }

            let phase_shift_elec = phase as f32 * alpha_m;
            let start_elec = phase_shift_elec + (pole as f32 * PI);
            let offset_elec = (q - 1.0 + pitch) / 2.0 * alpha;
            let center_elec = start_elec + offset_elec;
            let axis_angle = (center_elec / p_f32) + offset_mech;

            let color_srgba: bevy::color::Srgba = crate::phase::colors::phase_color(phase).into();
            let base_color = [color_srgba.red, color_srgba.green, color_srgba.blue, 1.0];

            let mesh = build_sector_mesh(
                r_inner,
                r_outer,
                y_bot,
                y_top,
                axis_angle,
                half_span,
                segments,
                gradient_intensity,
                1.0, // amplitude — will be updated every frame in animate_field
                base_color,
            );

            let material = materials.add(StandardMaterial {
                base_color: Color::WHITE,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            });

            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material),
                Transform::default(),
                MmfFieldSector {
                    phase,
                    pole,
                    base_color,
                    half_angular_span: half_span,
                    axis_angle,
                    segments,
                },
            ));
        }
    }
}

// ─── Animate (every frame) ───────────────────────────────────────────────────

fn animate_field(
    config: Res<MotorConfig>,
    state: Res<ElectricalState>,
    mut query: Query<(&MmfFieldSector, &Mesh3d, &mut Visibility)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !config.mmf_field.show {
        for (_, _, mut vis) in &mut query {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let m = config.phases;
    let p = config.pole_pairs;
    if m == 0 || p == 0 {
        return;
    }

    let alpha_m = if !m.is_multiple_of(2) {
        TAU / m as f32
    } else {
        PI / m as f32
    };

    let gradient_intensity = config.mmf_field.gradient_intensity;

    for (sector, mesh3d, mut vis) in &mut query {
        // Guard against stale entities from a previous config
        if sector.pole >= 2 * p || sector.phase >= m {
            *vis = Visibility::Hidden;
            continue;
        }

        if !config.mmf_field.phases_to_show[sector.phase] {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Visible;

        // Compute instantaneous current for this phase
        let phase_shift_elec = sector.phase as f32 * alpha_m;
        let current = (state.angle - phase_shift_elec).cos().powi(3);

        // Pole alternation: every other pole inverts the field direction
        let mmf_amplitude = current * if sector.pole % 2 == 0 { 1.0 } else { -1.0 };

        // Absolute amplitude drives the visual intensity; keep positive.
        let abs_amplitude = mmf_amplitude.abs();

        // Rebuild vertex colours for this sector
        if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
            recolor_sector_mesh(
                mesh,
                sector.axis_angle,
                sector.half_angular_span,
                sector.segments,
                gradient_intensity,
                abs_amplitude,
                sector.base_color,
            );
        }
    }
}

// ─── Mesh helpers ─────────────────────────────────────────────────────────────

/// Build an annular sector (ring sector) mesh in the XZ plane with per-vertex
/// colours encoding the MMF gradient.
///
/// Layout:
/// - `segments` angular steps across `[-half_span, +half_span]` around `axis_angle`
/// - Two rings at `r_inner` and `r_outer`
/// - Two Y-planes at `y_bot` and `y_top`
///   → 4 triangulated faces (top, bottom, inner wall, outer wall)
///
/// The gradient alpha at each vertex is:
///   `alpha = amplitude × (cos(delta / half_span × π/2) ^ gamma).max(0)`
/// where `delta` is the angular deviation from the axis.
fn build_sector_mesh(
    r_inner: f32,
    r_outer: f32,
    y_bot: f32,
    y_top: f32,
    axis_angle: f32,
    half_span: f32,
    segments: u32,
    gradient_intensity: f32,
    amplitude: f32,
    base_color: [f32; 4],
) -> Mesh {
    let vertex_count_ring = (segments + 1) as usize;
    let total_verts =
        // top cap:   (segs+1) inner + (segs+1) outer
        vertex_count_ring * 2
        // bottom cap: same
        + vertex_count_ring * 2
        // outer wall: (segs+1) bot + (segs+1) top
        + vertex_count_ring * 2
        // inner wall: same
        + vertex_count_ring * 2;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_verts);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(total_verts);
    let mut indices: Vec<u32> = Vec::new();

    // Helper: alpha at angle `a` from axis
    let alpha_at = |a: f32| -> f32 {
        let delta = (a - axis_angle).abs();
        let t = (delta / half_span).clamp(0.0, 1.0);
        let base = (1.0 - t * t).max(0.0_f32).sqrt(); // half-cosine bell
        let shaped = base.powf(gradient_intensity);
        amplitude * shaped
    };

    let color_at = |a: f32| -> [f32; 4] {
        let alpha = alpha_at(a);
        [base_color[0], base_color[1], base_color[2], alpha]
    };

    let ring_point = |r: f32, a: f32, y: f32| -> [f32; 3] { [r * a.cos(), y, r * a.sin()] };

    // ── Top cap (y = y_top, normal = +Y) ──────────────────────────────────
    {
        let base_idx = positions.len() as u32;
        // inner ring vertices
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            positions.push(ring_point(r_inner, a, y_top));
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([t, 0.0]);
            colors.push(color_at(a));
        }
        // outer ring vertices
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            positions.push(ring_point(r_outer, a, y_top));
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([t, 1.0]);
            colors.push(color_at(a));
        }
        let inner_base = base_idx;
        let outer_base = base_idx + (segments + 1);
        for i in 0..segments {
            let ii = inner_base + i;
            let oi = outer_base + i;
            // Two triangles: (ii, ii+1, oi+1) and (ii, oi+1, oi)
            indices.extend_from_slice(&[ii, ii + 1, oi + 1, ii, oi + 1, oi]);
        }
    }

    // ── Bottom cap (y = y_bot, normal = -Y) ───────────────────────────────
    {
        let base_idx = positions.len() as u32;
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            positions.push(ring_point(r_inner, a, y_bot));
            normals.push([0.0, -1.0, 0.0]);
            uvs.push([t, 0.0]);
            colors.push(color_at(a));
        }
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            positions.push(ring_point(r_outer, a, y_bot));
            normals.push([0.0, -1.0, 0.0]);
            uvs.push([t, 1.0]);
            colors.push(color_at(a));
        }
        let inner_base = base_idx;
        let outer_base = base_idx + (segments + 1);
        for i in 0..segments {
            let ii = inner_base + i;
            let oi = outer_base + i;
            // Flipped winding for bottom face
            indices.extend_from_slice(&[ii, oi + 1, ii + 1, ii, oi, oi + 1]);
        }
    }

    // ── Outer wall (r = r_outer, normal = outward radial) ─────────────────
    {
        let base_idx = positions.len() as u32;
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            let (c, s) = (a.cos(), a.sin());
            // bot vertex
            positions.push([r_outer * c, y_bot, r_outer * s]);
            normals.push([c, 0.0, s]);
            uvs.push([t, 0.0]);
            colors.push(color_at(a));
            // top vertex
            positions.push([r_outer * c, y_top, r_outer * s]);
            normals.push([c, 0.0, s]);
            uvs.push([t, 1.0]);
            colors.push(color_at(a));
        }
        for i in 0..segments {
            let b = base_idx + i * 2;
            indices.extend_from_slice(&[b, b + 1, b + 3, b, b + 3, b + 2]);
        }
    }

    // ── Inner wall (r = r_inner, normal = inward radial) ──────────────────
    {
        let base_idx = positions.len() as u32;
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            let (c, s) = (a.cos(), a.sin());
            positions.push([r_inner * c, y_bot, r_inner * s]);
            normals.push([-c, 0.0, -s]);
            uvs.push([t, 0.0]);
            colors.push(color_at(a));
            positions.push([r_inner * c, y_top, r_inner * s]);
            normals.push([-c, 0.0, -s]);
            uvs.push([t, 1.0]);
            colors.push(color_at(a));
        }
        for i in 0..segments {
            let b = base_idx + i * 2;
            // Flipped winding for inward-facing wall
            indices.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Re-write only the vertex colour attribute in an existing sector mesh without
/// reallocating the entire mesh.  The vertex layout must match `build_sector_mesh`.
fn recolor_sector_mesh(
    mesh: &mut Mesh,
    axis_angle: f32,
    half_span: f32,
    segments: u32,
    gradient_intensity: f32,
    amplitude: f32,
    base_color: [f32; 4],
) {
    let alpha_at = |a: f32| -> f32 {
        let delta = (a - axis_angle).abs();
        let t = (delta / half_span).clamp(0.0, 1.0);
        let base = (1.0 - t * t).max(0.0_f32).sqrt();
        amplitude * base.powf(gradient_intensity)
    };

    let color_at =
        |a: f32| -> [f32; 4] { [base_color[0], base_color[1], base_color[2], alpha_at(a)] };

    // Must regenerate all 4 faces' colour arrays in the same vertex order as
    // `build_sector_mesh`.
    let n = segments + 1;
    let total = (n * 8) as usize; // 4 faces × 2 rings each, each with n verts
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(total);

    for _face in 0..4 {
        // inner ring
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            colors.push(color_at(a));
        }
        // outer ring or paired interleaved (outer wall / inner wall)
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let a = (axis_angle - half_span) + t * 2.0 * half_span;
            colors.push(color_at(a));
        }
    }

    // For the wall faces vertices are interleaved (bot,top pairs) rather than
    // split into two rings, so rebuild those in the correct order.
    // We need to regenerate the full colour slice matching the exact build layout.
    let mut colors_correct: Vec<[f32; 4]> = Vec::with_capacity(total);

    // Face 0 — top cap: inner (segs+1) then outer (segs+1)
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = (axis_angle - half_span) + t * 2.0 * half_span;
        colors_correct.push(color_at(a));
    }
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = (axis_angle - half_span) + t * 2.0 * half_span;
        colors_correct.push(color_at(a));
    }

    // Face 1 — bottom cap: inner (segs+1) then outer (segs+1)
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = (axis_angle - half_span) + t * 2.0 * half_span;
        colors_correct.push(color_at(a));
    }
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = (axis_angle - half_span) + t * 2.0 * half_span;
        colors_correct.push(color_at(a));
    }

    // Face 2 — outer wall: interleaved (bot, top) pairs for each angle step
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = (axis_angle - half_span) + t * 2.0 * half_span;
        colors_correct.push(color_at(a)); // bot
        colors_correct.push(color_at(a)); // top
    }

    // Face 3 — inner wall: interleaved (bot, top) pairs
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = (axis_angle - half_span) + t * 2.0 * half_span;
        colors_correct.push(color_at(a)); // bot
        colors_correct.push(color_at(a)); // top
    }

    if let Some(attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
        *attr = VertexAttributeValues::Float32x4(colors_correct);
    }
}
