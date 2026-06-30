use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::window::PrimaryWindow;

use crate::app_state::GameState;
use crate::constants::SPRING_COLLIDER_SIZE;
use crate::level::{
    ActiveRoom, CollisionKind, CollisionRect, DEFAULT_TILESET_ART_TAG, ExitSide, LoadedMap,
    NamedPoint, RectData, RoomExitData, SpringData, SpringDirection, TILESET_ART_TAGS,
    normalize_tileset_art_tag, save_map_to_path_with_backup,
};

const GRID_SIZE: f32 = 8.0;
const MOVE_DRAG_GRID_SIZE: f32 = GRID_SIZE * 0.5;
const MIN_RECT_SIZE: f32 = 8.0;
const DEFAULT_CAMERA_SCALE: f32 = 1.0;
const DEFAULT_CAMERA_VIEWPORT_HEIGHT: f32 = 180.0;
const EDITOR_CAMERA_MIN_SCALE: f32 = 0.25;
const EDITOR_CAMERA_MAX_SCALE: f32 = 4.0;
const EDITOR_CAMERA_ZOOM_STEP: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorTool {
    Select,
    SolidGround,
    Hazard,
    Checkpoint,
    SpawnPoint,
    DashCrystal,
    Spring,
    Exit,
    CompletionZone,
    Grass,
}

impl EditorTool {
    fn is_rect_tool(self) -> bool {
        matches!(
            self,
            Self::SolidGround | Self::Hazard | Self::Exit | Self::CompletionZone | Self::Grass
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorSelection {
    Collision(usize),
    Hazard(usize),
    Checkpoint(usize),
    SpawnPoint(usize),
    DashCrystal(usize),
    Spring(usize),
    Exit(usize),
    CompletionZone(usize),
    Grass(usize),
}

#[derive(Clone, Copy, Debug)]
struct MoveDragState {
    selection: EditorSelection,
    start_mouse: Vec2,
    original_position: Vec2,
    moved: bool,
}

#[derive(Resource, Debug)]
pub struct EditorState {
    enabled: bool,
    tool: EditorTool,
    selected: Option<EditorSelection>,
    move_drag: Option<MoveDragState>,
    drag_start: Option<Vec2>,
    preview_end: Option<Vec2>,
    next_id: u32,
    dirty: bool,
    last_status: String,
    current_tileset: String,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            enabled: false,
            tool: EditorTool::Select,
            selected: None,
            move_drag: None,
            drag_start: None,
            preview_end: None,
            next_id: 1,
            dirty: false,
            last_status: "F1: toggle editor".to_string(),
            current_tileset: DEFAULT_TILESET_ART_TAG.to_string(),
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorState>().add_systems(
            Update,
            (
                editor_keyboard_shortcuts,
                editor_camera_zoom,
                editor_mouse_input,
                editor_overlay_gizmos,
            )
                .chain()
                .run_if(in_state(GameState::InGame)),
        );
    }
}

fn editor_keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut editor: ResMut<EditorState>,
    mut loaded_map: ResMut<LoadedMap>,
    active_room: Res<ActiveRoom>,
    mut camera_projection_query: Query<&mut OrthographicProjection, With<Camera2d>>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        editor.enabled = !editor.enabled;
        editor.move_drag = None;
        editor.drag_start = None;
        editor.preview_end = None;
        editor.selected = None;
        editor.last_status = if editor.enabled {
            format!(
                "Editor enabled: 1 select, 2 solid, 3 hazard, 4 checkpoint, 5 spawn, 6 dashcrystal, 7 exit, 8 complete, 9 spring, G grass, T tileset, Q/E direction, mouse wheel zoom (current: {})",
                editor.current_tileset
            )
        } else {
            reset_camera_projection(&mut camera_projection_query);
            "Editor disabled; camera zoom reset".to_string()
        };
        info!("{}", editor.last_status);
    }

    if !editor.enabled {
        return;
    }

    if keyboard.just_pressed(KeyCode::Digit1) {
        editor.tool = EditorTool::Select;
        editor.move_drag = None;
        editor.drag_start = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        editor.tool = EditorTool::SolidGround;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        editor.tool = EditorTool::Hazard;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        editor.tool = EditorTool::Checkpoint;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        editor.tool = EditorTool::SpawnPoint;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        editor.tool = EditorTool::DashCrystal;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit7) {
        editor.tool = EditorTool::Exit;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit8) {
        editor.tool = EditorTool::CompletionZone;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::Digit9) {
        editor.tool = EditorTool::Spring;
        editor.move_drag = None;
        editor.selected = None;
    } else if keyboard.just_pressed(KeyCode::KeyG) {
        editor.tool = EditorTool::Grass;
        editor.move_drag = None;
        editor.selected = None;
    }

    if keyboard.just_pressed(KeyCode::KeyT) {
        let next_tileset = next_tileset_art_tag(&editor.current_tileset).to_string();
        editor.current_tileset = next_tileset.clone();

        if editor.tool == EditorTool::Select {
            if let Some(EditorSelection::Collision(collision_index)) = editor.selected {
                if set_collision_tileset(
                    &mut loaded_map.data,
                    &active_room.room_id,
                    collision_index,
                    &next_tileset,
                ) {
                    editor.dirty = true;
                    editor.last_status = format!(
                        "Changed selected Collision {collision_index} tileset to {next_tileset}"
                    );
                    info!("{}", editor.last_status);
                } else {
                    editor.last_status = format!(
                        "Current tileset: {next_tileset}; selected Collision {collision_index} no longer exists"
                    );
                    warn!("{}", editor.last_status);
                }
            } else {
                editor.last_status = format!("Current tileset: {next_tileset}");
                info!("{}", editor.last_status);
            }
        } else {
            editor.last_status = format!("Current tileset: {next_tileset}");
            info!("{}", editor.last_status);
        }
    }

    if keyboard.just_pressed(KeyCode::Delete) {
        if let Some(selection) = editor.selected.take() {
            match delete_selection(&mut loaded_map.data, &active_room.room_id, selection) {
                Ok(()) => {
                    editor.dirty = true;
                    editor.last_status = format!("Deleted {selection:?}");
                    info!("{}", editor.last_status);
                }
                Err(error) => {
                    editor.last_status = error;
                    warn!("{}", editor.last_status);
                }
            }
        }
    }

