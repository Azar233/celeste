use bevy::prelude::*;

use crate::components::{
    AnimationState, AnimationTimer, Crouching, Facing, Grounded, PlayerAnimations, Velocity,
};

pub fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut Sprite,
        &mut AnimationState,
        &PlayerAnimations,
        &Velocity,
        &Facing,
        &Grounded,
        &Crouching,
    )>,
) {
    for (mut timer, mut sprite, mut state, animations, velocity, facing, grounded, crouching) in
        &mut query
    {
        sprite.flip_x = facing.0 < 0.0;

        let is_moving = velocity.0.x.abs() > 5.0;
        let next_state = if crouching.0 {
            AnimationState::Duck
        } else if grounded.0 && is_moving {
            AnimationState::Run
        } else {
            AnimationState::Idle
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
                };
                atlas.index = if atlas.index >= max_frames - 1 {
                    0
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}