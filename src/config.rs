use bevy::prelude::*;

/// Motor winding configuration — user-adjustable parameters.
#[derive(Resource, Clone, Debug)]
pub struct MotorConfig {
    pub groove_count: usize,
    pub phases: usize,
    pub short_pitched: bool,
    pub layers: usize,
    pub pole_pairs: usize,
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
        }
    }
}

// Geometry constants
pub const STATOR_OUTER_RADIUS: f32 = 3.0;
pub const STATOR_BORE_RADIUS: f32 = 2.0;
pub const SLOT_DEPTH: f32 = 0.6;
pub const STATOR_HEIGHT: f32 = 2.0;

pub fn slot_bottom_radius() -> f32 {
    STATOR_BORE_RADIUS + SLOT_DEPTH
}

/// Phase colors for winding visualization.
pub fn phase_color(phase: usize) -> Color {
    match phase % 6 {
        0 => Color::srgb(0.902, 0.224, 0.275), // Red
        1 => Color::srgb(0.165, 0.616, 0.561), // Teal
        2 => Color::srgb(0.271, 0.482, 0.616), // Blue
        3 => Color::srgb(0.957, 0.635, 0.380), // Orange
        4 => Color::srgb(0.416, 0.024, 0.447), // Purple
        _ => Color::srgb(0.914, 0.769, 0.416), // Yellow
    }
}
