use bevy::prelude::*;

use crate::constants::{BANGS_OFFSET, HAIR_ROOT_OFFSET, HAIR_SEGMENT_SIZES};

pub fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

pub fn check_collision(a_pos: Vec3, a_size: Vec2, b_pos: Vec3, b_size: Vec2) -> bool {
    let a_min = a_pos.truncate() - a_size / 2.0;
    let a_max = a_pos.truncate() + a_size / 2.0;
    let b_min = b_pos.truncate() - b_size / 2.0;
    let b_max = b_pos.truncate() + b_size / 2.0;
    a_min.x < b_max.x && a_max.x > b_min.x && a_min.y < b_max.y && a_max.y > b_min.y
}

pub fn can_use_collider(player_pos: Vec3, player_size: Vec2, obstacles: &[(Vec3, Vec2)]) -> bool {
    !obstacles.iter().any(|(ground_pos, ground_size)| {
        check_collision(player_pos, player_size, *ground_pos, *ground_size)
    })
}

pub fn dash_effect_color(_dashes_remaining: u8) -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.92)
}

pub fn color_to_vec4(color: Color) -> Vec4 {
    let linear = color.to_linear();
    Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
}

pub fn mirrored_offset(offset: Vec2, facing: f32) -> Vec2 {
    if facing < 0.0 {
        Vec2::new(-offset.x, offset.y)
    } else {
        offset
    }
}

pub fn hair_rest_offset(index: usize, facing: f32, motion_drag: Vec2) -> Vec2 {
    let step = index as f32;
    let horizontal = 1.0 + step * 0.9;
    let vertical = 0.6 + step * 1.15;
    let arc_lift = (2.0 - step).max(0.0) * 0.35;

    Vec2::new(-facing * horizontal, -vertical + arc_lift) + motion_drag * (0.3 + step * 0.12)
}

pub fn initial_hair_positions(player_pos: Vec2, facing: f32) -> Vec<Vec2> {
    let root_pos = player_pos + mirrored_offset(HAIR_ROOT_OFFSET, facing);
    let motion_drag = Vec2::ZERO;

    (0..HAIR_SEGMENT_SIZES.len())
        .map(|index| {
            if index == 0 {
                root_pos
            } else {
                root_pos + hair_rest_offset(index, facing, motion_drag)
            }
        })
        .collect()
}

pub fn bangs_base_offset() -> Vec2 {
    BANGS_OFFSET
}