    if handle_selected_exit_shortcuts(
        &keyboard,
        &mut editor,
        &mut loaded_map.data,
        &active_room.room_id,
    ) {
        editor.dirty = true;
    }

    if handle_selected_spring_shortcuts(
        &keyboard,
        &mut editor,
        &mut loaded_map.data,
        &active_room.room_id,
    ) {
        editor.dirty = true;
    }

    let save_pressed = keyboard.just_pressed(KeyCode::KeyS)
        && (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight));
    if save_pressed {
        let normalized_count = normalize_map_tileset_art_tags(&mut loaded_map.data);
        if normalized_count > 0 {
            editor.dirty = true;
            info!("Normalized {normalized_count} collision art_tag value(s) before save");
        }

        match validate_map_for_save(&loaded_map.data) {
            Ok(()) => match save_map_to_path_with_backup(&loaded_map.path, &loaded_map.data) {
                Ok(backup_path) => {
                    editor.dirty = false;
                    editor.last_status = format!(
                        "Saved map to {} and backup to {}",
                        loaded_map.path.display(),
                        backup_path.display()
                    );
                    info!("{}", editor.last_status);
                }
                Err(error) => {
                    editor.last_status = error.clone();
                    warn!("{error}");
                }
            },
            Err(error) => {
                editor.last_status = error.clone();
                warn!("{error}");
            }
        }
    }
}

fn editor_camera_zoom(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    editor: Res<EditorState>,
    mut camera_projection_query: Query<&mut OrthographicProjection, With<Camera2d>>,
) {
    let scroll_delta: f32 = mouse_wheel_events
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 16.0,
        })
        .sum();

    if !editor.enabled || scroll_delta == 0.0 {
        return;
    }

    let Ok(mut projection) = camera_projection_query.get_single_mut() else {
        return;
    };

    projection.scale = (projection.scale * EDITOR_CAMERA_ZOOM_STEP.powf(scroll_delta))
        .clamp(EDITOR_CAMERA_MIN_SCALE, EDITOR_CAMERA_MAX_SCALE);
}

fn reset_camera_projection(
    camera_projection_query: &mut Query<&mut OrthographicProjection, With<Camera2d>>,
) {
    if let Ok(mut projection) = camera_projection_query.get_single_mut() {
        projection.scale = DEFAULT_CAMERA_SCALE;
        projection.scaling_mode = ScalingMode::FixedVertical {
            viewport_height: DEFAULT_CAMERA_VIEWPORT_HEIGHT,
        };
    }
}

fn editor_mouse_input(
    mouse: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut editor: ResMut<EditorState>,
    mut loaded_map: ResMut<LoadedMap>,
    active_room: Res<ActiveRoom>,
) {
    if !editor.enabled {
        return;
    }

    let Some(world_pos) = cursor_world_position(&window_query, &camera_query) else {
        if mouse.just_released(MouseButton::Left) {
            editor.move_drag = None;
        }
        return;
    };
    let snapped_pos = snap_to_grid(world_pos);
    let snapped_move_pos = snap_to_move_drag_grid(world_pos);

    if mouse.just_pressed(MouseButton::Left) {
        if let Some(selection) = editor.selected {
            if pick_object(&loaded_map.data, &active_room.room_id, world_pos) == Some(selection) {
                if let Some(original_position) =
                    selection_position(&loaded_map.data, &active_room.room_id, selection)
                {
                    editor.move_drag = Some(MoveDragState {
                        selection,
                        start_mouse: snapped_move_pos,
                        original_position,
                        moved: false,
                    });
                    editor.drag_start = None;
                    editor.preview_end = None;
                    return;
                }
            }
        }
    }

    if let Some(mut move_drag) = editor.move_drag {
        if mouse.pressed(MouseButton::Left) {
            let moved_position =
                move_drag.original_position + (snapped_move_pos - move_drag.start_mouse);
            let target_position = if matches!(move_drag.selection, EditorSelection::Spring(_)) {
                moved_position
            } else {
                snap_to_move_drag_grid(moved_position)
            };
            if target_position != move_drag.original_position {
                match move_selection_to(
                    &mut loaded_map.data,
                    &active_room.room_id,
                    move_drag.selection,
                    target_position,
                ) {
                    Ok(()) => {
                        move_drag.moved = true;
                        editor.move_drag = Some(move_drag);
                        editor.dirty = true;
                    }
                    Err(error) => {
                        editor.move_drag = None;
                        editor.last_status = error;
                        warn!("{}", editor.last_status);
                    }
                }
            }
            return;
        }

        if mouse.just_released(MouseButton::Left) {
            editor.move_drag = None;
            if move_drag.moved {
                editor.last_status = format!("Moved {:?}", move_drag.selection);
                info!("{}", editor.last_status);
            }
            return;
        }
    }

    if editor.tool.is_rect_tool() {
        if mouse.just_pressed(MouseButton::Left) {
            editor.drag_start = Some(snapped_pos);
            editor.preview_end = Some(snapped_pos);
        }

        if mouse.pressed(MouseButton::Left) && editor.drag_start.is_some() {
            editor.preview_end = Some(snapped_pos);
        }

        if mouse.just_released(MouseButton::Left) {
            let drag_start = editor.drag_start.take();
            editor.preview_end = None;
            if let Some(start) = drag_start {
                let rect = rect_from_points(start, snapped_pos);
                if rect.w >= MIN_RECT_SIZE && rect.h >= MIN_RECT_SIZE {
                    let next_id = editor.next_id;
                    let current_tileset = editor.current_tileset.clone();
                    let created = add_rect_object(
                        &mut loaded_map.data,
                        &active_room.room_id,
                        editor.tool,
                        rect,
                        next_id,
                        &current_tileset,
                    );
                    if created {
                        editor.next_id += 1;
                        editor.dirty = true;
                        editor.last_status = if editor.tool == EditorTool::SolidGround {
                            format!("Created {:?} with tileset {current_tileset}", editor.tool)
                        } else {
                            format!("Created {:?}", editor.tool)
                        };
                        info!("{}", editor.last_status);
                    }
                }
            }
        }
    } else if mouse.just_pressed(MouseButton::Left) {
        match editor.tool {
            EditorTool::Select => {
                editor.selected = pick_object(&loaded_map.data, &active_room.room_id, world_pos);
                editor.last_status = editor
                    .selected
                    .map(|selection| format!("Selected {selection:?}"))
                    .unwrap_or_else(|| "Selection cleared".to_string());
                info!("{}", editor.last_status);
            }
            EditorTool::Checkpoint | EditorTool::SpawnPoint | EditorTool::DashCrystal => {
                let id = format!("editor_{}", editor.next_id);
                let point = NamedPoint {
                    id,
                    x: snapped_pos.x,
                    y: snapped_pos.y,
                };
                if add_point_object(
                    &mut loaded_map.data,
                    &active_room.room_id,
                    editor.tool,
                    point,
                ) {
                    editor.next_id += 1;
                    editor.dirty = true;
                    editor.last_status = format!("Created {:?}", editor.tool);
                    info!("{}", editor.last_status);
                }
            }
            EditorTool::Spring => {
                let spring_position = snap_spring_center_to_grid(world_pos, SpringDirection::Up);
                let spring = SpringData {
                    id: format!("editor_spring_{}", editor.next_id),
                    x: spring_position.x,
                    y: spring_position.y,
                    direction: SpringDirection::Up,
                };
                if add_spring_object(&mut loaded_map.data, &active_room.room_id, spring) {
                    editor.next_id += 1;
                    editor.dirty = true;
                    editor.last_status = "Created Spring".to_string();
                    info!("{}", editor.last_status);
                }
            }
            _ => {}
        }
    }
}

