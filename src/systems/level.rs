use bevy::prelude::*;

use crate::app_state::GameState;
use crate::components::{
    AnimationState, AnimationTimer, CheckpointMarker, ClimbStamina, ClimbTopOutState,
    ColliderSize, CompletionOverlay, CompletionZone, CornerBoostState, Crouching, DashCrystal,
    DashSlideState, DashState, DashTrailEmitter, Facing, GameplayEntity, Grounded, Hair, Hazard,
    JumpState, LevelEntity, MovementInput, Player, PlayerActionInput, PlayerAnimations, PlayerState,
    PlayerStateMachine, RoomExitMarker, Velocity, WallContact, WallJumpTimer, WeatherOverlay,
};
use crate::constants::{
    CLIMB_STAMINA_MAX, DASH_CRYSTAL_ANIMATION_INTERVAL, DASH_CRYSTAL_RESPAWN_TIME,
    PLAYER_COLLIDER_SIZE, PLAYER_RENDER_Z,
};
use crate::level::{ActiveRoom, LoadedMap, RectData};
use crate::scene::{LevelArt, is_chapter_02_map_path, spawn_room_geometry};
use crate::utils::{check_collision, initial_hair_positions};

const ROOM_TRANSITION_COOLDOWN_SECS: f32 = 0.18;
const DEATH_ANIMATION_FRAMES: usize = 13;
const DEATH_ANIMATION_FRAME_INTERVAL: f32 = 0.05;
const DEATH_WIPE_DURATION_SECS: f32 = 0.30;
const FORSAKEN_CITY_COMPLETION_ENDING_LAYERS: [&str; 14] = [
    "figs/ending/ForsakenCity/01.png",
    "figs/ending/ForsakenCity/02.png",
    "figs/ending/ForsakenCity/03.png",
    "figs/ending/ForsakenCity/04.png",
    "figs/ending/ForsakenCity/05.png",
    "figs/ending/ForsakenCity/06.png",
    "figs/ending/ForsakenCity/07.png",
    "figs/ending/ForsakenCity/08a.png",
    "figs/ending/ForsakenCity/08b.png",
    "figs/ending/ForsakenCity/09.png",
    "figs/ending/ForsakenCity/10.png",
    "figs/ending/ForsakenCity/11.png",
    "figs/ending/ForsakenCity/snow-back.png",
    "figs/ending/ForsakenCity/snow-front.png",
];
const SUMMIT_END_COMPLETION_ENDING_LAYERS: [&str; 31] = [
    "figs/ending/SummitEnd/00.png",
    "figs/ending/SummitEnd/01a.png",
    "figs/ending/SummitEnd/01b.png",
    "figs/ending/SummitEnd/01c.png",
    "figs/ending/SummitEnd/02a.png",
    "figs/ending/SummitEnd/02b.png",
    "figs/ending/SummitEnd/02c.png",
    "figs/ending/SummitEnd/03a.png",
    "figs/ending/SummitEnd/03b.png",
    "figs/ending/SummitEnd/03c.png",
    "figs/ending/SummitEnd/04.png",
    "figs/ending/SummitEnd/05.png",
    "figs/ending/SummitEnd/06.png",
    "figs/ending/SummitEnd/07a.png",
    "figs/ending/SummitEnd/07b.png",
    "figs/ending/SummitEnd/07c.png",
    "figs/ending/SummitEnd/07d.png",
    "figs/ending/SummitEnd/08.png",
    "figs/ending/SummitEnd/09a.png",
    "figs/ending/SummitEnd/09b.png",
    "figs/ending/SummitEnd/09c.png",
    "figs/ending/SummitEnd/09d.png",
    "figs/ending/SummitEnd/09e.png",
    "figs/ending/SummitEnd/09f.png",
    "figs/ending/SummitEnd/09g.png",
    "figs/ending/SummitEnd/09h.png",
    "figs/ending/SummitEnd/10.png",
    "figs/ending/SummitEnd/11a.png",
    "figs/ending/SummitEnd/11b.png",
    "figs/ending/SummitEnd/11c.png",
    "figs/ending/SummitEnd/12.png",
];

#[derive(Resource, Default)]
pub struct RoomTransitionCooldown {
    pub timer: f32,
}

