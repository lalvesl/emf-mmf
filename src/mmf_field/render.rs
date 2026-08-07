use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::{PI, TAU};

use crate::config::{MotorConfig, STATOR_BORE_RADIUS, STATOR_HEIGHT, ViewConfig};
use crate::electrical::ElectricalState;
use crate::winding::axis::{magnetic_axis, phase_current, phase_displacement};

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct MmfFieldRenderPlugin;

impl Plugin for MmfFieldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                regenerate_field.run_if(crate::config::scene_changed),
                animate_field,
                animate_result,
            ),
        );
    }
}

// ─── Components ──────────────────────────────────────────────────────────────

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

/// Marker for the resultant MMF field mesh (vector sum of all phases).
/// Rendered in white so it is distinguishable from any individual phase colour.
#[derive(Component)]
pub struct MmfResultSector {
    /// Angular half-width of this sector in mechanical radians.
    pub half_angular_span: f32,
    /// Number of ring segments used when building the mesh.
    pub segments: u32,
}

/// White RGBA used for the resultant MMF field.
const RESULT_BASE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// Rim colour of a north pole.
///
/// Deep and hard-saturated rather than bright: the material blends normally
/// over the scene, so a light red would wash out against the phase colours,
/// which sit at middling lightness. Driving the off-channels to near zero also
/// separates the rim from any reddish phase by luminance as well as hue.
pub(crate) const NORTH_COLOR: [f32; 3] = [0.78, 0.02, 0.03];
/// Rim colour of a south pole. Deep for the same reason as [`NORTH_COLOR`].
pub(crate) const SOUTH_COLOR: [f32; 3] = [0.02, 0.09, 0.82];

/// Radial rings across a sector's caps.
///
/// Two rings would leave the rasteriser to interpolate the rim colour linearly
/// all the way in to the bore centre. The polarity band has to stay bunched
/// against the outer edge, so the caps are subdivided and the falloff sampled
/// per ring instead.
const RADIAL_RINGS: usize = 6;

/// How sharply the polarity colour gives way to the identity colour going
/// inward. Higher keeps it harder against the rim.
const POLARITY_FALLOFF: f32 = 5.0;

/// Fraction of polarity colour at radial position `t` — 0 at the bore centre,
/// 1 at the rim.
#[inline]
fn polarity_mix(t: f32) -> f32 {
    t.clamp(0.0, 1.0).powf(POLARITY_FALLOFF)
}

/// Colour of one sector vertex: identity in the core, polarity on the rim.
///
/// `amplitude` carries the sign, so the rim goes red where the lobe is a north
/// pole and blue where it is a south, while the core keeps the phase hue — or
/// white, for the resultant. Alpha tracks magnitude alone, so a weak pole fades
/// out rather than changing colour.
///
/// Separating the two onto different parts of the lobe is what lets both be
/// read at once: hue is already fully spent identifying the phase.
fn sector_color(base_color: [f32; 4], amplitude: f32, bell: f32, radial: f32) -> [f32; 4] {
    let polarity = if amplitude >= 0.0 {
        NORTH_COLOR
    } else {
        SOUTH_COLOR
    };
    let mix = polarity_mix(radial);
    let blend = |core: f32, rim: f32| core + (rim - core) * mix;

    [
        blend(base_color[0], polarity[0]),
        blend(base_color[1], polarity[1]),
        blend(base_color[2], polarity[2]),
        // small improvement to make colol with derivate more in low values and more quick in high values
        amplitude.abs().powf(2.0) * bell,
    ]
}

/// Walks the vertex layout of a sector mesh in order, handing each vertex's
/// angle and radial position (0 at the bore centre, 1 at the rim) to `visit`.
///
/// The mesh is built once, recoloured when the configuration changes and
/// recoloured again every frame — from three separate places, each of which
/// used to spell the layout out for itself. Sharing this one walk is what stops
/// them drifting apart.
fn for_each_sector_vertex(
    axis_angle: f32,
    half_span: f32,
    segments: u32,
    mut visit: impl FnMut(f32, f32),
) {
    let angle_at = |i: u32| {
        let t = i as f32 / segments as f32;
        (axis_angle - half_span) + t * 2.0 * half_span
    };

    // Top cap then bottom cap, innermost ring first.
    for _cap in 0..2 {
        for ring in 0..RADIAL_RINGS {
            let radial = ring as f32 / (RADIAL_RINGS - 1) as f32;
            for i in 0..=segments {
                visit(angle_at(i), radial);
            }
        }
    }

    // Outer wall then inner wall, each interleaving (bottom, top) pairs.
    for radial in [1.0_f32, 0.0] {
        for i in 0..=segments {
            let a = angle_at(i);
            visit(a, radial);
            visit(a, radial);
        }
    }
}

