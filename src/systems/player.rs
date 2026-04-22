use bevy::prelude::*;

use crate::components::{
    ColliderSize, Crouching, DashSlideState, DashState, Facing, FreezeFrameState, Ground,
    Grounded, JumpState, MovementInput, Player, PlayerActionInput, PlayerState,
    PlayerStateMachine, Velocity, WallContact, WallJumpTimer,
};
use crate::constants::{
    AIR_ACCEL, AIR_FRICTION, AIR_TURN_FRICTION, APEX_HALF_GRAVITY_MULTIPLIER,
    CROUCH_COLLIDER_SIZE, DASH_DURATION, DASH_END_MULTIPLIER, DASH_SLIDE_SPEED_MULTIPLIER,
    DASH_SLIDE_WINDOW, DASH_SPEED, DASH_CORNER_CORRECTION, DASH_FREEZE_TIME, DEATH_THRESHOLD,
    DUCK_SUPER_JUMP_X_MULTIPLIER, DUCK_SUPER_JUMP_Y_MULTIPLIER,
    FALL_MULTIPLIER, GROUND_ACCEL, GROUND_FRICTION, GROUND_TURN_FRICTION, GRAVITY,
    HALF_GRAVITY_THRESHOLD, JUMP_BUFFER_TIME, JUMP_GRACE_TIME, JUMP_VELOCITY,
    LOW_JUMP_MULTIPLIER, MAX_RUN_SPEED, PLAYER_COLLIDER_SIZE, PLAYER_RENDER_Z, SPAWN_POSITION,
    SUPER_JUMP_APEX_GRAVITY_MULTIPLIER, SUPER_JUMP_APEX_THRESHOLD,
    SUPER_JUMP_FALL_MULTIPLIER, SUPER_JUMP_GRAVITY_WINDOW, SUPER_JUMP_HORIZONTAL_SPEED,
    SUPER_JUMP_TURN_FRICTION, SUPER_JUMP_VERTICAL_SPEED,
    UPWARD_CORNER_CORRECTION,
    WALL_CLIMB_JUMP_FORCE_Y, WALL_CLIMB_LOCK, WALL_CLIMB_SPEED, WALL_KICK_FORCE,
    WALL_KICK_LOCK, WALL_NEUTRAL_FORCE, WALL_NEUTRAL_LOCK, WALL_SLIDE_SPEED,
};
use crate::utils::{can_use_collider, check_collision, move_towards};

fn try_consume_ground_jump(jump_state: &mut JumpState, grounded: &Grounded, velocity: &mut Velocity) -> bool {
    if jump_state.jump_buffer_timer > 0.0 && (grounded.0 || jump_state.jump_grace_timer > 0.0) {
        velocity.0.y = JUMP_VELOCITY;
        jump_state.jump_grace_timer = 0.0;
        jump_state.jump_buffer_timer = 0.0;
        return true;
    }

    false
}

fn try_consume_super_jump(
    jump_state: &mut JumpState,
    dash_slide: &mut DashSlideState,
    velocity: &mut Velocity,
) -> bool {
    if jump_state.jump_buffer_timer > 0.0 && dash_slide.timer > 0.0 {
        perform_super_jump(jump_state, velocity, dash_slide.direction, true);
        dash_slide.timer = 0.0;
        return true;
    }

    false
}

fn perform_super_jump(
    jump_state: &mut JumpState,
    velocity: &mut Velocity,
    direction: f32,
    ducking: bool,
) {
    velocity.0.x = direction.signum() * SUPER_JUMP_HORIZONTAL_SPEED;
    velocity.0.y = SUPER_JUMP_VERTICAL_SPEED;

    if ducking {
        velocity.0.x *= DUCK_SUPER_JUMP_X_MULTIPLIER;
        velocity.0.y *= DUCK_SUPER_JUMP_Y_MULTIPLIER;
    }

    jump_state.jump_grace_timer = 0.0;
    jump_state.jump_buffer_timer = 0.0;
    jump_state.super_jump_timer = SUPER_JUMP_GRAVITY_WINDOW;
}

