use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;
use std::f32::consts::{PI, TAU};

use super::WindingPart;
use crate::config::STATOR_HEIGHT;

// ─── Endwinding stack ─────────────────────────────────────────────────────────

/// How far above the core face the endwinding basket should reach, as a
/// fraction of the stator height.
const BASKET_HEADROOM: f32 = 0.6;

/// Smallest stagger between neighbouring arcs, as a fraction of a wire
/// diameter.
///
/// The ceiling above cannot always be met. A 12-slot machine fills its slot
/// with a single wire 0.49 across — a quarter of the whole stator height — and
/// six phases of that simply do not nest in 1.2 of headroom. Compressed to fit,
/// the tubes would merge into one band and the phases would stop being
/// separable at all, which is worse than a tall basket. So the stagger stops
/// shrinking here and the basket is allowed to overrun instead: the wire is
/// genuinely that thick, and the picture says so.
const MIN_STAGGER: f32 = 0.35;

/// Where an arc sits in the stack, in units of [`stack_metrics`]'s step.
///
/// Phases carry the coarse term so the arcs of one phase stay together, and the
/// conductors within a phase the fine one so the wires of a single coil do not
/// fuse into a blob.
#[inline]
fn arc_rank(phase: usize, index: usize) -> f32 {
    phase as f32 * 1.1 + index as f32 * 0.8
}

/// Base clearance and per-rank step of the endwinding stack, in world units.
///
/// The stagger is measured in wire gauges, which is right for keeping tubes
/// apart — a fat wire needs more room than a thin one. But the gauge is set by
/// how much space the slot has, so it grows as slots get scarcer, while the
/// number of arcs to stack depends only on the phase and layer counts. The two
/// multiplied: six phases on a 12-slot machine climbed to 2.4× the height of
/// the motor itself, against 0.07× on a 144-slot one.
///
/// So the step is the gauge, shrunk if the stack would not otherwise fit under
/// [`BASKET_HEADROOM`], and never below [`MIN_STAGGER`]. The base clearance is
/// never compressed: that is what holds the arc clear of the core face.
fn stack_metrics(wire_size: f32, phases: usize, deep_count: usize) -> (f32, f32) {
    let base = wire_size * 1.6;
    let top_rank = arc_rank(phases.saturating_sub(1), deep_count.saturating_sub(1));
    if top_rank <= 0.0 {
        return (base, wire_size);
    }

    let headroom = (STATOR_HEIGHT * BASKET_HEADROOM - base).max(0.0);
    let step = (headroom / top_rank).clamp(wire_size * MIN_STAGGER, wire_size);
    (base, step)
}

// ─── Arc tube mesh builder ─────────────────────────────────────────────────────

/// Builds a single tube mesh that follows the arc of an endwinding.
///
/// The centre-line runs: a straight `lead` out of the slot, the arc sweeping
/// from `a_from` to `a_to` (handling wrap-around) lifted by `y_offset`, then a
/// straight `lead` back down into the slot it returns to. Both straight runs
/// are coaxial with the conductor they meet and end flush with the core face,
/// so tube and conductor butt together into one continuous wire.
struct ArcTubeParams {
    a_from: f32,
    a_diff: f32,
    /// Radius at the start of the arc — the deep coil side.
    r_from: f32,
    /// Radius at the end of the arc — the shallow coil side it returns into.
    r_to: f32,
    /// Height where the arc begins, i.e. the top of the straight lead.
    y_base: f32,
    y_offset: f32,
    /// Axial length of the straight run joining the arc to the conductor.
    lead: f32,
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

    // The lead runs back towards the core, i.e. opposite the lift.
    let lead = params.lead * -y_offset.signum();
    let at = |r: f32, a: f32, y: f32| Vec3::new(r * a.cos(), y, r * a.sin());

    // Centre-line points + Frenet frames
    let mut centers: Vec<Vec3> = Vec::with_capacity(arc_segments + 3);

    centers.push(at(r_from, a_from, y_base + lead));
    for seg in 0..=arc_segments {
        let t = seg as f32 / arc_segments as f32;
        let a = a_from + a_diff * t;
        let y = y_base + y_offset * (PI * t).sin();
        // Sweep radially as well, so the arc meets the shallow conductor it
        // actually connects to rather than floating at a fixed radius.
        let r = r_from + (r_to - r_from) * t;
        centers.push(at(r, a, y));
    }
    centers.push(at(r_to, a_from + a_diff, y_base + lead));