/// Angular falloff of a lobe: 1 on its axis, 0 at the edge of its span.
#[inline]
fn lobe_bell(a: f32, axis_angle: f32, half_span: f32, gradient_intensity: f32) -> f32 {
    let delta = angular_distance(a, axis_angle);
    let t = (delta / half_span).clamp(0.0, 1.0);
    (1.0 - t * t).max(0.0_f32).sqrt().powf(gradient_intensity)
}

/// Mechanical half-width of one phase×pole MMF lobe.
///
/// Consecutive pole axes sit one pole pitch (`π / p` mechanical) apart, so a
/// half-width of `π / 2p` makes the `2p` lobes of a phase tile the bore exactly
/// once, without overlapping.
#[inline]
fn lobe_half_span(pole_pairs: usize) -> f32 {
    PI / (2.0 * pole_pairs as f32)
}

/// Shortest angular distance between two angles, in `[0, π]`.
///
/// Needed wherever the sample angle and the axis angle are produced
/// independently: a naive `(a - b).abs()` reports ~2π for two angles that sit
/// on opposite sides of the ±π seam.
#[inline]
fn angular_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

// ─── Regenerate (on config change) ───────────────────────────────────────────

fn regenerate_field(
    mut commands: Commands,
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    phase_query: Query<Entity, With<MmfFieldSector>>,
    result_query: Query<Entity, With<MmfResultSector>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Despawn old phase field meshes
    for entity in &phase_query {
        commands.entity(entity).despawn();
    }
    // Despawn old result mesh
    for entity in &result_query {
        commands.entity(entity).despawn();
    }

    if !view.mmf_field.show {
        return;
    }

    let m = config.phases;
    let p = config.pole_pairs;
    if m == 0 || p == 0 {
        return;
    }

    // One lobe per phase per pole, each covering exactly one pole pitch.
    let half_span = lobe_half_span(p);

    let r_inner = 0.05; // tiny inner hole to avoid degenerate tris
    let r_outer = STATOR_BORE_RADIUS * 0.97; // just inside the bore surface
    let y_bot = -STATOR_HEIGHT / 2.0 + 0.02; // slightly above stator floor
    let y_top = STATOR_HEIGHT / 2.0 - 0.02; // slightly below stator ceiling
    let segments: u32 = 48;

    let gradient_intensity = view.mmf_field.gradient_intensity;

    // ── Per-phase sector meshes ────────────────────────────────────────────
    for pole in 0..(2 * p) {
        for phase in 0..m {
            if !view.mmf_field.shows_phase(phase) {
                continue;
            }

            let axis_angle = magnetic_axis(&config, phase, pole);

            let color_srgba: bevy::color::Srgba =
                crate::phase::colors::phase_color(phase, m).into();
            let base_color = [color_srgba.red, color_srgba.green, color_srgba.blue, 1.0];

            let mesh = build_sector_mesh(SectorMeshParams {
                r_inner,
                r_outer,
                y_bot,
                y_top,
                axis_angle,
                half_span,
                segments,
                gradient_intensity,
                amplitude: 1.0, // amplitude — will be updated every frame in animate_field
                base_color,
            });

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

    // ── Resultant MMF sector mesh (full 360° ring, white) ─────────────────
    // The result covers the full circle; its per-vertex colour is updated
    // every frame in `animate_result` based on the combined MMF waveform.
    {
        // Use full-circle half-span so we can sample the entire 360° ring.
        let result_half_span = PI;
        let result_axis = 0.0_f32; // axis at 0 rad; the full ring is symmetric
        let result_mesh = build_sector_mesh(SectorMeshParams {
            r_inner,
            r_outer,
            y_bot,
            y_top,
            axis_angle: result_axis,
            half_span: result_half_span,
            segments,
            gradient_intensity,
            amplitude: 0.0, // starts invisible; animate_result sets it
            base_color: RESULT_BASE_COLOR,
        });
        let result_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            double_sided: true,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(result_mesh)),
            MeshMaterial3d(result_material),
            Transform::default(),
            MmfResultSector {
                half_angular_span: result_half_span,
                segments,
            },
        ));
    }
}

