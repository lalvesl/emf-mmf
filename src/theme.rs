//! Shadcn theme and fonts for the egui layer.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use egui_sc::egui_components::{ShadcnTheme, register_font};

use crate::ui::PanelLayout;

/// Primary accent hue, or `None` for the neutral zinc default.
///
/// Left neutral on purpose. Phase colours and the red/blue of magnetic polarity
/// are the only things in this application whose colour carries meaning, and a
/// tinted accent on every slider and switch would compete with them for it.
const PRIMARY_HUE: Option<f32> = None;

/// The theme every panel reads from.
///
/// Held as a resource rather than rebuilt per frame so a future light/dark or
/// hue switch is a single write here.
#[derive(Resource, Clone, Deref)]
pub struct AppTheme(pub ShadcnTheme);

impl Default for AppTheme {
    fn default() -> Self {
        Self(ShadcnTheme::build(true, PRIMARY_HUE))
    }
}

/// Whether the icon font is bound and the UI is safe to paint.
///
/// `Context::set_fonts` only takes effect at the start of the *next* pass, so
/// for one pass after registration `FontFamily::Name("MaterialIcons")` still
/// resolves to nothing — and painting a single icon against a missing family
/// panics inside epaint. Everything that draws components waits on this.
#[derive(Resource, Default)]
pub struct FontsReady(bool);

/// Run condition for any system that paints egui components.
pub fn fonts_ready(ready: Res<FontsReady>) -> bool {
    ready.0
}

pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AppTheme>()
            .init_resource::<FontsReady>()
            .add_systems(Startup, match_clear_color_to_theme)
            .add_systems(
                EguiPrimaryContextPass,
                apply_theme.in_set(PanelLayout::Theme),
            );
    }
}

/// Paint the 3D viewport in the theme's background so the docked panels read as
/// part of the same surface rather than floating over a different one.
fn match_clear_color_to_theme(mut commands: Commands, theme: Res<AppTheme>) {
    let [r, g, b, _] = theme.background.to_array();
    commands.insert_resource(ClearColor(Color::srgb_u8(r, g, b)));
}

/// Publish the theme and register the icon font.
///
/// Both belong in the per-frame pass, not in `Startup`: the egui context does
/// not exist yet at startup, and `ShadcnTheme::get` reads from context memory,
/// which every component calls on every paint. The font is a one-shot — it
/// replaces the whole `FontDefinitions`, and doing that repeatedly would
/// rebuild the atlas every frame.
fn apply_theme(
    mut contexts: EguiContexts,
    theme: Res<AppTheme>,
    mut ready: ResMut<FontsReady>,
    mut font_registered: Local<bool>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    ShadcnTheme::set(ctx, theme.0.clone());
    theme.apply(ctx);

    if !*font_registered {
        register_font(ctx);
        *font_registered = true;
        // The new definitions are picked up when the next pass begins, so ask
        // for that pass rather than waiting on the next input event.
        ctx.request_repaint();
        return;
    }

    ready.0 = true;
}
