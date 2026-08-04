use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::{config::*, phase};

pub mod current;
pub mod header_coils;
pub mod ui;

/// Direction of current flow in a slot conductor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    /// The return side of the same coil carries the opposite direction.
    #[inline]
    pub const fn reversed(self) -> Self {
        match self {
            Self::In => Self::Out,
            Self::Out => Self::In,
        }
    }
}

/// Assignment of a conductor to a slot.
#[derive(Clone, Debug)]
pub struct SlotAssignment {
    pub phase: usize,
    pub direction: Direction,
}

/// One physical conductor sitting inside a slot.
///
/// `MotorConfig::layers` is the number of conductors packed into each slot.
/// They are split into two *electrical* layers: the deep half (towards the slot
/// bottom) carries the outgoing side of the coil starting at this slot, and the
/// shallow half (towards the bore) carries the return side of the coil that
/// started `pitch` slots earlier. With full pitch both halves land on the same
/// phase; short pitching is exactly what makes them differ.
#[derive(Clone, Copy, Debug)]
pub struct Conductor {
    pub slot: usize,
    /// Packing index within the slot; 0 is deepest, at the slot bottom.
    pub index: usize,
    /// 0 = deep electrical layer, 1 = shallow electrical layer.
    pub layer: usize,
    pub phase: usize,
    pub direction: Direction,
}

/// Geometric packing of the round conductors inside one slot.
///
/// Conductors are laid out two per row across the slot width, filling from the
/// slot bottom towards the bore: `count = 4` gives 2×2, `count = 6` gives 2×3.
/// A trailing odd conductor is centred in its row.
#[derive(Clone, Copy, Debug)]
pub struct SlotLayout {
    pub count: usize,
    pub rows: usize,
    pub cols: usize,
    /// Radius of a single round conductor.
    pub wire_radius: f32,
    row_pitch: f32,
    col_pitch: f32,
    r_slot_bot: f32,
}

impl SlotLayout {
    /// Packing that fits `count` conductors between the bore and slot bottom.
    pub fn new(count: usize, segment_angle: f32, r_bore: f32, r_slot_bot: f32) -> Self {
        let count = count.max(1);
        let cols = count.min(2);
        let rows = count.div_ceil(cols);

        let radial_span = r_slot_bot - r_bore;
        let row_pitch = radial_span / rows as f32;

        // The slot is the gap left between two teeth; the tooth takes half of
        // each angular segment, so the opening spans `segment_angle * 0.5`.
        // That opening is an arc, so it is narrowest at the innermost row —
        // size the packing there or the inner wires bite into the teeth.
        let r_innermost = r_bore + row_pitch * 0.5;
        let tangential_span = segment_angle * 0.5 * r_innermost;
        let col_pitch = tangential_span / cols as f32;

        // 0.82 leaves a visible gap between neighbouring wires.
        let wire_radius = 0.5 * row_pitch.min(col_pitch) * 0.82;

        Self {
            count,
            rows,
            cols,
            wire_radius,
            row_pitch,
            col_pitch,
            r_slot_bot,
        }
    }

    /// How many conductors share the row containing `index`.
    fn row_occupancy(&self, row: usize) -> usize {
        if row == self.rows - 1 {
            self.count - row * self.cols
        } else {
            self.cols
        }
    }

    /// Centre of conductor `index` as `(radius, tangential offset)`.
    ///
    /// The offset is an arc length, so the angle is `offset / radius`.
    pub fn placement(&self, index: usize) -> (f32, f32) {
        let row = index / self.cols;
        let col = index % self.cols;
        let radius = self.r_slot_bot - (row as f32 + 0.5) * self.row_pitch;

        let in_row = self.row_occupancy(row);
        let offset = (col as f32 - (in_row as f32 - 1.0) / 2.0) * self.col_pitch;

        (radius, offset)
    }

    /// World-space centre of conductor `index` in the slot at `slot_center`.
    pub fn position(&self, index: usize, slot_center: f32, y: f32) -> Vec3 {
        let (radius, offset) = self.placement(index);
        let angle = slot_center + offset / radius;
        Vec3::new(radius * angle.cos(), y, radius * angle.sin())
    }

