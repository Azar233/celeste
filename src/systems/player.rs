use bevy::prelude::*;

use crate::components::{
    ColliderSize, Crouching, DashState, Facing, Ground, Grounded, JumpState, MovementInput,
    Player, Velocity, WallContact, WallJumpTimer,
};
use crate::constants::{
    AIR_ACCEL, AIR_FRICTION, AIR_TURN_FRICTION, CROUCH_COLLIDER_SIZE, DASH_DURATION,
    DASH_END_MULTIPLIER, DASH_SPEED, DEATH_THRESHOLD, FALL_MULTIPLIER, GROUND_ACCEL,
    GROUND_FRICTION, GROUND_TURN_FRICTION, GRAVITY, JUMP_VELOCITY, LOW_JUMP_MULTIPLIER,
    MAX_RUN_SPEED, PLAYER_COLLIDER_SIZE, SPAWN_POSITION, WALL_CLIMB_JUMP_FORCE_Y,
    WALL_CLIMB_LOCK, WALL_CLIMB_SPEED, WALL_KICK_FORCE, WALL_KICK_LOCK, WALL_NEUTRAL_FORCE,
    WALL_NEUTRAL_LOCK, WALL_SLIDE_SPEED,
};
use crate::utils::{can_use_collider, check_collision, move_towards};

pub fn tick_timers(
    time: Res<Time>,
    mut q_wall: Query<&mut WallJumpTimer>,
    mut q_dash: Query<(&mut DashState, &mut Velocity)>,
) {
    let dt = time.delta_secs();

    for mut timer in &mut q_wall {
        if timer.0 > 0.0 {
            timer.0 -= dt;
        }
    }

    for (mut dash_state, mut velocity) in &mut q_dash {
        if dash_state.is_dashing {
            dash_state.timer -= dt;
            if dash_state.timer <= 0.0 {
                dash_state.is_dashing = false;
                velocity.0 *= DASH_END_MULTIPLIER;
                if velocity.0.y < 0.0 {
                    velocity.0.y *= 0.5;
                }
            }
        }
    }
}

pub fn update_crouch_state(
    mut query: Query<(
        &mut Transform,
        &Grounded,
        &MovementInput,
        &DashState,
        &mut Crouching,
        &mut ColliderSize,
    ), (With<Player>, Without<Ground>)>,
    obstacles_query: Query<(&Transform, &Sprite), (With<Ground>, Without<Player>)>,
) {
    let obstacles: Vec<(Vec3, Vec2)> = obstacles_query
        .iter()
        .filter_map(|(transform, sprite)| sprite.custom_size.map(|size| (transform.translation, size)))
        .collect();

    if let Ok((mut transform, grounded, move_input, dash_state, mut crouching, mut collider_size)) =
        query.get_single_mut()
    {
        let wants_crouch = grounded.0 && move_input.y < 0.0 && !dash_state.is_dashing;
        let height_delta = PLAYER_COLLIDER_SIZE.y - CROUCH_COLLIDER_SIZE.y;

        if wants_crouch && !crouching.0 {
            crouching.0 = true;
            collider_size.0 = CROUCH_COLLIDER_SIZE;
            transform.translation.y -= height_delta * 0.5;
        } else if !wants_crouch && crouching.0 {
            let expanded_translation = transform.translation + Vec3::new(0.0, height_delta * 0.5, 0.0);
            if can_use_collider(expanded_translation, PLAYER_COLLIDER_SIZE, &obstacles) {
                crouching.0 = false;
                collider_size.0 = PLAYER_COLLIDER_SIZE;
                transform.translation = expanded_translation;
            }
        }
    }
}

