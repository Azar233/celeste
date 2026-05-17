use std::fs;
use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;

pub const DEFAULT_MAP_PATH: &str = "assets/maps/chapter_01.json";

#[allow(dead_code)]
#[derive(Resource, Clone, Debug)]
pub struct LoadedMap {
    pub data: MapFile,
}

#[allow(dead_code)]
#[derive(Resource, Clone, Debug)]
pub struct ActiveRoom {
    pub map_id: String,
    pub room_id: String,
    pub respawn_point: Vec2,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MapFile {
    pub id: String,
    pub start_room: String,
    pub rooms: Vec<RoomData>,
}

#[derive(Debug, Clone, Deserialize)]
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
    pub exits: Vec<RoomExitData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NamedPoint {
    pub id: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RectData {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollisionKind {
    SolidGround,
    WallSurface,
    OneWayPlatform,
    CameraZone,
    EffectZone,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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
