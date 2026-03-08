use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::sprite::{AlphaMode2d, Anchor, Material2d, Material2dPlugin};

// --- 基础参数 ---
const PLAYER_COLLIDER_SIZE: Vec2 = Vec2::new(10.0, 18.0); 
const MAX_RUN_SPEED: f32 = 90.0;
const GRAVITY: f32 = 900.0;     
const JUMP_VELOCITY: f32 = 280.0; 
const LOW_JUMP_MULTIPLIER: f32 = 2.0;
const FALL_MULTIPLIER: f32 = 1.2;

// 移动物理
const GROUND_ACCEL: f32 = 1000.0;
const GROUND_FRICTION: f32 = 800.0;
const GROUND_TURN_FRICTION: f32 = 1200.0;
const AIR_ACCEL: f32 = 800.0;
const AIR_FRICTION: f32 = 200.0;
const AIR_TURN_FRICTION: f32 = 1000.0;

// 墙壁交互
const WALL_SLIDE_SPEED: f32 = 40.0;
const WALL_CLIMB_SPEED: f32 = 80.0;
const WALL_KICK_FORCE: Vec2 = Vec2::new(140.0, 290.0);
const WALL_KICK_LOCK: f32 = 0.20;
const WALL_NEUTRAL_FORCE: Vec2 = Vec2::new(80.0, 300.0);
const WALL_NEUTRAL_LOCK: f32 = 0.0; 
const WALL_CLIMB_JUMP_FORCE_Y: f32 = 260.0;
const WALL_CLIMB_LOCK: f32 = 0.1;

// 冲刺
const DASH_SPEED: f32 = 300.0;      
const DASH_DURATION: f32 = 0.15;     
const DASH_END_MULTIPLIER: f32 = 0.6;
const DASH_TRAIL_INTERVAL: f32 = 0.025;
const DASH_TRAIL_PARTICLE_COUNT: usize = 14;
const DASH_TRAIL_SPACING: f32 = 1.15;
const DASH_TRAIL_LIFETIME: f32 = 0.11;

// 头发参数
const HAIR_SEGMENT_LEN: f32 = 1.7;
const HAIR_GRAVITY: Vec2 = Vec2::new(0.0, -100.0); 
const HAIR_SEGMENT_SIZES: [f32; 5] = [6.0, 5.5, 5.0, 4.5, 4.0];
const HAIR_PIXEL_STEPS: f32 = 3.0;
const HAIR_OUTLINE_WIDTH: f32 = 0.2;
const HAIR_FOLLOW_STRENGTH: f32 = 14.0;
const HAIR_RESET_DISTANCE: f32 = 28.0;
const HAIR_ROOT_OFFSET: Vec2 = Vec2::new(-3.0, 3.5);
const BANGS_OFFSET: Vec2 = Vec2::new(0.0, 5.0);
const BANGS_Z: f32 = 9.6;
const HAIR_SEGMENT_Z: f32 = 9.0;

// 场景
const DEATH_THRESHOLD: f32 = -200.0;
const SPAWN_POSITION: Vec3 = Vec3::new(0.0, 0.0, 0.0);

#[derive(Component)] struct Player;
#[derive(Component)] struct Velocity(Vec2);
#[derive(Component)] struct Grounded(bool);
#[derive(Component, PartialEq, Debug, Clone, Copy)] enum WallContact { None, Left, Right }
#[derive(Component)] struct Facing(f32);
#[derive(Component)] struct Ground;
#[derive(Component)] struct JumpState { jumps_remaining: u8 }
#[derive(Component)] struct WallJumpTimer(f32);
#[derive(Component)] struct DashState { is_dashing: bool, timer: f32, direction: Vec2, dashes_remaining: u8 }
#[derive(Component)] struct DashTrailEmitter { cooldown: f32, was_dashing: bool }
#[derive(Component, Default)] struct MovementInput { x: f32, y: f32 }
#[derive(Component)] struct AnimationTimer(Timer);

#[derive(Component)]
struct DashTrailParticle {
    velocity: Vec2,
    lifetime: f32,
    max_lifetime: f32,
}