#[derive(Resource, Default)]
pub struct CompletionState {
    pub active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathSequencePhase {
    Inactive,
    PlayerDeathAnimation,
    ClosingWipe,
    RespawnReset,
    OpeningWipe,
}

#[derive(Resource, Debug)]
pub struct DeathSequence {
    pub phase: DeathSequencePhase,
    pub timer: f32,
    pub frame_index: usize,
}

impl Default for DeathSequence {
    fn default() -> Self {
        Self {
            phase: DeathSequencePhase::Inactive,
            timer: 0.0,
            frame_index: 0,
        }
    }
}

impl DeathSequence {
    pub fn active(&self) -> bool {
        self.phase != DeathSequencePhase::Inactive
    }

    pub fn start(&mut self) {
        if !self.active() {
            self.phase = DeathSequencePhase::PlayerDeathAnimation;
            self.timer = 0.0;
            self.frame_index = 0;
        }
    }
}

#[derive(Component)]
pub(crate) struct DeathWipePanel {
    side: DeathWipeSide,
}

#[derive(Clone, Copy)]
pub(crate) enum DeathWipeSide {
    Left,
    Right,
}

fn reset_player_to_spawn(
    player_transform: &mut Transform,
    velocity: &mut Velocity,
    grounded: &mut Grounded,
    wall_contact: &mut WallContact,
    collider_size: &mut ColliderSize,
    crouching: &mut Crouching,
    dash_state: &mut DashState,
    dash_slide: &mut DashSlideState,
    state_machine: &mut PlayerStateMachine,
    climb_top_out: &mut ClimbTopOutState,
    corner_boost: &mut CornerBoostState,
    hair: &mut Hair,
    climb_stamina: &mut ClimbStamina,
    facing: &Facing,
    spawn_point: Vec2,
    preserve_momentum: bool,
) {
    player_transform.translation = Vec3::new(spawn_point.x, spawn_point.y, PLAYER_RENDER_Z);
    if !preserve_momentum {
        velocity.0 = Vec2::ZERO;
    }
    grounded.0 = false;
    *wall_contact = WallContact::None;
    crouching.0 = false;
    dash_state.is_dashing = false;
    dash_state.timer = 0.0;
    dash_slide.timer = 0.0;
    dash_slide.direction = 0.0;
    climb_top_out.active = false;
    climb_top_out.timer = 0.0;
    climb_top_out.duration = 0.0;
    corner_boost.clear();
    climb_top_out.start = player_transform.translation;
    climb_top_out.target = player_transform.translation;
    collider_size.0 = PLAYER_COLLIDER_SIZE;
    climb_stamina.current = CLIMB_STAMINA_MAX;
    climb_stamina.low_flash_timer = 0.0;
    state_machine.current = PlayerState::Normal;
    state_machine.previous = PlayerState::Normal;
    hair.sim_positions = initial_hair_positions(spawn_point, facing.0);
}

fn reset_player_runtime_input(
    jump_state: &mut JumpState,
    wall_jump_timer: &mut WallJumpTimer,
    movement_input: &mut MovementInput,
    actions: &mut PlayerActionInput,
    dash_trail: &mut DashTrailEmitter,
) {
    jump_state.jump_grace_timer = 0.0;
    jump_state.jump_buffer_timer = 0.0;
    jump_state.super_jump_timer = 0.0;
    jump_state.fast_jump_active = false;
    wall_jump_timer.0 = 0.0;
    movement_input.x = 0.0;
    movement_input.y = 0.0;
    actions.jump_pressed = false;
    actions.jump_held = false;
    actions.dash_pressed = false;
    actions.dash_requires_release = false;
    actions.grab_held = false;
    dash_trail.cooldown = 0.0;
    dash_trail.was_dashing = false;
}

pub fn death_sequence_inactive(death_sequence: Res<DeathSequence>) -> bool {
    !death_sequence.active()
}

pub fn reset_death_sequence(mut death_sequence: ResMut<DeathSequence>) {
    *death_sequence = DeathSequence::default();
}

fn spawn_death_wipe(commands: &mut Commands) {
    commands
        .spawn((
            GameplayEntity,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(190),
        ))
        .with_children(|parent| {
            for side in [DeathWipeSide::Left, DeathWipeSide::Right] {
                parent.spawn((
                    DeathWipePanel { side },
                    Node {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                ));
            }
        });
}

fn set_death_wipe_width(
    panel_query: &mut Query<(Entity, &mut Node, &DeathWipePanel)>,
    coverage: f32,
) {
    let width = Val::Percent((coverage.clamp(0.0, 1.0) * 50.0).ceil());
    for (_, mut node, panel) in panel_query.iter_mut() {
        node.width = width;
        match panel.side {
            DeathWipeSide::Left => {
                node.left = Val::Px(0.0);
                node.right = Val::Auto;
            }
            DeathWipeSide::Right => {
                node.left = Val::Auto;
                node.right = Val::Px(0.0);
            }
        }
    }
}

pub fn update_death_sequence(
    mut commands: Commands,
    time: Res<Time>,
    active_room: Res<ActiveRoom>,
    mut death_sequence: ResMut<DeathSequence>,
    mut core_query: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut Grounded,
            &mut WallContact,
            &mut ColliderSize,
            &mut Crouching,
            &mut DashState,
            &mut DashSlideState,
            &mut PlayerStateMachine,
            &mut ClimbTopOutState,
            &mut CornerBoostState,
            &mut Hair,
            &mut ClimbStamina,
            &Facing,
        ),
        With<Player>,
    >,
    mut runtime_query: Query<
        (
            &mut JumpState,
            &mut WallJumpTimer,
            &mut MovementInput,
            &mut PlayerActionInput,
            &mut DashTrailEmitter,
        ),
        With<Player>,
    >,
    mut animation_query: Query<
        (
            &PlayerAnimations,
            &mut Sprite,
            &mut AnimationTimer,
            &mut AnimationState,
        ),
        With<Player>,
    >,
    mut panel_query: Query<(Entity, &mut Node, &DeathWipePanel)>,
    mut visibility_query: Query<&mut Visibility>,
) {
    let dt = time.delta_secs();

    match death_sequence.phase {
        DeathSequencePhase::Inactive => {}
        DeathSequencePhase::PlayerDeathAnimation => {
            let Ok((
                _,
                mut velocity,
                mut grounded,
                mut wall_contact,
                mut collider_size,
                mut crouching,
                mut dash_state,
                mut dash_slide,
                mut state_machine,
                mut climb_top_out,
                mut corner_boost,
                hair,
                mut climb_stamina,
                _,
            )) = core_query.get_single_mut()
            else {
                return;
            };
            let Ok((
                mut jump_state,
                mut wall_jump_timer,
                mut movement_input,
                mut actions,
                mut dash_trail,
            )) = runtime_query.get_single_mut()
            else {
                return;
            };
            let Ok((animations, mut sprite, mut animation_timer, mut animation_state)) =
                animation_query.get_single_mut()
            else {
                return;
            };

            velocity.0 = Vec2::ZERO;
            grounded.0 = false;
            *wall_contact = WallContact::None;
            collider_size.0 = PLAYER_COLLIDER_SIZE;
            crouching.0 = false;
            dash_state.is_dashing = false;
            dash_state.timer = 0.0;
            dash_slide.timer = 0.0;
            dash_slide.direction = 0.0;
            state_machine.current = PlayerState::Normal;
            state_machine.previous = PlayerState::Normal;
            climb_top_out.active = false;
            climb_top_out.timer = 0.0;
            climb_top_out.duration = 0.0;
            corner_boost.clear();
            climb_stamina.low_flash_timer = 0.0;
            reset_player_runtime_input(
                &mut jump_state,
                &mut wall_jump_timer,
                &mut movement_input,
                &mut actions,
                &mut dash_trail,
            );

            set_hair_visibility(&hair, false, &mut visibility_query);

            if *animation_state != AnimationState::Death {
                *animation_state = AnimationState::Death;
                animation_timer.0 = Timer::from_seconds(
                    DEATH_ANIMATION_FRAME_INTERVAL,
                    TimerMode::Repeating,
                );
                sprite.image = animations.death_texture.clone();
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: animations.death_layout.clone(),
                    index: 0,
                });
                sprite.color = Color::WHITE;
                death_sequence.frame_index = 0;
                death_sequence.timer = 0.0;
            }

            death_sequence.timer += dt;
            let frame_index = (death_sequence.timer / DEATH_ANIMATION_FRAME_INTERVAL).floor()
                as usize;
            death_sequence.frame_index = frame_index.min(DEATH_ANIMATION_FRAMES - 1);
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = death_sequence.frame_index;
            }

