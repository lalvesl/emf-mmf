use bevy::prelude::*;

/// Motor winding configuration — user-adjustable parameters.
#[derive(Resource, Clone, Debug)]
pub struct MotorConfig {
    pub groove_count: usize,
    pub phases: usize,
    pub short_pitched: bool,
    pub layers: usize,
    pub pole_pairs: usize,
    pub show_endwindings: bool,
}

impl MotorConfig {
    pub const MIN: MotorConfig = MotorConfig {
        groove_count: 6,
        phases: 1,
        short_pitched: false,
        layers: 1,
        pole_pairs: 1,
        show_endwindings: false,
    };

    pub const MAX: MotorConfig = MotorConfig {
        groove_count: 144,
        phases: 6,
        short_pitched: true,
        layers: 2,
        pole_pairs: 6,
        show_endwindings: true,
    };
}

/// Event triggered when motor configuration changes.
#[derive(Message)]
pub struct MotorConfigChanged;

impl Default for MotorConfig {
    fn default() -> Self {
        Self {
            groove_count: 24,
            phases: 3,
            short_pitched: false,
            layers: 1,
            pole_pairs: 1,
            show_endwindings: true,
        }
    }
}

// Geometry constants
pub const STATOR_OUTER_RADIUS: f32 = 3.0;
pub const STATOR_BORE_RADIUS: f32 = 2.0;
pub const SLOT_DEPTH: f32 = 0.6;
pub const STATOR_HEIGHT: f32 = 2.0;

#[inline]
pub const fn slot_bottom_radius() -> f32 {
    STATOR_BORE_RADIUS + SLOT_DEPTH
}
