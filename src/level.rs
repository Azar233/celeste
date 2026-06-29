use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const DEFAULT_MAP_PATH: &str = "assets/maps/chapter_01.json";
pub const DEFAULT_TILESET_ART_TAG: &str = "dirt";
pub const TILESET_ART_TAGS: [&str; 6] = ["dirt", "grass", "snow", "templeA", "templeB", "girder"];

pub fn normalize_tileset_art_tag(art_tag: &str) -> Option<&'static str> {
    let tag = art_tag.trim();

    if TILESET_ART_TAGS.contains(&tag) {
        return TILESET_ART_TAGS.iter().copied().find(|valid_tag| *valid_tag == tag);
    }

    let lowercase_tag = tag.to_ascii_lowercase();
    if lowercase_tag.starts_with("snow") {
        Some("snow")
    } else if lowercase_tag.starts_with("temple_a") || lowercase_tag.starts_with("templea") {
        Some("templeA")
    } else if lowercase_tag.starts_with("temple_b") || lowercase_tag.starts_with("templeb") {
        Some("templeB")
    } else if lowercase_tag.starts_with("grass") {
        Some("grass")
    } else if lowercase_tag.starts_with("dirt") {
        Some("dirt")
    } else if lowercase_tag.starts_with("girder") {
        Some("girder")
    } else {
        None
    }
}

#[allow(dead_code)]
#[derive(Resource, Clone, Debug)]
pub struct LoadedMap {
    pub data: MapFile,
    pub path: PathBuf,
}

#[allow(dead_code)]
#[derive(Resource, Clone, Debug)]
pub struct ActiveRoom {
    pub map_id: String,
    pub room_id: String,
    pub respawn_point: Vec2,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MapFile {
    pub id: String,
    pub start_room: String,
    pub rooms: Vec<RoomData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoomData {
    pub id: String,
    pub bounds: RectData,
    pub default_spawn: String,
    #[serde(default)]
    pub spawn_points: Vec<NamedPoint>,
    #[serde(default)]
    pub collision: Vec<CollisionRect>,
    #[serde(default)]
    pub hazards: Vec<RectData>,
    #[serde(default)]
    pub checkpoints: Vec<NamedPoint>,
    #[serde(default)]
    pub dashcrystals: Vec<NamedPoint>,
    #[serde(default)]
    pub exits: Vec<RoomExitData>,
    #[serde(default)]
    pub completion_zones: Vec<RectData>,
    #[serde(default)]
    pub grasses: Vec<RectData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NamedPoint {
    pub id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RectData {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CollisionRect {
    pub kind: CollisionKind,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[allow(dead_code)]
    #[serde(default)]
    pub art_tag: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollisionKind {
    SolidGround,
    WallSurface,
    OneWayPlatform,
    CameraZone,
    EffectZone,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoomExitData {
    pub id: String,
    pub side: ExitSide,
    pub target_room: String,
    pub target_spawn: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub preserve_momentum: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExitSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl MapFile {
    pub fn starting_room(&self) -> Option<&RoomData> {
        self.room(&self.start_room)
    }

    pub fn room(&self, room_id: &str) -> Option<&RoomData> {
        self.rooms.iter().find(|room| room.id == room_id)
    }
}

impl RoomData {
    pub fn spawn_point(&self, spawn_id: &str) -> Option<Vec2> {
        self.spawn_points
            .iter()
            .find(|spawn| spawn.id == spawn_id)
            .map(|spawn| Vec2::new(spawn.x, spawn.y))
    }

    pub fn default_spawn_point(&self) -> Option<Vec2> {
        self.spawn_point(&self.default_spawn)
    }
}

impl RectData {
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.w, self.h)
    }
}

pub fn load_map_from_path(path: impl AsRef<Path>) -> Result<MapFile, String> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read map '{}': {error}", path.display()))?;

    serde_json::from_str::<MapFile>(&content)
        .map_err(|error| format!("failed to parse map '{}': {error}", path.display()))
}

pub fn save_map_to_path(path: impl AsRef<Path>, map: &MapFile) -> Result<(), String> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create map save directory '{}': {error}",
                parent.display()
            )
        })?;
    }

    let content = serde_json::to_string_pretty(map)
        .map_err(|error| format!("failed to serialize map '{}': {error}", map.id))?;

    fs::write(path, format!("{content}\n"))
        .map_err(|error| format!("failed to write map '{}': {error}", path.display()))
}

pub fn backup_path_for_map(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let mut backup_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_string();

    if backup_extension.is_empty() {
        backup_extension.push_str("bak");
    } else {
        backup_extension.push_str(".bak");
    }

    path.with_extension(backup_extension)
}

pub fn save_map_to_path_with_backup(path: impl AsRef<Path>, map: &MapFile) -> Result<PathBuf, String> {
    let path = path.as_ref();
    let backup_path = backup_path_for_map(path);

    if path.exists() {
        fs::copy(path, &backup_path).map_err(|error| {
            format!(
                "failed to create map backup '{}' from '{}': {error}",
                backup_path.display(),
                path.display()
            )
        })?;
    }

    save_map_to_path(path, map)?;
    Ok(backup_path)
}