// --- 头发组件 ---
#[derive(Component)]
struct Hair {
    // 存储头发节点的物理位置 (World Space)
    sim_positions: Vec<Vec2>, 
    // 存储对应的实体 ID (Mesh2d)
    entities: Vec<Entity>, 
    bangs_entity: Option<Entity>,
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
struct HairSegment;

#[derive(Component)]
struct HairBangs;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct HairMaterial {
    #[uniform(0)]
    fill_color: Vec4,
    #[uniform(1)]
    outline_color: Vec4,
    #[uniform(2)]
    effect_params: Vec4,
}

impl Material2d for HairMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/hair_outline.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

// --- 动画相关结构体 ---
#[derive(Component, PartialEq, Eq, Hash, Clone, Copy, Debug)]
enum AnimationState {
    Idle,
    Run,
}

#[derive(Component)]
struct PlayerAnimations {
    idle_texture: Handle<Image>,
    idle_layout: Handle<TextureAtlasLayout>,
    run_texture: Handle<Image>,
    run_layout: Handle<TextureAtlasLayout>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(Material2dPlugin::<HairMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (
            tick_timers,
            player_input,
            apply_physics,
            player_movement,
            emit_dash_trail,
            update_dash_trail,
            animate_sprite,
            update_hair, // <--- 新增头发更新系统
            debug_gizmos,
        ).chain())
        .run();
}

fn debug_gizmos(mut gizmos: Gizmos, query: Query<&Transform, With<Player>>) {
    for transform in &query {
        gizmos.rect_2d(
            transform.translation.truncate(), 
            PLAYER_COLLIDER_SIZE,             
            Color::srgb(0.0, 1.0, 0.0),       
        );
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut hair_materials: ResMut<Assets<HairMaterial>>,
) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::FixedVertical { viewport_height: 180.0 };
    
    commands.spawn((
        Camera2d,
        projection, 
    ));
    
    // --- 加载资源 ---
    let run_texture = asset_server.load("run_sheet.png");
    let run_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 12, 1, None, None);
    let run_layout_handle = texture_atlas_layouts.add(run_layout);

    let idle_texture = asset_server.load("idle_sheet.png");
    let idle_layout = TextureAtlasLayout::from_grid(UVec2::new(34, 34), 9, 1, None, None);
    let idle_layout_handle = texture_atlas_layouts.add(idle_layout);
    let bangs_texture = asset_server.load("bangs.png");

    // --- 生成头发实体 ---
    let mut hair_entities = Vec::new();
    let hair_mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let hair_material = hair_materials.add(HairMaterial {
        fill_color: color_to_vec4(Color::srgb(0.9, 0.25, 0.3)),
        outline_color: color_to_vec4(Color::BLACK),
        effect_params: Vec4::new(HAIR_PIXEL_STEPS, HAIR_OUTLINE_WIDTH, 0.35, 0.0),
    });

    for size in HAIR_SEGMENT_SIZES {
        let id = commands.spawn((
            HairSegment,
            Mesh2d(hair_mesh.clone()),
            MeshMaterial2d(hair_material.clone()),
            Transform {
                translation: Vec3::new(0.0, 0.0, HAIR_SEGMENT_Z),
                scale: Vec3::splat(size),
                ..default()
            },
        )).id();
        hair_entities.push(id);
    }

    let bangs_entity = commands.spawn((
        HairBangs,
        Sprite {
            image: bangs_texture,
            color: Color::srgb(0.9, 0.25, 0.3),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, BANGS_Z),
    )).id();

    let initial_hair_positions = initial_hair_positions(SPAWN_POSITION.truncate(), 1.0);

    // --- 生成玩家 ---
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
        
        // 头发组件持有实体 ID
        Hair {
            sim_positions: initial_hair_positions,
            entities: hair_entities,
            bangs_entity: Some(bangs_entity),
        },

