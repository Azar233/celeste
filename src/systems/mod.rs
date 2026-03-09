pub mod animation;
pub mod effects;
pub mod hair;
pub mod player;
pub mod weather;

use bevy::prelude::*;

pub use animation::animate_sprite;
pub use effects::{emit_dash_trail, update_dash_trail};
pub use hair::update_hair;
pub use player::{apply_physics, player_input, player_movement, tick_timers, update_crouch_state};
pub use weather::update_weather_material;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				tick_timers,
				player_input,
				update_crouch_state,
				apply_physics,
				player_movement,
				emit_dash_trail,
				update_dash_trail,
				animate_sprite,
				update_hair,
				update_weather_material,
			)
				.chain(),
		);
	}
}