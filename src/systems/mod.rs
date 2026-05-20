pub mod animation;
pub mod effects;
pub mod hair;
pub mod level;
pub mod player;
pub mod weather;

use bevy::prelude::*;

use crate::components::FreezeFrameState;
use crate::menu::MenuOpen;

pub use animation::animate_sprite;
pub use effects::{emit_dash_trail, update_dash_trail};
pub use hair::update_hair;
pub use level::{camera_follow, handle_hazard_respawn, handle_room_transitions, update_checkpoint_respawn};
pub use player::{
    apply_physics, cache_player_input, player_input, player_movement, tick_timers,
    update_crouch_state, update_player_state_machine,
};
pub use weather::update_weather_material;

fn gameplay_active(freeze_frames: Res<FreezeFrameState>, menu_state: Res<MenuOpen>) -> bool {
    freeze_frames.timer <= 0.0 && !menu_state.0
}

fn tick_freeze_frames(time: Res<Time<Real>>, mut freeze_frames: ResMut<FreezeFrameState>) {
    if freeze_frames.timer > 0.0 {
        freeze_frames.timer = (freeze_frames.timer - time.delta_secs()).max(0.0);
    }
}

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FreezeFrameState::default())
            .add_systems(Update, tick_freeze_frames)
            .add_systems(Update, cache_player_input.run_if(gameplay_active))
            .add_systems(
                FixedUpdate,
                (
                    tick_timers,
                    update_player_state_machine,
                    player_input,
                    update_crouch_state,
                    apply_physics,
                    player_movement,
                    handle_room_transitions,
                    handle_hazard_respawn,
                    update_checkpoint_respawn,
                )
                    .chain()
                    .run_if(gameplay_active),
            )
            .add_systems(
            Update,
            (
                emit_dash_trail,
                update_dash_trail,
                animate_sprite,
                update_hair,
                camera_follow,
                update_weather_material,
            )
                .chain()
                .run_if(gameplay_active),
        );
    }
}