        // 动画初始状态
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        AnimationState::Idle,
    ));

    player.insert((
        PlayerAnimations {
            idle_texture: idle_texture.clone(),
            idle_layout: idle_layout_handle.clone(),
            run_texture: run_texture.clone(),
            run_layout: run_layout_handle.clone(),
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
    
    // 地面
    commands.spawn((
        Ground, 
        Sprite { 
            color: Color::srgb(0.2, 0.2, 0.2), 
            custom_size: Some(Vec2::new(400.0, 24.0)), 
            ..default() 
        },
        Transform::from_xyz(0.0, -80.0, 0.0),
    ));
    
    // 墙壁 1
    commands.spawn((
        Ground, 
        Sprite { 
            color: Color::srgb(0.4, 0.4, 0.4), 
            custom_size: Some(Vec2::new(16.0, 200.0)),
            ..default() 
        },
        Transform::from_xyz(-100.0, 0.0, 0.0),
    ));

    // 墙壁 2
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

fn tick_timers(
    time: Res<Time>, 
    mut q_wall: Query<&mut WallJumpTimer>,
    mut q_dash: Query<(&mut DashState, &mut Velocity)>, 
) {
    let dt = time.delta_secs();
    
    for mut timer in &mut q_wall {
        if timer.0 > 0.0 { timer.0 -= dt; }
    }

    for (mut dash_state, mut velocity) in &mut q_dash {
        if dash_state.is_dashing {
            dash_state.timer -= dt;
            if dash_state.timer <= 0.0 {
                dash_state.is_dashing = false;
                velocity.0 *= DASH_END_MULTIPLIER;
                if velocity.0.y < 0.0 { velocity.0.y *= 0.5; }
            }
        }
    }
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

fn dash_effect_color(_dashes_remaining: u8) -> Color {
    Color::srgba(1.0, 1.0, 1.0, 0.92)
}

fn color_to_vec4(color: Color) -> Vec4 {
    let linear = color.to_linear();
    Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
}

fn mirrored_offset(offset: Vec2, facing: f32) -> Vec2 {
    if facing < 0.0 {
        Vec2::new(-offset.x, offset.y)
    } else {
        offset
    }
}

fn hair_rest_offset(index: usize, facing: f32, motion_drag: Vec2) -> Vec2 {
    let step = index as f32;
    let horizontal = 1.0 + step * 0.9;
    let vertical = 0.6 + step * 1.15;
    let arc_lift = (2.0 - step).max(0.0) * 0.35;

    Vec2::new(-facing * horizontal, -vertical + arc_lift) + motion_drag * (0.3 + step * 0.12)
}

fn initial_hair_positions(player_pos: Vec2, facing: f32) -> Vec<Vec2> {
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

// 头发更新系统
fn update_hair(
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
        Query<(&mut Transform, &mut Sprite), (With<HairBangs>, Without<Player>)>,
    )>,
) {
    let dt = time.delta_secs();

    for (player_transform, mut hair, facing, anim_state, sprite, velocity, dash_state) in &mut player_query {
        
        // 1. 确定颜色 (根据剩余冲刺次数)
        let target_color = if dash_state.dashes_remaining > 0 {
            Color::srgb(0.9, 0.25, 0.3) // Red
        } else {
            Color::srgb(0.3, 0.7, 0.95) // Blue
        };

        // 2. 计算发根位置
        let frame_index = sprite.texture_atlas.as_ref().map(|a| a.index).unwrap_or(0);
        
        let hair_anim_offset = match anim_state {
            AnimationState::Idle => {
                match frame_index {
                    0..=3 => Vec2::new(0.0, -2.0),
                    4..=8 => Vec2::new(0.0, -1.0),
                    _ => Vec2::ZERO,
                }
            },
            AnimationState::Run => {
                match frame_index % 4 {
                    0 | 2 => Vec2::new(0.0, -1.0),
                    1 => Vec2::new(0.0, 0.0),
                    3 => Vec2::new(0.0, -2.0),
                    _ => Vec2::ZERO,
                }
            }
        };

        let bangs_anim_offset = match anim_state {
            AnimationState::Idle => hair_anim_offset,
            AnimationState::Run => Vec2::ZERO,
        };

        let root_pos = player_transform.translation.truncate()
            + mirrored_offset(HAIR_ROOT_OFFSET, facing.0)
            + hair_anim_offset;
        let bangs_pos = player_transform.translation.truncate()
            + mirrored_offset(BANGS_OFFSET, facing.0)
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

        // 3. 物理模拟
        if hair.sim_positions.len() < HAIR_SEGMENT_SIZES.len() { continue; }
        hair.sim_positions[0] = root_pos;

        for i in 1..hair.sim_positions.len() {
            let prev_pos = hair.sim_positions[i-1];
            let mut curr_pos = hair.sim_positions[i];
            let rest_target = root_pos + hair_rest_offset(i, facing.0, motion_drag);

            let wind_force = Vec2::new(-facing.0 * 14.0, 0.0);
            let force = (HAIR_GRAVITY + wind_force) * dt;
            curr_pos = curr_pos.lerp(rest_target, (HAIR_FOLLOW_STRENGTH * dt).min(1.0));
            curr_pos += force * dt * 16.0;

            if dash_state.is_dashing {
                curr_pos += Vec2::new(-dash_state.direction.x, -dash_state.direction.y) * (0.18 * i as f32);
            }

            // 距离约束
            let diff = curr_pos - prev_pos;
            let dist = diff.length();
            
            if dist > HAIR_SEGMENT_LEN {
                curr_pos = prev_pos + diff.normalize() * HAIR_SEGMENT_LEN;
            } else if dist < HAIR_SEGMENT_LEN * 0.55 {
                curr_pos = prev_pos + (rest_target - prev_pos).normalize_or_zero() * HAIR_SEGMENT_LEN;
            }

            hair.sim_positions[i] = curr_pos;
        }

        // 4. 更新实体
        for (i, entity) in hair.entities.iter().enumerate() {
            // 更新位置
            if let Ok(mut transform) = hair_render_queries.p0().get_mut(*entity) {
                transform.translation = hair.sim_positions[i].extend(HAIR_SEGMENT_Z);
            }

            // 更新颜色
            if let Ok(mat_handle) = hair_render_queries.p1().get(*entity) {
                if let Some(material) = materials.get_mut(mat_handle) {
                    material.fill_color = color_to_vec4(target_color);
                }
            }
        }
    }
}

fn emit_dash_trail(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        &Transform,
        &DashState,
        &mut DashTrailEmitter,
        Option<&Sprite>,
    ), With<Player>>,
) {
    let dt = time.delta_secs();

    for (transform, dash_state, mut emitter, sprite) in &mut query {
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

        let dash_dir = dash_state.direction.normalize_or_zero();
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
            let position = base_pos
                + Vec2::new(face_offset, 3.0)
                + backward * (step * DASH_TRAIL_SPACING + 1.0);
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

fn update_dash_trail(
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

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut Sprite,
        &mut AnimationState,
        &PlayerAnimations,
        &Velocity,
        &Facing,
        &Grounded,
    )>,
) {
    for (mut timer, mut sprite, mut state, animations, velocity, facing, grounded) in &mut query {
        
        if facing.0 < 0.0 {
            sprite.flip_x = true;
        } else {
            sprite.flip_x = false;
        }

        let is_moving = velocity.0.x.abs() > 5.0;
        let next_state = if grounded.0 && is_moving {
            AnimationState::Run
        } else {
            AnimationState::Idle
        };

        if *state != next_state {
            *state = next_state;
            let (new_image, new_layout) = match next_state {
                AnimationState::Idle => (animations.idle_texture.clone(), animations.idle_layout.clone()),
                AnimationState::Run => (animations.run_texture.clone(), animations.run_layout.clone()),
            };

            sprite.image = new_image;
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.layout = new_layout;
                atlas.index = 0;
            }
        }

        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                let max_frames = match *state {
                    AnimationState::Idle => 9,
                    AnimationState::Run => 12,
                };
                if atlas.index >= max_frames - 1 {
                    atlas.index = 0;
                } else {
                    atlas.index += 1;
                }
            }
        }
    }
}

fn player_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Velocity, &mut JumpState, &Grounded, &WallContact, &mut WallJumpTimer, &mut DashState, &mut Facing, &mut MovementInput), With<Player>>,
) {
    if let Ok((mut velocity, mut jump_state, grounded, wall_contact, mut wall_jump_timer, mut dash_state, mut facing, mut move_input)) = query.get_single_mut() {
        
        if grounded.0 && velocity.0.y <= 0.0 {
            jump_state.jumps_remaining = 1;
            dash_state.dashes_remaining = 1;
        }

        let left = keyboard_input.pressed(KeyCode::KeyA);
        let right = keyboard_input.pressed(KeyCode::KeyD);
        let up = keyboard_input.pressed(KeyCode::KeyW);
        let down = keyboard_input.pressed(KeyCode::KeyS);

        let input_x = if right && !left { 1.0 } else if left && !right { -1.0 } else { 0.0 };
        let input_y = if up && !down { 1.0 } else if down && !up { -1.0 } else { 0.0 };

        move_input.x = input_x;
        move_input.y = input_y;

        if input_x != 0.0 { facing.0 = input_x; }

        let is_dash_pressed = keyboard_input.just_pressed(KeyCode::KeyK); 
        if is_dash_pressed && dash_state.dashes_remaining > 0 {
            dash_state.is_dashing = true;
            dash_state.timer = DASH_DURATION;
            dash_state.dashes_remaining -= 1;
            
            let mut dash_dir = Vec2::new(input_x, input_y);
            if dash_dir == Vec2::ZERO { dash_dir = Vec2::new(facing.0, 0.0); }
            dash_state.direction = dash_dir.normalize_or_zero();
            velocity.0 = dash_state.direction * DASH_SPEED;
            return; 
        }

        if dash_state.is_dashing { return; }

        let is_grabbing = keyboard_input.pressed(KeyCode::KeyJ);
        
        if keyboard_input.just_pressed(KeyCode::Space) {
            if grounded.0 || jump_state.jumps_remaining > 0 {
                velocity.0.y = JUMP_VELOCITY;
                jump_state.jumps_remaining = jump_state.jumps_remaining.saturating_sub(1);
            } else if !grounded.0 && *wall_contact != WallContact::None {
                let wall_dir = match wall_contact {
                    WallContact::Left => -1.0,
                    WallContact::Right => 1.0,
                    _ => 0.0,
                };

                if is_grabbing {
                    velocity.0.y = WALL_CLIMB_JUMP_FORCE_Y;
                    velocity.0.x = 0.0; 
                    wall_jump_timer.0 = WALL_CLIMB_LOCK; 
                } else {
                    if input_x == 0.0 {
                        velocity.0.x = -wall_dir * WALL_NEUTRAL_FORCE.x;
                        velocity.0.y = WALL_NEUTRAL_FORCE.y;
                        wall_jump_timer.0 = WALL_NEUTRAL_LOCK; 
                    } else {
                        velocity.0.x = -wall_dir * WALL_KICK_FORCE.x;
                        velocity.0.y = WALL_KICK_FORCE.y;
                        wall_jump_timer.0 = WALL_KICK_LOCK;
                    }
                }
            }
        }
    }
}

