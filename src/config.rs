use bevy::prelude::*;

/// Upper bound on the number of phases the simulator supports.
///
/// This is the length of [`MmfFieldConfig::phases_to_show`], the `phases` field
/// of [`MotorConfig::MAX`] and the size of the phase palette, so none of the
/// three can drift apart. The palette is what fixes the value: each phase gets
/// a hand-picked colour that has to stay clear of the two the field overlay
/// reserves for magnetic polarity, and eight is as far as that stretches.
pub const MAX_PHASES: usize = 6;

#[derive(Resource, Clone, Debug, PartialEq)]
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
        gradient_intensity: 2.0,
    };

    pub const MAX: Self = Self {
        show: true,
        phases_to_show: [true; MAX_PHASES],
        show_result: true,
        gradient_intensity: 20.0,
    };
}

/// What the machine *is* — the parameters that decide its shape.
///
/// Kept apart from [`ViewConfig`] so Bevy's change detection can tell the two
/// apart on its own. Rebuilding the stator core costs one mesh per tooth — up
/// to 144 — and nothing about it depends on what is currently shown, so the
/// stator watches this resource alone.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct MotorConfig {
    pub groove_count: usize,
    pub phases: usize,
    pub short_pitched: bool,
    /// Conductors packed into each slot. Split into two electrical layers:
    /// the deep half starts a coil, the shallow half receives the return side
    /// of the coil that started `coil_pitch` slots earlier.
    pub layers: usize,
    pub pole_pairs: usize,
}

impl MotorConfig {
    pub const MIN: Self = Self {
        groove_count: 6,
        phases: 2,
        short_pitched: false,
        layers: 1,
        pole_pairs: 1,
    };

    pub const MAX: Self = Self {
        groove_count: 144,
        phases: MAX_PHASES,
        short_pitched: true,
        layers: 6,
        pole_pairs: 6,
    };
}

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

/// What is drawn. Never changes the shape of the machine, only what of it is
/// visible, so the stator is free to ignore it.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ViewConfig {
    pub show_endwindings: bool,
    pub show_vectors: bool,
    pub show_rotor: bool,
    pub show_winding_scheme: bool,
    pub mmf_field: MmfFieldConfig,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            show_endwindings: true,
            show_vectors: true,
            show_rotor: true,
            show_winding_scheme: false,
            mmf_field: MmfFieldConfig::default(),
        }
    }
}

/// Run condition for everything that is rebuilt from the machine *or* from what
/// is currently shown.
///
/// Bevy counts a resource as changed on the frame it is inserted, so this also
/// fires the first build — nothing has to seed it by hand.
pub fn scene_changed(config: Res<MotorConfig>, view: Res<ViewConfig>) -> bool {
    config.is_changed() || view.is_changed()
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