    /// Number of conductors in the deep (layer 0) half.
    ///
    /// Both coil sides must carry the same number of turns, so an odd `count`
    /// gives the deep half the extra conductor rather than splitting by row.
    #[inline]
    pub fn deep_count(&self) -> usize {
        self.count.div_ceil(2)
    }

    /// Index in the destination slot that conductor `index` connects to.
    ///
    /// A coil leaves the deep half of one slot and returns into the shallow
    /// half of the slot `pitch` steps away. With a single conductor per slot
    /// there is only one layer, so it connects straight across.
    pub fn coil_partner(&self, index: usize) -> usize {
        if self.count == 1 {
            return 0;
        }
        let deep = self.deep_count();
        let shallow = self.count - deep;
        deep + (index % shallow.max(1))
    }
}

/// Marker for winding entities.
#[derive(Component)]
pub struct WindingPart;

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

    let mut assignments: Vec<Option<SlotAssignment>> = vec![None; n];

    for k in 0..(2 * p * m) {
        let k_elec = k % (2 * m);
        let (phase, direction) = if !m.is_multiple_of(2) {
            // odd phases
            if k_elec.is_multiple_of(2) {
                let f = (k_elec / 2) % m;
                (f, Direction::In)
            } else {
                let f = ((k_elec as isize - m as isize) / 2).rem_euclid(m as isize) as usize;
                (f, Direction::Out)
            }
        } else {
            // even phases
            if k_elec < m {
                (k_elec, Direction::In)
            } else {
                (k_elec - m, Direction::Out)
            }
        };

        for j in 0..q {
            let slot_idx = (k * q + j) % n;
            assignments[slot_idx] = Some(SlotAssignment { phase, direction });
        }
    }

    assignments
}

/// Expands the per-slot belt assignment into individual conductors.
///
/// The deep half of a slot takes the belt assignment directly. The shallow half
/// takes the *return* side of the coil that started `pitch` slots earlier —
/// same phase, reversed direction. At full pitch that resolves to the same
/// phase as the deep half; short pitching makes the halves disagree, which is
/// what chording physically is.
pub fn compute_conductors(
    config: &MotorConfig,
    assignments: &[Option<SlotAssignment>],
) -> Vec<Conductor> {
    let n = config.groove_count;
    if n == 0 || assignments.len() != n {
        return Vec::new();
    }

    let count = config.layers.max(1);
    let deep = count.div_ceil(2);
    let pitch = coil_pitch(config) % n;

    let mut conductors = Vec::with_capacity(n * count);
    for slot in 0..n {
        for index in 0..count {
            // With one conductor per slot there is no second layer.
            let layer = usize::from(index >= deep && count > 1);

            let assignment = if layer == 0 {
                assignments[slot].clone()
            } else {
                assignments[(slot + n - pitch) % n]
                    .as_ref()
                    .map(|a| SlotAssignment {
                        phase: a.phase,
                        direction: a.direction.reversed(),
                    })
            };

            let Some(a) = assignment else { continue };
            conductors.push(Conductor {
                slot,
                index,
                layer,
                phase: a.phase,
                direction: a.direction,
            });
        }
    }
    conductors
}

/// Whether an endwinding arc leaves this conductor.
///
/// A double-layer winding has one coil per slot: every deep conductor starts
/// one, whichever way its current runs. A single-layer winding has half as many
/// coils, each spanning two slots, so only the outgoing sides start one — the
/// returning side is already the far end of somebody else's coil.
pub fn starts_coil(conductor: &Conductor, layers: usize) -> bool {
    conductor.layer == 0 && (layers > 1 || conductor.direction == Direction::In)
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

/// Data parameters for winding rendering, reducing argument counts for clippy.
pub struct WindingData<'a> {
    pub config: &'a MotorConfig,
    pub conductors: &'a [Conductor],
    pub layout: SlotLayout,
    pub segment_angle: f32,
    pub tooth_angle: f32,
    pub half_h: f32,
    pub pitch: usize,
}