fn apply_physics(
    mut query: Query<(&mut Velocity, &WallContact, &Grounded, &WallJumpTimer, &DashState, &MovementInput), With<Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    if let Ok((mut velocity, wall_contact, grounded, wall_jump_timer, dash_state, move_input)) = query.get_single_mut() {
        
        if dash_state.is_dashing {
            velocity.0 = dash_state.direction * DASH_SPEED;
            return; 
        }

        let is_holding_wall = keyboard_input.pressed(KeyCode::KeyJ) 
            && *wall_contact != WallContact::None 
            && wall_jump_timer.0 <= 0.0; 
        
        if is_holding_wall {
            velocity.0.x = 0.0;
            let mut climb_dir = 0.0;
            if keyboard_input.pressed(KeyCode::KeyW) { climb_dir += 1.0; }
            if keyboard_input.pressed(KeyCode::KeyS) { climb_dir -= 1.0; }
            velocity.0.y = climb_dir * WALL_CLIMB_SPEED;
            return;
        }

        let dt = time.delta_secs();
        
        if wall_jump_timer.0 <= 0.0 {
            let target_speed = move_input.x * MAX_RUN_SPEED;
            let current_speed = velocity.0.x;

            let (accel, friction, turn_friction) = if grounded.0 {
                (GROUND_ACCEL, GROUND_FRICTION, GROUND_TURN_FRICTION)
            } else {
                (AIR_ACCEL, AIR_FRICTION, AIR_TURN_FRICTION)
            };

            let final_accel = if move_input.x != 0.0 {
                if current_speed.signum() != move_input.x.signum() && current_speed != 0.0 {
                    turn_friction 
                } else {
                    accel 
                }
            } else {
                friction 
            };

            velocity.0.x = move_towards(current_speed, target_speed, final_accel * dt);
        } else {
            velocity.0.x = move_towards(velocity.0.x, 0.0, AIR_FRICTION * 0.5 * dt);
        }

        let mut gravity_multiplier = 1.0;
        if velocity.0.y > 0.0 {
            if !keyboard_input.pressed(KeyCode::Space) {
                gravity_multiplier = LOW_JUMP_MULTIPLIER;
            }
        } else {
            gravity_multiplier = FALL_MULTIPLIER;
        }
        velocity.0.y -= GRAVITY * gravity_multiplier * time.delta_secs();

        if *wall_contact != WallContact::None && velocity.0.y < 0.0 && !grounded.0 && wall_jump_timer.0 <= 0.0 {
            if velocity.0.y < -WALL_SLIDE_SPEED {
                velocity.0.y = -WALL_SLIDE_SPEED;
            }
        }
    }
}