            if death_sequence.timer
                >= DEATH_ANIMATION_FRAME_INTERVAL * DEATH_ANIMATION_FRAMES as f32
            {
                death_sequence.phase = DeathSequencePhase::ClosingWipe;
                death_sequence.timer = 0.0;
                spawn_death_wipe(&mut commands);
                set_death_wipe_width(&mut panel_query, 0.0);
            }
        }
        DeathSequencePhase::ClosingWipe => {
            death_sequence.timer += dt;
            let coverage = death_sequence.timer / DEATH_WIPE_DURATION_SECS;
            set_death_wipe_width(&mut panel_query, coverage);
            if coverage >= 1.0 {
                death_sequence.phase = DeathSequencePhase::RespawnReset;
                death_sequence.timer = 0.0;
            }
        }
        DeathSequencePhase::RespawnReset => {
            let Ok((
                mut player_transform,
                mut velocity,
                mut grounded,
                mut wall_contact,
                mut collider_size,
                mut crouching,
                mut dash_state,
                mut dash_slide,
                mut state_machine,
                mut climb_top_out,
                mut corner_boost,
                mut hair,
                mut climb_stamina,
                facing,
            )) = core_query.get_single_mut()
            else {
                return;
            };
            let Ok((
                mut jump_state,
                mut wall_jump_timer,
                mut movement_input,
                mut actions,
                mut dash_trail,
            )) = runtime_query.get_single_mut()
            else {
                return;
            };
            let Ok((animations, mut sprite, mut animation_timer, mut animation_state)) =
                animation_query.get_single_mut()
            else {
                return;
            };

            reset_player_to_spawn(
                &mut player_transform,
                &mut velocity,
                &mut grounded,
                &mut wall_contact,
                &mut collider_size,
                &mut crouching,
                &mut dash_state,
                &mut dash_slide,
                &mut state_machine,
                &mut climb_top_out,
                &mut corner_boost,
                &mut hair,
                &mut climb_stamina,
                facing,
                active_room.respawn_point,
                false,
            );
            reset_player_runtime_input(
                &mut jump_state,
                &mut wall_jump_timer,
                &mut movement_input,
                &mut actions,
                &mut dash_trail,
            );
            *animation_state = AnimationState::Idle;
            animation_timer.0 = Timer::from_seconds(0.1, TimerMode::Repeating);
            sprite.image = animations.idle_texture.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: animations.idle_layout.clone(),
                index: 0,
            });
            sprite.color = Color::WHITE;
            set_hair_visibility(&hair, true, &mut visibility_query);
            set_death_wipe_width(&mut panel_query, 1.0);
            death_sequence.phase = DeathSequencePhase::OpeningWipe;
            death_sequence.timer = 0.0;
        }
        DeathSequencePhase::OpeningWipe => {
            death_sequence.timer += dt;
            let coverage = 1.0 - death_sequence.timer / DEATH_WIPE_DURATION_SECS;
            set_death_wipe_width(&mut panel_query, coverage);
            if coverage <= 0.0 {
                for (entity, _, _) in panel_query.iter_mut() {
                    commands.entity(entity).despawn_recursive();
                }
                death_sequence.phase = DeathSequencePhase::Inactive;
                death_sequence.timer = 0.0;
                death_sequence.frame_index = 0;
            }
        }
    }
}

