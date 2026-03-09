use bevy::prelude::*;

use crate::components::{
    Crouching, DashState, DashTrailEmitter, DashTrailParticle, Grounded, Player,
};
use crate::constants::{
    DASH_TRAIL_INTERVAL, DASH_TRAIL_LIFETIME, DASH_TRAIL_PARTICLE_COUNT, DASH_TRAIL_SPACING,
};
use crate::utils::dash_effect_color;

pub fn emit_dash_trail(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        &Transform,
        &DashState,
        &Grounded,
        &Crouching,
        &mut DashTrailEmitter,
        Option<&Sprite>,
    ), With<Player>>,
) {
    let dt = time.delta_secs();

    for (transform, dash_state, grounded, crouching, mut emitter, sprite) in &mut query {
        if !dash_state.is_dashing {
            emitter.was_dashing = false;
            emitter.cooldown = 0.0;
            continue;
        }

        emitter.cooldown -= dt;
        let should_emit = !emitter.was_dashing || emitter.cooldown <= 0.0;
        if !should_emit {
            continue;
        }

        emitter.was_dashing = true;
        emitter.cooldown = DASH_TRAIL_INTERVAL;

        let mut effect_dir = dash_state.direction;
        if grounded.0 && crouching.0 {
            effect_dir.y = 0.0;
            if effect_dir.x.abs() <= f32::EPSILON {
                effect_dir.x = sprite
                    .map(|current_sprite| if current_sprite.flip_x { -1.0 } else { 1.0 })
                    .unwrap_or(1.0);
            }
        }

        let dash_dir = effect_dir.normalize_or_zero();
        if dash_dir == Vec2::ZERO {
            continue;
        }

        let backward = -dash_dir;
        let base_pos = transform.translation.truncate();
        let color = dash_effect_color(dash_state.dashes_remaining);
        let face_offset = sprite
            .map(|current_sprite| if current_sprite.flip_x { -1.5 } else { 1.5 })
            .unwrap_or(0.0);

        for index in 0..DASH_TRAIL_PARTICLE_COUNT {
            let step = index as f32;
            let position = base_pos + Vec2::new(face_offset, 3.0) + backward * (step * DASH_TRAIL_SPACING + 1.0);
            let velocity = backward * (10.0 + step * 3.0);
            let size = 1.0_f32.max(1.6 - step * 0.03);
            let lifetime = DASH_TRAIL_LIFETIME + step * 0.004;

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(size)),
                    ..default()
                },
                Transform::from_xyz(position.x, position.y, 8.8 - step * 0.01),
                DashTrailParticle {
                    velocity,
                    lifetime,
                    max_lifetime: lifetime,
                },
            ));
        }
    }
}

pub fn update_dash_trail(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DashTrailParticle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();

    for (entity, mut particle, mut transform, mut sprite) in &mut query {
        particle.lifetime -= dt;
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += (particle.velocity * dt).extend(0.0);
        let life_ratio = particle.lifetime / particle.max_lifetime;
        let mut color = sprite.color.to_srgba();
        color.alpha *= life_ratio;
        sprite.color = Color::Srgba(color);

        if let Some(size) = &mut sprite.custom_size {
            *size *= 0.985;
        }
    }
}