fn check_collision(a_pos: Vec3, a_size: Vec2, b_pos: Vec3, b_size: Vec2) -> bool {
    let a_min = a_pos.truncate() - a_size / 2.0;
    let a_max = a_pos.truncate() + a_size / 2.0;
    let b_min = b_pos.truncate() - b_size / 2.0;
    let b_max = b_pos.truncate() + b_size / 2.0;
    a_min.x < b_max.x && a_max.x > b_min.x && a_min.y < b_max.y && a_max.y > b_min.y
}

fn player_movement(
    mut param_set: ParamSet<(
        Query<(&mut Transform, &mut Velocity, &mut Grounded, &mut WallContact), With<Player>>,
        Query<(&Transform, &Sprite), With<Ground>>,
    )>,
    time: Res<Time>,
) {
    let obstacles: Vec<(Vec3, Vec2)> = param_set.p1().iter()
        .filter_map(|(t, s)| s.custom_size.map(|size| (t.translation, size)))
        .collect();

    if let Ok((mut player_transform, mut velocity, mut grounded, mut wall_contact)) = param_set.p0().get_single_mut() {
        let player_size = PLAYER_COLLIDER_SIZE;
        let delta_seconds = time.delta_secs();

        grounded.0 = false;
        
        player_transform.translation.x += velocity.0.x * delta_seconds;
        for (ground_pos, ground_size) in &obstacles {
            if check_collision(player_transform.translation, player_size, *ground_pos, *ground_size) {
                if velocity.0.x > 0.0 {
                    player_transform.translation.x = ground_pos.x - ground_size.x / 2.0 - player_size.x / 2.0;
                } else if velocity.0.x < 0.0 {
                    player_transform.translation.x = ground_pos.x + ground_size.x / 2.0 + player_size.x / 2.0;
                }
                velocity.0.x = 0.0;
            }
        }

        *wall_contact = WallContact::None; 
        let sensor_margin = 1.0; 
        for (ground_pos, ground_size) in &obstacles {
            let left_check_pos = player_transform.translation - Vec3::new(sensor_margin, 0.0, 0.0);
            if check_collision(left_check_pos, player_size, *ground_pos, *ground_size) {
                *wall_contact = WallContact::Left;
            }
            let right_check_pos = player_transform.translation + Vec3::new(sensor_margin, 0.0, 0.0);
            if check_collision(right_check_pos, player_size, *ground_pos, *ground_size) {
                *wall_contact = WallContact::Right;
            }
        }

        player_transform.translation.y += velocity.0.y * delta_seconds;
        for (ground_pos, ground_size) in &obstacles {
            if check_collision(player_transform.translation, player_size, *ground_pos, *ground_size) {
                if velocity.0.y > 0.0 {
                    player_transform.translation.y = ground_pos.y - ground_size.y / 2.0 - player_size.y / 2.0;
                    velocity.0.y = 0.0;
                } else if velocity.0.y < 0.0 {
                    player_transform.translation.y = ground_pos.y + ground_size.y / 2.0 + player_size.y / 2.0;
                    grounded.0 = true;
                    velocity.0.y = 0.0;
                }
            }
        }
        
        if grounded.0 { *wall_contact = WallContact::None; }
        if player_transform.translation.y < DEATH_THRESHOLD {
            player_transform.translation = SPAWN_POSITION;
            velocity.0 = Vec2::ZERO;
            *wall_contact = WallContact::None;
        }
    }
}