fn try_consume_grounded_super_jump(
    jump_state: &mut JumpState,
    dash_state: &mut DashState,
    grounded: &Grounded,
    move_input: &MovementInput,
    velocity: &mut Velocity,
) -> bool {
    if jump_state.jump_buffer_timer <= 0.0
        || !grounded.0
        || move_input.y >= 0.0
        || !dash_state.is_dashing
        || dash_state.direction.x == 0.0
        || dash_state.direction.y != 0.0
    {
        return false;
    }

    perform_super_jump(jump_state, velocity, dash_state.direction.x, true);
    dash_state.is_dashing = false;
    dash_state.timer = 0.0;
    return true;
}

fn try_dash_corner_correction(
    translation: &mut Vec3,
    player_size: Vec2,
    dash_state: &DashState,
    obstacles: &[(Vec3, Vec2)],
) -> bool {
    if !dash_state.is_dashing || dash_state.direction.y != 0.0 || dash_state.direction.x == 0.0 {
        return false;
    }

    for offset in 1..=DASH_CORNER_CORRECTION {
        let candidate = *translation + Vec3::new(0.0, offset as f32, 0.0);
        if can_use_collider(candidate, player_size, obstacles) {
            *translation = candidate;
            return true;
        }
    }

    false
}

fn try_upward_corner_correction(
    translation: &mut Vec3,
    player_size: Vec2,
    move_input: &MovementInput,
    obstacles: &[(Vec3, Vec2)],
) -> bool {
    let horizontal_preferences = if move_input.x > 0.0 {
        [1.0, -1.0]
    } else if move_input.x < 0.0 {
        [-1.0, 1.0]
    } else {
        [1.0, -1.0]
    };

    for offset in 1..=UPWARD_CORNER_CORRECTION {
        for direction in horizontal_preferences {
            let candidate = *translation + Vec3::new(offset as f32 * direction, 0.0, 0.0);
            if can_use_collider(candidate, player_size, obstacles) {
                *translation = candidate;
                return true;
            }
        }
    }

    false
}

fn resolve_dash_direction(
    move_input: &MovementInput,
    grounded: &Grounded,
    crouching: &Crouching,
    facing: &Facing,
) -> Vec2 {
    let mut dash_dir = Vec2::new(move_input.x, move_input.y);

    if grounded.0 && crouching.0 {
        dash_dir.y = 0.0;
    }

    if dash_dir == Vec2::ZERO {
        dash_dir = Vec2::new(facing.0.signum().max(-1.0), 0.0);
        if dash_dir.x == 0.0 {
            dash_dir.x = 1.0;
        }
    }

    dash_dir.normalize_or_zero()
}

fn transition_player_state(state_machine: &mut PlayerStateMachine, next_state: PlayerState) {
    if state_machine.current != next_state {
        state_machine.previous = state_machine.current;
        state_machine.current = next_state;
    }
}

fn resolve_player_state(
    actions: &PlayerActionInput,
    grounded: &Grounded,
    wall_contact: &WallContact,
    wall_jump_timer: &WallJumpTimer,
    dash_state: &DashState,
) -> PlayerState {
    if dash_state.is_dashing {
        PlayerState::Dash
    } else if actions.grab_held
        && !grounded.0
        && *wall_contact != WallContact::None
        && wall_jump_timer.0 <= 0.0
    {
        PlayerState::Climb
    } else {
        PlayerState::Normal
    }
}

fn apply_state_end(
    previous_state: PlayerState,
    next_state: PlayerState,
    velocity: &mut Velocity,
    _dash_state: &DashState,
) {
    if previous_state == PlayerState::Dash && next_state != PlayerState::Dash {
        velocity.0 *= DASH_END_MULTIPLIER;
        if velocity.0.y < 0.0 {
            velocity.0.y *= 0.5;
        }
    }
}

