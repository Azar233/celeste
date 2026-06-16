use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;

use bevy::math::URect;
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::sprite::Anchor;

use crate::app_state::{GameState, PendingMapPath};
use crate::components::{
    AnimationState, AnimationTimer, CheckpointMarker, ClimbStamina, ClimbTopOutState, ColliderSize,
    CompletionZone, CornerBoostState, Crouching, DashCrystal, DashSlideState, DashState, DashTrailEmitter, Facing,
    GameplayEntity, Ground, Grounded, Hair, HairBangs, HairMaterial, HairSegment, Hazard, JumpState,
    LevelEntity, MovementInput, Player, PlayerActionInput, PlayerAnimations, PlayerState,
    PlayerStateMachine, RoomExitMarker, Velocity, WallContact, WallJumpTimer, WeatherMaterial,
    WeatherOverlay,
};
use crate::constants::{
    BACKGROUND_Z, BANGS_Z, CLIMB_STAMINA_MAX, DASH_CRYSTAL_SIZE, DASH_CRYSTAL_Z,
    HAIR_OUTLINE_WIDTH, HAIR_PIXEL_STEPS, HAIR_SEGMENT_SIZES, HAIR_SEGMENT_Z,
    PLAYER_COLLIDER_SIZE, PLAYER_RENDER_Z, WEATHER_OVERLAY_SIZE, WEATHER_OVERLAY_Z,
};
use crate::level::{
    ActiveRoom, CollisionKind, CollisionRect, DEFAULT_MAP_PATH, DEFAULT_TILESET_ART_TAG, ExitSide,
    LoadedMap, RectData, RoomData, TILESET_ART_TAGS, load_map_from_path,
    normalize_tileset_art_tag,
};
use crate::utils::{color_to_vec4, initial_hair_positions};

const TILE_SIZE: f32 = 8.0;
const CHAPTER_02_MAP_PATH: &str = "assets/maps/chapter_02.json";
const GROUND_TILE_COLUMNS: usize = 6;
const GROUND_TILE_ROWS: usize = 15;
const DANGER_UPDOWN_SIZE: Vec2 = Vec2::new(10.0, 9.0);
const DANGER_LEFTRIGHT_SIZE: Vec2 = Vec2::new(9.0, 10.0);
const DEATH_SHEET_SIZE: UVec2 = UVec2::new(224, 26);
const DEATH_FRAME_RECTS: [URect; 13] = [
    URect {
        min: UVec2::new(1, 5),
        max: UVec2::new(20, 26),
    },
    URect {
        min: UVec2::new(22, 7),
        max: UVec2::new(39, 25),
    },
    URect {
        min: UVec2::new(46, 9),
        max: UVec2::new(56, 20),
    },
    URect {
        min: UVec2::new(67, 9),
        max: UVec2::new(76, 19),
    },
    URect {
        min: UVec2::new(83, 4),
        max: UVec2::new(100, 23),
    },
    URect {
        min: UVec2::new(103, 4),
        max: UVec2::new(110, 23),
    },
    URect {
        min: UVec2::new(115, 4),
        max: UVec2::new(122, 23),
    },
    URect {
        min: UVec2::new(123, 3),
        max: UVec2::new(131, 21),
    },
    URect {
        min: UVec2::new(134, 7),
        max: UVec2::new(142, 25),
    },
    URect {
        min: UVec2::new(144, 2),
        max: UVec2::new(162, 26),
    },
    URect {
        min: UVec2::new(164, 2),
        max: UVec2::new(183, 26),
    },
    URect {
        min: UVec2::new(184, 3),
        max: UVec2::new(203, 25),
    },
    URect {
        min: UVec2::new(207, 5),
        max: UVec2::new(221, 22),
    },
];

