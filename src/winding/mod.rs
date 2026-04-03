use bevy::prelude::*;
use std::f32::consts::TAU;

use crate::colors::*;
use crate::config::*;

pub mod current;
pub mod header_coils;
pub mod ui;

/// Direction of current flow in a slot conductor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

/// Assignment of a conductor to a slot.
#[derive(Clone, Debug)]
pub struct SlotAssignment {
    pub phase: usize,
    pub direction: Direction,
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
    pub assignments: &'a [Option<SlotAssignment>],
    pub segment_angle: f32,
    pub tooth_angle: f32,
    pub half_h: f32,
    pub r_bore: f32,
    pub r_slot_bot: f32,
    pub pitch: usize,
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
    let n = config.groove_count;
    let segment_angle = TAU / n as f32;
    let tooth_angle = segment_angle * 0.5;
    let half_h = STATOR_HEIGHT / 2.0;
    let r_bore = STATOR_BORE_RADIUS;
    let r_slot_bot = slot_bottom_radius();
    let pitch = coil_pitch(&config);

    let data = WindingData {
        config: &config,
        assignments: &assignments,
        segment_angle,
        tooth_angle,
        half_h,
        r_bore,
        r_slot_bot,
        pitch,
    };

    // Pre-create phase materials
    let phase_mats: Vec<_> = (0..config.phases)
        .map(|p| {
            let color = phase_color(p);
            materials.add(StandardMaterial {
                base_color: color,
                emissive: color.into(),
                ..default()
            })
        })
        .collect();

    let phase_mats_opp: Vec<_> = (0..config.phases)
        .map(|p| {
            let color = phase_color_opposite(p);
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