fn editor_overlay_gizmos(
    mut gizmos: Gizmos,
    editor: Res<EditorState>,
    loaded_map: Option<Res<LoadedMap>>,
    active_room: Option<Res<ActiveRoom>>,
) {
    if !editor.enabled {
        return;
    }

    let Some(loaded_map) = loaded_map else {
        return;
    };
    let Some(active_room) = active_room else {
        return;
    };
    let Some(room) = loaded_map.data.room(&active_room.room_id) else {
        return;
    };

    draw_grid(&mut gizmos, &room.bounds);

    for (index, collision) in room.collision.iter().enumerate() {
        let Some(color) = editor_collision_color(&collision.kind) else {
            continue;
        };
        let selected = editor.selected == Some(EditorSelection::Collision(index));
        draw_rect_outline(
            &mut gizmos,
            Vec2::new(collision.x, collision.y),
            Vec2::new(collision.w, collision.h),
            if selected { Color::WHITE } else { color },
        );
    }

    for (index, hazard) in room.hazards.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::Hazard(index));
        draw_rect_outline(
            &mut gizmos,
            hazard.center(),
            hazard.size(),
            if selected {
                Color::WHITE
            } else {
                Color::srgb(1.0, 0.2, 0.2)
            },
        );
    }

    for (index, checkpoint) in room.checkpoints.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::Checkpoint(index));
        draw_point_marker(
            &mut gizmos,
            Vec2::new(checkpoint.x, checkpoint.y),
            6.0,
            if selected {
                Color::WHITE
            } else {
                Color::srgb(1.0, 0.9, 0.2)
            },
        );
    }

    for (index, spawn) in room.spawn_points.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::SpawnPoint(index));
        draw_point_marker(
            &mut gizmos,
            Vec2::new(spawn.x, spawn.y),
            5.0,
            if selected {
                Color::WHITE
            } else {
                Color::srgb(0.25, 0.65, 1.0)
            },
        );
    }

    for (index, dashcrystal) in room.dashcrystals.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::DashCrystal(index));
        draw_point_marker(
            &mut gizmos,
            Vec2::new(dashcrystal.x, dashcrystal.y),
            6.0,
            if selected {
                Color::WHITE
            } else {
                Color::srgb(0.35, 1.0, 1.0)
            },
        );
    }

    for (index, spring) in room.springs.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::Spring(index));
        let position = Vec2::new(spring.x, spring.y);
        draw_rect_outline(
            &mut gizmos,
            position,
            spring_collision_size(spring.direction),
            if selected {
                Color::WHITE
            } else {
                Color::srgb(1.0, 0.55, 0.25)
            },
        );
        draw_spring_direction_marker(&mut gizmos, position, spring.direction, selected);
    }

    for (index, grass_rect) in room.grasses.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::Grass(index));
        draw_rect_outline(
            &mut gizmos,
            grass_rect.center(),
            grass_rect.size(),
            if selected {
                Color::WHITE
            } else {
                Color::srgb(0.3, 0.85, 0.2)
            },
        );
    }

    for (index, completion_zone) in room.completion_zones.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::CompletionZone(index));
        draw_rect_outline(
            &mut gizmos,
            completion_zone.center(),
            completion_zone.size(),
            if selected {
                Color::WHITE
            } else {
                Color::srgb(0.35, 1.0, 0.35)
            },
        );
    }

    for (index, exit) in room.exits.iter().enumerate() {
        let selected = editor.selected == Some(EditorSelection::Exit(index));
        draw_rect_outline(
            &mut gizmos,
            Vec2::new(exit.x, exit.y),
            Vec2::new(exit.w, exit.h),
            if selected {
                Color::WHITE
            } else {
                Color::srgb(0.75, 0.35, 1.0)
            },
        );
        draw_exit_markers(&mut gizmos, exit, selected);
    }

    if let (Some(start), Some(end)) = (editor.drag_start, editor.preview_end) {
        let rect = rect_from_points(start, end);
        draw_rect_outline(
            &mut gizmos,
            rect.center(),
            rect.size(),
            Color::srgba(1.0, 1.0, 1.0, 0.85),
        );
    }
}

fn cursor_world_position(
    window_query: &Query<&Window, With<PrimaryWindow>>,
    camera_query: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let window = window_query.get_single().ok()?;
    let cursor_position = window.cursor_position()?;
    let (camera, camera_transform) = camera_query.get_single().ok()?;
    camera
        .viewport_to_world_2d(camera_transform, cursor_position)
        .ok()
}

fn snap_to_grid(position: Vec2) -> Vec2 {
    snap_to_step(position, GRID_SIZE)
}

fn snap_to_move_drag_grid(position: Vec2) -> Vec2 {
    snap_to_step(position, MOVE_DRAG_GRID_SIZE)
}

fn snap_spring_center_to_grid(position: Vec2, direction: SpringDirection) -> Vec2 {
    snap_spring_center_to_step(position, direction, GRID_SIZE)
}