// ─── Animate (every frame) ───────────────────────────────────────────────────

fn animate_field(
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    state: Res<ElectricalState>,
    mut query: Query<(&MmfFieldSector, &Mesh3d, &mut Visibility)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if !view.mmf_field.show {
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

    let alpha_m = phase_displacement(m);

    let gradient_intensity = view.mmf_field.gradient_intensity;

    for (sector, mesh3d, mut vis) in &mut query {
        // Guard against stale entities from a previous config
        if sector.pole >= 2 * p || sector.phase >= m {
            *vis = Visibility::Hidden;
            continue;
        }

        if !view.mmf_field.shows_phase(sector.phase) {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Visible;

        // Compute instantaneous current for this phase
        let current = phase_current(state.angle, sector.phase, alpha_m);

        // Pole alternation: every other pole inverts the field direction
        let mmf_amplitude = current * if sector.pole % 2 == 0 { 1.0 } else { -1.0 };

        // The sign is kept, not thrown away: magnitude drives the alpha and the
        // sign picks the rim colour, so a north lobe and a south lobe of the
        // same phase no longer render identically.
        if let Some(mut mesh) = meshes.get_mut(&mesh3d.0) {
            recolor_sector_mesh(
                &mut mesh,
                sector.axis_angle,
                sector.half_angular_span,
                sector.segments,
                gradient_intensity,
                mmf_amplitude,
                sector.base_color,
            );
        }
    }
}

// ─── Animate result (every frame) ────────────────────────────────────────────

/// Computes the resultant MMF waveform (sum of all active phase fields) and
/// updates the `MmfResultSector` mesh vertex colours every frame.
///
/// For each angular sample around the ring the instantaneous contribution from
/// every phase×pole is accumulated.  The total is normalised by the number of
/// phases so the alpha stays within [0, 1].
fn animate_result(
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    state: Res<ElectricalState>,
    mut query: Query<(&MmfResultSector, &Mesh3d, &mut Visibility)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (sector, mesh3d, mut vis) in &mut query {
        if !view.mmf_field.show || !view.mmf_field.show_result {
            *vis = Visibility::Hidden;
            continue;
        }

        let m = config.phases;
        let p = config.pole_pairs;
        if m == 0 || p == 0 {
            *vis = Visibility::Hidden;
            continue;
        }

        *vis = Visibility::Visible;

        let alpha_m = phase_displacement(m);
        let gradient_intensity = view.mmf_field.gradient_intensity;
        let half_span = sector.half_angular_span;
        let segments = sector.segments;

        // For each angle sample, accumulate the MMF contribution from every
        // phase×pole so we can build a full-ring colour array.
        let sample_count = segments + 1;

        // Must match the lobe width used for the individual phase sectors, so
        // the resultant really is the sum of what is drawn per phase.
        let lobe_half_span = lobe_half_span(p);

        let lobes: Vec<(f32, f32)> = (0..(2 * p))
            .flat_map(|pole| (0..m).map(move |phase| (pole, phase)))
            .map(|(pole, phase)| {
                let axis_angle = magnetic_axis(&config, phase, pole);
                let sign = if pole % 2 == 0 { 1.0 } else { -1.0 };
                let amplitude = phase_current(state.angle, phase, alpha_m) * sign;
                (axis_angle, amplitude)
            })
            .collect();

        // Build a per-angle resultant amplitude array over the full ring.
        // We step from -PI to +PI (axis=0, half_span=PI covers all 360°).
        let mmf_at_angle = |a_mech: f32| -> f32 {
            let total: f32 = lobes
                .iter()
                .map(|&(axis_angle, amplitude)| {
                    let delta = angular_distance(a_mech, axis_angle);
                    let t = (delta / lobe_half_span).clamp(0.0, 1.0);
                    let bell = (1.0 - t * t).max(0.0_f32).sqrt().powf(gradient_intensity);
                    bell * amplitude
                })
                .sum();
            // Normalise so maximum amplitude is ~1 when all phases are at peak.
            // Dividing by m spreads the scale evenly across all phases. The
            // sign survives: it is what tells a north lobe from a south.
            (total / m as f32).clamp(-1.0, 1.0)
        };

        let Some(mut mesh) = meshes.get_mut(&mesh3d.0) else {
            continue;
        };

        // The resultant ring is one sector spanning the whole bore, so the same
        // walk that laid the mesh out drives its colours too.
        let mut colors: Vec<[f32; 4]> = Vec::with_capacity((sample_count * 8) as usize);
        for_each_sector_vertex(0.0, half_span, segments, |a, radial| {
            // The bell is already folded into `mmf_at_angle`, which sums every
            // lobe, so the angular falloff here is 1.
            colors.push(sector_color(
                RESULT_BASE_COLOR,
                mmf_at_angle(a),
                1.0,
                radial,
            ));
        });

        if let Some(attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
            *attr = bevy::mesh::VertexAttributeValues::Float32x4(colors);
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
///   `alpha = amplitude × (√(1 - t²)) ^ gamma`,  `t = delta / half_span`
/// where `delta` is the angular deviation from the axis and `gamma` is
/// `MmfFieldConfig::gradient_intensity`.
struct SectorMeshParams {
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
}

fn build_sector_mesh(params: SectorMeshParams) -> Mesh {
    let r_inner = params.r_inner;
    let r_outer = params.r_outer;
    let y_bot = params.y_bot;
    let y_top = params.y_top;
    let axis_angle = params.axis_angle;
    let half_span = params.half_span;
    let segments = params.segments;
    let gradient_intensity = params.gradient_intensity;
    let amplitude = params.amplitude;
    let base_color = params.base_color;
    let vertex_count_ring = (segments + 1) as usize;
    let total_verts =
        // two caps of RADIAL_RINGS rings each
        vertex_count_ring * RADIAL_RINGS * 2
        // outer wall: (segs+1) bot + (segs+1) top
        + vertex_count_ring * 2
        // inner wall: same
        + vertex_count_ring * 2;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_verts);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_verts);
    let mut indices: Vec<u32> = Vec::new();

    let ring_point = |r: f32, a: f32, y: f32| -> [f32; 3] { [r * a.cos(), y, r * a.sin()] };
    let radius_at = |radial: f32| r_inner + (r_outer - r_inner) * radial;

    // ── Caps (top then bottom) ────────────────────────────────────────────
    // Both are subdivided radially so the polarity rim has geometry to sit on.
    for (y, normal_y) in [(y_top, 1.0_f32), (y_bot, -1.0)] {
        let base_idx = positions.len() as u32;

        for ring in 0..RADIAL_RINGS {
            let radial = ring as f32 / (RADIAL_RINGS - 1) as f32;
            let r = radius_at(radial);
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let a = (axis_angle - half_span) + t * 2.0 * half_span;
                positions.push(ring_point(r, a, y));
                normals.push([0.0, normal_y, 0.0]);
                uvs.push([t, radial]);
            }
        }

        let stride = segments + 1;
        for ring in 0..(RADIAL_RINGS as u32 - 1) {
            let inner_base = base_idx + ring * stride;
            let outer_base = inner_base + stride;
            for i in 0..segments {
                let ii = inner_base + i;
                let oi = outer_base + i;
                if normal_y > 0.0 {
                    indices.extend_from_slice(&[ii, ii + 1, oi + 1, ii, oi + 1, oi]);
                } else {
                    // Flipped winding for the downward-facing cap
                    indices.extend_from_slice(&[ii, oi + 1, ii + 1, ii, oi, oi + 1]);
                }
            }
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
            // top vertex
            positions.push([r_outer * c, y_top, r_outer * s]);
            normals.push([c, 0.0, s]);
            uvs.push([t, 1.0]);
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
            positions.push([r_inner * c, y_top, r_inner * s]);
            normals.push([-c, 0.0, -s]);
            uvs.push([t, 1.0]);
        }
        for i in 0..segments {
            let b = base_idx + i * 2;
            // Flipped winding for inward-facing wall
            indices.extend_from_slice(&[b, b + 3, b + 1, b, b + 2, b + 3]);
        }
    }

    // Colours come from the shared walk, so they land on the vertices above in
    // the same order the two recolour paths will assume.
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(total_verts);
    for_each_sector_vertex(axis_angle, half_span, segments, |a, radial| {
        let bell = lobe_bell(a, axis_angle, half_span, gradient_intensity);
        colors.push(sector_color(base_color, amplitude, bell, radial));
    });
    debug_assert_eq!(colors.len(), positions.len());

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
    let mut colors: Vec<[f32; 4]> = Vec::new();
    // Small improvement to make more visiable the poles such as the phases
    let base_color: [f32; 4] = [base_color[0], base_color[1], base_color[2], 0.5];
    for_each_sector_vertex(axis_angle, half_span, segments, |a, radial| {
        let bell = lobe_bell(a, axis_angle, half_span, gradient_intensity);
        colors.push(sector_color(base_color, amplitude, bell, radial));
    });

    if let Some(attr) = mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR) {
        *attr = VertexAttributeValues::Float32x4(colors);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn sector_params(amplitude: f32, base_color: [f32; 4]) -> SectorMeshParams {
        SectorMeshParams {
            r_inner: 0.05,
            r_outer: 1.94,
            y_bot: -0.98,
            y_top: 0.98,
            axis_angle: 0.6,
            half_span: 0.5,
            segments: 8,
            gradient_intensity: 2.0,
            amplitude,
            base_color,
        }
    }

    fn mesh_colors(mesh: &Mesh) -> Vec<[f32; 4]> {
        match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(VertexAttributeValues::Float32x4(c)) => c.clone(),
            _ => panic!("sector mesh has no colour attribute"),
        }
    }

    fn mesh_positions(mesh: &Mesh) -> Vec<[f32; 3]> {
        match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(p)) => p.clone(),
            _ => panic!("sector mesh has no positions"),
        }
    }

    /// The mesh is laid out in one place and recoloured in two others, so the
    /// colour of a vertex must follow from where that vertex actually is. This
    /// recovers each vertex's radius from its position and checks the colour
    /// against it — if any of the three walks drifts, this catches it.
    #[test]
    fn every_vertex_is_coloured_for_the_radius_it_sits_at() {
        let params = sector_params(0.8, [0.2, 0.9, 0.3, 1.0]);
        let (r_inner, r_outer) = (params.r_inner, params.r_outer);
        let mesh = build_sector_mesh(params);

        let positions = mesh_positions(&mesh);
        let colors = mesh_colors(&mesh);
        assert_eq!(positions.len(), colors.len());

        for (position, color) in positions.iter().zip(colors.iter()) {
            let radius = (position[0] * position[0] + position[2] * position[2]).sqrt();
            let radial = ((radius - r_inner) / (r_outer - r_inner)).clamp(0.0, 1.0);
            let expected_mix = polarity_mix(radial);

            // Red channel: blends from the phase's 0.2 towards the north 1.0.
            let expected_red = 0.2 + (NORTH_COLOR[0] - 0.2) * expected_mix;
            assert!(
                (color[0] - expected_red).abs() < 1e-3,
                "vertex at r={radius:.3} (radial {radial:.3}) is {:.3} red, expected {expected_red:.3}",
                color[0]
            );
        }
    }

    /// Rebuilding the colours must land on exactly the same array the builder
    /// produced for the same parameters.
    #[test]
    fn recolouring_reproduces_the_built_colours() {
        let params = sector_params(-0.6, [0.2, 0.9, 0.3, 1.0]);
        let (axis_angle, half_span, segments, gradient, amplitude, base) = (
            params.axis_angle,
            params.half_span,
            params.segments,
            params.gradient_intensity,
            params.amplitude,
            params.base_color,
        );
        let mut mesh = build_sector_mesh(params);
        let built = mesh_colors(&mesh);

        recolor_sector_mesh(
            &mut mesh, axis_angle, half_span, segments, gradient, amplitude, base,
        );
        assert_eq!(built, mesh_colors(&mesh));
    }

    /// North and south must be told apart at the rim, and must agree in the
    /// core — that is the whole point of splitting the two encodings.
    #[test]
    fn polarity_shows_on_the_rim_and_not_in_the_core() {
        let base = [0.2, 0.9, 0.3, 1.0];
        let bell = 1.0;

        let north_rim = sector_color(base, 0.7, bell, 1.0);
        let south_rim = sector_color(base, -0.7, bell, 1.0);

        // At the rim the blend is complete, whatever the two are tuned to.
        for channel in 0..3 {
            assert!((north_rim[channel] - NORTH_COLOR[channel]).abs() < EPS);
            assert!((south_rim[channel] - SOUTH_COLOR[channel]).abs() < EPS);
        }
        // And they must be plainly opposite: red leads one, blue the other.
        assert!(
            north_rim[0] > north_rim[2],
            "the north rim should read red: {north_rim:?}"
        );
        assert!(
            south_rim[2] > south_rim[0],
            "the south rim should read blue: {south_rim:?}"
        );

        // Same magnitude, so the same opacity either way.
        assert!((north_rim[3] - south_rim[3]).abs() < EPS);

        // At the centre both collapse onto the identity colour.
        for amplitude in [0.7_f32, -0.7] {
            let core = sector_color(base, amplitude, bell, 0.0);
            for channel in 0..3 {
                assert!((core[channel] - base[channel]).abs() < EPS);
            }
        }
    }

    /// "Aggressive" means the polarity stays bunched against the rim rather
    /// than bleeding across the whole lobe.
    #[test]
    fn the_polarity_band_stays_near_the_rim() {
        assert!((polarity_mix(1.0) - 1.0).abs() < EPS);
        assert!(polarity_mix(0.0) < EPS);

        // Half way in, almost nothing of the polarity colour is left.
        assert!(
            polarity_mix(0.5) < 0.05,
            "mid-radius still carries {} of the rim colour",
            polarity_mix(0.5)
        );
        // It only really takes hold in the outer fifth.
        assert!(polarity_mix(0.8) < 0.4);
        assert!(polarity_mix(0.95) > 0.7);
    }

    /// Regression: `(a - b).abs()` reported ~2π for two angles straddling the
    /// ±π seam, which blanked the resultant field along that seam.
    #[test]
    fn angular_distance_takes_the_short_way_around() {
        assert!((angular_distance(0.3, 0.1) - 0.2).abs() < EPS);
        // Straddling the seam: the naive difference is 6.0, the real one is
        // TAU - 6.0 ≈ 0.283.
        assert!((angular_distance(-3.0, 3.0) - (TAU - 6.0)).abs() < EPS);
        assert!((angular_distance(0.1, TAU - 0.1) - 0.2).abs() < EPS);
        // Never exceeds half a turn, whatever the winding of the inputs.
        for i in -20..20 {
            let a = i as f32 * 0.7;
            let d = angular_distance(a, 1.234);
            assert!((0.0..=PI + EPS).contains(&d), "distance {d} out of range");
        }
    }

    /// The `2p` lobes of one phase must tile the bore exactly once: adjacent
    /// pole axes are one full lobe apart, and the lobes together span 2π.
    #[test]
    fn lobes_tile_the_bore_without_gaps_or_overlap() {
        for pole_pairs in 1..=6_usize {
            let p = pole_pairs as f32;
            let half = lobe_half_span(pole_pairs);

            let total = (2 * pole_pairs) as f32 * 2.0 * half;
            assert!(
                (total - TAU).abs() < EPS,
                "p={pole_pairs}: lobes cover {total} rad, expected {TAU}"
            );

            // Consecutive poles of the same phase sit exactly one lobe apart.
            let _ = p;
            let cfg = MotorConfig {
                groove_count: 12 * pole_pairs,
                phases: 3,
                pole_pairs,
                ..bevy::prelude::default()
            };
            let axis0 = magnetic_axis(&cfg, 0, 0);
            let axis1 = magnetic_axis(&cfg, 0, 1);
            assert!(
                (angular_distance(axis0, axis1) - 2.0 * half).abs() < EPS,
                "p={pole_pairs}: adjacent axes are not one lobe apart"
            );
        }
    }

    /// Phase currents must be a plain sine — the cubed version used by the
    /// field renderer disagreed with the arrows, the waveform panel and the
    /// current strip, which all share this one formula.
    #[test]
    fn phase_currents_are_balanced_sines() {
        for phases in [2_usize, 3, 5, 6] {
            let alpha_m = phase_displacement(phases);

            // Phase 0 begins its cycle at the origin: zero, and rising.
            assert!(phase_current(0.0, 0, alpha_m).abs() < EPS);
            assert!(phase_current(0.01, 0, alpha_m) > 0.0);

            // Each phase begins exactly one displacement later, and so peaks a
            // quarter turn after that.
            for phase in 0..phases {
                let start = phase as f32 * alpha_m;
                assert!(
                    phase_current(start, phase, alpha_m).abs() < EPS,
                    "m={phases}, phase {phase} does not start at its own angle"
                );
                assert!(
                    (phase_current(start + PI / 2.0, phase, alpha_m) - 1.0).abs() < EPS,
                    "m={phases}, phase {phase} does not peak a quarter turn after its start"
                );
            }
        }

        // A balanced odd-phase set sums to zero at every instant.
        let alpha_m = phase_displacement(3);
        for step in 0..16 {
            let t = step as f32 * TAU / 16.0;
            let sum: f32 = (0..3).map(|k| phase_current(t, k, alpha_m)).sum();
            assert!(sum.abs() < EPS, "3-phase currents sum to {sum} at t={t}");
        }
    }
}