pub fn player_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &mut Velocity,
        &mut JumpState,
        &Grounded,
        &WallContact,
        &mut WallJumpTimer,
        &mut DashState,
        &mut Facing,
        &mut MovementInput,
    ), With<Player>>,
) {
    if let Ok((
        mut velocity,
        mut jump_state,
        grounded,
        wall_contact,
        mut wall_jump_timer,
        mut dash_state,
        mut facing,
        mut move_input,
    )) = query.get_single_mut()
    {
        if grounded.0 && velocity.0.y <= 0.0 {
            jump_state.jumps_remaining = 1;
            dash_state.dashes_remaining = 1;
        }

        let left = keyboard_input.pressed(KeyCode::KeyA);
        let right = keyboard_input.pressed(KeyCode::KeyD);
        let up = keyboard_input.pressed(KeyCode::KeyW);
        let down = keyboard_input.pressed(KeyCode::KeyS);

        let input_x = if right && !left {
            1.0
        } else if left && !right {
            -1.0
        } else {
            0.0
        };
        let input_y = if up && !down {
            1.0
        } else if down && !up {
            -1.0
        } else {
            0.0
        };

        move_input.x = input_x;
        move_input.y = input_y;

        if input_x != 0.0 {
            facing.0 = input_x;
        }

        if keyboard_input.just_pressed(KeyCode::KeyK) && dash_state.dashes_remaining > 0 {
            dash_state.is_dashing = true;
            dash_state.timer = DASH_DURATION;
            dash_state.dashes_remaining -= 1;

            let mut dash_dir = Vec2::new(input_x, input_y);
            if dash_dir == Vec2::ZERO {
                dash_dir = Vec2::new(facing.0, 0.0);
            }
            dash_state.direction = dash_dir.normalize_or_zero();
            velocity.0 = dash_state.direction * DASH_SPEED;
            return;
        }

        if dash_state.is_dashing {
            return;
        }

        let is_grabbing = keyboard_input.pressed(KeyCode::KeyJ);

        if keyboard_input.just_pressed(KeyCode::Space) {
            if grounded.0 || jump_state.jumps_remaining > 0 {
                velocity.0.y = JUMP_VELOCITY;
                jump_state.jumps_remaining = jump_state.jumps_remaining.saturating_sub(1);
            } else if !grounded.0 && *wall_contact != WallContact::None {
                let wall_dir = match wall_contact {
                    WallContact::Left => -1.0,
                    WallContact::Right => 1.0,
                    WallContact::None => 0.0,
                };

                if is_grabbing {
                    velocity.0.y = WALL_CLIMB_JUMP_FORCE_Y;
                    velocity.0.x = 0.0;
                    wall_jump_timer.0 = WALL_CLIMB_LOCK;
                } else if input_x == 0.0 {
                    velocity.0.x = -wall_dir * WALL_NEUTRAL_FORCE.x;
                    velocity.0.y = WALL_NEUTRAL_FORCE.y;
                    wall_jump_timer.0 = WALL_NEUTRAL_LOCK;
                } else {
                    velocity.0.x = -wall_dir * WALL_KICK_FORCE.x;
                    velocity.0.y = WALL_KICK_FORCE.y;
                    wall_jump_timer.0 = WALL_KICK_LOCK;
                }
            }
        }
    }
}

pub fn apply_physics(
    mut query: Query<(
        &mut Velocity,
        &WallContact,
        &Grounded,
        &WallJumpTimer,
        &DashState,
        &MovementInput,
        &Crouching,
    ), With<Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if let Ok((mut velocity, wall_contact, grounded, wall_jump_timer, dash_state, move_input, crouching)) =
        query.get_single_mut()
    {
        if dash_state.is_dashing {
            velocity.0 = dash_state.direction * DASH_SPEED;
            return;
        }

        let is_holding_wall = keyboard_input.pressed(KeyCode::KeyJ)
            && *wall_contact != WallContact::None
            && wall_jump_timer.0 <= 0.0;

        if is_holding_wall {
            velocity.0.x = 0.0;
            let mut climb_dir = 0.0;
            if keyboard_input.pressed(KeyCode::KeyW) {
                climb_dir += 1.0;
            }
            if keyboard_input.pressed(KeyCode::KeyS) {
                climb_dir -= 1.0;
            }
            velocity.0.y = climb_dir * WALL_CLIMB_SPEED;
            return;
        }

        let dt = time.delta_secs();

        if wall_jump_timer.0 <= 0.0 {
            let target_speed = if crouching.0 { 0.0 } else { move_input.x * MAX_RUN_SPEED };
            let current_speed = velocity.0.x;

            let (accel, friction, turn_friction) = if grounded.0 {
                (GROUND_ACCEL, GROUND_FRICTION, GROUND_TURN_FRICTION)
            } else {
                (AIR_ACCEL, AIR_FRICTION, AIR_TURN_FRICTION)
            };

            let final_accel = if move_input.x != 0.0 {
                if current_speed.signum() != move_input.x.signum() && current_speed != 0.0 {
                    turn_friction
                } else {
                    accel
                }
            } else {
                friction
            };

            velocity.0.x = move_towards(current_speed, target_speed, final_accel * dt);
        } else {
            velocity.0.x = move_towards(velocity.0.x, 0.0, AIR_FRICTION * 0.5 * dt);
        }

        let gravity_multiplier = if velocity.0.y > 0.0 {
            if keyboard_input.pressed(KeyCode::Space) {
                1.0
            } else {
                LOW_JUMP_MULTIPLIER
            }
        } else {
            FALL_MULTIPLIER
        };

        velocity.0.y -= GRAVITY * gravity_multiplier * dt;

        if *wall_contact != WallContact::None && velocity.0.y < 0.0 && !grounded.0 && wall_jump_timer.0 <= 0.0 {
            if velocity.0.y < -WALL_SLIDE_SPEED {
                velocity.0.y = -WALL_SLIDE_SPEED;
            }
        }
    }
}