fn snap_spring_center_to_move_drag_grid(position: Vec2, direction: SpringDirection) -> Vec2 {
    snap_spring_center_to_step(position, direction, MOVE_DRAG_GRID_SIZE)
}

fn snap_spring_center_to_step(position: Vec2, direction: SpringDirection, step: f32) -> Vec2 {
    let half_thickness = SPRING_COLLIDER_SIZE.y * 0.5;

    match direction {
        SpringDirection::Up => Vec2::new(
            snap_axis_to_step(position.x, step),
            snap_axis_to_step(position.y - half_thickness, step) + half_thickness,
        ),
        SpringDirection::Down => Vec2::new(
            snap_axis_to_step(position.x, step),
            snap_axis_to_step(position.y + half_thickness, step) - half_thickness,
        ),
        SpringDirection::Left => Vec2::new(
            snap_axis_to_step(position.x + half_thickness, step) - half_thickness,
            snap_axis_to_step(position.y, step),
        ),
        SpringDirection::Right => Vec2::new(
            snap_axis_to_step(position.x - half_thickness, step) + half_thickness,
            snap_axis_to_step(position.y, step),
        ),
    }
}

fn snap_to_step(position: Vec2, step: f32) -> Vec2 {
    Vec2::new(
        snap_axis_to_step(position.x, step),
        snap_axis_to_step(position.y, step),
    )
}

fn snap_axis_to_step(value: f32, step: f32) -> f32 {
    (value / step).round() * step
}

fn rect_from_points(a: Vec2, b: Vec2) -> RectData {
    let min = a.min(b);
    let max = a.max(b);
    let size = (max - min).max(Vec2::splat(MIN_RECT_SIZE));
    let center = (min + max) * 0.5;

    RectData {
        x: center.x,
        y: center.y,
        w: size.x,
        h: size.y,
    }
}

fn room_mut<'a>(
    map: &'a mut crate::level::MapFile,
    room_id: &str,
) -> Option<&'a mut crate::level::RoomData> {
    map.rooms.iter_mut().find(|room| room.id == room_id)
}

fn add_rect_object(
    map: &mut crate::level::MapFile,
    room_id: &str,
    tool: EditorTool,
    rect: RectData,
    next_id: u32,
    current_tileset: &str,
) -> bool {
    let Some(room) = room_mut(map, room_id) else {
        return false;
    };

    match tool {
        EditorTool::SolidGround => {
            room.collision.push(CollisionRect {
                kind: CollisionKind::SolidGround,
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
                art_tag: Some(current_tileset.to_string()),
            });
            true
        }
        EditorTool::Hazard => {
            room.hazards.push(rect);
            true
        }
        EditorTool::Exit => {
            room.exits.push(RoomExitData {
                id: format!("editor_exit_{next_id}"),
                side: ExitSide::Right,
                target_room: room.id.clone(),
                target_spawn: room.default_spawn.clone(),
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
                preserve_momentum: false,
            });
            true
        }
        EditorTool::CompletionZone => {
            room.completion_zones.push(rect);
            true
        }
        EditorTool::Grass => {
            room.grasses.push(rect);
            true
        }
        _ => false,
    }
}

fn next_tileset_art_tag(current: &str) -> &'static str {
    let normalized_current = normalize_tileset_art_tag(current).unwrap_or(DEFAULT_TILESET_ART_TAG);
    let current_index = TILESET_ART_TAGS
        .iter()
        .position(|tag| *tag == normalized_current)
        .unwrap_or(0);
    TILESET_ART_TAGS[(current_index + 1) % TILESET_ART_TAGS.len()]
}

fn is_valid_tileset_art_tag(art_tag: &str) -> bool {
    normalize_tileset_art_tag(art_tag).is_some()
}

fn normalize_map_tileset_art_tags(map: &mut crate::level::MapFile) -> usize {
    let mut normalized_count = 0;

    for room in &mut map.rooms {
        for collision in &mut room.collision {
            let Some(art_tag) = collision.art_tag.as_mut() else {
                continue;
            };
            let Some(normalized_art_tag) = normalize_tileset_art_tag(art_tag) else {
                continue;
            };

            if art_tag != normalized_art_tag {
                *art_tag = normalized_art_tag.to_string();
                normalized_count += 1;
            }
        }
    }

    normalized_count
}

fn set_collision_tileset(
    map: &mut crate::level::MapFile,
    room_id: &str,
    collision_index: usize,
    art_tag: &str,
) -> bool {
    let Some(room) = room_mut(map, room_id) else {
        return false;
    };
    let Some(collision) = room.collision.get_mut(collision_index) else {
        return false;
    };

    collision.art_tag = Some(art_tag.to_string());
    true
}

fn selection_position(
    map: &crate::level::MapFile,
    room_id: &str,
    selection: EditorSelection,
) -> Option<Vec2> {
    let room = map.room(room_id)?;

    match selection {
        EditorSelection::Collision(index) => room
            .collision
            .get(index)
            .map(|collision| Vec2::new(collision.x, collision.y)),
        EditorSelection::Hazard(index) => room.hazards.get(index).map(RectData::center),
        EditorSelection::Checkpoint(index) => room
            .checkpoints
            .get(index)
            .map(|checkpoint| Vec2::new(checkpoint.x, checkpoint.y)),
        EditorSelection::SpawnPoint(index) => room
            .spawn_points
            .get(index)
            .map(|spawn| Vec2::new(spawn.x, spawn.y)),
        EditorSelection::DashCrystal(index) => room
            .dashcrystals
            .get(index)
            .map(|dashcrystal| Vec2::new(dashcrystal.x, dashcrystal.y)),
        EditorSelection::Spring(index) => room
            .springs
            .get(index)
            .map(|spring| Vec2::new(spring.x, spring.y)),
        EditorSelection::Exit(index) => room.exits.get(index).map(|exit| Vec2::new(exit.x, exit.y)),
        EditorSelection::CompletionZone(index) => {
            room.completion_zones.get(index).map(RectData::center)
        }
        EditorSelection::Grass(index) => room.grasses.get(index).map(RectData::center),
    }
}

