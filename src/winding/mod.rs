use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::{config::*, phase};

pub mod axis;
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

/// How the conductors of one slot are arranged, independent of any dimensions.
///
/// Conductors are laid out two per row, filling from the slot bottom towards
/// the bore: `count = 4` gives 2×2, `count = 6` gives 2×3. A trailing odd
/// conductor is centred in its row.
///
/// This is deliberately free of radii and wire gauges so the 2D winding diagram
/// can draw the same arrangement as the 3D view from the same source, rather
/// than reproducing the arithmetic and silently drifting from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotPacking {
    pub count: usize,
    pub rows: usize,
    pub cols: usize,
}

impl SlotPacking {
    pub fn new(count: usize) -> Self {
        let count = count.max(1);
        let cols = count.min(2);
        Self {
            count,
            rows: count.div_ceil(cols),
            cols,
        }
    }

    /// Grid position of `index` as `(row, col)`; row 0 is the slot bottom.
    #[inline]
    pub fn row_col(&self, index: usize) -> (usize, usize) {
        (index / self.cols, index % self.cols)
    }

    /// How many conductors share the row containing `index`.
    pub fn row_occupancy(&self, row: usize) -> usize {
        if row == self.rows - 1 {
            self.count - row * self.cols
        } else {
            self.cols
        }
    }

    /// Tangential offset of `index` from the slot centre, in column pitches.
    ///
    /// A row that is not full is centred, so a lone trailing conductor sits on
    /// the slot axis rather than hugging one tooth.
    pub fn column_offset(&self, index: usize) -> f32 {
        let (row, col) = self.row_col(index);
        let in_row = self.row_occupancy(row);
        col as f32 - (in_row as f32 - 1.0) / 2.0
    }

    /// Number of conductors in the deep (layer 0) half.
    ///
    /// Both coil sides must carry the same number of turns, so an odd `count`
    /// gives the deep half the extra conductor rather than splitting by row.
    #[inline]
    pub fn deep_count(&self) -> usize {
        self.count.div_ceil(2)
    }

