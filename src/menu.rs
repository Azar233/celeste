use std::fs;

use bevy::prelude::*;

use crate::components::{
    ClimbTopOutState, ColliderSize, Crouching, DashSlideState, DashState, Facing,
    Grounded, Hair, LevelEntity, Player, PlayerState, PlayerStateMachine, Velocity,
    WallContact, WeatherOverlay,
};
use crate::constants::{PLAYER_COLLIDER_SIZE, PLAYER_RENDER_Z};
use crate::level::{load_map_from_path, ActiveRoom, LoadedMap};
use crate::scene::{spawn_room_geometry, LevelArt};
use crate::utils::initial_hair_positions;

// ── Resources ──────────────────────────────────────────

#[derive(Resource)]
pub struct MenuOpen(pub bool);

impl Default for MenuOpen {
    fn default() -> Self {
        Self(false)
    }
}

#[derive(Resource)]
pub struct MapRegistry {
    pub maps: Vec<MapEntry>,
}

pub struct MapEntry {
    pub name: String,
}

// ── UI markers ─────────────────────────────────────────

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct MenuAction(&'static str);

#[derive(Component)]
struct MapList;

#[derive(Component)]
struct MapItem(pub String);

// Action tags
const ACT_CONTINUE: &str = "continue";
const ACT_RELOAD_CURRENT: &str = "reload_current";
const ACT_SWITCH_MAP: &str = "switch_map";
const ACT_EXIT: &str = "exit";

// ── Plugin ─────────────────────────────────────────────

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        let maps = scan_maps();
        app.insert_resource(MenuOpen::default())
            .insert_resource(MapRegistry { maps })
            .add_systems(Update, (handle_esc, handle_button_interaction));
    }
}

fn scan_maps() -> Vec<MapEntry> {
    let mut maps = Vec::new();
    if let Ok(entries) = fs::read_dir("assets/maps") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                maps.push(MapEntry { name });
            }
        }
    }
    maps
}

// ── ESC toggle ─────────────────────────────────────────

fn handle_esc(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut menu_open: ResMut<MenuOpen>,
    mut commands: Commands,
    menu_root: Query<Entity, With<MenuRoot>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if menu_open.0 {
            for entity in &menu_root {
                commands.entity(entity).despawn_recursive();
            }
            menu_open.0 = false;
        } else {
            menu_open.0 = true;
            spawn_menu_root(&mut commands);
        }
    }
}

// ── UI construction ────────────────────────────────────

fn spawn_menu_root(commands: &mut Commands) {
    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(100),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 42.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));

            spawn_button(parent, "Continue", ACT_CONTINUE);
            spawn_button(parent, "Reload Current Room", ACT_RELOAD_CURRENT);
            spawn_button(parent, "Switch Map", ACT_SWITCH_MAP);
            spawn_button(parent, "Exit Game", ACT_EXIT);
        });
}