fn move_selection_to(
    map: &mut crate::level::MapFile,
    room_id: &str,
    selection: EditorSelection,
    target_position: Vec2,
) -> Result<(), String> {
    let Some(room) = room_mut(map, room_id) else {
        return Err(format!(
            "Cannot move {selection:?}: active room '{room_id}' does not exist"
        ));
    };

    match selection {
        EditorSelection::Collision(index) => {
            let Some(collision) = room.collision.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            collision.x = target_position.x;
            collision.y = target_position.y;
        }
        EditorSelection::Hazard(index) => {
            let Some(hazard) = room.hazards.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            hazard.x = target_position.x;
            hazard.y = target_position.y;
        }
        EditorSelection::Checkpoint(index) => {
            let Some(checkpoint) = room.checkpoints.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            checkpoint.x = target_position.x;
            checkpoint.y = target_position.y;
        }
        EditorSelection::SpawnPoint(index) => {
            let Some(spawn) = room.spawn_points.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            spawn.x = target_position.x;
            spawn.y = target_position.y;
        }
        EditorSelection::DashCrystal(index) => {
            let Some(dashcrystal) = room.dashcrystals.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            dashcrystal.x = target_position.x;
            dashcrystal.y = target_position.y;
        }
        EditorSelection::Spring(index) => {
            let Some(spring) = room.springs.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            let snapped_position =
                snap_spring_center_to_move_drag_grid(target_position, spring.direction);
            spring.x = snapped_position.x;
            spring.y = snapped_position.y;
        }
        EditorSelection::Exit(index) => {
            let Some(exit) = room.exits.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            exit.x = target_position.x;
            exit.y = target_position.y;
        }
        EditorSelection::CompletionZone(index) => {
            let Some(completion_zone) = room.completion_zones.get_mut(index) else {
                return Err(format!(
                    "Cannot move {selection:?}: selected object no longer exists"
                ));
            };
            completion_zone.x = target_position.x;
            completion_zone.y = target_position.y;
        }
        EditorSelection::Grass(index) => {
            let Some(grass) = room.grasses.get_mut(index) else {
                return Err(format!("Cannot move {selection:?}: selected object no longer exists"));
            };
            grass.x = target_position.x;
            grass.y = target_position.y;
        }
    }

    Ok(())
}

fn handle_selected_exit_shortcuts(
    keyboard: &ButtonInput<KeyCode>,
    editor: &mut EditorState,
    map: &mut crate::level::MapFile,
    active_room_id: &str,
) -> bool {
    let Some(EditorSelection::Exit(exit_index)) = editor.selected else {
        return false;
    };

    let cycle_room = keyboard.just_pressed(KeyCode::Tab);
    let previous_spawn = keyboard.just_pressed(KeyCode::BracketLeft);
    let next_spawn = keyboard.just_pressed(KeyCode::BracketRight);
    let previous_side = keyboard.just_pressed(KeyCode::KeyQ);
    let next_side = keyboard.just_pressed(KeyCode::KeyE);
    let toggle_momentum = keyboard.just_pressed(KeyCode::KeyM);

    if !(cycle_room
        || previous_spawn
        || next_spawn
        || previous_side
        || next_side
        || toggle_momentum)
    {
        return false;
    }

    let room_infos: Vec<(String, String, Vec<String>)> = map
        .rooms
        .iter()
        .map(|room| {
            (
                room.id.clone(),
                room.default_spawn.clone(),
                room.spawn_points
                    .iter()
                    .map(|spawn| spawn.id.clone())
                    .collect(),
            )
        })
        .collect();
    let Some(active_room_index) = map.rooms.iter().position(|room| room.id == active_room_id)
    else {
        editor.last_status =
            format!("Cannot edit Exit: active room '{active_room_id}' does not exist");
        warn!("{}", editor.last_status);
        return false;
    };
    let Some(exit_snapshot) = map.rooms[active_room_index].exits.get(exit_index).cloned() else {
        editor.last_status = format!("Cannot edit Exit: index {exit_index} no longer exists");
        warn!("{}", editor.last_status);
        return false;
    };

    let mut target_room = exit_snapshot.target_room;
    let mut target_spawn = exit_snapshot.target_spawn;
    let mut side = exit_snapshot.side;
    let mut preserve_momentum = exit_snapshot.preserve_momentum;
    let mut changed = false;

    if cycle_room && !room_infos.is_empty() {
        let current_index = room_infos
            .iter()
            .position(|(room_id, _, _)| room_id == &target_room)
            .unwrap_or(0);
        let next_index = (current_index + 1) % room_infos.len();
        target_room = room_infos[next_index].0.clone();
        target_spawn = preferred_spawn_for_room(&room_infos[next_index]);
        changed = true;
    }

    if previous_spawn || next_spawn {
        if let Some(room_info) = room_infos
            .iter()
            .find(|(room_id, _, _)| room_id == &target_room)
        {
            let spawn_ids = spawn_choices_for_room(room_info);
            if !spawn_ids.is_empty() {
                let current_index = spawn_ids
                    .iter()
                    .position(|spawn_id| spawn_id == &target_spawn)
                    .unwrap_or(0);
                let next_index = if previous_spawn {
                    (current_index + spawn_ids.len() - 1) % spawn_ids.len()
                } else {
                    (current_index + 1) % spawn_ids.len()
                };
                target_spawn = spawn_ids[next_index].clone();
                changed = true;
            }
        } else {
            editor.last_status = format!(
                "Cannot cycle Exit spawn: target room '{}' does not exist",
                target_room
            );
            warn!("{}", editor.last_status);
        }
    }

    if previous_side {
        side = cycle_exit_side(&side, -1);
        changed = true;
    }
    if next_side {
        side = cycle_exit_side(&side, 1);
        changed = true;
    }
    if toggle_momentum {
        preserve_momentum = !preserve_momentum;
        changed = true;
    }

    if !changed {
        return false;
    }

    let Some(exit) = map.rooms[active_room_index].exits.get_mut(exit_index) else {
        editor.last_status = format!("Cannot edit Exit: index {exit_index} no longer exists");
        warn!("{}", editor.last_status);
        return false;
    };
    exit.target_room = target_room;
    exit.target_spawn = target_spawn;
    exit.side = side;
    exit.preserve_momentum = preserve_momentum;

    editor.last_status = describe_exit(exit);
    info!("{}", editor.last_status);
    true
}