#[derive(Clone)]
pub struct TilesetArt {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

#[derive(Resource, Clone)]
pub struct LevelArt {
    pub default_backgrounds: Vec<Handle<Image>>,
    pub chapter2_backgrounds: Vec<Handle<Image>>,
    pub current_map_path: std::path::PathBuf,
    pub tilesets: HashMap<String, TilesetArt>,
    pub danger_up: Handle<Image>,
    pub danger_down: Handle<Image>,
    pub danger_left: Handle<Image>,
    pub danger_right: Handle<Image>,
    pub dash_crystal_frames: Vec<Handle<Image>>,
    pub dash_crystal_vanished: Handle<Image>,
}

#[derive(Clone, Copy)]
enum HazardDirection {
    Up,
    Down,
    Left,
    Right,
}

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), setup)
            .add_systems(OnExit(GameState::InGame), cleanup_gameplay_entities)
            .add_systems(Update, debug_gizmos.run_if(in_state(GameState::InGame)));
    }
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut hair_materials: ResMut<Assets<HairMaterial>>,
    mut weather_materials: ResMut<Assets<WeatherMaterial>>,
    mut pending_map_path: ResMut<PendingMapPath>,
) {
    let run_texture = asset_server.load("run_sheet.png");
    let run_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 12, 1, None, None);
    let run_layout_handle = texture_atlas_layouts.add(run_layout);

    let idle_texture = asset_server.load("idle_sheet.png");
    let idle_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 9, 1, None, None);
    let idle_layout_handle = texture_atlas_layouts.add(idle_layout);
    let duck_texture = asset_server.load("duck.png");
    let dash_texture = asset_server.load("dash_sheet.png");
    let dash_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 4, 1, None, None);
    let dash_layout_handle = texture_atlas_layouts.add(dash_layout);
    let jump_slow_texture = asset_server.load("jumpslow_sheet.png");
    let jump_slow_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 2, 1, None, None);
    let jump_slow_layout_handle = texture_atlas_layouts.add(jump_slow_layout);
    let jump_fast_texture = asset_server.load("jumpfast_sheet.png");
    let jump_fast_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 2, 1, None, None);
    let jump_fast_layout_handle = texture_atlas_layouts.add(jump_fast_layout);
    let fall_slow_texture = asset_server.load("fallslow_sheet.png");
    let fall_slow_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 2, 1, None, None);
    let fall_slow_layout_handle = texture_atlas_layouts.add(fall_slow_layout);
    let fall_fast_texture = asset_server.load("fallfast_sheet.png");
    let fall_fast_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 2, 1, None, None);
    let fall_fast_layout_handle = texture_atlas_layouts.add(fall_fast_layout);
    let climb_texture = asset_server.load("climb_sheet.png");
    let climb_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 6, 1, None, None);
    let climb_layout_handle = texture_atlas_layouts.add(climb_layout);
    let climb_lookback_texture = asset_server.load("climb_lookback_sheet.png");
    let climb_lookback_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 3, 1, None, None);
    let climb_lookback_layout_handle = texture_atlas_layouts.add(climb_lookback_layout);
    let death_texture = asset_server.load("figs/death/death.png");
    let mut death_layout = TextureAtlasLayout::new_empty(DEATH_SHEET_SIZE);
    for rect in DEATH_FRAME_RECTS {
        death_layout.add_texture(rect);
    }
    let death_layout_handle = texture_atlas_layouts.add(death_layout);
    let bangs_texture = asset_server.load("bangs.png");

    let mut tilesets = HashMap::new();
    for art_tag in TILESET_ART_TAGS {
        tilesets.insert(
            art_tag.to_string(),
            TilesetArt {
                texture: asset_server.load(format!("figs/tilesets/{art_tag}.png")),
                layout: texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::splat(TILE_SIZE as u32),
                    GROUND_TILE_COLUMNS as u32,
                    GROUND_TILE_ROWS as u32,
                    None,
                    None,
                )),
            },
        );
    }

    let map_path = pending_map_path
        .path
        .take()
        .unwrap_or_else(|| DEFAULT_MAP_PATH.into());
    let level_art = LevelArt {
        default_backgrounds: vec![
            asset_server.load("figs/bgs/01/bg0.png"),
            asset_server.load("figs/bgs/01/bg1.png"),
            asset_server.load("figs/bgs/01/bg2.png"),
        ],
        chapter2_backgrounds: vec![
            asset_server.load("figs/bgs/07/07/bg00.png"),
            asset_server.load("figs/bgs/07/07/bg01.png"),
            asset_server.load("figs/bgs/07/07/bg02.png"),
            asset_server.load("figs/bgs/07/07/bg03.png"),
            asset_server.load("figs/bgs/07/07/bg04.png"),
        ],
        current_map_path: map_path.clone(),
        tilesets,
        danger_up: asset_server.load("figs/danger/outline_up00.png"),
        danger_down: asset_server.load("figs/danger/outline_down00.png"),
        danger_left: asset_server.load("figs/danger/outline_left00.png"),
        danger_right: asset_server.load("figs/danger/outline_right00.png"),
        dash_crystal_frames: vec![
            asset_server.load("figs/DashCystal/F1.png"),
            asset_server.load("figs/DashCystal/F2.png"),
            asset_server.load("figs/DashCystal/F3.png"),
            asset_server.load("figs/DashCystal/F4.png"),
            asset_server.load("figs/DashCystal/F5.png"),
        ],
        dash_crystal_vanished: asset_server.load("figs/DashCystal/vanished.png"),
    };

    let map = load_map_from_path(&map_path)
        .unwrap_or_else(|error| panic!("unable to load initial map data: {error}"));
    let room = map
        .starting_room()
        .unwrap_or_else(|| panic!("map '{}' is missing its start room", map.id));
    let spawn_position = room.default_spawn_point().unwrap_or_else(|| {
        panic!(
            "room '{}' is missing default spawn point '{}'",
            room.id, room.default_spawn
        )
    });

    commands.insert_resource(LoadedMap {
        data: map.clone(),
        path: map_path,
    });
    commands.insert_resource(ActiveRoom {
        map_id: map.id.clone(),
        room_id: room.id.clone(),
        respawn_point: spawn_position,
    });
    commands.insert_resource(level_art.clone());

    spawn_camera(&mut commands);
    spawn_weather_overlay(&mut commands, &mut meshes, &mut weather_materials);

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
        dash_texture,
        dash_layout_handle,
        jump_slow_texture,
        jump_slow_layout_handle,
        jump_fast_texture,
        jump_fast_layout_handle,
        fall_slow_texture,
        fall_slow_layout_handle,
        fall_fast_texture,
        fall_fast_layout_handle,
        climb_texture,
        climb_layout_handle,
        climb_lookback_texture,
        climb_lookback_layout_handle,
        death_texture,
        death_layout_handle,
        spawn_position,
        hair_entities,
        bangs_entity,
    );
    spawn_room_geometry(&mut commands, room, &level_art);
}