fn spawn_button(parent: &mut ChildBuilder, label: &str, action: &'static str) {
    parent
        .spawn((
            Button,
            MenuAction(action),
            Node {
                width: Val::Px(220.0),
                height: Val::Px(48.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.22, 0.22, 0.32)),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_child((
            Text::new(label.to_string()),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn spawn_map_list(parent: &mut ChildBuilder, maps: &[MapEntry]) {
    parent
        .spawn((
            MapList,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
        ))
        .with_children(|list| {
            for map in maps {
                list.spawn((
                    Button,
                    MapItem(map.name.clone()),
                    Node {
                        width: Val::Px(180.0),
                        height: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.18, 0.28)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .with_child((
                    Text::new(map.name.clone()),
                    TextFont {
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.85)),
                ));
            }
        });
}

// ── Button interaction ─────────────────────────────────

fn handle_button_interaction(
    mut commands: Commands,
    mut menu_open: ResMut<MenuOpen>,
    menu_root: Query<Entity, With<MenuRoot>>,
    map_registry: Res<MapRegistry>,
    map_list_query: Query<Entity, With<MapList>>,
    menu_buttons: Query<(Entity, &Interaction, Option<&MenuAction>, Option<&MapItem>), (Changed<Interaction>, With<Button>)>,
    mut bg_query: Query<&mut BackgroundColor>,
    level_art: Res<LevelArt>,
    loaded_map: Res<LoadedMap>,
    mut active_room: ResMut<ActiveRoom>,
    level_entities: Query<Entity, With<LevelEntity>>,
    mut player_query: Query<
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
            &mut Hair,
            &Facing,
        ),
        (With<Player>, Without<Camera2d>, Without<WeatherOverlay>),
    >,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>, Without<WeatherOverlay>)>,
    mut weather_query: Query<&mut Transform, (With<WeatherOverlay>, Without<Player>, Without<Camera2d>)>,
    mut app_exit: EventWriter<AppExit>,
) {
    for (entity, interaction, action, map_item) in &menu_buttons {
        match *interaction {
            Interaction::Hovered => {
                if let Ok(mut bg) = bg_query.get_mut(entity) {
                    if action.is_some() {
                        *bg = BackgroundColor(Color::srgb(0.38, 0.38, 0.50));
                    } else {
                        *bg = BackgroundColor(Color::srgb(0.30, 0.30, 0.42));
                    }
                }
            }
            Interaction::None => {
                if let Ok(mut bg) = bg_query.get_mut(entity) {
                    if action.is_some() {
                        *bg = BackgroundColor(Color::srgb(0.22, 0.22, 0.32));
                    } else {
                        *bg = BackgroundColor(Color::srgb(0.18, 0.18, 0.28));
                    }
                }
            }
            Interaction::Pressed => {
                if let Ok(mut bg) = bg_query.get_mut(entity) {
                    if action.is_some() {
                        *bg = BackgroundColor(Color::srgb(0.14, 0.14, 0.22));
                    } else {
                        *bg = BackgroundColor(Color::srgb(0.10, 0.10, 0.18));
                    }
                }
                if let Some(action) = action {
                    match action.0 {
                        ACT_CONTINUE => {
                            close_menu(&mut commands, &menu_root, &mut menu_open);
                        }
                        ACT_RELOAD_CURRENT => {
                            reload_current_room(
                                &mut commands,
                                &loaded_map,
                                &level_art,
                                &mut active_room,
                                &level_entities,
                                &mut player_query,
                                &mut camera_query,
                                &mut weather_query,
                            );
                            close_menu(&mut commands, &menu_root, &mut menu_open);
                        }
                        ACT_EXIT => {
                            app_exit.send(AppExit::Success);
                        }
                        ACT_SWITCH_MAP => {
                            for entity in &map_list_query {
                                commands.entity(entity).despawn_recursive();
                            }
                            reload_menu_with_map_list(
                                &mut commands,
                                &menu_root,
                                &map_registry.maps,
                            );
                        }
                        _ => {}
                    }
                }
                if let Some(map_item) = map_item {
                    let map_name = map_item.0.clone();
                    switch_map(
                        &mut commands,
                        &map_name,
                        &level_art,
                        &mut active_room,
                        &level_entities,
                        &mut player_query,
                        &mut camera_query,
                        &mut weather_query,
                    );
                    close_menu(&mut commands, &menu_root, &mut menu_open);
                }
            }
        }
    }
}

// ── Helpers ────────────────────────────────────────────

fn close_menu(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    menu_open: &mut ResMut<MenuOpen>,
) {
    for entity in menu_root {
        commands.entity(entity).despawn_recursive();
    }
    menu_open.0 = false;
}

fn reload_menu_with_map_list(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    maps: &[MapEntry],
) {
    for entity in menu_root {
        commands.entity(entity).despawn_recursive();
    }
    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(100),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 42.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Node {
                    margin: UiRect::bottom(Val::Px(24.0)),
                    ..default()
                },
            ));

            spawn_button(parent, "Continue", ACT_CONTINUE);
            spawn_button(parent, "Reload Current Room", ACT_RELOAD_CURRENT);
            spawn_button(parent, "Switch Map", ACT_SWITCH_MAP);

            if maps.is_empty() {
                parent.spawn((
                    Text::new("(no maps found)"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                ));
            } else {
                spawn_map_list(parent, maps);
            }

            spawn_button(parent, "Exit Game", ACT_EXIT);
        });
}

fn reload_current_room(
    commands: &mut Commands,
    loaded_map: &Res<LoadedMap>,
    level_art: &Res<LevelArt>,
    active_room: &mut ResMut<ActiveRoom>,
    level_entities: &Query<Entity, With<LevelEntity>>,
    player_query: &mut Query<
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
            &mut Hair,
            &Facing,
        ),
        (With<Player>, Without<Camera2d>, Without<WeatherOverlay>),
    >,
    camera_query: &mut Query<&mut Transform, (With<Camera2d>, Without<Player>, Without<WeatherOverlay>)>,
    weather_query: &mut Query<&mut Transform, (With<WeatherOverlay>, Without<Player>, Without<Camera2d>)>,
) {
    let path = loaded_map.path.clone();
    let new_map = match load_map_from_path(&path) {
        Ok(map) => map,
        Err(e) => {
            error!("Failed to reload current map '{}': {e}", path.display());
            return;
        }
    };

    let current_room_id = active_room.room_id.clone();
    let preserved_respawn = active_room.respawn_point;
    let Some(room) = new_map.room(&current_room_id) else {
        error!(
            "Reloaded map '{}' no longer contains current room '{}'",
            path.display(),
            current_room_id
        );
        return;
    };

    let spawn_point = preserved_respawn;
    let new_map_id = new_map.id.clone();

    for entity in level_entities {
        commands.entity(entity).despawn();
    }

    spawn_room_geometry(commands, room, level_art);

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
        mut hair,
        facing,
    )) = player_query.get_single_mut()
    else {
        error!("Player entity not found during current room reload");
        return;
    };

    player_transform.translation = Vec3::new(spawn_point.x, spawn_point.y, PLAYER_RENDER_Z);
    velocity.0 = Vec2::ZERO;
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
    climb_top_out.start = player_transform.translation;
    climb_top_out.target = player_transform.translation;
    collider_size.0 = PLAYER_COLLIDER_SIZE;
    state_machine.current = PlayerState::Normal;
    state_machine.previous = PlayerState::Normal;
    hair.sim_positions = initial_hair_positions(spawn_point, facing.0);

    **active_room = ActiveRoom {
        map_id: new_map_id,
        room_id: current_room_id,
        respawn_point: spawn_point,
    };
    commands.insert_resource(LoadedMap {
        data: new_map,
        path,
    });

    for mut camera_transform in camera_query {
        camera_transform.translation.x = spawn_point.x;
        camera_transform.translation.y = spawn_point.y;
    }
    for mut overlay_transform in weather_query {
        overlay_transform.translation.x = spawn_point.x;
        overlay_transform.translation.y = spawn_point.y;
    }
}