fn spawn_choices_for_room(room_info: &(String, String, Vec<String>)) -> Vec<String> {
    let (_, default_spawn, spawn_ids) = room_info;
    if spawn_ids.is_empty() {
        vec![default_spawn.clone()]
    } else {
        spawn_ids.clone()
    }
}

fn preferred_spawn_for_room(room_info: &(String, String, Vec<String>)) -> String {
    let (_, default_spawn, spawn_ids) = room_info;
    if spawn_ids.iter().any(|spawn_id| spawn_id == default_spawn) || spawn_ids.is_empty() {
        default_spawn.clone()
    } else {
        spawn_ids[0].clone()
    }
}

fn cycle_exit_side(side: &ExitSide, direction: i32) -> ExitSide {
    let current_index = match side {
        ExitSide::Left => 0,
        ExitSide::Right => 1,
        ExitSide::Top => 2,
        ExitSide::Bottom => 3,
    };
    let next_index = (current_index + direction).rem_euclid(4);
    match next_index {
        0 => ExitSide::Left,
        1 => ExitSide::Right,
        2 => ExitSide::Top,
        _ => ExitSide::Bottom,
    }
}

fn describe_exit(exit: &RoomExitData) -> String {
    format!(
        "Exit '{}' -> room '{}' spawn '{}' side {:?} preserve_momentum={}",
        exit.id, exit.target_room, exit.target_spawn, exit.side, exit.preserve_momentum
    )
}

fn handle_selected_spring_shortcuts(
    keyboard: &ButtonInput<KeyCode>,
    editor: &mut EditorState,
    map: &mut crate::level::MapFile,
    active_room_id: &str,
) -> bool {
    let Some(EditorSelection::Spring(spring_index)) = editor.selected else {
        return false;
    };

    let previous_direction = keyboard.just_pressed(KeyCode::KeyQ);
    let next_direction = keyboard.just_pressed(KeyCode::KeyE);
    if !(previous_direction || next_direction) {
        return false;
    }

    let Some(room) = room_mut(map, active_room_id) else {
        editor.last_status =
            format!("Cannot edit Spring: active room '{active_room_id}' does not exist");
        warn!("{}", editor.last_status);
        return false;
    };
    let Some(spring) = room.springs.get_mut(spring_index) else {
        editor.last_status = format!("Cannot edit Spring: index {spring_index} no longer exists");
        warn!("{}", editor.last_status);
        return false;
    };

    spring.direction = if next_direction {
        spring.direction.next()
    } else {
        previous_spring_direction(spring.direction)
    };
    let snapped_position =
        snap_spring_center_to_grid(Vec2::new(spring.x, spring.y), spring.direction);
    spring.x = snapped_position.x;
    spring.y = snapped_position.y;
    editor.last_status = describe_spring(spring);
    info!("{}", editor.last_status);
    true
}

fn previous_spring_direction(direction: SpringDirection) -> SpringDirection {
    match direction {
        SpringDirection::Up => SpringDirection::Left,
        SpringDirection::Left => SpringDirection::Down,
        SpringDirection::Down => SpringDirection::Right,
        SpringDirection::Right => SpringDirection::Up,
    }
}

fn describe_spring(spring: &SpringData) -> String {
    format!("Spring '{}' direction {:?}", spring.id, spring.direction)
}

fn add_spring_object(map: &mut crate::level::MapFile, room_id: &str, spring: SpringData) -> bool {
    let Some(room) = room_mut(map, room_id) else {
        return false;
    };

    room.springs.push(spring);
    true
}

fn add_point_object(
    map: &mut crate::level::MapFile,
    room_id: &str,
    tool: EditorTool,
    point: NamedPoint,
) -> bool {
    let Some(room) = room_mut(map, room_id) else {
        return false;
    };

    match tool {
        EditorTool::Checkpoint => {
            room.checkpoints.push(point);
            true
        }
        EditorTool::SpawnPoint => {
            room.spawn_points.push(point);
            true
        }
        EditorTool::DashCrystal => {
            room.dashcrystals.push(point);
            true
        }
        _ => false,
    }
}

fn delete_selection(
    map: &mut crate::level::MapFile,
    room_id: &str,
    selection: EditorSelection,
) -> Result<(), String> {
    if matches!(selection, EditorSelection::SpawnPoint(_)) {
        return delete_spawn_point_selection(map, room_id, selection);
    }

    let Some(room) = room_mut(map, room_id) else {
        return Err(format!(
            "Cannot delete {selection:?}: active room '{room_id}' does not exist"
        ));
    };

    match selection {
        EditorSelection::Collision(index) if index < room.collision.len() => {
            room.collision.remove(index);
            Ok(())
        }
        EditorSelection::Hazard(index) if index < room.hazards.len() => {
            room.hazards.remove(index);
            Ok(())
        }
        EditorSelection::Checkpoint(index) if index < room.checkpoints.len() => {
            room.checkpoints.remove(index);
            Ok(())
        }
        EditorSelection::DashCrystal(index) if index < room.dashcrystals.len() => {
            room.dashcrystals.remove(index);
            Ok(())
        }
        EditorSelection::Spring(index) if index < room.springs.len() => {
            room.springs.remove(index);
            Ok(())
        }
        EditorSelection::Exit(index) if index < room.exits.len() => {
            room.exits.remove(index);
            Ok(())
        }
        EditorSelection::CompletionZone(index) if index < room.completion_zones.len() => {
            room.completion_zones.remove(index);
            Ok(())
        }
        EditorSelection::Grass(index) if index < room.grasses.len() => {
            room.grasses.remove(index);
            Ok(())
        }
        _ => Err(format!(
            "Cannot delete {selection:?}: selected object no longer exists"
        )),
    }
}