fn spawn_camera(commands: &mut Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedVertical {
        viewport_height: 180.0,
    };

    commands.spawn((GameplayEntity, Camera2d, projection, IsDefaultUiCamera));
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
                GameplayEntity,
                HairSegment,
                Mesh2d(hair_mesh.clone()),
                MeshMaterial2d(hair_material.clone()),
                Visibility::Visible,
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
            GameplayEntity,
            HairBangs,
            Visibility::Visible,
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
    dash_texture: Handle<Image>,
    dash_layout_handle: Handle<TextureAtlasLayout>,
    jump_slow_texture: Handle<Image>,
    jump_slow_layout_handle: Handle<TextureAtlasLayout>,
    jump_fast_texture: Handle<Image>,
    jump_fast_layout_handle: Handle<TextureAtlasLayout>,
    fall_slow_texture: Handle<Image>,
    fall_slow_layout_handle: Handle<TextureAtlasLayout>,
    fall_fast_texture: Handle<Image>,
    fall_fast_layout_handle: Handle<TextureAtlasLayout>,
    climb_texture: Handle<Image>,
    climb_layout_handle: Handle<TextureAtlasLayout>,
    climb_lookback_texture: Handle<Image>,
    climb_lookback_layout_handle: Handle<TextureAtlasLayout>,
    death_texture: Handle<Image>,
    death_layout_handle: Handle<TextureAtlasLayout>,
    spawn_position: Vec2,
    hair_entities: Vec<Entity>,
    bangs_entity: Entity,
) {
    let initial_hair_positions = initial_hair_positions(spawn_position, 1.0);

    let mut player = commands.spawn((
        Player,
        Velocity(Vec2::ZERO),
        Grounded(false),
        WallContact::None,
        Facing(1.0),
        MovementInput::default(),
        PlayerActionInput::default(),
        JumpState {
            jump_grace_timer: 0.0,
            jump_buffer_timer: 0.0,
            super_jump_timer: 0.0,
            fast_jump_active: false,
        },
        WallJumpTimer(0.0),
        PlayerStateMachine {
            current: PlayerState::Normal,
            previous: PlayerState::Normal,
        },
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
        GameplayEntity,
        Crouching(false),
        ColliderSize(PLAYER_COLLIDER_SIZE),
        ClimbStamina {
            current: CLIMB_STAMINA_MAX,
            low_flash_timer: 0.0,
        },
        ClimbTopOutState {
            active: false,
            timer: 0.0,
            duration: 0.0,
            start: Vec3::ZERO,
            target: Vec3::ZERO,
        },
        CornerBoostState::default(),
        DashSlideState {
            timer: 0.0,
            direction: 0.0,
        },
        PlayerAnimations {
            idle_texture: idle_texture.clone(),
            idle_layout: idle_layout_handle.clone(),
            run_texture: run_texture.clone(),
            run_layout: run_layout_handle.clone(),
            duck_texture,
            dash_texture,
            dash_layout: dash_layout_handle,
            jump_slow_texture,
            jump_slow_layout: jump_slow_layout_handle,
            jump_fast_texture,
            jump_fast_layout: jump_fast_layout_handle,
            fall_slow_texture,
            fall_slow_layout: fall_slow_layout_handle,
            fall_fast_texture,
            fall_fast_layout: fall_fast_layout_handle,
            climb_texture,
            climb_layout: climb_layout_handle,
            climb_lookback_texture,
            climb_lookback_layout: climb_lookback_layout_handle,
            death_texture,
            death_layout: death_layout_handle,
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
        Transform::from_xyz(spawn_position.x, spawn_position.y, PLAYER_RENDER_Z),
    ));
}

pub fn spawn_room_geometry(commands: &mut Commands, room: &RoomData, level_art: &LevelArt) {
    spawn_room_background(commands, room, level_art);

    let solid_cells = collect_solid_cells(room);

    for collision in &room.collision {
        let color = match collision.kind {
            CollisionKind::SolidGround
            | CollisionKind::WallSurface
            | CollisionKind::OneWayPlatform => Color::NONE,
            CollisionKind::CameraZone | CollisionKind::EffectZone => Color::NONE,
        };

        let sprite = Sprite {
            color,
            custom_size: Some(Vec2::new(collision.w, collision.h)),
            ..default()
        };
        let transform = Transform::from_xyz(collision.x, collision.y, 0.0);

        match collision.kind {
            CollisionKind::SolidGround
            | CollisionKind::WallSurface
            | CollisionKind::OneWayPlatform => {
                commands.spawn((Ground, LevelEntity, sprite, transform));
                spawn_ground_tiles(commands, collision, &solid_cells, level_art);
            }
            CollisionKind::CameraZone | CollisionKind::EffectZone => {
                commands.spawn((LevelEntity, sprite, transform));
            }
        }
    }

    for hazard in &room.hazards {
        commands.spawn((
            Hazard,
            LevelEntity,
            Sprite {
                color: Color::NONE,
                custom_size: Some(hazard.size()),
                ..default()
            },
            Transform::from_xyz(hazard.x, hazard.y, 0.0),
        ));
        spawn_hazard_tiles(commands, hazard, &solid_cells, level_art);
    }

    for completion_zone in &room.completion_zones {
        commands.spawn((
            CompletionZone,
            LevelEntity,
            Sprite {
                color: Color::NONE,
                custom_size: Some(completion_zone.size()),
                ..default()
            },
            Transform::from_xyz(completion_zone.x, completion_zone.y, 0.0),
        ));
    }

    for checkpoint in &room.checkpoints {
        commands.spawn((
            CheckpointMarker {
                id: checkpoint.id.clone(),
            },
            LevelEntity,
            Sprite {
                color: Color::srgb(0.95, 0.85, 0.2),
                custom_size: Some(Vec2::new(10.0, 18.0)),
                ..default()
            },
            Transform::from_xyz(checkpoint.x, checkpoint.y, 0.0),
        ));
    }

    for dashcrystal in &room.dashcrystals {
        commands.spawn((
            DashCrystal {
                id: dashcrystal.id.clone(),
                respawn_timer: 0.0,
                animation_timer: 0.0,
                frame_index: 0,
                active_frames: level_art.dash_crystal_frames.clone(),
                vanished_frame: level_art.dash_crystal_vanished.clone(),
            },
            LevelEntity,
            Sprite {
                image: level_art.dash_crystal_frames[0].clone(),
                custom_size: Some(DASH_CRYSTAL_SIZE),
                ..default()
            },
            Transform::from_xyz(dashcrystal.x, dashcrystal.y, DASH_CRYSTAL_Z),
        ));
    }

    for exit in &room.exits {
        let tint = match exit.side {
            ExitSide::Left | ExitSide::Right => Color::srgba(0.3, 0.9, 0.5, 0.25),
            ExitSide::Top | ExitSide::Bottom => Color::srgba(0.3, 0.6, 1.0, 0.25),
        };

        commands.spawn((
            RoomExitMarker {
                id: exit.id.clone(),
                target_room: exit.target_room.clone(),
                target_spawn: exit.target_spawn.clone(),
                preserve_momentum: exit.preserve_momentum,
            },
            LevelEntity,
            Sprite {
                color: tint,
                custom_size: Some(Vec2::new(exit.w, exit.h)),
                ..default()
            },
            Transform::from_xyz(exit.x, exit.y, 0.0),
        ));
    }

    commands.spawn((
        LevelEntity,
        Name::new(format!("room_bounds:{}", room.id)),
        Sprite {
            color: Color::srgba(1.0, 1.0, 1.0, 0.05),
            custom_size: Some(room.bounds.size()),
            ..default()
        },
        Transform::from_xyz(room.bounds.x, room.bounds.y, -1.0),
    ));
}

fn spawn_room_background(commands: &mut Commands, room: &RoomData, level_art: &LevelArt) {
    for (index, background) in level_art.backgrounds().iter().enumerate() {
        commands.spawn((
            LevelEntity,
            Name::new(format!("room_background:{}:{}", room.id, index)),
            Sprite {
                image: background.clone(),
                custom_size: Some(room.bounds.size()),
                ..default()
            },
            Transform::from_xyz(room.bounds.x, room.bounds.y, BACKGROUND_Z + index as f32 * 0.1),
        ));
    }
}

fn collect_solid_cells(room: &RoomData) -> HashSet<(i32, i32)> {
    let mut cells = HashSet::new();

    for collision in &room.collision {
        if !matches!(
            collision.kind,
            CollisionKind::SolidGround | CollisionKind::WallSurface | CollisionKind::OneWayPlatform
        ) {
            continue;
        }

        let (min_x, max_x, min_y, max_y) =
            rect_to_grid_bounds(collision.x, collision.y, collision.w, collision.h);
        for gx in min_x..=max_x {
            for gy in min_y..=max_y {
                cells.insert((gx, gy));
            }
        }
    }

    cells
}

fn rect_to_grid_bounds(x: f32, y: f32, w: f32, h: f32) -> (i32, i32, i32, i32) {
    let min_x = ((x - w * 0.5) / TILE_SIZE).floor() as i32;
    let max_x = ((x + w * 0.5) / TILE_SIZE).ceil() as i32 - 1;
    let min_y = ((y - h * 0.5) / TILE_SIZE).floor() as i32;
    let max_y = ((y + h * 0.5) / TILE_SIZE).ceil() as i32 - 1;
    (min_x, max_x, min_y, max_y)
}

fn spawn_ground_tiles(
    commands: &mut Commands,
    collision: &CollisionRect,
    solid_cells: &HashSet<(i32, i32)>,
    level_art: &LevelArt,
) {
    let (min_x, max_x, min_y, max_y) =
        rect_to_grid_bounds(collision.x, collision.y, collision.w, collision.h);
    let tileset = level_art.tileset_for_tag(collision.art_tag.as_deref());

    for gx in min_x..=max_x {
        for gy in min_y..=max_y {
            if !solid_cells.contains(&(gx, gy)) {
                continue;
            }

            let atlas_index = choose_dirt_tile(tile_exposure_mask(gx, gy, solid_cells), gx, gy);
            let world_x = gx as f32 * TILE_SIZE + TILE_SIZE * 0.5;
            let world_y = gy as f32 * TILE_SIZE + TILE_SIZE * 0.5;

            commands.spawn((
                LevelEntity,
                Sprite {
                    image: tileset.texture.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: tileset.layout.clone(),
                        index: atlas_index,
                    }),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, 0.2),
            ));
        }
    }
}