fn switch_map(
    commands: &mut Commands,
    map_name: &str,
    level_art: &Res<LevelArt>,
    active_room: &mut ResMut<ActiveRoom>,
    level_entities: &Query<Entity, With<LevelEntity>>,
    player_query: &mut Query<
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
            &mut Hair,
            &Facing,
        ),
        (With<Player>, Without<Camera2d>, Without<WeatherOverlay>),
    >,
    camera_query: &mut Query<&mut Transform, (With<Camera2d>, Without<Player>, Without<WeatherOverlay>)>,
    weather_query: &mut Query<&mut Transform, (With<WeatherOverlay>, Without<Player>, Without<Camera2d>)>,
) {
    let path = format!("assets/maps/{}.json", map_name);
    let new_map = match load_map_from_path(&path) {
        Ok(map) => map,
        Err(e) => {
            error!("Failed to load map '{path}': {e}");
            return;
        }
    };

    let Some(room) = new_map.starting_room() else {
        error!("Map '{map_name}' has no starting room");
        return;
    };

    let Some(spawn_point) = room.default_spawn_point() else {
        error!("Map '{map_name}' start room has no default spawn");
        return;
    };

    // Extract room data before moving new_map
    let new_map_id = new_map.id.clone();
    let new_room_id = room.id.clone();

    // Despawn old level entities
    for entity in level_entities {
        commands.entity(entity).despawn();
    }

    // Spawn new room
    spawn_room_geometry(commands, room, level_art);

    // Reset player
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
        mut hair,
        facing,
    )) = player_query.get_single_mut()
    else {
        error!("Player entity not found during map switch");
        return;
    };

    player_transform.translation = Vec3::new(spawn_point.x, spawn_point.y, PLAYER_RENDER_Z);
    velocity.0 = Vec2::ZERO;
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
    climb_top_out.start = player_transform.translation;
    climb_top_out.target = player_transform.translation;
    collider_size.0 = PLAYER_COLLIDER_SIZE;
    state_machine.current = PlayerState::Normal;
    state_machine.previous = PlayerState::Normal;
    hair.sim_positions = initial_hair_positions(spawn_point, facing.0);

    // Update resources
    **active_room = ActiveRoom {
        map_id: new_map_id,
        room_id: new_room_id,
        respawn_point: spawn_point,
    };
    commands.insert_resource(LoadedMap {
        data: new_map,
        path: path.into(),
    });

    // Move camera and weather to spawn point
    for mut camera_transform in camera_query {
        camera_transform.translation.x = spawn_point.x;
        camera_transform.translation.y = spawn_point.y;
    }
    for mut overlay_transform in weather_query {
        overlay_transform.translation.x = spawn_point.x;
        overlay_transform.translation.y = spawn_point.y;
    }
}