fn delete_spawn_point_selection(
    map: &mut crate::level::MapFile,
    room_id: &str,
    selection: EditorSelection,
) -> Result<(), String> {
    let EditorSelection::SpawnPoint(index) = selection else {
        return Err(format!(
            "Cannot delete {selection:?}: expected a spawn point selection"
        ));
    };

    let Some(room_index) = map.rooms.iter().position(|room| room.id == room_id) else {
        return Err(format!(
            "Cannot delete SpawnPoint({index}): active room '{room_id}' does not exist"
        ));
    };
    let Some(spawn) = map.rooms[room_index].spawn_points.get(index) else {
        return Err(format!(
            "Cannot delete SpawnPoint({index}): selected spawn point no longer exists"
        ));
    };
    let spawn_id = spawn.id.clone();

    if map.rooms[room_index].default_spawn == spawn_id {
        return Err(format!(
            "Cannot delete spawn point '{spawn_id}' in room '{room_id}': it is this room's default_spawn. Select/create another spawn and update the map before deleting it."
        ));
    }

    if let Some((source_room, exit_id)) = map.rooms.iter().find_map(|room| {
        room.exits
            .iter()
            .find(|exit| exit.target_room == room_id && exit.target_spawn == spawn_id)
            .map(|exit| (room.id.clone(), exit.id.clone()))
    }) {
        return Err(format!(
            "Cannot delete spawn point '{spawn_id}' in room '{room_id}': exit '{exit_id}' in room '{source_room}' targets it. Retarget or delete that exit first."
        ));
    }

    map.rooms[room_index].spawn_points.remove(index);
    Ok(())
}

fn pick_object(
    map: &crate::level::MapFile,
    room_id: &str,
    position: Vec2,
) -> Option<EditorSelection> {
    let room = map.room(room_id)?;

    for (index, exit) in room.exits.iter().enumerate().rev() {
        if point_in_rect(
            position,
            Vec2::new(exit.x, exit.y),
            Vec2::new(exit.w, exit.h),
        ) {
            return Some(EditorSelection::Exit(index));
        }
    }

    for (index, completion_zone) in room.completion_zones.iter().enumerate().rev() {
        if point_in_rect(position, completion_zone.center(), completion_zone.size()) {
            return Some(EditorSelection::CompletionZone(index));
        }
    }

    for (index, hazard) in room.hazards.iter().enumerate().rev() {
        if point_in_rect(position, hazard.center(), hazard.size()) {
            return Some(EditorSelection::Hazard(index));
        }
    }

    for (index, grass) in room.grasses.iter().enumerate().rev() {
        if point_in_rect(position, grass.center(), grass.size()) {
            return Some(EditorSelection::Grass(index));
        }
    }

    for (index, collision) in room.collision.iter().enumerate().rev() {
        if editor_collision_color(&collision.kind).is_some()
            && point_in_rect(
                position,
                Vec2::new(collision.x, collision.y),
                Vec2::new(collision.w, collision.h),
            )
        {
            return Some(EditorSelection::Collision(index));
        }
    }

    for (index, spring) in room.springs.iter().enumerate().rev() {
        if point_in_rect(
            position,
            Vec2::new(spring.x, spring.y),
            spring_collision_size(spring.direction),
        ) {
            return Some(EditorSelection::Spring(index));
        }
    }

    for (index, dashcrystal) in room.dashcrystals.iter().enumerate().rev() {
        if position.distance(Vec2::new(dashcrystal.x, dashcrystal.y)) <= 8.0 {
            return Some(EditorSelection::DashCrystal(index));
        }
    }

    for (index, checkpoint) in room.checkpoints.iter().enumerate().rev() {
        if position.distance(Vec2::new(checkpoint.x, checkpoint.y)) <= 8.0 {
            return Some(EditorSelection::Checkpoint(index));
        }
    }

    for (index, spawn) in room.spawn_points.iter().enumerate().rev() {
        if position.distance(Vec2::new(spawn.x, spawn.y)) <= 8.0 {
            return Some(EditorSelection::SpawnPoint(index));
        }
    }

    None
}

fn editor_collision_color(kind: &CollisionKind) -> Option<Color> {
    match kind {
        CollisionKind::SolidGround => Some(Color::srgb(0.2, 0.9, 0.35)),
        CollisionKind::WallSurface => Some(Color::srgb(0.0, 0.85, 1.0)),
        CollisionKind::OneWayPlatform => Some(Color::srgb(1.0, 0.9, 0.1)),
        CollisionKind::CameraZone | CollisionKind::EffectZone => None,
    }
}

fn spring_collision_size(direction: SpringDirection) -> Vec2 {
    match direction {
        SpringDirection::Up | SpringDirection::Down => SPRING_COLLIDER_SIZE,
        SpringDirection::Left | SpringDirection::Right => {
            Vec2::new(SPRING_COLLIDER_SIZE.y, SPRING_COLLIDER_SIZE.x)
        }
    }
}

fn point_in_rect(point: Vec2, center: Vec2, size: Vec2) -> bool {
    let half = size * 0.5;
    point.x >= center.x - half.x
        && point.x <= center.x + half.x
        && point.y >= center.y - half.y
        && point.y <= center.y + half.y
}

fn draw_grid(gizmos: &mut Gizmos, bounds: &RectData) {
    let left = bounds.x - bounds.w * 0.5;
    let right = bounds.x + bounds.w * 0.5;
    let bottom = bounds.y - bounds.h * 0.5;
    let top = bounds.y + bounds.h * 0.5;
    let grid_color = Color::srgba(0.45, 0.55, 0.65, 0.25);
    let bounds_color = Color::srgba(1.0, 1.0, 1.0, 0.7);

    let mut x = (left / GRID_SIZE).floor() * GRID_SIZE;
    while x <= right {
        gizmos.line_2d(Vec2::new(x, bottom), Vec2::new(x, top), grid_color);
        x += GRID_SIZE;
    }

    let mut y = (bottom / GRID_SIZE).floor() * GRID_SIZE;
    while y <= top {
        gizmos.line_2d(Vec2::new(left, y), Vec2::new(right, y), grid_color);
        y += GRID_SIZE;
    }

    draw_rect_outline(gizmos, bounds.center(), bounds.size(), bounds_color);
}

fn draw_rect_outline(gizmos: &mut Gizmos, center: Vec2, size: Vec2, color: Color) {
    let half = size * 0.5;
    let min = center - half;
    let max = center + half;
    let bl = Vec2::new(min.x, min.y);
    let br = Vec2::new(max.x, min.y);
    let tr = Vec2::new(max.x, max.y);
    let tl = Vec2::new(min.x, max.y);

    gizmos.line_2d(bl, br, color);
    gizmos.line_2d(br, tr, color);
    gizmos.line_2d(tr, tl, color);
    gizmos.line_2d(tl, bl, color);
}

