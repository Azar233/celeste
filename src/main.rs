mod components;
mod constants;
mod scene;
mod systems;
mod utils;

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;

use components::HairMaterial;
use scene::ScenePlugin;
use systems::GameplayPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(Material2dPlugin::<HairMaterial>::default())
        .add_plugins((ScenePlugin, GameplayPlugin))
        .run();
}