impl WindingData<'_> {
    /// Centre angle of `slot`; the slot sits just after its tooth.
    #[inline]
    pub fn slot_center(&self, slot: usize) -> f32 {
        slot as f32 * self.segment_angle + self.tooth_angle + self.segment_angle * 0.25
    }
}

/// System: generate winding conductors and endwindings when config changes.
pub fn regenerate_winding(
    mut commands: Commands,
    config: Res<MotorConfig>,
    mut ev_config: MessageReader<MotorConfigChanged>,
    query: Query<Entity, With<WindingPart>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if ev_config.read().next().is_none() {
        return;
    }

    // Despawn old winding geometry
    for entity in &query {
        commands.entity(entity).despawn();
    }

    let assignments = compute_winding(&config);
    let conductors = compute_conductors(&config, &assignments);
    let n = config.groove_count;
    let segment_angle = TAU / n as f32;
    let tooth_angle = segment_angle * 0.5;
    let half_h = STATOR_HEIGHT / 2.0;
    let pitch = coil_pitch(&config);
    let layout = SlotLayout::new(
        config.layers,
        segment_angle,
        STATOR_BORE_RADIUS,
        slot_bottom_radius(),
    );

    let data = WindingData {
        config: &config,
        conductors: &conductors,
        layout,
        segment_angle,
        tooth_angle,
        half_h,
        pitch,
    };

    // Pre-create phase materials
    let phase_mats: Vec<_> = (0..config.phases)
        .map(|p| {
            let color = phase::colors::phase_color(p, config.phases);
            materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })
        })
        .collect();

    let phase_mats_opp: Vec<_> = (0..config.phases)
        .map(|p| {
            let color = phase::colors::phase_color_opposite(p, config.phases);
            materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })
        })
        .collect();

    // Call split rendering functions
    header_coils::render_conductors(&mut commands, &mut meshes, &data, &phase_mats);
    header_coils::render_header_coils(&mut commands, &mut meshes, &data, &phase_mats);
    current::render_current_directions(&mut commands, &mut meshes, &data, &phase_mats_opp);
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn config(groove_count: usize, phases: usize, pole_pairs: usize, layers: usize) -> MotorConfig {
        MotorConfig {
            groove_count,
            phases,
            pole_pairs,
            layers,
            ..default()
        }
    }

    /// Groups the conductors of one slot, keeping packing order.
    fn slot_of(conductors: &[Conductor], slot: usize) -> Vec<&Conductor> {
        conductors.iter().filter(|c| c.slot == slot).collect()
    }

    /// A full-pitch winding has both coil sides of a slot on the same phase,
    /// so the slot behaves exactly like the single-layer model.
    #[test]
    fn full_pitch_keeps_a_slot_on_one_phase() {
        let cfg = config(12, 3, 1, 4);
        assert!(!cfg.short_pitched);
        let assignments = compute_winding(&cfg);
        let conductors = compute_conductors(&cfg, &assignments);

        for slot in 0..cfg.groove_count {
            let wires = slot_of(&conductors, slot);
            assert_eq!(wires.len(), 4, "slot {slot}");
            let first = wires[0];
            for w in &wires {
                assert_eq!(w.phase, first.phase, "slot {slot} mixed phases at full pitch");
                assert_eq!(w.direction, first.direction, "slot {slot} mixed directions");
            }
        }
    }

    /// Chording is precisely what makes the two layers of a slot disagree.
    #[test]
    fn short_pitch_mixes_two_phases_in_some_slots() {
        let mut cfg = config(12, 3, 1, 4);
        cfg.short_pitched = true;
        let assignments = compute_winding(&cfg);
        let conductors = compute_conductors(&cfg, &assignments);

        let mixed = (0..cfg.groove_count)
            .filter(|&slot| {
                let wires = slot_of(&conductors, slot);
                wires.iter().any(|w| w.phase != wires[0].phase)
            })
            .count();

        assert!(mixed > 0, "short pitching produced no mixed slot");
        assert!(
            mixed < cfg.groove_count,
            "every slot mixed — chording should only touch belt boundaries"
        );
    }

    /// One conductor per slot must reproduce the plain single-layer winding.
    #[test]
    fn single_conductor_matches_the_belt_assignment() {
        let mut cfg = config(24, 3, 2, 1);
        cfg.short_pitched = true;
        let assignments = compute_winding(&cfg);
        let conductors = compute_conductors(&cfg, &assignments);

        assert_eq!(conductors.len(), cfg.groove_count);
        for conductor in &conductors {
            let belt = assignments[conductor.slot].as_ref().expect("belt");
            assert_eq!(conductor.layer, 0);
            assert_eq!(conductor.phase, belt.phase);
            assert_eq!(conductor.direction, belt.direction);
        }
    }

    /// Both coil sides must carry the same number of turns.
    #[test]
    fn layers_split_evenly_for_even_counts() {
        for count in [2_usize, 4, 6] {
            let cfg = config(12, 3, 1, count);
            let assignments = compute_winding(&cfg);
            let conductors = compute_conductors(&cfg, &assignments);

            let wires = slot_of(&conductors, 0);
            let deep = wires.iter().filter(|w| w.layer == 0).count();
            let shallow = wires.iter().filter(|w| w.layer == 1).count();
            assert_eq!(deep, shallow, "layers={count} unbalanced: {deep} vs {shallow}");
        }
    }

    /// The described packing: two per row, filling from the slot bottom.
    #[test]
    fn packing_is_two_per_row() {
        let cases = [(1_usize, 1_usize, 1_usize), (2, 1, 2), (4, 2, 2), (6, 3, 2)];
        for (count, rows, cols) in cases {
            let layout = SlotLayout::new(count, TAU / 24.0, 2.0, 2.6);
            assert_eq!(layout.rows, rows, "layers={count}");
            assert_eq!(layout.cols, cols, "layers={count}");
        }
    }

    /// Conductors must stay inside the slot: within the radial channel and
    /// within the angular opening left between two teeth, at every row.
    #[test]
    fn conductors_stay_inside_the_slot() {
        let r_bore = 2.0_f32;
        let r_slot_bot = 2.6_f32;

        for n in [12_usize, 24, 48, 144] {
            let segment_angle = TAU / n as f32;
            for count in 1..=6_usize {
                let layout = SlotLayout::new(count, segment_angle, r_bore, r_slot_bot);
                assert!(layout.wire_radius > 0.0);

                for index in 0..count {
                    let (radius, offset) = layout.placement(index);

                    assert!(
                        radius - layout.wire_radius >= r_bore - 1e-4,
                        "n={n} layers={count} wire {index} pokes through the bore"
                    );
                    assert!(
                        radius + layout.wire_radius <= r_slot_bot + 1e-4,
                        "n={n} layers={count} wire {index} pokes past the slot bottom"
                    );

                    // Half the angular opening, measured as an arc at this radius.
                    let half_opening = segment_angle * 0.25 * radius;
                    assert!(
                        offset.abs() + layout.wire_radius <= half_opening + 1e-4,
                        "n={n} layers={count} wire {index} bites into a tooth: \
                         |{offset}| + {} > {half_opening}",
                        layout.wire_radius
                    );
                }
            }
        }
    }

    /// A coil always leaves the deep half and returns into the shallow half.
    #[test]
    fn coil_partner_crosses_to_the_other_layer() {
        for count in [2_usize, 4, 6] {
            let layout = SlotLayout::new(count, TAU / 24.0, 2.0, 2.6);
            let deep = layout.deep_count();
            for index in 0..deep {
                let partner = layout.coil_partner(index);
                assert!(
                    partner >= deep && partner < count,
                    "layers={count}: wire {index} returns to {partner}, not a shallow wire"
                );
            }
        }
        // With a single conductor there is no second layer to cross into.
        let layout = SlotLayout::new(1, TAU / 24.0, 2.0, 2.6);
        assert_eq!(layout.coil_partner(0), 0);
    }
}