fn set_hair_visibility(
    hair: &Hair,
    visible: bool,
    visibility_query: &mut Query<&mut Visibility>,
) {
    let visibility = if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for entity in hair.entities.iter().copied().chain(hair.bangs_entity) {
        if let Ok(mut entity_visibility) = visibility_query.get_mut(entity) {
            *entity_visibility = visibility;
        }
    }
}

pub fn update_checkpoint_respawn(
    mut active_room: ResMut<ActiveRoom>,
    player_query: Query<(&Transform, &ColliderSize), With<Player>>,
    checkpoints: Query<(&Transform, &Sprite, &CheckpointMarker), Without<Player>>,
) {
    let Ok((player_transform, player_size)) = player_query.get_single() else {
        return;
    };

    for (checkpoint_transform, checkpoint_sprite, checkpoint) in &checkpoints {
        let Some(checkpoint_size) = checkpoint_sprite.custom_size else {
            continue;
        };

        if check_collision(
            player_transform.translation,
            player_size.0,
            checkpoint_transform.translation,
            checkpoint_size,
        ) {
            let _checkpoint_id = &checkpoint.id;
            active_room.respawn_point = checkpoint_transform.translation.truncate();
        }
    }
}

fn clamped_camera_position(room_bounds: &RectData, focus: Vec2, window: &Window) -> Vec2 {
    let viewport_height = 180.0;
    let aspect = window.width() / window.height();
    let viewport_width = viewport_height * aspect;
    let half_w = viewport_width * 0.5;
    let half_h = viewport_height * 0.5;

    let room_left = room_bounds.x - room_bounds.w * 0.5;
    let room_right = room_bounds.x + room_bounds.w * 0.5;
    let room_bottom = room_bounds.y - room_bounds.h * 0.5;
    let room_top = room_bounds.y + room_bounds.h * 0.5;

    let x = if room_bounds.w >= viewport_width {
        focus.x.clamp(room_left + half_w, room_right - half_w)
    } else {
        room_bounds.x
    };
    let y = if room_bounds.h >= viewport_height {
        focus.y.clamp(room_bottom + half_h, room_top - half_h)
    } else {
        room_bounds.y
    };

    Vec2::new(x, y)
}

