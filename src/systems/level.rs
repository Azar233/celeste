use bevy::prelude::*;

use crate::components::{
    CheckpointMarker, ClimbTopOutState, ColliderSize, Crouching, DashCrystal, DashSlideState,
    DashState, Facing, Grounded, Hair, Hazard, LevelEntity, Player, PlayerState,
    PlayerStateMachine, RoomExitMarker, Velocity, WallContact, WeatherOverlay,
};
use crate::constants::{
    DASH_CRYSTAL_ANIMATION_INTERVAL, DASH_CRYSTAL_RESPAWN_TIME, PLAYER_COLLIDER_SIZE,
    PLAYER_RENDER_Z,
};
use crate::level::{ActiveRoom, LoadedMap};
use crate::scene::{LevelArt, spawn_room_geometry};
use crate::utils::{check_collision, initial_hair_positions};

fn reset_player_to_spawn(
    player_transform: &mut Transform,
    velocity: &mut Velocity,
    grounded: &mut Grounded,
    wall_contact: &mut WallContact,
    collider_size: &mut ColliderSize,
    crouching: &mut Crouching,
    dash_state: &mut DashState,
    dash_slide: &mut DashSlideState,
    state_machine: &mut PlayerStateMachine,
    climb_top_out: &mut ClimbTopOutState,
    hair: &mut Hair,
    facing: &Facing,
    spawn_point: Vec2,
    preserve_momentum: bool,
) {
    player_transform.translation = Vec3::new(spawn_point.x, spawn_point.y, PLAYER_RENDER_Z);
    if !preserve_momentum {
        velocity.0 = Vec2::ZERO;
    }
    grounded.0 = false;
    *wall_contact = WallContact::None;
    crouching.0 = false;
    dash_state.is_dashing = false;
    dash_state.timer = 0.0;
    dash_slide.timer = 0.0;
    dash_slide.direction = 0.0;
    climb_top_out.active = false;
    climb_top_out.timer = 0.0;
    climb_top_out.duration = 0.0;
    climb_top_out.start = player_transform.translation;
    climb_top_out.target = player_transform.translation;
    collider_size.0 = PLAYER_COLLIDER_SIZE;
    state_machine.current = PlayerState::Normal;
    state_machine.previous = PlayerState::Normal;
    hair.sim_positions = initial_hair_positions(spawn_point, facing.0);
}

pub fn update_checkpoint_respawn(
    mut active_room: ResMut<ActiveRoom>,
    player_query: Query<(&Transform, &ColliderSize), With<Player>>,
    checkpoints: Query<(&Transform, &Sprite, &CheckpointMarker), Without<Player>>,
) {
    let Ok((player_transform, player_size)) = player_query.get_single() else {
        return;
    };

    for (checkpoint_transform, checkpoint_sprite, checkpoint) in &checkpoints {
        let Some(checkpoint_size) = checkpoint_sprite.custom_size else {
            continue;
        };

        if check_collision(
            player_transform.translation,
            player_size.0,
            checkpoint_transform.translation,
            checkpoint_size,
        ) {
            let _checkpoint_id = &checkpoint.id;
            active_room.respawn_point = checkpoint_transform.translation.truncate();
        }
    }
}

pub fn update_dash_crystals(
    time: Res<Time>,
    mut crystals: Query<(&Transform, &mut Sprite, &mut DashCrystal), Without<Player>>,
    mut player_query: Query<(&Transform, &ColliderSize, &mut DashState), With<Player>>,
) {
    let dt = time.delta_secs();
    let Ok((player_transform, player_size, mut dash_state)) = player_query.get_single_mut() else {
        return;
    };

    for (crystal_transform, mut sprite, mut crystal) in &mut crystals {
        if crystal.respawn_timer > 0.0 {
            crystal.respawn_timer = (crystal.respawn_timer - dt).max(0.0);
            if crystal.respawn_timer <= 0.0 && !crystal.active_frames.is_empty() {
                crystal.frame_index = 0;
                crystal.animation_timer = 0.0;
                sprite.image = crystal.active_frames[0].clone();
            }
            continue;
        }

        if !crystal.active_frames.is_empty() {
            crystal.animation_timer += dt;
            while crystal.animation_timer >= DASH_CRYSTAL_ANIMATION_INTERVAL {
                crystal.animation_timer -= DASH_CRYSTAL_ANIMATION_INTERVAL;
                crystal.frame_index = (crystal.frame_index + 1) % crystal.active_frames.len();
                sprite.image = crystal.active_frames[crystal.frame_index].clone();
            }
        }

        let Some(crystal_size) = sprite.custom_size else {
            continue;
        };

        if dash_state.dashes_remaining == 0
            && check_collision(
                player_transform.translation,
                player_size.0,
                crystal_transform.translation,
                crystal_size,
            )
        {
            let _crystal_id = &crystal.id;
            dash_state.dashes_remaining = 1;
            crystal.respawn_timer = DASH_CRYSTAL_RESPAWN_TIME;
            crystal.animation_timer = 0.0;
            sprite.image = crystal.vanished_frame.clone();
        }
    }
}