impl LevelArt {
    pub fn set_current_map_path(&mut self, path: impl Into<std::path::PathBuf>) {
        self.current_map_path = path.into();
    }

    fn backgrounds(&self) -> &[Handle<Image>] {
        if is_chapter_02_map_path(&self.current_map_path) {
            &self.chapter2_backgrounds
        } else {
            &self.default_backgrounds
        }
    }

    fn tileset_for_tag(&self, art_tag: Option<&str>) -> &TilesetArt {
        art_tag
            .and_then(normalize_tileset_art_tag)
            .and_then(|tag| self.tilesets.get(tag))
            .or_else(|| self.tilesets.get(DEFAULT_TILESET_ART_TAG))
            .expect("default dirt tileset should be loaded")
    }
}

pub fn is_chapter_02_map_path(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .to_string_lossy()
        .replace('\\', "/")
        .ends_with(CHAPTER_02_MAP_PATH)
}

fn tile_exposure_mask(gx: i32, gy: i32, solid_cells: &HashSet<(i32, i32)>) -> u8 {
    let mut mask = 0u8;

    if !solid_cells.contains(&(gx, gy + 1)) {
        mask |= 1;
    }
    if !solid_cells.contains(&(gx, gy - 1)) {
        mask |= 2;
    }
    if !solid_cells.contains(&(gx - 1, gy)) {
        mask |= 4;
    }
    if !solid_cells.contains(&(gx + 1, gy)) {
        mask |= 8;
    }

    mask
}

