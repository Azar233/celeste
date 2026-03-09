use bevy::prelude::*;

use crate::components::{
    AnimationState, AnimationTimer, Crouching, DashState, Facing, Grounded, MovementInput,
    PlayerAnimations, Velocity, WallContact, WallJumpTimer,
};

pub fn animate_sprite(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut Sprite,
        &mut AnimationState,
        &PlayerAnimations,
        &Velocity,
        &Facing,
        &Grounded,
        &Crouching,
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
        velocity,
        facing,
        grounded,
        crouching,
        move_input,
        wall_contact,
        wall_jump_timer,
        dash_state,
    ) in
        &mut query
    {
        let is_holding_wall = keyboard_input.pressed(KeyCode::KeyJ)
            && !grounded.0
            && !dash_state.is_dashing
            && *wall_contact != WallContact::None
            && wall_jump_timer.0 <= 0.0;

        let away_from_wall = match wall_contact {
            WallContact::Left => 1.0,
            WallContact::Right => -1.0,
            WallContact::None => 0.0,
        };
        let is_lookback = is_holding_wall && move_input.x == away_from_wall && away_from_wall != 0.0;

        let is_moving = velocity.0.x.abs() > 5.0;
        let next_state = if is_lookback {
            AnimationState::ClimbLookback
        } else if is_holding_wall {
            AnimationState::Climb
        } else if crouching.0 {
            AnimationState::Duck
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