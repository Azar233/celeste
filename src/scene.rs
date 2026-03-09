use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::sprite::Anchor;

use crate::components::{
    AnimationState, AnimationTimer, ColliderSize, Crouching, DashState, DashTrailEmitter, Facing,
    Ground, Grounded, Hair, HairBangs, HairMaterial, HairSegment, JumpState, MovementInput,
    Player, PlayerAnimations, Velocity, WallContact, WallJumpTimer,
};
use crate::constants::{
    BANGS_Z, HAIR_OUTLINE_WIDTH, HAIR_PIXEL_STEPS, HAIR_SEGMENT_SIZES, HAIR_SEGMENT_Z,
    PLAYER_COLLIDER_SIZE, SPAWN_POSITION,
};
use crate::utils::{color_to_vec4, initial_hair_positions};

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, debug_gizmos);
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut hair_materials: ResMut<Assets<HairMaterial>>,
) {
    let run_texture = asset_server.load("run_sheet.png");
    let run_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 12, 1, None, None);
    let run_layout_handle = texture_atlas_layouts.add(run_layout);

    let idle_texture = asset_server.load("idle_sheet.png");
    let idle_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 9, 1, None, None);
    let idle_layout_handle = texture_atlas_layouts.add(idle_layout);
    let duck_texture = asset_server.load("duck.png");
    let bangs_texture = asset_server.load("bangs.png");

    spawn_camera(&mut commands);

    let (hair_entities, bangs_entity) = spawn_hair_entities(
        &mut commands,
        bangs_texture,
        &mut meshes,
        &mut hair_materials,
    );

    spawn_player(
        &mut commands,
        idle_texture,
        idle_layout_handle,
        run_texture,
        run_layout_handle,
        duck_texture,
        hair_entities,
        bangs_entity,
    );
    spawn_level_geometry(&mut commands);
}

fn spawn_camera(commands: &mut Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedVertical {
        viewport_height: 180.0,
    };

    commands.spawn((Camera2d, projection));
}

fn spawn_hair_entities(
    commands: &mut Commands,
    bangs_texture: Handle<Image>,
    meshes: &mut ResMut<Assets<Mesh>>,
    hair_materials: &mut ResMut<Assets<HairMaterial>>,
) -> (Vec<Entity>, Entity) {
    let mut hair_entities = Vec::new();
    let hair_mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let hair_material = hair_materials.add(HairMaterial {
        fill_color: color_to_vec4(Color::srgb(0.9, 0.25, 0.3)),
        outline_color: color_to_vec4(Color::BLACK),
        effect_params: Vec4::new(HAIR_PIXEL_STEPS, HAIR_OUTLINE_WIDTH, 0.35, 0.0),
    });

    for size in HAIR_SEGMENT_SIZES {
        let id = commands
            .spawn((
                HairSegment,
                Mesh2d(hair_mesh.clone()),
                MeshMaterial2d(hair_material.clone()),
                Transform {
                    translation: Vec3::new(0.0, 0.0, HAIR_SEGMENT_Z),
                    scale: Vec3::splat(size),
                    ..default()
                },
            ))
            .id();
        hair_entities.push(id);
    }

    let bangs_entity = commands
        .spawn((
            HairBangs,
            Sprite {
                image: bangs_texture,
                color: Color::srgb(0.9, 0.25, 0.3),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, BANGS_Z),
        ))
        .id();

    (hair_entities, bangs_entity)
}

fn spawn_player(
    commands: &mut Commands,
    idle_texture: Handle<Image>,
    idle_layout_handle: Handle<TextureAtlasLayout>,
    run_texture: Handle<Image>,
    run_layout_handle: Handle<TextureAtlasLayout>,
    duck_texture: Handle<Image>,
    hair_entities: Vec<Entity>,
    bangs_entity: Entity,
) {
    let initial_hair_positions = initial_hair_positions(SPAWN_POSITION.truncate(), 1.0);

    let mut player = commands.spawn((
        Player,
        Velocity(Vec2::ZERO),
        Grounded(false),
        WallContact::None,
        Facing(1.0),
        MovementInput::default(),
        JumpState { jumps_remaining: 1 },
        WallJumpTimer(0.0),
        DashState {
            is_dashing: false,
            timer: 0.0,
            direction: Vec2::ZERO,
            dashes_remaining: 1,
        },
        DashTrailEmitter {
            cooldown: 0.0,
            was_dashing: false,
        },
        Hair {
            sim_positions: initial_hair_positions,
            entities: hair_entities,
            bangs_entity: Some(bangs_entity),
        },
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        AnimationState::Idle,
    ));

    player.insert((
        Crouching(false),
        ColliderSize(PLAYER_COLLIDER_SIZE),
        PlayerAnimations {
            idle_texture: idle_texture.clone(),
            idle_layout: idle_layout_handle.clone(),
            run_texture: run_texture.clone(),
            run_layout: run_layout_handle.clone(),
            duck_texture,
        },
        Sprite {
            image: idle_texture,
            texture_atlas: Some(TextureAtlas {
                layout: idle_layout_handle,
                index: 0,
            }),
            anchor: Anchor::Custom(Vec2::new(0.0, -0.235)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 10.0),
    ));
}

fn spawn_level_geometry(commands: &mut Commands) {
    commands.spawn((
        Ground,
        Sprite {
            color: Color::srgb(0.2, 0.2, 0.2),
            custom_size: Some(Vec2::new(400.0, 24.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -80.0, 0.0),
    ));

    commands.spawn((
        Ground,
        Sprite {
            color: Color::srgb(0.4, 0.4, 0.4),
            custom_size: Some(Vec2::new(16.0, 200.0)),
            ..default()
        },
        Transform::from_xyz(-100.0, 0.0, 0.0),
    ));

    commands.spawn((
        Ground,
        Sprite {
            color: Color::srgb(0.4, 0.4, 0.4),
            custom_size: Some(Vec2::new(16.0, 150.0)),
            ..default()
        },
        Transform::from_xyz(80.0, -20.0, 0.0),
    ));
}

pub fn debug_gizmos(mut gizmos: Gizmos, query: Query<(&Transform, &ColliderSize), With<Player>>) {
    for (transform, collider_size) in &query {
        gizmos.rect_2d(
            transform.translation.truncate(),
            collider_size.0,
            Color::srgb(0.0, 1.0, 0.0),
        );
    }
}