fn choose_dirt_tile(mask: u8, gx: i32, gy: i32) -> usize {
    if mask == 0 {
        return  5;
        //let seed = (gx.wrapping_mul(73856093) ^ gy.wrapping_mul(19349663)).unsigned_abs() as usize;
        //return if seed % 100 < 85 { 11 } else { 4 };
    }

    let candidates: &[usize] = match mask {
        1 => &[3],
        2 => &[6],
        4 => &[13, 14, 16],
        8 => &[19],
        5 => &[12, 66, 67, 68, 69],
        9 => &[0, 1, 18, 20, 72, 73, 74, 75],
        6 => &[8, 9, 78, 79, 80, 81],
        10 => &[7, 84, 85, 86, 87],
        3 => &[3, 6],
        12 => &[13, 19],
        7 => &[24, 26, 27, 48, 49, 50, 51],
        11 => &[25, 27, 54, 55, 56, 57, 62],
        13 => &[30, 32, 33, 36, 37, 38, 39],
        14 => &[31, 42, 43, 44, 45],
        15 => &[60, 61, 63],
        _ => &[4, 5],
    };

    let seed = (gx.wrapping_mul(73856093) ^ gy.wrapping_mul(19349663)).unsigned_abs() as usize;
    candidates[seed % candidates.len()]
}

