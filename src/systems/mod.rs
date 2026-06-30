pub mod animation;
pub mod effects;
pub mod hair;
pub mod level;
pub mod player;
pub mod weather;

use bevy::prelude::*;

use crate::app_state::GameState;
use crate::components::FreezeFrameState;
use crate::editor::editor_inactive;
use crate::menu::MenuOpen;

pub use animation::animate_sprite;
pub use effects::{emit_dash_trail, update_dash_trail};
pub use hair::update_hair;
pub use level::{
    CompletionState, DeathSequence, RoomTransitionCooldown, camera_follow, completion_inactive,
    death_sequence_inactive, handle_completion_overlay_input, handle_completion_zones,
    handle_hazard_respawn, handle_room_transitions, reset_completion_state, reset_death_sequence,
    update_checkpoint_respawn, update_dash_crystals, update_death_sequence, update_springs,
};
pub use player::{
    apply_physics, cache_player_input, player_input, player_movement, tick_timers,
    trigger_fall_death, update_crouch_state, update_player_state_machine,
};
pub use weather::update_weather_material;

fn gameplay_active(freeze_frames: Res<FreezeFrameState>, menu_state: Res<MenuOpen>) -> bool {
    freeze_frames.timer <= 0.0 && !menu_state.0
}

fn gameplay_active_outside_editor(
    freeze_frames: Res<FreezeFrameState>,
    menu_state: Res<MenuOpen>,
    editor: Option<Res<crate::editor::EditorState>>,
) -> bool {
    gameplay_active(freeze_frames, menu_state) && editor_inactive(editor)
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
            .insert_resource(RoomTransitionCooldown::default())
            .insert_resource(CompletionState::default())
            .insert_resource(DeathSequence::default())
            .add_systems(
                OnEnter(GameState::InGame),
                (reset_completion_state, reset_death_sequence),
            )
            .add_systems(
                Update,
                tick_freeze_frames.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                cache_player_input
                    .run_if(in_state(GameState::InGame))
                    .run_if(gameplay_active_outside_editor),
            )
            .add_systems(
                FixedUpdate,
                (
                    tick_timers,
                    update_player_state_machine,
                    player_input,
                    update_crouch_state,
                    apply_physics,
                    player_movement,
                    trigger_fall_death,
                    update_dash_crystals,
                    update_springs,
                    handle_room_transitions,
                    handle_hazard_respawn,
                    update_checkpoint_respawn,
                    handle_completion_zones,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame))
                    .run_if(gameplay_active_outside_editor)
                    .run_if(completion_inactive)
                    .run_if(death_sequence_inactive),
            )
            .add_systems(
                Update,
                update_death_sequence
                    .run_if(in_state(GameState::InGame))
                    .run_if(completion_inactive),
            )
            .add_systems(
                Update,
                handle_completion_overlay_input.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                (
                    emit_dash_trail,
                    update_dash_trail,
                    animate_sprite,
                    camera_follow,
                    update_weather_material,
                )
                    .chain()
                    .run_if(in_state(GameState::InGame))
                    .run_if(gameplay_active)
                    .run_if(completion_inactive)
                    .run_if(death_sequence_inactive),
            )
            .add_systems(
                Update,
                update_hair
                    .run_if(in_state(GameState::InGame))
                    .run_if(gameplay_active)
                    .run_if(completion_inactive)
                    .after(update_death_sequence),
            );
    }
}