pub fn cache_player_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut MovementInput, &mut PlayerActionInput, &mut Facing, &WallContact), With<Player>>,
) {
    if let Ok((mut move_input, mut actions, mut facing, wall_contact)) = query.get_single_mut() {
        let left = keyboard_input.pressed(KeyCode::KeyA)
            || keyboard_input.pressed(KeyCode::ArrowLeft);
        let right = keyboard_input.pressed(KeyCode::KeyD)
            || keyboard_input.pressed(KeyCode::ArrowRight);
        let up = keyboard_input.pressed(KeyCode::KeyW)
            || keyboard_input.pressed(KeyCode::ArrowUp);
        let down = keyboard_input.pressed(KeyCode::KeyS)
            || keyboard_input.pressed(KeyCode::ArrowDown);

        move_input.x = if right && !left {
            1.0
        } else if left && !right {
            -1.0
        } else {
            0.0
        };
        move_input.y = if up && !down {
            1.0
        } else if down && !up {
            -1.0
        } else {
            0.0
        };

        actions.jump_pressed = keyboard_input.just_pressed(KeyCode::Space)
            || keyboard_input.just_pressed(KeyCode::KeyZ);
        actions.jump_held = keyboard_input.pressed(KeyCode::Space)
            || keyboard_input.pressed(KeyCode::KeyZ);
        actions.dash_pressed = keyboard_input.just_pressed(KeyCode::KeyK)
            || keyboard_input.just_pressed(KeyCode::KeyC);
        actions.grab_held = keyboard_input.pressed(KeyCode::KeyJ)
            || keyboard_input.pressed(KeyCode::KeyX);

        if actions.grab_held && *wall_contact != WallContact::None {
            facing.0 = match wall_contact {
                WallContact::Left => -1.0,
                WallContact::Right => 1.0,
                WallContact::None => facing.0,
            };
        } else if move_input.x != 0.0 {
            facing.0 = move_input.x;
        }
    }
}

pub fn update_player_state_machine(
    mut query: Query<(
        &mut Velocity,
        &PlayerActionInput,
        &Grounded,
        &WallContact,
        &WallJumpTimer,
        &DashState,
        &mut PlayerStateMachine,
    ), With<Player>>,
) {
    if let Ok((
        mut velocity,
        actions,
        grounded,
        wall_contact,
        wall_jump_timer,
        dash_state,
        mut state_machine,
    )) =
        query.get_single_mut()
    {
        let next_state = resolve_player_state(actions, grounded, wall_contact, wall_jump_timer, dash_state);

        if state_machine.current != next_state {
            apply_state_end(state_machine.current, next_state, &mut velocity, dash_state);
            transition_player_state(&mut state_machine, next_state);
        }
    }
}

pub fn tick_timers(
    time: Res<Time<Fixed>>,
    mut q_jump: Query<(&Grounded, &mut JumpState), With<Player>>,
    mut q_slide: Query<&mut DashSlideState, With<Player>>,
    mut q_wall: Query<&mut WallJumpTimer>,
    mut q_dash: Query<&mut DashState>,
) {
    let dt = time.delta_secs();

    for (grounded, mut jump_state) in &mut q_jump {
        if grounded.0 {
            jump_state.jump_grace_timer = JUMP_GRACE_TIME;
            jump_state.super_jump_timer = 0.0;
        } else if jump_state.jump_grace_timer > 0.0 {
            jump_state.jump_grace_timer = (jump_state.jump_grace_timer - dt).max(0.0);
        }

        if jump_state.jump_buffer_timer > 0.0 {
            jump_state.jump_buffer_timer = (jump_state.jump_buffer_timer - dt).max(0.0);
        }

        if jump_state.super_jump_timer > 0.0 {
            jump_state.super_jump_timer = (jump_state.super_jump_timer - dt).max(0.0);
        }
    }

    for mut timer in &mut q_wall {
        if timer.0 > 0.0 {
            timer.0 -= dt;
        }
    }

    for mut dash_state in &mut q_dash {
        if dash_state.is_dashing {
            dash_state.timer -= dt;
            if dash_state.timer <= 0.0 {
                dash_state.is_dashing = false;
            }
        }
    }

    for mut dash_slide in &mut q_slide {
        if dash_slide.timer > 0.0 {
            dash_slide.timer = (dash_slide.timer - dt).max(0.0);
        }
    }
}

