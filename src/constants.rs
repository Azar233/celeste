use bevy::prelude::*;

pub const PLAYER_COLLIDER_SIZE: Vec2 = Vec2::new(10.0, 18.0);
pub const CROUCH_COLLIDER_SIZE: Vec2 = Vec2::new(10.0, 9.0);
pub const MAX_RUN_SPEED: f32 = 90.0;
pub const GRAVITY: f32 = 900.0;
pub const JUMP_VELOCITY: f32 = 280.0;
pub const LOW_JUMP_MULTIPLIER: f32 = 4.0;
pub const FALL_MULTIPLIER: f32 = 1.2;

pub const GROUND_ACCEL: f32 = 1000.0;
pub const GROUND_FRICTION: f32 = 800.0;
pub const GROUND_TURN_FRICTION: f32 = 1200.0;
pub const AIR_ACCEL: f32 = 800.0;
pub const AIR_FRICTION: f32 = 200.0;
pub const AIR_TURN_FRICTION: f32 = 1000.0;

pub const WALL_SLIDE_SPEED: f32 = 40.0;
pub const WALL_CLIMB_SPEED: f32 = 80.0;
pub const WALL_KICK_FORCE: Vec2 = Vec2::new(140.0, 290.0);
pub const WALL_KICK_LOCK: f32 = 0.20;
pub const WALL_NEUTRAL_FORCE: Vec2 = Vec2::new(80.0, 300.0);
pub const WALL_NEUTRAL_LOCK: f32 = 0.0;
pub const WALL_CLIMB_JUMP_FORCE_Y: f32 = 260.0;
pub const WALL_CLIMB_LOCK: f32 = 0.1;

pub const DASH_SPEED: f32 = 300.0;
pub const DASH_DURATION: f32 = 0.15;
pub const DASH_END_MULTIPLIER: f32 = 0.6;
pub const DASH_TRAIL_INTERVAL: f32 = 0.025;
pub const DASH_TRAIL_PARTICLE_COUNT: usize = 30;
pub const DASH_TRAIL_SPACING: f32 = 1.15;
pub const DASH_TRAIL_LIFETIME: f32 = 0.15;

pub const HAIR_SEGMENT_LEN: f32 = 1.7;
pub const HAIR_GRAVITY: Vec2 = Vec2::new(0.0, -100.0);
pub const HAIR_SEGMENT_SIZES: [f32; 5] = [6.0, 5.5, 5.0, 4.5, 4.0];
pub const HAIR_PIXEL_STEPS: f32 = 3.0;
pub const HAIR_OUTLINE_WIDTH: f32 = 0.2;
pub const HAIR_FOLLOW_STRENGTH: f32 = 14.0;
pub const HAIR_RESET_DISTANCE: f32 = 28.0;
pub const HAIR_ROOT_OFFSET: Vec2 = Vec2::new(-3.0, 3.5);
pub const BANGS_OFFSET: Vec2 = Vec2::new(0.0, 5.0);
pub const BANGS_Z: f32 = 9.6;
pub const HAIR_SEGMENT_Z: f32 = 9.0;

pub const DEATH_THRESHOLD: f32 = -200.0;
pub const SPAWN_POSITION: Vec3 = Vec3::new(0.0, 0.0, 0.0);