pub fn handle_hazard_respawn(
    active_room: Res<ActiveRoom>,
    hazards: Query<(&Transform, &Sprite), (With<Hazard>, Without<Player>)>,
    mut player_query: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut Grounded,
            &mut WallContact,
            &mut ColliderSize,
            &mut Crouching,
            &mut DashState,
            &mut DashSlideState,
            &mut PlayerStateMachine,
            &mut ClimbTopOutState,
            &mut Hair,
            &Facing,
        ),
        With<Player>,
    >,
) {
    let Ok((
        mut player_transform,
        mut velocity,
        mut grounded,
        mut wall_contact,
        mut collider_size,
        mut crouching,
        mut dash_state,
        mut dash_slide,
        mut state_machine,
        mut climb_top_out,
        mut hair,
        facing,
    )) = player_query.get_single_mut()
    else {
        return;
    };

    let player_size = collider_size.0;
    for (hazard_transform, hazard_sprite) in &hazards {
        let Some(hazard_size) = hazard_sprite.custom_size else {
            continue;
        };

        if check_collision(
            player_transform.translation,
            player_size,
            hazard_transform.translation,
            hazard_size,
        ) {
            reset_player_to_spawn(
                &mut player_transform,
                &mut velocity,
                &mut grounded,
                &mut wall_contact,
                &mut collider_size,
                &mut crouching,
                &mut dash_state,
                &mut dash_slide,
                &mut state_machine,
                &mut climb_top_out,
                &mut hair,
                facing,
                active_room.respawn_point,
                false,
            );
            break;
        }
    }
}

