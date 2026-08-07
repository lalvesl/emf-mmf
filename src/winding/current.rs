use bevy::prelude::*;
use std::f32::consts::PI;

use super::{Direction, WindingPart, axis};
use crate::config::{MotorConfig, ViewConfig};
use crate::electrical::ElectricalState;
use crate::vectors::arrow::{self, ArrowHead, ArrowShaft, HeadQuery, ShaftQuery};

/// Arrow length at full current, in world units.
const FULL_LENGTH: f32 = 0.5;

/// Clearance between the core face and the foot of an arrow.
const LIFT: f32 = 0.05;

pub fn render_current_directions(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    data: &super::WindingData,
    phase_mats_opp: &[Handle<StandardMaterial>],
) {
    if data.view.show_endwindings {
        return;
    }

    let layout = data.layout;
    let wire_height = data.conductor_height();

    let top_y = wire_height / 2.0 + 0.002;
    let bottom_y = -wire_height / 2.0 - 0.002;

    // Symbols sit on the end face of each conductor, so they scale with it.
    let symbol_radius = layout.wire_radius * 0.8;
    let line_thickness = (symbol_radius * 0.28).max(0.004);

    // The cross bar and the dot are identical in every slot; build one of each
    // up front rather than a fresh asset per slot.
    let mesh_bar = meshes.add(Cuboid::new(
        symbol_radius * 2.0,
        line_thickness,
        line_thickness,
    ));
    let mesh_dot = meshes.add(Cylinder::new(symbol_radius * 0.6, line_thickness));

    // --- Show current directions (crosses for In, dots for Out) over the coils ---
    for conductor in data.conductors {
        let mat = phase_mats_opp[conductor.phase % phase_mats_opp.len()].clone();

        let slot_center = data.slot_center(conductor.slot);
        let position = layout.position(conductor.index, slot_center, 0.0);
        let (x, z) = (position.x, position.z);

        // A dot is current coming towards the viewer and a cross is current
        // going away, so a conductor reads oppositely from its two ends —
        // whatever leaves the top face is what enters the bottom one. Drawing
        // `conductor.direction` on both put the wrong sense on the underside of
        // the machine.
        for (y, direction) in [
            (top_y, conductor.direction),
            (bottom_y, conductor.direction.reversed()),
        ] {
            match direction {
                // Cross (X): two bars laid across the slot's own axis.
                Direction::In => {
                    let facing = Quat::from_rotation_y(-slot_center + PI / 2.0);
                    for tilt in [PI / 4.0, -PI / 4.0] {
                        commands.spawn((
                            Mesh3d(mesh_bar.clone()),
                            MeshMaterial3d(mat.clone()),
                            Transform::from_xyz(x, y, z)
                                .with_rotation(facing * Quat::from_rotation_y(tilt)),
                            WindingPart,
                        ));
                    }
                }
                // Dot (cylinder).
                Direction::Out => {
                    commands.spawn((
                        Mesh3d(mesh_dot.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(x, y, z),
                        WindingPart,
                    ));
                }
            }
        }
    }
}

/// One conductor's current arrow, standing on the top face of the core.
///
/// The ⊙/⊗ symbols on the end faces report the *sense* of the winding; this
/// reports how much current is in it right now, so the two only ever appear
/// together — with the endwindings hidden.
#[derive(Component)]
pub struct CurrentVector {
    phase: usize,
    /// `+1` when this conductor carries the phase current along `+Y`.
    sense: f32,
    /// Where the arrow meets the core face — one end of it, always.
    anchor: Vec3,
    /// Which way along `Y` is *away* from the iron: `+1` on the top face, `-1`
    /// on the bottom. The arrow is hung so it occupies the band on that side
    /// whichever way the current happens to run, instead of sinking into the
    /// slot it came out of.
    outward: f32,
}

pub fn render_current_vectors(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    data: &super::WindingData,
    phase_mats: &[Handle<StandardMaterial>],
) {
    if data.view.show_endwindings || phase_mats.is_empty() {
        return;
    }

    let layout = data.layout;
    let face_y = data.conductor_height() / 2.0 + LIFT;

    // Built from the wire gauge, so a 144-slot machine gets slender arrows
    // instead of a thicket of full-sized ones over hair-thin conductors.
    let head_height = layout.wire_radius * 2.5;
    let shaft_mesh = meshes.add(Cylinder::new(layout.wire_radius * 0.35, 1.0));
    let head_mesh = meshes.add(Cone {
        radius: layout.wire_radius * 0.9,
        height: head_height,
    });

    for conductor in data.conductors {
        let mat = phase_mats[conductor.phase % phase_mats.len()].clone();
        let slot_center = data.slot_center(conductor.slot);
        let sense = match conductor.direction {
            Direction::Out => 1.0,
            Direction::In => -1.0,
        };

        // Both end faces, so the reading holds from underneath the machine as
        // well — the same two faces the ⊙/⊗ symbols sit on.
        for outward in [1.0_f32, -1.0] {
            let anchor = layout.position(conductor.index, slot_center, outward * face_y);

            // Only the parent carries `WindingPart`: despawning it takes the
            // shaft and head with it, and marking those too would have the
            // regeneration query hand back entities that are already gone.
            commands
                .spawn((
                    Transform::from_translation(anchor),
                    Visibility::default(),
                    WindingPart,
                    CurrentVector {
                        phase: conductor.phase,
                        sense,
                        anchor,
                        outward,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Mesh3d(shaft_mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::default(),
                        ArrowShaft,
                    ));
                    parent.spawn((
                        Mesh3d(head_mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::default(),
                        ArrowHead {
                            height: head_height,
                        },
                    ));
                });
        }
    }
}

/// Where an arrow of `length` starts, and whether it points along `-Y`.
///
/// The returned offset is from the anchor to the arrow's tail. The arrow always
/// occupies the band `[anchor, anchor + outward·length]`, so it never crosses
/// the core face into the iron — only which end of that band is its tail
/// depends on which way the current runs. An arrow flowing back towards the
/// face is therefore hung from its tip.
fn arrow_placement(current: f32, length: f32, outward: f32) -> (f32, bool) {
    let along = if current >= 0.0 { 1.0 } else { -1.0 };
    let tail = if along == outward {
        0.0
    } else {
        outward * length
    };
    (tail, along < 0.0)
}

/// Swing the current arrows with the waveform.
pub fn animate_current_vectors(
    config: Res<MotorConfig>,
    view: Res<ViewConfig>,
    state: Res<ElectricalState>,
    mut vectors: Query<(&CurrentVector, &Children, &mut Transform)>,
    mut shafts: ShaftQuery<CurrentVector>,
    mut heads: HeadQuery<CurrentVector>,
) {
    if view.show_endwindings {
        return;
    }

    let m = config.phases;
    if m == 0 {
        return;
    }
    let alpha_m = axis::phase_displacement(m);

    for (vector, children, mut transform) in &mut vectors {
        // Defend against entities from a previous configuration still waiting
        // on the commands buffer.
        if vector.phase >= m {
            continue;
        }

        let current = axis::phase_current(state.angle, vector.phase, alpha_m) * vector.sense;
        let length = current.abs() * FULL_LENGTH;

        if length <= 1e-3 {
            transform.scale = Vec3::ZERO;
            continue;
        }
        transform.scale = Vec3::ONE;

        let (tail, flipped) = arrow_placement(current, length, vector.outward);
        transform.translation = vector.anchor + Vec3::Y * tail;
        transform.rotation = if flipped {
            Quat::from_rotation_x(PI)
        } else {
            Quat::IDENTITY
        };

        arrow::lay_out(children, length, &mut shafts, &mut heads);
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// The iron is opaque, so an arrow that crossed its own core face would
    /// simply vanish into it. Whichever way the current runs, and on whichever
    /// face, the whole arrow has to stay on the outside.
    #[test]
    fn a_current_arrow_never_crosses_its_core_face() {
        for outward in [1.0_f32, -1.0] {
            for current in [-1.0_f32, -0.4, 0.4, 1.0] {
                let length = current.abs() * FULL_LENGTH;
                let (tail, flipped) = arrow_placement(current, length, outward);
                let tip = tail + if flipped { -length } else { length };

                for end in [tail, tip] {
                    assert!(
                        end * outward >= -1e-6,
                        "outward={outward} current={current}: an end sits at {end}, \
                         inside the iron"
                    );
                    assert!(
                        end.abs() <= length + 1e-6,
                        "outward={outward} current={current}: an end sits at {end}, \
                         beyond the {length} the arrow is long"
                    );
                }

                assert_eq!(
                    flipped,
                    current < 0.0,
                    "outward={outward} current={current}: the arrow points against \
                     the current"
                );
            }
        }
    }

    /// The end that touches the face reports what the current does there: the
    /// tail sits on the face the current leaves through, the tip on the face it
    /// arrives at. The two faces are therefore *not* mirror images — the same
    /// current runs one way through the whole conductor, so both arrows point
    /// the same way and only their footing differs.
    #[test]
    fn the_arrow_meets_the_face_tail_first_only_where_the_current_leaves() {
        for outward in [1.0_f32, -1.0] {
            for current in [-1.0_f32, -0.3, 0.3, 1.0] {
                let length = current.abs() * FULL_LENGTH;
                let (tail, flipped) = arrow_placement(current, length, outward);
                let tip = tail + if flipped { -length } else { length };

                let leaving = (current >= 0.0) == (outward > 0.0);
                let on_face = if leaving { tail } else { tip };
                assert!(
                    on_face.abs() < 1e-6,
                    "outward={outward} current={current}: the arrow meets its face at \
                     {on_face} rather than on it"
                );
            }
        }
    }

    /// Both faces read the same current, so both arrows point the same way.
    #[test]
    fn the_two_faces_agree_on_the_direction() {
        for current in [-1.0_f32, -0.3, 0.3, 1.0] {
            let length = current.abs() * FULL_LENGTH;
            let (_, top_flipped) = arrow_placement(current, length, 1.0);
            let (_, bottom_flipped) = arrow_placement(current, length, -1.0);

            assert_eq!(
                top_flipped, bottom_flipped,
                "current={current}: the same current read two different ways"
            );
        }
    }
}
