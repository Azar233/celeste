use bevy::prelude::*;

use crate::components::{
    AnimationState, AnimationTimer, Crouching, DashState, Facing, Grounded, MovementInput,
    JumpState, PlayerActionInput, PlayerAnimations, Velocity, WallContact, WallJumpTimer,
};
use crate::constants::FALL_FAST_ANIMATION_SPEED;

pub fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut Sprite,
        &mut AnimationState,
        &PlayerAnimations,
        &PlayerActionInput,
        &Velocity,
        &Facing,
        &Grounded,
        &Crouching,
        &JumpState,
        &MovementInput,
        &WallContact,
        &WallJumpTimer,
        &DashState,
    )>,
) {
    for (
        mut timer,
        mut sprite,
        mut state,
        animations,
        actions,
        velocity,
        facing,
        grounded,
        crouching,
        jump_state,
        move_input,
        wall_contact,
        wall_jump_timer,
        dash_state,
    ) in
        &mut query
    {
        let is_facing_wall = match wall_contact {
            WallContact::Left => facing.0 < 0.0,
            WallContact::Right => facing.0 > 0.0,
            WallContact::None => false,
        };

        let is_holding_wall = actions.grab_held
            && !dash_state.is_dashing
            && *wall_contact != WallContact::None
            && is_facing_wall
            && wall_jump_timer.0 <= 0.0;

        let away_from_wall = match wall_contact {
            WallContact::Left => 1.0,
            WallContact::Right => -1.0,
            WallContact::None => 0.0,
        };
        let is_lookback = is_holding_wall && move_input.x == away_from_wall && away_from_wall != 0.0;

        let is_moving = velocity.0.x.abs() > 5.0;
        let is_jumping_up = !grounded.0 && velocity.0.y > 0.0;
        let is_falling = !grounded.0 && velocity.0.y <= 0.0;
        let is_fall_fast = jump_state.fast_jump_active || -velocity.0.y >= FALL_FAST_ANIMATION_SPEED;
        let next_state = if dash_state.is_dashing {
            AnimationState::Dash
        } else if is_lookback {
            AnimationState::ClimbLookback
        } else if is_holding_wall {
            AnimationState::Climb
        } else if crouching.0 {
            AnimationState::Duck
        } else if is_jumping_up && jump_state.super_jump_timer > 0.0 {
            AnimationState::JumpFast
        } else if is_jumping_up {
            AnimationState::JumpSlow
        } else if is_falling && is_fall_fast {
            AnimationState::FallFast
        } else if is_falling {
            AnimationState::FallSlow
        } else if grounded.0 && is_moving {
            AnimationState::Run
        } else {
            AnimationState::Idle
        };

        sprite.flip_x = match next_state {
            AnimationState::Climb | AnimationState::ClimbLookback => *wall_contact == WallContact::Left,
            _ => facing.0 < 0.0,
        };

        if *state != next_state {
            *state = next_state;
            match next_state {
                AnimationState::Idle => {
                    sprite.image = animations.idle_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.idle_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::Run => {
                    sprite.image = animations.run_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.run_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::Duck => {
                    sprite.image = animations.duck_texture.clone();
                    sprite.texture_atlas = None;
                }
                AnimationState::Dash => {
                    sprite.image = animations.dash_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.dash_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::JumpSlow => {
                    sprite.image = animations.jump_slow_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.jump_slow_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::JumpFast => {
                    sprite.image = animations.jump_fast_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.jump_fast_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::FallSlow => {
                    sprite.image = animations.fall_slow_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.fall_slow_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::FallFast => {
                    sprite.image = animations.fall_fast_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.fall_fast_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::Climb => {
                    sprite.image = animations.climb_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.climb_layout.clone(),
                        index: 0,
                    });
                }
                AnimationState::ClimbLookback => {
                    sprite.image = animations.climb_lookback_texture.clone();
                    sprite.texture_atlas = Some(TextureAtlas {
                        layout: animations.climb_lookback_layout.clone(),
                        index: 0,
                    });
                }
            }
        }

        if *state == AnimationState::Duck {
            continue;
        }

        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                let max_frames = match *state {
                    AnimationState::Idle => 9,
                    AnimationState::Run => 12,
                    AnimationState::Duck => 1,
                    AnimationState::Dash => 4,
                    AnimationState::JumpSlow => 2,
                    AnimationState::JumpFast => 2,
                    AnimationState::FallSlow => 2,
                    AnimationState::FallFast => 2,
                    AnimationState::Climb => 6,
                    AnimationState::ClimbLookback => 3,
                };

                match *state {
                    AnimationState::Climb | AnimationState::ClimbLookback => {
                        if move_input.y > 0.0 {
                            atlas.index = if atlas.index >= max_frames - 1 {
                                0
                            } else {
                                atlas.index + 1
                            };
                        } else if move_input.y < 0.0 {
                            atlas.index = if atlas.index == 0 {
                                max_frames - 1
                            } else {
                                atlas.index - 1
                            };
                        } else {
                            atlas.index = 0;
                        }
                    }
                    _ => {
                        atlas.index = if atlas.index >= max_frames - 1 {
                            0
                        } else {
                            atlas.index + 1
                        };
                    }
                }
            }
        }
    }
}