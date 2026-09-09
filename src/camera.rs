use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;

/// Orbit camera controller component.
#[derive(Component)]
pub struct OrbitCamera {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            distance: 10.0,
            yaw: 0.8,
            pitch: 0.5,
        }
    }
}

/// System: orbit the camera around the origin using mouse drag + scroll.
///
/// Input is handed to the panels first, through [`crate::ui::PointerOverPanel`]
/// rather than `EguiWantsInput`. That is not a preference: a guard reading the
/// resource left the camera zooming straight through the config panel, and the
/// same question answered from `PanelSpace` fixed it. See that type for what
/// was and was not established about why.
///
/// The events are drained rather than left unread. Skipping the system with a
/// run condition would let them queue up while the pointer sat over a panel and
/// then apply all at once the moment it left, snapping the camera.
pub fn orbit_camera(
    mut query: Query<(&mut Transform, &mut OrbitCamera)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut scroll: MessageReader<MouseWheel>,
    egui_input: Res<EguiWantsInput>,
    over_panel: Res<crate::ui::PointerOverPanel>,
) {
    // Two checks, because neither covers the other. `PointerOverPanel` is the
    // one that catches the docked panels; `EguiWantsInput` covers what floats
    // above them — the minimized button, tooltips, popups — and a drag that is
    // already in progress.
    if over_panel.0 || egui_input.wants_pointer_input() {
        mouse_motion.clear();
        scroll.clear();
        return;
    }

    let Ok((mut transform, mut orbit)) = query.single_mut() else {
        return;
    };

    if mouse_button.pressed(MouseButton::Left) {
        for event in mouse_motion.read() {
            orbit.yaw -= event.delta.x * 0.005;
            orbit.pitch -= event.delta.y * 0.005;
            orbit.pitch = orbit.pitch.clamp(-1.4, 1.4);
        }
    } else {
        // Drain unread events
        mouse_motion.clear();
    }

    for event in scroll.read() {
        let delta = match event.unit {
            MouseScrollUnit::Line => event.y * 0.5,
            MouseScrollUnit::Pixel => event.y * 0.01,
        };
        orbit.distance -= delta;
        orbit.distance = orbit.distance.clamp(3.0, 25.0);
    }

    let eye = Vec3::new(
        orbit.distance * orbit.pitch.cos() * orbit.yaw.sin(),
        orbit.distance * orbit.pitch.sin(),
        orbit.distance * orbit.pitch.cos() * orbit.yaw.cos(),
    );

    *transform = Transform::from_translation(eye).looking_at(Vec3::ZERO, Dir3::Y);
}