pub fn update_crouch_state(
    mut query: Query<(
        &mut Transform,
        &Grounded,
        &MovementInput,
        &DashState,
        &DashSlideState,
        &mut Crouching,
        &mut ColliderSize,
    ), (With<Player>, Without<Ground>)>,
    obstacles_query: Query<(&Transform, &Sprite), (With<Ground>, Without<Player>)>,
) {
    let obstacles: Vec<(Vec3, Vec2)> = obstacles_query
        .iter()
        .filter_map(|(transform, sprite)| sprite.custom_size.map(|size| (transform.translation, size)))
        .collect();

    if let Ok((mut transform, grounded, move_input, dash_state, dash_slide, mut crouching, mut collider_size)) =
        query.get_single_mut()
    {
        let wants_crouch = (grounded.0 && move_input.y < 0.0 && !dash_state.is_dashing)
            || dash_slide.timer > 0.0;
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
    mut freeze_frames: ResMut<FreezeFrameState>,
    mut query: Query<(
        &mut Velocity,
        &mut JumpState,
        &Grounded,
        &WallContact,
        &mut WallJumpTimer,
        &mut DashState,
        &mut DashSlideState,
        &MovementInput,
        &mut PlayerActionInput,
        &Facing,
        &Crouching,
        &mut PlayerStateMachine,
    ), With<Player>>,
) {
    if let Ok((
        mut velocity,
        mut jump_state,
        grounded,
        wall_contact,
        mut wall_jump_timer,
        mut dash_state,
        mut dash_slide,
        move_input,
        mut actions,
        facing,
        crouching,
        mut state_machine,
    )) = query.get_single_mut()
    {
        if actions.jump_pressed {
            jump_state.jump_buffer_timer = JUMP_BUFFER_TIME;
            actions.jump_pressed = false;
        }

        if grounded.0 && velocity.0.y <= 0.0 && !dash_state.is_dashing {
            dash_state.dashes_remaining = 1;
        }

        if actions.dash_pressed && dash_state.dashes_remaining > 0 {
            dash_state.is_dashing = true;
            dash_state.timer = DASH_DURATION;
            dash_state.dashes_remaining -= 1;
            dash_slide.timer = 0.0;
            dash_slide.direction = 0.0;
            if freeze_frames.timer <= 0.0 {
                freeze_frames.timer = DASH_FREEZE_TIME;
            }

            dash_state.direction = resolve_dash_direction(move_input, grounded, crouching, facing);
            velocity.0 = dash_state.direction * DASH_SPEED;
            transition_player_state(&mut state_machine, PlayerState::Dash);
        }

        if try_consume_grounded_super_jump(
            &mut jump_state,
            &mut dash_state,
            grounded,
            move_input,
            &mut velocity,
        ) {
            transition_player_state(&mut state_machine, PlayerState::Normal);
            return;
        }

        if state_machine.current == PlayerState::Dash && dash_slide.timer <= 0.0 {
            return;
        }

        if try_consume_super_jump(&mut jump_state, &mut dash_slide, &mut velocity) {
            transition_player_state(&mut state_machine, PlayerState::Normal);
            return;
        }

        if try_consume_ground_jump(&mut jump_state, grounded, &mut velocity) {
            return;
        }

        if jump_state.jump_buffer_timer > 0.0 && !grounded.0 && *wall_contact != WallContact::None {
            let wall_dir = match wall_contact {
                WallContact::Left => -1.0,
                WallContact::Right => 1.0,
                WallContact::None => 0.0,
            };

            if state_machine.current == PlayerState::Climb {
                velocity.0.y = WALL_CLIMB_JUMP_FORCE_Y;
                velocity.0.x = 0.0;
                wall_jump_timer.0 = WALL_CLIMB_LOCK;
            } else if move_input.x == 0.0 {
                velocity.0.x = -wall_dir * WALL_NEUTRAL_FORCE.x;
                velocity.0.y = WALL_NEUTRAL_FORCE.y;
                wall_jump_timer.0 = WALL_NEUTRAL_LOCK;
            } else {
                velocity.0.x = -wall_dir * WALL_KICK_FORCE.x;
                velocity.0.y = WALL_KICK_FORCE.y;
                wall_jump_timer.0 = WALL_KICK_LOCK;
            }

            jump_state.jump_buffer_timer = 0.0;
        }
    }
}

pub fn apply_physics(
    mut query: Query<(
        &mut Velocity,
        &JumpState,
        &PlayerStateMachine,
        &WallContact,
        &Grounded,
        &WallJumpTimer,
        &DashState,
        &MovementInput,
        &PlayerActionInput,
        &Crouching,
    ), With<Player>>,
    time: Res<Time<Fixed>>,
) {
    if let Ok((mut velocity, jump_state, state_machine, wall_contact, grounded, wall_jump_timer, dash_state, move_input, actions, crouching)) =
        query.get_single_mut()
    {
        let dt = time.delta_secs();

        match state_machine.current {
            PlayerState::Dash => {
                velocity.0 = dash_state.direction * DASH_SPEED;
            }
            PlayerState::Climb => {
                velocity.0.x = 0.0;
                let mut climb_dir = 0.0;
                if move_input.y > 0.0 {
                    climb_dir += 1.0;
                }
                if move_input.y < 0.0 {
                    climb_dir -= 1.0;
                }
                velocity.0.y = climb_dir * WALL_CLIMB_SPEED;
            }
            PlayerState::Normal => {
                if wall_jump_timer.0 <= 0.0 {
                    let current_speed = velocity.0.x;

                    if jump_state.super_jump_timer > 0.0 && !grounded.0 {
                        if move_input.x != 0.0
                            && current_speed != 0.0
                            && current_speed.signum() != move_input.x.signum()
                        {
                            velocity.0.x = move_towards(
                                current_speed,
                                move_input.x * MAX_RUN_SPEED,
                                SUPER_JUMP_TURN_FRICTION * dt,
                            );
                        } else {
                            velocity.0.x = current_speed;
                        }
                    } else {
                        let target_speed = if crouching.0 { 0.0 } else { move_input.x * MAX_RUN_SPEED };

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
                    }
                } else {
                    velocity.0.x = move_towards(velocity.0.x, 0.0, AIR_FRICTION * 0.5 * dt);
                }

                let gravity_multiplier = if jump_state.super_jump_timer > 0.0 {
                    if velocity.0.y.abs() < SUPER_JUMP_APEX_THRESHOLD {
                        SUPER_JUMP_APEX_GRAVITY_MULTIPLIER
                    } else if velocity.0.y < 0.0 {
                        SUPER_JUMP_FALL_MULTIPLIER
                    } else if actions.jump_held {
                        0.9
                    } else {
                        1.1
                    }
                } else if velocity.0.y > 0.0 {
                    if actions.jump_held {
                        1.0
                    } else {
                        LOW_JUMP_MULTIPLIER
                    }
                } else {
                    FALL_MULTIPLIER
                };

                let gravity_multiplier = if actions.jump_held
                    && velocity.0.y.abs() < HALF_GRAVITY_THRESHOLD
                {
                    gravity_multiplier * APEX_HALF_GRAVITY_MULTIPLIER
                } else {
                    gravity_multiplier
                };

                velocity.0.y -= GRAVITY * gravity_multiplier * dt;

                if *wall_contact != WallContact::None
                    && velocity.0.y < 0.0
                    && !grounded.0
                    && wall_jump_timer.0 <= 0.0
                    && velocity.0.y < -WALL_SLIDE_SPEED
                {
                    velocity.0.y = -WALL_SLIDE_SPEED;
                }
            }
        }
    }
}

pub fn player_movement(
    mut param_set: ParamSet<(
        Query<(
            &mut Transform,
            &mut Velocity,
            &mut JumpState,
            &mut Grounded,
            &mut WallContact,
            &mut ColliderSize,
            &mut Crouching,
            &mut DashState,
            &mut DashSlideState,
            &mut PlayerStateMachine,
        ), With<Player>>,
        Query<(&Transform, &Sprite), With<Ground>>,
    )>,
    time: Res<Time<Fixed>>,
) {
    let obstacles: Vec<(Vec3, Vec2)> = param_set
        .p1()
        .iter()
        .filter_map(|(transform, sprite)| sprite.custom_size.map(|size| (transform.translation, size)))
        .collect();

    if let Ok((
        mut player_transform,
        mut velocity,
        mut jump_state,
        mut grounded,
        mut wall_contact,
        mut collider_size,
        mut crouching,
        mut dash_state,
        mut dash_slide,
        mut state_machine,
    )) = param_set.p0().get_single_mut()
    {
        let player_size = collider_size.0;
        let delta_seconds = time.delta_secs();
        let was_grounded = grounded.0;

        grounded.0 = false;

        player_transform.translation.x += velocity.0.x * delta_seconds;
        for (ground_pos, ground_size) in &obstacles {
            if check_collision(player_transform.translation, player_size, *ground_pos, *ground_size) {
                if !try_dash_corner_correction(
                    &mut player_transform.translation,
                    player_size,
                    &dash_state,
                    &obstacles,
                ) {
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
                    if !try_upward_corner_correction(
                        &mut player_transform.translation,
                        player_size,
                        &MovementInput {
                            x: velocity.0.x.signum(),
                            y: velocity.0.y.signum(),
                        },
                        &obstacles,
                    ) {
                        player_transform.translation.y =
                            ground_pos.y - ground_size.y / 2.0 - player_size.y / 2.0;
                        velocity.0.y = 0.0;
                    }
                } else if velocity.0.y < 0.0 {
                    player_transform.translation.y =
                        ground_pos.y + ground_size.y / 2.0 + player_size.y / 2.0;
                    grounded.0 = true;
                    velocity.0.y = 0.0;
                }
            }
        }

        if grounded.0
            && !was_grounded
            && dash_state.is_dashing
            && dash_state.direction.x != 0.0
            && dash_state.direction.y < 0.0
        {
            dash_state.is_dashing = false;
            dash_state.timer = 0.0;
            dash_slide.timer = DASH_SLIDE_WINDOW;
            dash_slide.direction = dash_state.direction.x.signum();
            velocity.0.y = 0.0;
            velocity.0.x = dash_slide.direction * (DASH_SPEED * DASH_SLIDE_SPEED_MULTIPLIER);
            crouching.0 = true;
            transition_player_state(&mut state_machine, PlayerState::Normal);
        }

        if grounded.0 {
            *wall_contact = WallContact::None;
        }
        if player_transform.translation.y < DEATH_THRESHOLD {
            player_transform.translation = Vec3::new(SPAWN_POSITION.x, SPAWN_POSITION.y, PLAYER_RENDER_Z);
            velocity.0 = Vec2::ZERO;
            jump_state.jump_grace_timer = 0.0;
            jump_state.jump_buffer_timer = 0.0;
            jump_state.super_jump_timer = 0.0;
            *wall_contact = WallContact::None;
            grounded.0 = false;
            crouching.0 = false;
            dash_state.is_dashing = false;
            dash_state.timer = 0.0;
            dash_slide.timer = 0.0;
            dash_slide.direction = 0.0;
            collider_size.0 = PLAYER_COLLIDER_SIZE;
            state_machine.current = PlayerState::Normal;
            state_machine.previous = PlayerState::Normal;
        }
    }
}