fn spawn_hazard_tiles(
    commands: &mut Commands,
    hazard: &RectData,
    solid_cells: &HashSet<(i32, i32)>,
    level_art: &LevelArt,
) {
    let direction = detect_hazard_direction(hazard, solid_cells);
    let (image, tile_size, horizontal) = match direction {
        HazardDirection::Up => (level_art.danger_up.clone(), DANGER_UPDOWN_SIZE, true),
        HazardDirection::Down => (level_art.danger_down.clone(), DANGER_UPDOWN_SIZE, true),
        HazardDirection::Left => (level_art.danger_left.clone(), DANGER_LEFTRIGHT_SIZE, false),
        HazardDirection::Right => (level_art.danger_right.clone(), DANGER_LEFTRIGHT_SIZE, false),
    };

    if horizontal {
        let count = ((hazard.w / TILE_SIZE).round() as i32).max(1);
        let start_x = hazard.x - count as f32 * TILE_SIZE * 0.5 + TILE_SIZE * 0.5;
        for i in 0..count {
            commands.spawn((
                LevelEntity,
                Sprite {
                    image: image.clone(),
                    custom_size: Some(tile_size),
                    ..default()
                },
                Transform::from_xyz(start_x + i as f32 * TILE_SIZE, hazard.y, 0.3),
            ));
        }
    } else {
        let count = ((hazard.h / TILE_SIZE).round() as i32).max(1);
        let start_y = hazard.y - count as f32 * TILE_SIZE * 0.5 + TILE_SIZE * 0.5;
        for i in 0..count {
            commands.spawn((
                LevelEntity,
                Sprite {
                    image: image.clone(),
                    custom_size: Some(tile_size),
                    ..default()
                },
                Transform::from_xyz(hazard.x, start_y + i as f32 * TILE_SIZE, 0.3),
            ));
        }
    }
}