fn draw_exit_markers(gizmos: &mut Gizmos, exit: &RoomExitData, selected: bool) {
    let center = Vec2::new(exit.x, exit.y);
    let half = Vec2::new(exit.w, exit.h) * 0.5;
    let marker_color = if selected {
        Color::srgb(1.0, 0.95, 0.2)
    } else {
        Color::srgb(0.95, 0.55, 1.0)
    };
    let momentum_color = Color::srgb(0.2, 1.0, 0.65);
    let (edge, inward) = match exit.side {
        ExitSide::Left => (center + Vec2::new(-half.x, 0.0), Vec2::X),
        ExitSide::Right => (center + Vec2::new(half.x, 0.0), -Vec2::X),
        ExitSide::Top => (center + Vec2::new(0.0, half.y), -Vec2::Y),
        ExitSide::Bottom => (center + Vec2::new(0.0, -half.y), Vec2::Y),
    };
    let marker_size = 6.0;

    gizmos.line_2d(edge, edge + inward * marker_size, marker_color);
    draw_point_marker(gizmos, edge + inward * marker_size, 3.0, marker_color);

    if exit.preserve_momentum {
        let marker_position = center + Vec2::new(half.x.min(10.0) - 4.0, half.y.min(10.0) - 4.0);
        gizmos.line_2d(
            marker_position + Vec2::new(-4.0, 0.0),
            marker_position + Vec2::new(0.0, 4.0),
            momentum_color,
        );
        gizmos.line_2d(
            marker_position + Vec2::new(0.0, 4.0),
            marker_position + Vec2::new(4.0, 0.0),
            momentum_color,
        );
        gizmos.line_2d(
            marker_position + Vec2::new(4.0, 0.0),
            marker_position + Vec2::new(0.0, -4.0),
            momentum_color,
        );
        gizmos.line_2d(
            marker_position + Vec2::new(0.0, -4.0),
            marker_position + Vec2::new(-4.0, 0.0),
            momentum_color,
        );
    }
}

fn draw_spring_direction_marker(
    gizmos: &mut Gizmos,
    position: Vec2,
    direction: SpringDirection,
    selected: bool,
) {
    let color = if selected {
        Color::srgb(1.0, 0.95, 0.2)
    } else {
        Color::srgb(1.0, 0.7, 0.35)
    };
    let vector = match direction {
        SpringDirection::Up => Vec2::Y,
        SpringDirection::Down => -Vec2::Y,
        SpringDirection::Left => -Vec2::X,
        SpringDirection::Right => Vec2::X,
    };
    let tip = position + vector * 8.0;
    let side = Vec2::new(-vector.y, vector.x) * 3.0;

    gizmos.line_2d(position, tip, color);
    gizmos.line_2d(tip, tip - vector * 4.0 + side, color);
    gizmos.line_2d(tip, tip - vector * 4.0 - side, color);
}

fn draw_point_marker(gizmos: &mut Gizmos, position: Vec2, radius: f32, color: Color) {
    gizmos.line_2d(
        position + Vec2::new(-radius, 0.0),
        position + Vec2::new(radius, 0.0),
        color,
    );
    gizmos.line_2d(
        position + Vec2::new(0.0, -radius),
        position + Vec2::new(0.0, radius),
        color,
    );
    draw_rect_outline(gizmos, position, Vec2::splat(radius * 1.5), color);
}

pub fn editor_inactive(editor: Option<Res<EditorState>>) -> bool {
    editor.map(|state| !state.enabled).unwrap_or(true)
}

pub fn editor_active(editor: Option<Res<EditorState>>) -> bool {
    editor.map(|state| state.enabled).unwrap_or(false)
}

fn validate_map_for_save(map: &crate::level::MapFile) -> Result<(), String> {
    if map.id.trim().is_empty() {
        return Err("cannot save map with empty id".to_string());
    }
    if map.rooms.is_empty() {
        return Err("cannot save map without rooms".to_string());
    }
    if map.room(&map.start_room).is_none() {
        return Err(format!(
            "cannot save map because start_room '{}' does not exist",
            map.start_room
        ));
    }

    for room in &map.rooms {
        if room.id.trim().is_empty() {
            return Err("cannot save map with empty room id".to_string());
        }
        if room.bounds.w <= 0.0 || room.bounds.h <= 0.0 {
            return Err(format!("room '{}' has invalid bounds", room.id));
        }
        if room
            .spawn_points
            .iter()
            .all(|spawn| spawn.id != room.default_spawn)
        {
            return Err(format!(
                "room '{}' default_spawn '{}' does not match any spawn point; create that spawn point or choose an existing spawn before saving",
                room.id, room.default_spawn
            ));
        }
        for collision in &room.collision {
            if collision.w <= 0.0 || collision.h <= 0.0 {
                return Err(format!(
                    "room '{}' has invalid collision rectangle",
                    room.id
                ));
            }
            if let Some(art_tag) = collision.art_tag.as_deref() {
                if !is_valid_tileset_art_tag(art_tag) {
                    return Err(format!(
                        "room '{}' has collision with invalid art_tag '{}'; expected a known tileset or supported legacy alias for one of: {}",
                        room.id,
                        art_tag,
                        TILESET_ART_TAGS.join(", ")
                    ));
                }
            }
        }
        for hazard in &room.hazards {
            if hazard.w <= 0.0 || hazard.h <= 0.0 {
                return Err(format!("room '{}' has invalid hazard rectangle", room.id));
            }
        }
        for completion_zone in &room.completion_zones {
            if completion_zone.w <= 0.0 || completion_zone.h <= 0.0 {
                return Err(format!(
                    "room '{}' has invalid completion zone rectangle",
                    room.id
                ));
            }
        }
        for exit in &room.exits {
            if exit.w <= 0.0 || exit.h <= 0.0 {
                return Err(format!("room '{}' has invalid exit rectangle", room.id));
            }
            let Some(target_room) = map.room(&exit.target_room) else {
                return Err(format!(
                    "room '{}' exit '{}' targets missing room '{}'",
                    room.id, exit.id, exit.target_room
                ));
            };
            if target_room
                .spawn_points
                .iter()
                .all(|spawn| spawn.id != exit.target_spawn)
            {
                return Err(format!(
                    "room '{}' exit '{}' targets missing spawn '{}' in room '{}'; create that spawn point or retarget/delete the exit before saving",
                    room.id, exit.id, exit.target_spawn, exit.target_room
                ));
            }
        }
    }

    Ok(())
}
