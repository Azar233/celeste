mod components;
mod constants;
mod scene;
mod systems;
mod utils;

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;
use bevy::window::{PresentMode, WindowPlugin};
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};

use components::{HairMaterial, WeatherMaterial};
use scene::ScenePlugin;
use systems::GameplayPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(FramepacePlugin)
        .insert_resource(FramepaceSettings {
            limiter: Limiter::from_framerate(60.0),
        })
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_plugins((
            Material2dPlugin::<HairMaterial>::default(),
            Material2dPlugin::<WeatherMaterial>::default(),
        ))
        .add_plugins((ScenePlugin, GameplayPlugin))
        .run();
}