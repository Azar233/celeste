use bevy::prelude::*;

use crate::components::{
    AnimationState, DashState, Facing, Ground, Hair, HairBangs, HairMaterial, HairSegment,
    Player, Velocity,
};
use crate::constants::{
    BANGS_Z, HAIR_FOLLOW_STRENGTH, HAIR_GRAVITY, HAIR_RESET_DISTANCE, HAIR_ROOT_OFFSET,
    HAIR_SEGMENT_LEN, HAIR_SEGMENT_SIZES, HAIR_SEGMENT_Z,
};
use crate::utils::{bangs_base_offset, color_to_vec4, hair_rest_offset, initial_hair_positions, mirrored_offset};

pub fn update_hair(
    time: Res<Time>,
    mut materials: ResMut<Assets<HairMaterial>>,
    mut player_query: Query<(
        &Transform,
        &mut Hair,
        &Facing,
        &AnimationState,
        &Sprite,
        &Velocity,
        &DashState,
    ), With<Player>>,
    mut hair_render_queries: ParamSet<(
        Query<&mut Transform, (With<HairSegment>, Without<Player>)>,
        Query<&MeshMaterial2d<HairMaterial>, (With<HairSegment>, Without<Player>)>,
        Query<(&mut Transform, &mut Sprite), (With<HairBangs>, Without<Player>, Without<Ground>)>,
    )>,
) {
    let dt = time.delta_secs();

    for (player_transform, mut hair, facing, anim_state, sprite, velocity, dash_state) in &mut player_query {
        let target_color = if dash_state.dashes_remaining > 0 {
            Color::srgb(0.9, 0.25, 0.3)
        } else {
            Color::srgb(0.3, 0.7, 0.95)
        };

        let frame_index = sprite.texture_atlas.as_ref().map(|atlas| atlas.index).unwrap_or(0);

        let hair_anim_offset = match anim_state {
            AnimationState::Idle => match frame_index {
                0..=3 => Vec2::new(0.0, -2.0),
                4..=8 => Vec2::new(0.0, -1.0),
                _ => Vec2::ZERO,
            },
            AnimationState::Run => match frame_index % 4 {
                0 | 2 => Vec2::new(0.0, -1.0),
                1 => Vec2::new(0.0, 0.0),
                3 => Vec2::new(0.0, -2.0),
                _ => Vec2::ZERO,
            },
            AnimationState::Duck => Vec2::new(0.0, -4.0),
            AnimationState::Dash => Vec2::new(0.0, -2.0),
            AnimationState::JumpSlow
            | AnimationState::JumpFast
            | AnimationState::FallSlow
            | AnimationState::FallFast => Vec2::new(0.0, -2.0),
            AnimationState::Climb | AnimationState::ClimbLookback => Vec2::new(0.0, -1.0),
        };

        let bangs_anim_offset = match anim_state {
            AnimationState::Idle => hair_anim_offset,
            AnimationState::Run => Vec2::ZERO,
            AnimationState::Duck => Vec2::new(0.0, -4.0),
            AnimationState::Dash => Vec2::new(0.0, -2.0),
            AnimationState::JumpSlow
            | AnimationState::JumpFast
            | AnimationState::FallSlow
            | AnimationState::FallFast => Vec2::new(0.0, -2.0),
            AnimationState::Climb | AnimationState::ClimbLookback => Vec2::new(0.0, -1.0),
        };

        let root_pos = player_transform.translation.truncate()
            + mirrored_offset(HAIR_ROOT_OFFSET, facing.0)
            + hair_anim_offset;
        let bangs_pos = player_transform.translation.truncate()
            + mirrored_offset(bangs_base_offset(), facing.0)
            + bangs_anim_offset;
        let motion_drag = Vec2::new(
            (-velocity.0.x * 0.03).clamp(-3.0, 3.0),
            (-velocity.0.y * 0.012).clamp(-2.0, 2.5),
        );

        if hair.sim_positions.len() != HAIR_SEGMENT_SIZES.len()
            || hair.sim_positions[0].distance(root_pos) > HAIR_RESET_DISTANCE
        {
            hair.sim_positions = initial_hair_positions(player_transform.translation.truncate(), facing.0);
        }

        if let Some(bangs_entity) = hair.bangs_entity {
            if let Ok((mut bangs_transform, mut bangs_sprite)) = hair_render_queries.p2().get_mut(bangs_entity) {
                bangs_transform.translation = bangs_pos.extend(BANGS_Z);
                bangs_sprite.color = target_color;
                bangs_sprite.flip_x = facing.0 < 0.0;
            }
        }

        if hair.sim_positions.len() < HAIR_SEGMENT_SIZES.len() {
            continue;
        }
        hair.sim_positions[0] = root_pos;

        for index in 1..hair.sim_positions.len() {
            let prev_pos = hair.sim_positions[index - 1];
            let mut curr_pos = hair.sim_positions[index];
            let rest_target = root_pos + hair_rest_offset(index, facing.0, motion_drag);

            let wind_force = Vec2::new(-facing.0 * 14.0, 0.0);
            let force = (HAIR_GRAVITY + wind_force) * dt;
            curr_pos = curr_pos.lerp(rest_target, (HAIR_FOLLOW_STRENGTH * dt).min(1.0));
            curr_pos += force * dt * 16.0;

            if dash_state.is_dashing {
                curr_pos += Vec2::new(-dash_state.direction.x, -dash_state.direction.y) * (0.18 * index as f32);
            }

            let diff = curr_pos - prev_pos;
            let dist = diff.length();

            if dist > HAIR_SEGMENT_LEN {
                curr_pos = prev_pos + diff.normalize() * HAIR_SEGMENT_LEN;
            } else if dist < HAIR_SEGMENT_LEN * 0.55 {
                curr_pos = prev_pos + (rest_target - prev_pos).normalize_or_zero() * HAIR_SEGMENT_LEN;
            }

            hair.sim_positions[index] = curr_pos;
        }

        for (index, entity) in hair.entities.iter().enumerate() {
            if let Ok(mut transform) = hair_render_queries.p0().get_mut(*entity) {
                transform.translation = hair.sim_positions[index].extend(HAIR_SEGMENT_Z);
            }

            if let Ok(mat_handle) = hair_render_queries.p1().get(*entity) {
                if let Some(material) = materials.get_mut(mat_handle) {
                    material.fill_color = color_to_vec4(target_color);
                }
            }
        }
    }
}