    let n = centers.len();
    let rings = n - 1;
    let mut tangents: Vec<Vec3> = Vec::with_capacity(n);

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
        let ring_u = i as f32 / rings as f32;

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
    for i in 0..(rings as u32) {
        for j in 0..rv {
            let a = i * rv + j;
            let b = i * rv + (j + 1) % rv;
            let c = (i + 1) * rv + j;
            let d = (i + 1) * rv + (j + 1) % rv;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
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
    let wire_height = data.conductor_height();

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
    if !data.view.show_endwindings {
        return;
    }

    let n = data.config.groove_count;
    let layout = data.layout;
    let pitch = data.pitch;

    // Arc geometry constants (shared across all arcs)
    let arc_segments = 24; // was 120 separate entities; now 24 segments per tube mesh
    let cross_sides = 12; // the tube butts against a round conductor, so keep it round

    // Same gauge as the slot conductors, so a coil reads as one continuous wire.
    let wire_size = layout.wire_radius * 2.0;

    // The tube picks up exactly where the conductor stops: flush with the core
    // face, on the same axis, at the same gauge.
    let lead = data.endwinding_lead();
    let y_arc = data.endwinding_y();

    // Stagger the arcs so overlapping coil heads stay readable, bounded so the
    // basket cannot outgrow the machine it sits on.
    let (base_lift, stagger) = stack_metrics(
        wire_size,
        data.config.phases,
        data.layout.packing.deep_count(),
    );

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

        let lift = base_lift + arc_rank(conductor.phase, conductor.index) * stagger;

        for (y_base, y_offset) in [(y_arc, lift), (-y_arc, -lift)] {
            let mesh = build_arc_tube_mesh(ArcTubeParams {
                a_from,
                a_diff,
                r_from,
                r_to,
                y_base,
                y_offset,
                lead,
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{STATOR_BORE_RADIUS, slot_bottom_radius};
    use crate::winding::{SlotLayout, SlotPacking};

    /// Every layout the panel can ask for, as `(wire diameter, phases, deep)`.
    fn cases() -> impl Iterator<Item = (usize, usize, usize, f32, usize)> {
        let grooves = [6_usize, 12, 24, 48, 144];
        grooves.into_iter().flat_map(|n| {
            (1..=6_usize).flat_map(move |layers| {
                (1..=crate::config::MAX_PHASES).map(move |phases| {
                    let layout = SlotLayout::new(
                        layers,
                        TAU / n as f32,
                        STATOR_BORE_RADIUS,
                        slot_bottom_radius(),
                    );
                    let deep = SlotPacking::new(layers).deep_count();
                    (n, layers, phases, layout.wire_radius * 2.0, deep)
                })
            })
        })
    }

    /// Height of the tallest arc above the core face.
    fn basket_height(wire_size: f32, phases: usize, deep: usize) -> f32 {
        let (base, step) = stack_metrics(wire_size, phases, deep);
        base + arc_rank(phases.saturating_sub(1), deep.saturating_sub(1)) * step
    }

    /// The endwindings sit on the machine; they must not dwarf it.
    ///
    /// The stagger is measured in wire gauges and the gauge grows as slots get
    /// scarcer, so six phases on a coarse machine used to climb to 3.49 — 2.4×
    /// the height of the whole stator — while the same six phases on a 144-slot
    /// machine reached 0.15.
    #[test]
    fn the_basket_never_outgrows_the_motor() {
        for (n, layers, phases, wire_size, deep) in cases() {
            let height = basket_height(wire_size, phases, deep);
            assert!(
                height <= STATOR_HEIGHT,
                "n={n} layers={layers} m={phases}: the basket reaches {height:.3}, \
                 taller than the {STATOR_HEIGHT} stator it stands on"
            );
        }
    }

    /// The bound must only bite where it is needed. A machine fine enough to
    /// fit under the headroom keeps the full wire-gauge stagger, so capping the
    /// coarse end did not quietly flatten every other winding.
    #[test]
    fn a_fine_winding_keeps_its_full_stagger() {
        for (n, layers, phases, wire_size, deep) in cases() {
            let (_, step) = stack_metrics(wire_size, phases, deep);
            if basket_height(wire_size, phases, deep) < STATOR_HEIGHT * BASKET_HEADROOM {
                assert!(
                    (step - wire_size).abs() < 1e-6,
                    "n={n} layers={layers} m={phases}: fits with room to spare but was \
                     compressed to {step:.4} against a {wire_size:.4} gauge"
                );
            }
        }
    }

    /// Arcs must stay in stack order and never share a height, or the phases
    /// they carry would be impossible to follow where the coil heads overlap.
    #[test]
    fn each_rank_sits_above_the_one_below() {
        for (n, layers, phases, wire_size, deep) in cases() {
            let (base, step) = stack_metrics(wire_size, phases, deep);
            assert!(step > 0.0, "n={n} layers={layers} m={phases}: flat stack");

            let mut ranks: Vec<f32> = (0..phases)
                .flat_map(|p| (0..deep).map(move |i| arc_rank(p, i)))
                .collect();
            ranks.sort_by(|a, b| a.partial_cmp(b).expect("ranks are finite"));

            for pair in ranks.windows(2) {
                let (lower, upper) = (base + pair[0] * step, base + pair[1] * step);
                assert!(
                    upper > lower,
                    "n={n} layers={layers} m={phases}: two arcs share height {lower:.4}"
                );
            }
        }
    }

    /// The base clearance is what holds the arc off the core face, so the cap
    /// must compress the stagger and leave it alone.
    #[test]
    fn the_cap_never_eats_the_base_clearance() {
        for (n, layers, phases, wire_size, deep) in cases() {
            let (base, _) = stack_metrics(wire_size, phases, deep);
            assert!(
                (base - wire_size * 1.6).abs() < 1e-6,
                "n={n} layers={layers} m={phases}: base clearance was compressed"
            );
        }
    }
}