pub fn update_dash_crystals(
    time: Res<Time>,
    mut crystals: Query<(&Transform, &mut Sprite, &mut DashCrystal), Without<Player>>,
    mut player_query: Query<
        (&Transform, &ColliderSize, &mut DashState, &mut PlayerActionInput),
        With<Player>,
    >,
) {
    let dt = time.delta_secs();
    let Ok((player_transform, player_size, mut dash_state, mut actions)) =
        player_query.get_single_mut()
    else {
        return;
    };

    for (crystal_transform, mut sprite, mut crystal) in &mut crystals {
        if crystal.respawn_timer > 0.0 {
            crystal.respawn_timer = (crystal.respawn_timer - dt).max(0.0);
            if crystal.respawn_timer <= 0.0 && !crystal.active_frames.is_empty() {
                crystal.frame_index = 0;
                crystal.animation_timer = 0.0;
                sprite.image = crystal.active_frames[0].clone();
            }
            continue;
        }

        if !crystal.active_frames.is_empty() {
            crystal.animation_timer += dt;
            while crystal.animation_timer >= DASH_CRYSTAL_ANIMATION_INTERVAL {
                crystal.animation_timer -= DASH_CRYSTAL_ANIMATION_INTERVAL;
                crystal.frame_index = (crystal.frame_index + 1) % crystal.active_frames.len();
                sprite.image = crystal.active_frames[crystal.frame_index].clone();
            }
        }

        let Some(crystal_size) = sprite.custom_size else {
            continue;
        };

        let just_started_dash_this_frame = actions.dash_pressed && dash_state.is_dashing;
        if dash_state.dashes_remaining == 0
            && !just_started_dash_this_frame
            && check_collision(
                player_transform.translation,
                player_size.0,
                crystal_transform.translation,
                crystal_size,
            )
        {
            let _crystal_id = &crystal.id;
            dash_state.dashes_remaining = 1;
            actions.dash_requires_release = true;
            crystal.respawn_timer = DASH_CRYSTAL_RESPAWN_TIME;
            crystal.animation_timer = 0.0;
            sprite.image = crystal.vanished_frame.clone();
        }
    }
}

pub fn handle_completion_zones(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_map: Res<LoadedMap>,
    mut completion_state: ResMut<CompletionState>,
    player_query: Query<(&Transform, &ColliderSize), With<Player>>,
    completion_zones: Query<(&Transform, &Sprite), (With<CompletionZone>, Without<Player>)>,
) {
    if completion_state.active {
        return;
    }

    let Ok((player_transform, player_size)) = player_query.get_single() else {
        return;
    };

    for (zone_transform, zone_sprite) in &completion_zones {
        let Some(zone_size) = zone_sprite.custom_size else {
            continue;
        };

        if check_collision(
            player_transform.translation,
            player_size.0,
            zone_transform.translation,
            zone_size,
        ) {
            completion_state.active = true;
            spawn_completion_overlay(&mut commands, &asset_server, &loaded_map);
            break;
        }
    }
}

