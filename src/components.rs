use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{AlphaMode2d, Material2d};

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Grounded(pub bool);

#[derive(Component, PartialEq, Debug, Clone, Copy)]
pub enum WallContact {
    None,
    Left,
    Right,
}

#[derive(Component)]
pub struct Facing(pub f32);

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct JumpState {
    pub jumps_remaining: u8,
}

#[derive(Component)]
pub struct WallJumpTimer(pub f32);

#[derive(Component)]
pub struct DashState {
    pub is_dashing: bool,
    pub timer: f32,
    pub direction: Vec2,
    pub dashes_remaining: u8,
}

#[derive(Component)]
pub struct DashTrailEmitter {
    pub cooldown: f32,
    pub was_dashing: bool,
}

#[derive(Component)]
pub struct Crouching(pub bool);

#[derive(Component)]
pub struct ColliderSize(pub Vec2);

#[derive(Component, Default)]
pub struct MovementInput {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct AnimationTimer(pub Timer);

#[derive(Component)]
pub struct DashTrailParticle {
    pub velocity: Vec2,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

#[derive(Component)]
pub struct Hair {
    pub sim_positions: Vec<Vec2>,
    pub entities: Vec<Entity>,
    pub bangs_entity: Option<Entity>,
}

impl Default for Hair {
    fn default() -> Self {
        Self {
            sim_positions: vec![Vec2::ZERO; 5],
            entities: Vec::new(),
            bangs_entity: None,
        }
    }
}

#[derive(Component)]
pub struct HairSegment;

#[derive(Component)]
pub struct HairBangs;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct HairMaterial {
    #[uniform(0)]
    pub fill_color: Vec4,
    #[uniform(1)]
    pub outline_color: Vec4,
    #[uniform(2)]
    pub effect_params: Vec4,
}

impl Material2d for HairMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/hair_outline.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component)]
pub struct WeatherOverlay;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WeatherMaterial {
    #[uniform(0)]
    pub weather_data: Vec4,
}

impl Material2d for WeatherMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/weather.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AnimationState {
    Idle,
    Run,
    Duck,
    Climb,
    ClimbLookback,
}

#[derive(Component)]
pub struct PlayerAnimations {
    pub idle_texture: Handle<Image>,
    pub idle_layout: Handle<TextureAtlasLayout>,
    pub run_texture: Handle<Image>,
    pub run_layout: Handle<TextureAtlasLayout>,
    pub duck_texture: Handle<Image>,
    pub climb_texture: Handle<Image>,
    pub climb_layout: Handle<TextureAtlasLayout>,
    pub climb_lookback_texture: Handle<Image>,
    pub climb_lookback_layout: Handle<TextureAtlasLayout>,
}