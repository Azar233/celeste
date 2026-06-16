use std::path::PathBuf;

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    MainMenu,
    InGame,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct PendingMapPath {
    pub path: Option<PathBuf>,
}
