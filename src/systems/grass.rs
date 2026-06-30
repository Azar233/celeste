use bevy::prelude::*;

use crate::components::{Grass, GrassAnimation};
use crate::constants::GRASS_ANIMATION_INTERVAL;

pub fn animate_grass(time: Res<Time>, mut query: Query<(&mut GrassAnimation, &mut Sprite), With<Grass>>) {
    let dt = time.delta_secs();

    for (mut anim, mut sprite) in &mut query {
        if anim.frames.is_empty() {
            continue;
        }

        anim.timer += dt;
        while anim.timer >= GRASS_ANIMATION_INTERVAL {
            anim.timer -= GRASS_ANIMATION_INTERVAL;
            anim.frame_index = (anim.frame_index + 1) % anim.frames.len();
        }

        sprite.image = anim.frames[anim.frame_index].clone();
    }
}