pub fn player_movement(
    mut param_set: ParamSet<(
        Query<(
            &mut Transform,
            &mut Velocity,
            &mut Grounded,
            &mut WallContact,
            &mut ColliderSize,
            &mut Crouching,
        ), With<Player>>,
        Query<(&Transform, &Sprite), With<Ground>>,
    )>,
    time: Res<Time>,
) {
    let obstacles: Vec<(Vec3, Vec2)> = param_set
        .p1()
        .iter()
        .filter_map(|(transform, sprite)| sprite.custom_size.map(|size| (transform.translation, size)))
        .collect();

    if let Ok((
        mut player_transform,
        mut velocity,
        mut grounded,
        mut wall_contact,
        mut collider_size,
        mut crouching,
    )) = param_set.p0().get_single_mut()
    {
        let player_size = collider_size.0;
        let delta_seconds = time.delta_secs();

        grounded.0 = false;

        player_transform.translation.x += velocity.0.x * delta_seconds;
        for (ground_pos, ground_size) in &obstacles {
            if check_collision(player_transform.translation, player_size, *ground_pos, *ground_size) {
                if velocity.0.x > 0.0 {
                    player_transform.translation.x =
                        ground_pos.x - ground_size.x / 2.0 - player_size.x / 2.0;
                } else if velocity.0.x < 0.0 {
                    player_transform.translation.x =
                        ground_pos.x + ground_size.x / 2.0 + player_size.x / 2.0;
                }
                velocity.0.x = 0.0;
            }
        }

        *wall_contact = WallContact::None;
        let sensor_margin = 1.0;
        for (ground_pos, ground_size) in &obstacles {
            let left_check_pos = player_transform.translation - Vec3::new(sensor_margin, 0.0, 0.0);
            if check_collision(left_check_pos, player_size, *ground_pos, *ground_size) {
                *wall_contact = WallContact::Left;
            }
            let right_check_pos = player_transform.translation + Vec3::new(sensor_margin, 0.0, 0.0);
            if check_collision(right_check_pos, player_size, *ground_pos, *ground_size) {
                *wall_contact = WallContact::Right;
            }
        }

        player_transform.translation.y += velocity.0.y * delta_seconds;
        for (ground_pos, ground_size) in &obstacles {
            if check_collision(player_transform.translation, player_size, *ground_pos, *ground_size) {
                if velocity.0.y > 0.0 {
                    player_transform.translation.y =
                        ground_pos.y - ground_size.y / 2.0 - player_size.y / 2.0;
                    velocity.0.y = 0.0;
                } else if velocity.0.y < 0.0 {
                    player_transform.translation.y =
                        ground_pos.y + ground_size.y / 2.0 + player_size.y / 2.0;
                    grounded.0 = true;
                    velocity.0.y = 0.0;
                }
            }
        }

        if grounded.0 {
            *wall_contact = WallContact::None;
        }
        if player_transform.translation.y < DEATH_THRESHOLD {
            player_transform.translation = SPAWN_POSITION;
            velocity.0 = Vec2::ZERO;
            *wall_contact = WallContact::None;
            grounded.0 = false;
            crouching.0 = false;
            collider_size.0 = PLAYER_COLLIDER_SIZE;
        }
    }
}