pub fn handle_completion_overlay_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    completion_state: Res<CompletionState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if completion_state.active && keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::MainMenu);
    }
}

pub fn completion_inactive(completion_state: Res<CompletionState>) -> bool {
    !completion_state.active
}

fn spawn_completion_overlay(
    commands: &mut Commands,
    asset_server: &AssetServer,
    loaded_map: &LoadedMap,
) {
    commands
        .spawn((
            CompletionOverlay,
            GameplayEntity,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            GlobalZIndex(200),
        ))
        .with_children(|parent| {
            spawn_completion_ending_layers(parent, asset_server, loaded_map);
            spawn_completion_text(parent);
        });
}

fn spawn_completion_ending_layers(
    parent: &mut ChildBuilder,
    asset_server: &AssetServer,
    loaded_map: &LoadedMap,
) {
    parent
        .spawn((
            CompletionOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            ZIndex(-1),
        ))
        .with_children(|layers| {
            let ending_layers = if is_chapter_02_map_path(&loaded_map.path) {
                SUMMIT_END_COMPLETION_ENDING_LAYERS.as_slice()
            } else {
                FORSAKEN_CITY_COMPLETION_ENDING_LAYERS.as_slice()
            };

            for path in ending_layers {
                layers.spawn((
                    CompletionOverlay,
                    ImageNode::new(asset_server.load(*path)),
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        max_width: Val::Percent(100.0),
                        max_height: Val::Percent(100.0),
                        ..default()
                    },
                ));
            }
        });
}

