use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

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
pub fn orbit_camera(
    mut query: Query<(&mut Transform, &mut OrbitCamera)>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut scroll: MessageReader<MouseWheel>,
) {
    let Ok((mut transform, mut orbit)) = query.single_mut() else {
        return;
    };

    // Rotate on right-click drag (left-click is reserved for UI)
    if mouse_button.pressed(MouseButton::Right) {
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