    /// Which electrical layer `index` belongs to: 0 deep, 1 shallow.
    ///
    /// With a single conductor per slot there is no second layer.
    #[inline]
    pub fn layer_of(&self, index: usize) -> usize {
        usize::from(index >= self.deep_count() && self.count > 1)
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

/// [`SlotPacking`] placed into the real slot, with wire gauge and radii.
#[derive(Clone, Copy, Debug)]
pub struct SlotLayout {
    pub packing: SlotPacking,
    /// Radius of a single round conductor.
    pub wire_radius: f32,
    row_pitch: f32,
    col_pitch: f32,
    r_slot_bot: f32,
}

impl SlotLayout {
    /// Packing that fits `count` conductors between the bore and slot bottom.
    pub fn new(count: usize, segment_angle: f32, r_bore: f32, r_slot_bot: f32) -> Self {
        let packing = SlotPacking::new(count);
        let (rows, cols) = (packing.rows, packing.cols);

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
            packing,
            wire_radius,
            row_pitch,
            col_pitch,
            r_slot_bot,
        }
    }

    /// Centre of conductor `index` as `(radius, tangential offset)`.
    ///
    /// The offset is an arc length, so the angle is `offset / radius`.
    pub fn placement(&self, index: usize) -> (f32, f32) {
        let (row, _) = self.packing.row_col(index);
        let radius = self.r_slot_bot - (row as f32 + 0.5) * self.row_pitch;
        (radius, self.packing.column_offset(index) * self.col_pitch)
    }

    /// World-space centre of conductor `index` in the slot at `slot_center`.
    pub fn position(&self, index: usize, slot_center: f32, y: f32) -> Vec3 {
        let (radius, offset) = self.placement(index);
        let angle = slot_center + offset / radius;
        Vec3::new(radius * angle.cos(), y, radius * angle.sin())
    }

    #[inline]
    pub fn deep_count(&self) -> usize {
        self.packing.deep_count()
    }

    #[inline]
    pub fn coil_partner(&self, index: usize) -> usize {
        self.packing.coil_partner(index)
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

    let packing = SlotPacking::new(config.layers);
    let count = packing.count;
    let pitch = coil_pitch(config) % n;

    let mut conductors = Vec::with_capacity(n * count);
    for slot in 0..n {
        for index in 0..count {
            let layer = packing.layer_of(index);

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
///
/// Chording needs a slot to hold two coil sides that can belong to different
/// phases, so it is only possible with two electrical layers. A single-layer
/// winding has one coil side per slot and the phase-belt allocation already
/// fixes the phase and polarity of every slot; the coils merely pair up slots
/// that are already assigned. Changing their span would only reroute wire
/// through the air outside the core, leaving the MMF — and therefore the
/// winding factor — untouched. Its pitch factor is always 1, so the request is
/// ignored rather than drawing coils that short two phases together.
pub fn coil_pitch(config: &MotorConfig) -> usize {
    let slots_per_pole = config.groove_count / (2 * config.pole_pairs);
    if config.short_pitched && config.layers > 1 {
        slots_per_pole.saturating_sub(1).max(1)
    } else {
        slots_per_pole
    }
}

/// Whether chording actually does anything for this machine.
#[inline]
pub fn can_short_pitch(config: &MotorConfig) -> bool {
    config.layers > 1
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
        axis::slot_center(slot, self.config.groove_count)
    }

    /// Axial length of a slot conductor.
    ///
    /// With the endwindings hidden the wire stays tucked inside the core, so
    /// the current-direction symbols on its end faces read cleanly. With them
    /// shown it must reach exactly the core face, where the endwinding tube
    /// takes over — the two butt together into one continuous wire.
    #[inline]
    pub fn conductor_height(&self) -> f32 {
        if self.config.show_endwindings {
            STATOR_HEIGHT
        } else {
            STATOR_HEIGHT * 0.95
        }
    }

    /// Straight axial run the endwinding makes on leaving the slot.
    ///
    /// The arc sweeps tangentially, so it has to clear the core face before it
    /// starts turning or it would cut through the teeth. One and a half wire
    /// radii puts the underside of the tube above the face with margin.
    #[inline]
    pub fn endwinding_lead(&self) -> f32 {
        self.layout.wire_radius * 1.5
    }

    /// Height at which the endwinding arc proper begins, above the core face.
    #[inline]
    pub fn endwinding_y(&self) -> f32 {
        self.half_h + self.endwinding_lead()
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

    /// The 2D diagram and the 3D view must place a conductor the same way.
    /// They used to derive rows/cols/centring independently, so a change to one
    /// would silently desynchronise the other.
    #[test]
    fn the_metric_layout_places_conductors_where_the_packing_says() {
        for count in 1..=6_usize {
            let packing = SlotPacking::new(count);
            let layout = SlotLayout::new(count, TAU / 24.0, 2.0, 2.6);

            assert_eq!(layout.packing, packing, "layers={count}");

            for index in 0..count {
                let (row, _) = packing.row_col(index);
                let (radius, offset) = layout.placement(index);

                // Deeper rows sit further out, and the tangential offset is the
                // packing's column offset scaled by the real column pitch.
                let expected_radius = 2.6 - (row as f32 + 0.5) * layout.row_pitch;
                assert!((radius - expected_radius).abs() < 1e-5, "layers={count}");
                assert!(
                    (offset - packing.column_offset(index) * layout.col_pitch).abs() < 1e-5,
                    "layers={count} index={index}"
                );
            }
        }
    }

    /// A row that is not full must be centred on the slot axis.
    #[test]
    fn a_lone_trailing_conductor_sits_on_the_slot_axis() {
        // 1, 3 and 5 all end with a single conductor in the last row.
        for count in [1_usize, 3, 5] {
            let packing = SlotPacking::new(count);
            assert!(
                packing.column_offset(count - 1).abs() < 1e-6,
                "layers={count}: last conductor is off-axis"
            );
        }
        // An even count fills its last row, so the two straddle the axis.
        let packing = SlotPacking::new(4);
        assert!((packing.column_offset(2) + packing.column_offset(3)).abs() < 1e-6);
    }

    /// The described packing: two per row, filling from the slot bottom.
    #[test]
    fn packing_is_two_per_row() {
        let cases = [(1_usize, 1_usize, 1_usize), (2, 1, 2), (4, 2, 2), (6, 3, 2)];
        for (count, rows, cols) in cases {
            let packing = SlotPacking::new(count);
            assert_eq!(packing.rows, rows, "layers={count}");
            assert_eq!(packing.cols, cols, "layers={count}");
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

    /// Regression: the coil-start filter was inherited from the single-layer
    /// model, which drops half the coils of a double-layer winding. With full
    /// pitch the gap is invisible (slot `i+pitch` is always an `Out` slot, so
    /// every slot still gets touched); short pitching exposed it.
    #[test]
    fn every_slot_starts_a_coil_in_a_double_layer_winding() {
        for short_pitched in [false, true] {
            for count in [2_usize, 4, 6] {
                let mut cfg = config(12, 3, 1, count);
                cfg.short_pitched = short_pitched;
                let assignments = compute_winding(&cfg);
                let conductors = compute_conductors(&cfg, &assignments);

                let deep = SlotLayout::new(count, TAU / 12.0, 2.0, 2.6).deep_count();
                let starting: Vec<_> = conductors
                    .iter()
                    .filter(|c| starts_coil(c, cfg.layers))
                    .collect();

                // One coil per slot, carried by every conductor of the deep half.
                assert_eq!(
                    starting.len(),
                    cfg.groove_count * deep,
                    "layers={count} short_pitched={short_pitched}"
                );

                for slot in 0..cfg.groove_count {
                    let from_slot = starting.iter().filter(|c| c.slot == slot).count();
                    assert_eq!(
                        from_slot, deep,
                        "slot {slot} starts {from_slot} coils, expected {deep} \
                         (layers={count}, short_pitched={short_pitched})"
                    );
                }
            }
        }
    }

    /// A single-layer winding has half as many coils: only the outgoing sides
    /// start one, since the return side is the far end of another coil.
    #[test]
    fn single_layer_starts_half_as_many_coils() {
        let cfg = config(12, 3, 1, 1);
        let assignments = compute_winding(&cfg);
        let conductors = compute_conductors(&cfg, &assignments);

        let starting = conductors
            .iter()
            .filter(|c| starts_coil(c, cfg.layers))
            .count();
        assert_eq!(starting, cfg.groove_count / 2);
    }

    fn winding_data<'a>(config: &'a MotorConfig, conductors: &'a [Conductor]) -> WindingData<'a> {
        let segment_angle = TAU / config.groove_count as f32;
        WindingData {
            config,
            conductors,
            layout: SlotLayout::new(
                config.layers,
                segment_angle,
                STATOR_BORE_RADIUS,
                slot_bottom_radius(),
            ),
            segment_angle,
            tooth_angle: segment_angle * 0.5,
            half_h: STATOR_HEIGHT / 2.0,
            pitch: coil_pitch(config),
        }
    }

    /// The endwinding tube and the slot conductor must butt together into one
    /// continuous wire: the conductor ends flush with the core face and the
    /// tube's straight lead starts there, on the same axis and gauge.
    #[test]
    fn endwindings_meet_the_conductors_at_the_core_face() {
        for count in [1_usize, 2, 4, 6] {
            let mut cfg = config(24, 3, 2, count);
            cfg.show_endwindings = true;
            let assignments = compute_winding(&cfg);
            let conductors = compute_conductors(&cfg, &assignments);
            let data = winding_data(&cfg, &conductors);

            let conductor_tip = data.conductor_height() / 2.0;
            let lead_bottom = data.endwinding_y() - data.endwinding_lead();

            assert!(
                (conductor_tip - lead_bottom).abs() < 1e-6,
                "layers={count}: conductor ends at {conductor_tip}, tube starts at {lead_bottom}"
            );
            assert!(
                (conductor_tip - data.half_h).abs() < 1e-6,
                "layers={count}: the joint should sit on the core face"
            );
        }
    }

    /// The arc sweeps tangentially, so it has to be clear of the core face
    /// before it starts turning or it would cut straight through the teeth.
    #[test]
    fn the_arc_starts_above_the_core_face() {
        for n in [12_usize, 24, 144] {
            for count in [1_usize, 2, 4, 6] {
                let mut cfg = config(n, 3, 2, count);
                cfg.show_endwindings = true;
                let assignments = compute_winding(&cfg);
                let conductors = compute_conductors(&cfg, &assignments);
                let data = winding_data(&cfg, &conductors);

                // Underside of the tube where the arc begins, worst case: the
                // tube is still horizontal there.
                let underside = data.endwinding_y() - data.layout.wire_radius;
                assert!(
                    underside > data.half_h,
                    "n={n} layers={count}: the arc grazes the core at {underside}"
                );
            }
        }
    }

    /// Hiding the endwindings tucks the wire back inside the core, so the
    /// current-direction symbols on its end faces stay readable.
    #[test]
    fn hidden_endwindings_keep_the_conductor_inside_the_core() {
        let mut cfg = config(24, 3, 2, 4);
        cfg.show_endwindings = false;
        let assignments = compute_winding(&cfg);
        let conductors = compute_conductors(&cfg, &assignments);
        let data = winding_data(&cfg, &conductors);

        assert!(data.conductor_height() / 2.0 < data.half_h);
    }

    /// Chording is only possible when a slot holds two coil sides that can
    /// belong to different phases, so a single-layer winding cannot be
    /// short-pitched: its pitch factor is always 1. Asking for it must be a
    /// no-op, not a set of coils shorting two phases together.
    #[test]
    fn a_single_layer_winding_cannot_be_short_pitched() {
        for n in [12_usize, 24, 36] {
            let mut full = config(n, 3, 2, 1);
            full.short_pitched = false;
            let mut chorded = full.clone();
            chorded.short_pitched = true;

            assert!(!can_short_pitch(&chorded));
            assert_eq!(
                coil_pitch(&chorded),
                coil_pitch(&full),
                "n={n}: chording changed the pitch of a single-layer winding"
            );

            // The slot occupancy is what produces the MMF; it must be identical.
            let a = compute_conductors(&full, &compute_winding(&full));
            let b = compute_conductors(&chorded, &compute_winding(&chorded));
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(b.iter()) {
                assert_eq!((x.slot, x.phase, x.direction), (y.slot, y.phase, y.direction));
            }
        }
    }

    /// With two layers or more it must still do something, or the guard above
    /// would be hiding a broken feature rather than an impossible one.
    #[test]
    fn two_layers_can_still_be_short_pitched() {
        let mut cfg = config(24, 3, 2, 2);
        cfg.short_pitched = true;
        assert!(can_short_pitch(&cfg));

        let mut full = cfg.clone();
        full.short_pitched = false;
        assert_ne!(coil_pitch(&cfg), coil_pitch(&full));
    }

    /// A coil is a loop of one phase: it must return into a slot carrying that
    /// same phase with the opposite polarity. Anything else is not a coil.
    #[test]
    fn every_endwinding_returns_into_its_own_phase() {
        for short_pitched in [false, true] {
            for count in [1_usize, 2, 4, 6] {
                let mut cfg = config(24, 3, 2, count);
                cfg.short_pitched = short_pitched;
                let assignments = compute_winding(&cfg);
                let conductors = compute_conductors(&cfg, &assignments);
                let pitch = coil_pitch(&cfg);
                let n = cfg.groove_count;

                for start in conductors.iter().filter(|c| starts_coil(c, cfg.layers)) {
                    let return_slot = (start.slot + pitch) % n;
                    let partner = SlotLayout::new(count, TAU / n as f32, 2.0, 2.6)
                        .coil_partner(start.index);
                    let far_side = conductors
                        .iter()
                        .find(|c| c.slot == return_slot && c.index == partner)
                        .expect("the endwinding must land on a real conductor");

                    assert_eq!(
                        far_side.phase, start.phase,
                        "layers={count} short_pitched={short_pitched}: slot {} phase {} \
                         wires to slot {return_slot} phase {}",
                        start.slot, start.phase, far_side.phase
                    );
                    assert_eq!(
                        far_side.direction,
                        start.direction.reversed(),
                        "layers={count} short_pitched={short_pitched}: slot {} does not \
                         close a loop with slot {return_slot}",
                        start.slot
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
