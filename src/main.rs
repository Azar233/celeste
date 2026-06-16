mod app_state;
mod components;
mod constants;
mod editor;
mod level;
mod main_menu;
mod menu;
mod scene;
mod systems;
mod utils;

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;
use bevy::window::{PresentMode, WindowPlugin};
use bevy_framepace::{FramepacePlugin, FramepaceSettings, Limiter};

use app_state::{GameState, PendingMapPath};
use components::{HairMaterial, WeatherMaterial};
use editor::EditorPlugin;
use main_menu::MainMenuPlugin;
use menu::MenuPlugin;
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
        .init_state::<GameState>()
        .init_resource::<PendingMapPath>()
        .add_plugins((
            MainMenuPlugin,
            ScenePlugin,
            GameplayPlugin,
            MenuPlugin,
            EditorPlugin,
        ))
        .run();
}
