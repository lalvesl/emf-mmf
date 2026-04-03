mod camera;
mod colors;
mod config;
mod eletrical;
mod i18n;
mod stator;
mod setup;
mod ui;
mod vectors;
mod winding;

use bevy::prelude::*;

fn main() {
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "EMF-MMF — Stator Winding Simulator".into(),
            #[cfg(feature = "web")]
            fit_canvas_to_parent: true,
            #[cfg(feature = "web")]
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    });

    #[cfg(feature = "web")]
    let plugins = plugins.set(TaskPoolPlugin {
        task_pool_options: TaskPoolOptions::with_num_threads(1),
    });

    App::new()
        .add_plugins(plugins)
        .add_plugins(ui::UiPlugin)
        .add_plugins(eletrical::EletricalPlugin)
        .add_plugins(vectors::MmfVectorsPlugin)
        .init_resource::<config::MotorConfig>()
        .add_message::<config::MotorConfigChanged>()
        .add_systems(Startup, setup::setup)
        .add_systems(
            Update,
            (
                camera::orbit_camera,
                stator::regenerate_stator,
                winding::regenerate_winding,
            ),
        )
        .run();
}

}

