use bevy::prelude::*;

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

pub fn phase_color_opposite(phase: usize) -> Color {
    let color = phase_color(phase);
    let hsla: bevy::color::Hsla = color.into();
    Color::from(bevy::color::Hsla::new(
        (hsla.hue + 180.0) % 360.0,
        hsla.saturation,
        hsla.lightness,
        hsla.alpha,
    ))
}
