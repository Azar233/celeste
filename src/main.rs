use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::sprite::Anchor;

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

// 头发参数
const HAIR_SEGMENT_LEN: f32 = 3.0; 
const HAIR_GRAVITY: Vec2 = Vec2::new(0.0, -100.0); 
// 头部偏移 (相对于 Collider 中心): X=-1.0(后脑勺), Y=11.0(脖子高度)
const HEAD_BASE_OFFSET: Vec2 = Vec2::new(-1.0, 11.0); 

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
#[derive(Component, Default)] struct MovementInput { x: f32, y: f32 }
#[derive(Component)] struct AnimationTimer(Timer);

// --- 头发组件 ---
#[derive(Component)]
struct Hair {
    // 存储头发节点的物理位置 (World Space)
    sim_positions: Vec<Vec2>, 
    // 存储对应的实体 ID (Mesh2d)
    entities: Vec<Entity>, 
}

impl Default for Hair {
    fn default() -> Self {
        Self {
            sim_positions: vec![Vec2::ZERO; 5],
            entities: Vec::new(),
        }
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
        .add_systems(Startup, setup)
        .add_systems(Update, (
            tick_timers,
            player_input,
            apply_physics,
            player_movement,
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
    mut materials: ResMut<Assets<ColorMaterial>>,
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

    // --- 生成头发实体 ---
    // 半径递减: 3.5 -> 1.5
    let radii = [3.5, 3.0, 2.5, 2.0, 1.5];
    let mut hair_entities = Vec::new();
    
    // 初始材质 (红色)
    let hair_material = materials.add(ColorMaterial::from(Color::srgb(0.9, 0.25, 0.3)));

    for i in 0..5 {
        let id = commands.spawn((
            Mesh2d(meshes.add(Circle::new(radii[i]))),
            MeshMaterial2d(hair_material.clone()),
            // Z=9.0，确保在玩家(Z=10.0)后面
            Transform::from_xyz(0.0, 0.0, 9.0), 
        )).id();
        hair_entities.push(id);
    }

    // --- 生成玩家 ---
    commands.spawn((
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
        
        // 头发组件持有实体 ID
        Hair {
            sim_positions: vec![Vec2::ZERO; 5],
            entities: hair_entities,
        },

        // 动画初始状态
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        AnimationState::Idle,
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

// 头发更新系统
fn update_hair(
    time: Res<Time>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut player_query: Query<(
        &Transform,
        &mut Hair,
        &Facing,
        &AnimationState,
        &Sprite, 
        &DashState, 
    ), With<Player>>,
    // 查询头发实体的 Transform (不含 Player)
    mut hair_transform_query: Query<&mut Transform, (Without<Player>, Without<Ground>)>,
    // 查询头发材质句柄
    hair_mat_query: Query<&MeshMaterial2d<ColorMaterial>>,
) {
    let dt = time.delta_secs();

    for (player_transform, mut hair, facing, anim_state, sprite, dash_state) in &mut player_query {
        
        // 1. 确定颜色 (根据剩余冲刺次数)
        let target_color = if dash_state.dashes_remaining > 0 {
            Color::srgb(0.9, 0.25, 0.3) // Red
        } else {
            Color::srgb(0.3, 0.7, 0.95) // Blue
        };

        // 2. 计算发根位置
        let frame_index = sprite.texture_atlas.as_ref().map(|a| a.index).unwrap_or(0);
        
        let anim_offset = match anim_state {
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

        let mut final_head_offset = HEAD_BASE_OFFSET + anim_offset;
        // 如果朝左，X 偏移取反 (保持在后脑勺)
        if facing.0 < 0.0 {
            final_head_offset.x = -final_head_offset.x;
        }

        let root_pos = player_transform.translation.truncate() + final_head_offset;

        // 3. 物理模拟
        if hair.sim_positions.len() < 5 { continue; }
        hair.sim_positions[0] = root_pos;

        for i in 1..hair.sim_positions.len() {
            let prev_pos = hair.sim_positions[i-1];
            let mut curr_pos = hair.sim_positions[i];

            // 简单的风力/重力模拟
            let wind_force = Vec2::new(-facing.0 * 20.0, 0.0); 
            let force = (HAIR_GRAVITY + wind_force) * dt;
            curr_pos += force * dt * 50.0; 

            // 距离约束
            let diff = curr_pos - prev_pos;
            let dist = diff.length();
            
            if dist > HAIR_SEGMENT_LEN {
                curr_pos = prev_pos + diff.normalize() * HAIR_SEGMENT_LEN;
            } else if dist < 0.1 {
                curr_pos = prev_pos + Vec2::new(-facing.0, -1.0).normalize() * HAIR_SEGMENT_LEN;
            }

            hair.sim_positions[i] = curr_pos;
        }

        // 4. 更新实体
        for (i, entity) in hair.entities.iter().enumerate() {
            // 更新位置
            if let Ok(mut transform) = hair_transform_query.get_mut(*entity) {
                transform.translation = hair.sim_positions[i].extend(9.0);
            }

            // 更新颜色
            if let Ok(mat_handle) = hair_mat_query.get(*entity) {
                if let Some(material) = materials.get_mut(mat_handle) {
                    material.color = target_color;
                }
            }
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