fn spawn_completion_text(parent: &mut ChildBuilder) {
    parent
        .spawn((
            CompletionOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            ZIndex(1),
        ))
        .with_children(|text| {
            text.spawn((
                CompletionOverlay,
                Text::new("Press Space to return to Main Menu"),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            text.spawn((
                CompletionOverlay,
                Text::new("Complete"),
                TextFont {
                    font_size: 56.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.45)),
            ));
        });
}

pub fn reset_completion_state(mut completion_state: ResMut<CompletionState>) {
    completion_state.active = false;
}

pub fn handle_hazard_respawn(
    hazards: Query<(&Transform, &Sprite), (With<Hazard>, Without<Player>)>,
    mut death_sequence: ResMut<DeathSequence>,
    player_query: Query<(&Transform, &ColliderSize), With<Player>>,
) {
    if death_sequence.active() {
        return;
    }

    let Ok((player_transform, collider_size)) = player_query.get_single() else {
        return;
    };

    let player_size = collider_size.0;
    for (hazard_transform, hazard_sprite) in &hazards {
        let Some(hazard_size) = hazard_sprite.custom_size else {
            continue;
        };

        if check_collision(
            player_transform.translation,
            player_size,
            hazard_transform.translation,
            hazard_size,
        ) {
            death_sequence.start();
            break;
        }
    }
}

pub fn handle_room_transitions(
    mut commands: Commands,
    time: Res<Time>,
    loaded_map: Res<LoadedMap>,
    level_art: Res<LevelArt>,
    mut active_room: ResMut<ActiveRoom>,
    mut transition_cooldown: ResMut<RoomTransitionCooldown>,
    level_entities: Query<Entity, With<LevelEntity>>,
    window_query: Query<&Window>,
    mut param_set: ParamSet<(
        Query<(&Transform, &ColliderSize), With<Player>>,
        Query<(&Transform, &Sprite, &RoomExitMarker), Without<Player>>,
        Query<
            (
                &mut Transform,
                &mut Velocity,
                &mut Grounded,
                &mut WallContact,
                &mut ColliderSize,
                &mut Crouching,
                &mut DashState,
                &mut DashSlideState,
                &mut PlayerStateMachine,
                &mut ClimbTopOutState,
                &mut CornerBoostState,
                &mut Hair,
                &mut ClimbStamina,
                &Facing,
            ),
            With<Player>,
        >,
        Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
        Query<&mut Transform, (With<WeatherOverlay>, (Without<Player>, Without<Camera2d>))>,
    )>,
) {
    if transition_cooldown.timer > 0.0 {
        transition_cooldown.timer = (transition_cooldown.timer - time.delta_secs()).max(0.0);
        return;
    }

    let player_query = param_set.p0();
    let Ok((player_transform, collider_size)) = player_query.get_single() else {
        return;
    };

    let player_translation = player_transform.translation;
    let player_size = collider_size.0;
    let triggered_exit = {
        let exits_query = param_set.p1();
        let mut result: Option<(String, String, bool)> = None;

        for (exit_transform, exit_sprite, exit) in &exits_query {
            let Some(exit_size) = exit_sprite.custom_size else {
                continue;
            };

            if check_collision(
                player_translation,
                player_size,
                exit_transform.translation,
                exit_size,
            ) {
                let _exit_id = &exit.id;
                result = Some((
                    exit.target_room.clone(),
                    exit.target_spawn.clone(),
                    exit.preserve_momentum,
                ));
                break;
            }
        }

        result
    };

    let Some((target_room_id, target_spawn_id, preserve_momentum)) = triggered_exit else {
        return;
    };

    let Some(target_room) = loaded_map.data.room(&target_room_id) else {
        return;
    };
    let Some(target_spawn) = target_room.spawn_point(&target_spawn_id) else {
        return;
    };

    let mut player_query = param_set.p2();
    let Ok((
        mut player_transform,
        mut velocity,
        mut grounded,
        mut wall_contact,
        mut collider_size,
        mut crouching,
        mut dash_state,
        mut dash_slide,
        mut state_machine,
        mut climb_top_out,
        mut corner_boost,
        mut hair,
        mut climb_stamina,
        facing,
    )) = player_query.get_single_mut()
    else {
        return;
    };

    for entity in &level_entities {
        commands.entity(entity).despawn();
    }

    spawn_room_geometry(&mut commands, target_room, &level_art);

    active_room.room_id = target_room.id.clone();
    active_room.respawn_point = target_room
        .checkpoints
        .first()
        .map(|checkpoint| Vec2::new(checkpoint.x, checkpoint.y))
        .or_else(|| target_room.default_spawn_point())
        .unwrap_or(target_spawn);

    reset_player_to_spawn(
        &mut player_transform,
        &mut velocity,
        &mut grounded,
        &mut wall_contact,
        &mut collider_size,
        &mut crouching,
        &mut dash_state,
        &mut dash_slide,
        &mut state_machine,
        &mut climb_top_out,
        &mut corner_boost,
        &mut hair,
        &mut climb_stamina,
        facing,
        target_spawn,
        preserve_momentum,
    );

    transition_cooldown.timer = ROOM_TRANSITION_COOLDOWN_SECS;

    let camera_position = window_query
        .get_single()
        .map(|window| clamped_camera_position(&target_room.bounds, target_spawn, window))
        .unwrap_or_else(|_| target_spawn);

    for mut camera_transform in &mut param_set.p3() {
        camera_transform.translation.x = camera_position.x;
        camera_transform.translation.y = camera_position.y;
    }

    for mut overlay_transform in &mut param_set.p4() {
        overlay_transform.translation.x = camera_position.x;
        overlay_transform.translation.y = camera_position.y;
    }
}

pub fn camera_follow(
    player_query: Query<&Transform, With<Player>>,
    active_room: Res<ActiveRoom>,
    loaded_map: Res<LoadedMap>,
    window_query: Query<&Window>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>, Without<WeatherOverlay>)>,
    mut weather_query: Query<&mut Transform, (With<WeatherOverlay>, Without<Player>, Without<Camera2d>)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok(window) = window_query.get_single() else {
        return;
    };
    let Some(room) = loaded_map.data.room(&active_room.room_id) else {
        return;
    };

    let target = clamped_camera_position(&room.bounds, player_transform.translation.truncate(), window);

    // Smooth follow (exponential ease)
    let follow_speed = 8.0;
    let t = 1.0 - (-follow_speed * time.delta_secs()).exp();

    if let Ok(mut cam) = camera_query.get_single_mut() {
        cam.translation.x += (target.x - cam.translation.x) * t;
        cam.translation.y += (target.y - cam.translation.y) * t;

        // Keep weather overlay at camera position
        for mut overlay in &mut weather_query {
            overlay.translation.x = cam.translation.x;
            overlay.translation.y = cam.translation.y;
        }
    }
}