fn detect_hazard_direction(
    hazard: &RectData,
    solid_cells: &HashSet<(i32, i32)>,
) -> HazardDirection {
    // Direction names describe the spike tip direction, not the side of the solid it is attached to:
    // top face -> Up, ceiling underside -> Down, left wall side -> Right, right wall side -> Left.
    let (min_x, max_x, min_y, max_y) = rect_to_grid_bounds(hazard.x, hazard.y, hazard.w, hazard.h);
    let mut up_score = 0;
    let mut down_score = 0;
    let mut left_score = 0;
    let mut right_score = 0;

    for gx in min_x..=max_x {
        if solid_cells.contains(&(gx, min_y - 1)) {
            up_score += 1;
        }
        if solid_cells.contains(&(gx, max_y + 1)) {
            down_score += 1;
        }
    }

    for gy in min_y..=max_y {
        if solid_cells.contains(&(min_x - 1, gy)) {
            right_score += 1;
        }
        if solid_cells.contains(&(max_x + 1, gy)) {
            left_score += 1;
        }
    }

    let prefer_horizontal = hazard.w >= hazard.h;
    let scores = if prefer_horizontal {
        [
            (HazardDirection::Up, up_score),
            (HazardDirection::Down, down_score),
            (HazardDirection::Left, left_score),
            (HazardDirection::Right, right_score),
        ]
    } else {
        [
            (HazardDirection::Left, left_score),
            (HazardDirection::Right, right_score),
            (HazardDirection::Up, up_score),
            (HazardDirection::Down, down_score),
        ]
    };

    scores
        .into_iter()
        .max_by_key(|(_, score)| *score)
        .map(|(direction, _)| direction)
        .unwrap_or(HazardDirection::Up)
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

fn cleanup_gameplay_entities(
    mut commands: Commands,
    gameplay_entities: Query<Entity, With<GameplayEntity>>,
    level_entities: Query<Entity, With<LevelEntity>>,
    mut pending_map_path: ResMut<PendingMapPath>,
) {
    for entity in &gameplay_entities {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &level_entities {
        commands.entity(entity).despawn_recursive();
    }
    pending_map_path.path = None;
}

fn spawn_weather_overlay(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    weather_materials: &mut ResMut<Assets<WeatherMaterial>>,
) {
    let weather_mesh = meshes.add(Rectangle::new(
        WEATHER_OVERLAY_SIZE.x,
        WEATHER_OVERLAY_SIZE.y,
    ));
    let weather_material = weather_materials.add(WeatherMaterial {
        weather_data: Vec4::ZERO,
    });

    commands.spawn((
        GameplayEntity,
        WeatherOverlay,
        Mesh2d(weather_mesh),
        MeshMaterial2d(weather_material),
        Transform::from_xyz(0.0, 0.0, WEATHER_OVERLAY_Z),
    ));
}
