//! Shadcn theme and fonts for the egui layer.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use egui_sc::egui_components::{ShadcnTheme, register_font};

use crate::ui::PanelLayout;

/// Primary accent hue, or `None` for the neutral zinc default.
///
/// 217° is the library's own blue preset, so the panel matches the component
/// demo. It is close to the 240° the field overlay paints magnetic south with,
/// but the two never share a surface: this tints panel chrome only — sliders,
/// switches, the pressed state of a toggle — while polarity is painted in the
/// 3D viewport and the winding diagram.
const PRIMARY_HUE: Option<f32> = Some(217.0);

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

// ─── Plot surfaces ────────────────────────────────────────────────────────────
//
// The 2D plots sit on `ShadcnTheme::muted` rather than the window background,
// so they read as their own surface. `border` is unusable on them: in the dark
// theme it is *the very same colour* as `muted`, so a line drawn in it there is
// invisible — which is how the electrical plot lost its zero line. These are
// stepped alphas of `muted_foreground` instead, still theme-derived and ordered
// by how much attention each mark deserves.

/// Corner radius of a plot surface.
pub fn plot_corner(theme: &ShadcnTheme) -> egui::CornerRadius {
    egui::CornerRadius::same(theme.radius as u8)
}

/// The frame drawn around a plot area.
pub fn outline_color(theme: &ShadcnTheme) -> egui::Color32 {
    ShadcnTheme::with_alpha(theme.muted_foreground, 60)
}

/// The zero line and other axes a reader should notice but not read past.
pub fn axis_color(theme: &ShadcnTheme) -> egui::Color32 {
    ShadcnTheme::with_alpha(theme.muted_foreground, 90)
}

/// Reference gridlines — present, but never competing with the data.
pub fn grid_color(theme: &ShadcnTheme) -> egui::Color32 {
    ShadcnTheme::with_alpha(theme.muted_foreground, 40)
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