pub fn handle_room_transitions(
    mut commands: Commands,
    loaded_map: Res<LoadedMap>,
    level_art: Res<LevelArt>,
    mut active_room: ResMut<ActiveRoom>,
    level_entities: Query<Entity, With<LevelEntity>>,
    mut param_set: ParamSet<(
        Query<(&Transform, &ColliderSize), With<Player>>,
        Query<(&Transform, &Sprite, &RoomExitMarker), Without<Player>>,
        Query<
            (
                &mut Transform,
                &mut Velocity,
                &mut Grounded,
                &mut WallContact,
                &mut ColliderSize,
                &mut Crouching,
                &mut DashState,
                &mut DashSlideState,
                &mut PlayerStateMachine,
                &mut ClimbTopOutState,
                &mut Hair,
                &Facing,
            ),
            With<Player>,
        >,
        Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
        Query<&mut Transform, (With<WeatherOverlay>, (Without<Player>, Without<Camera2d>))>,
    )>,
) {
    let player_query = param_set.p0();
    let Ok((player_transform, collider_size)) = player_query.get_single() else {
        return;
    };

    let player_translation = player_transform.translation;
    let player_size = collider_size.0;
    let triggered_exit = {
        let exits_query = param_set.p1();
        let mut result: Option<(String, String, bool)> = None;

        for (exit_transform, exit_sprite, exit) in &exits_query {
            let Some(exit_size) = exit_sprite.custom_size else {
                continue;
            };

            if check_collision(
                player_translation,
                player_size,
                exit_transform.translation,
                exit_size,
            ) {
                let _exit_id = &exit.id;
                result = Some((
                    exit.target_room.clone(),
                    exit.target_spawn.clone(),
                    exit.preserve_momentum,
                ));
                break;
            }
        }

        result
    };

    let Some((target_room_id, target_spawn_id, preserve_momentum)) = triggered_exit else {
        return;
    };

    let Some(target_room) = loaded_map.data.room(&target_room_id) else {
        return;
    };
    let Some(target_spawn) = target_room.spawn_point(&target_spawn_id) else {
        return;
    };

    let mut player_query = param_set.p2();
    let Ok((
        mut player_transform,
        mut velocity,
        mut grounded,
        mut wall_contact,
        mut collider_size,
        mut crouching,
        mut dash_state,
        mut dash_slide,
        mut state_machine,
        mut climb_top_out,
        mut hair,
        facing,
    )) = player_query.get_single_mut()
    else {
        return;
    };

    for entity in &level_entities {
        commands.entity(entity).despawn();
    }

    spawn_room_geometry(&mut commands, target_room, &level_art);

    active_room.room_id = target_room.id.clone();
    active_room.respawn_point = target_room
        .checkpoints
        .first()
        .map(|checkpoint| Vec2::new(checkpoint.x, checkpoint.y))
        .or_else(|| target_room.default_spawn_point())
        .unwrap_or(target_spawn);

    reset_player_to_spawn(
        &mut player_transform,
        &mut velocity,
        &mut grounded,
        &mut wall_contact,
        &mut collider_size,
        &mut crouching,
        &mut dash_state,
        &mut dash_slide,
        &mut state_machine,
        &mut climb_top_out,
        &mut hair,
        facing,
        target_spawn,
        preserve_momentum,
    );

    for mut camera_transform in &mut param_set.p3() {
        camera_transform.translation.x = target_room.bounds.center().x;
        camera_transform.translation.y = target_room.bounds.center().y;
    }

    for mut overlay_transform in &mut param_set.p4() {
        overlay_transform.translation.x = target_room.bounds.center().x;
        overlay_transform.translation.y = target_room.bounds.center().y;
    }
}

pub fn camera_follow(
    player_query: Query<&Transform, With<Player>>,
    active_room: Res<ActiveRoom>,
    loaded_map: Res<LoadedMap>,
    window_query: Query<&Window>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>, Without<WeatherOverlay>)>,
    mut weather_query: Query<&mut Transform, (With<WeatherOverlay>, Without<Player>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok(window) = window_query.get_single() else {
        return;
    };
    let Some(room) = loaded_map.data.room(&active_room.room_id) else {
        return;
    };

    // Viewport: height fixed at 180px, width derived from window aspect
    let viewport_height = 180.0;
    let aspect = window.width() / window.height();
    let viewport_width = viewport_height * aspect;
    let half_w = viewport_width * 0.5;
    let half_h = viewport_height * 0.5;

    // Room bounds edges (room.bounds.x/y is the center)
    let room_left = room.bounds.x - room.bounds.w * 0.5;
    let room_right = room.bounds.x + room.bounds.w * 0.5;
    let room_bottom = room.bounds.y - room.bounds.h * 0.5;
    let room_top = room.bounds.y + room.bounds.h * 0.5;

    // Target follows player, clamped so camera never shows outside the room
    let target_x = if room.bounds.w >= viewport_width {
        player_transform.translation.x.clamp(room_left + half_w, room_right - half_w)
    } else {
        room.bounds.x
    };
    let target_y = if room.bounds.h >= viewport_height {
        player_transform.translation.y.clamp(room_bottom + half_h, room_top - half_h)
    } else {
        room.bounds.y
    };

    // Smooth follow (exponential ease)
    let follow_speed = 8.0;
    let t = 1.0 - (-follow_speed * time.delta_secs()).exp();

    if let Ok(mut cam) = camera_query.get_single_mut() {
        cam.translation.x += (target_x - cam.translation.x) * t;
        cam.translation.y += (target_y - cam.translation.y) * t;

        // Keep weather overlay at camera position
        for mut overlay in &mut weather_query {
            overlay.translation.x = cam.translation.x;
            overlay.translation.y = cam.translation.y;
        }
    }
}
