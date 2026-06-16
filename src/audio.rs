use bevy::prelude::*;

use crate::app_state::GameState;

const MAIN_MENU_BGM_PATH: &str = "audio/bgm/Postcard from Celeste Mountain - Lena Raine.flac";
const GAMEPLAY_BGM_PATH: &str = "audio/music/gameplayBGM.mp3";
const DASH_SFX_PATH: &str = "audio/music/dash.mp3";
const DEATH_SFX_PATH: &str = "audio/music/death.mp3";

#[derive(Component)]
struct MainMenuBgm;

#[derive(Component)]
struct GameplayBgm;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), play_main_menu_bgm)
            .add_systems(OnExit(GameState::MainMenu), stop_main_menu_bgm)
            .add_systems(OnEnter(GameState::InGame), play_gameplay_bgm)
            .add_systems(OnExit(GameState::InGame), stop_gameplay_bgm);
    }
}

fn play_main_menu_bgm(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing_bgm: Query<Entity, With<MainMenuBgm>>,
) {
    if !existing_bgm.is_empty() {
        return;
    }

    commands.spawn((
        MainMenuBgm,
        AudioPlayer::new(asset_server.load(MAIN_MENU_BGM_PATH)),
        PlaybackSettings::LOOP,
    ));
}

fn stop_main_menu_bgm(mut commands: Commands, bgm_entities: Query<Entity, With<MainMenuBgm>>) {
    for entity in &bgm_entities {
        commands.entity(entity).despawn_recursive();
    }
}

fn play_gameplay_bgm(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing_bgm: Query<Entity, With<GameplayBgm>>,
) {
    if !existing_bgm.is_empty() {
        return;
    }

    commands.spawn((
        GameplayBgm,
        AudioPlayer::new(asset_server.load(GAMEPLAY_BGM_PATH)),
        PlaybackSettings::LOOP,
    ));
}

fn stop_gameplay_bgm(mut commands: Commands, bgm_entities: Query<Entity, With<GameplayBgm>>) {
    for entity in &bgm_entities {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn play_dash_sfx(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        AudioPlayer::new(asset_server.load(DASH_SFX_PATH)),
        PlaybackSettings::DESPAWN,
    ));
}

pub fn play_death_sfx(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        AudioPlayer::new(asset_server.load(DEATH_SFX_PATH)),
        PlaybackSettings::DESPAWN,
    ));
}
