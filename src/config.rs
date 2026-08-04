use bevy::prelude::*;

/// Upper bound on the number of phases the simulator supports.
///
/// This is both the length of [`MmfFieldConfig::phases_to_show`] and the
/// `phases` field of [`MotorConfig::MAX`], so the two can never drift apart.
pub const MAX_PHASES: usize = 10;

#[derive(Resource, Clone, Debug)]
pub struct MmfFieldConfig {
    pub show: bool,
    pub phases_to_show: [bool; MAX_PHASES],
    /// Whether to render the resultant (sum of all phases) MMF field.
    pub show_result: bool,
    /// Controls the sharpness of the gradient falloff:
    /// 1.0 = linear fade, higher values produce a more peaked/concentrated field.
    pub gradient_intensity: f32,
}

impl Default for MmfFieldConfig {
    fn default() -> Self {
        Self {
            show: false,
            phases_to_show: [false; MAX_PHASES],
            show_result: false,
            gradient_intensity: 2.0,
        }
    }
}

impl MmfFieldConfig {
    /// Whether `phase` is currently selected for rendering.
    ///
    /// Indices beyond [`MAX_PHASES`] report as hidden instead of panicking, so
    /// a stale phase index from a previous configuration can never crash a
    /// render system.
    #[inline]
    pub fn shows_phase(&self, phase: usize) -> bool {
        self.phases_to_show.get(phase).copied().unwrap_or(false)
    }

    pub const MIN: Self = Self {
        show: false,
        phases_to_show: [false; MAX_PHASES],
        show_result: false,
        gradient_intensity: 0.5,
    };

    pub const MAX: Self = Self {
        show: true,
        phases_to_show: [true; MAX_PHASES],
        show_result: true,
        gradient_intensity: 8.0,
    };
}

/// Motor winding configuration — user-adjustable parameters.
#[derive(Resource, Clone, Debug)]
pub struct MotorConfig {
    pub groove_count: usize,
    pub phases: usize,
    pub short_pitched: bool,
    pub layers: usize,
    pub pole_pairs: usize,
    pub show_endwindings: bool,
    pub show_vectors: bool,
    pub show_fields: bool,
    pub show_rotor: bool,
    pub show_winding_scheme: bool,
    pub mmf_field: MmfFieldConfig,
}

impl MotorConfig {
    pub const MIN: Self = Self {
        groove_count: 6,
        phases: 2,
        short_pitched: false,
        layers: 1,
        pole_pairs: 1,
        show_endwindings: false,
        show_vectors: false,
        show_fields: false,
        show_rotor: false,
        show_winding_scheme: false,
        mmf_field: MmfFieldConfig::MIN,
    };

    pub const MAX: Self = Self {
        groove_count: 144,
        phases: MAX_PHASES,
        short_pitched: true,
        layers: 2,
        pole_pairs: 6,
        show_endwindings: true,
        show_vectors: true,
        show_fields: true,
        show_rotor: true,
        show_winding_scheme: true,
        mmf_field: MmfFieldConfig::MAX,
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
            show_vectors: true,
            show_fields: true,
            show_rotor: true,
            show_winding_scheme: true,
            mmf_field: MmfFieldConfig::default(),
        }
    }
}

// Geometry constants
pub const STATOR_OUTER_RADIUS: f32 = 3.0;
pub const STATOR_BORE_RADIUS: f32 = 2.0;
pub const ROTOR_RADIUS: f32 = 1.95;
pub const SLOT_DEPTH: f32 = 0.6;
pub const STATOR_HEIGHT: f32 = 2.0;

#[inline]
pub const fn slot_bottom_radius() -> f32 {
    STATOR_BORE_RADIUS + SLOT_DEPTH
}
