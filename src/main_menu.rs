use std::fs;
use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::app_state::{GameState, PendingMapPath};
use crate::level::DEFAULT_MAP_PATH;

#[derive(Component)]
struct MainMenuEntity;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct MainMenuAction(MenuAction);

#[derive(Component)]
struct ChapterButton {
    path: PathBuf,
}

#[derive(Clone, Copy)]
enum MenuAction {
    StartGame,
    SelectChapter,
    ReturnToRoot,
    ExitGame,
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum MainMenuPage {
    #[default]
    Root,
    ChapterSelect,
}

struct ChapterEntry {
    label: String,
    path: PathBuf,
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MainMenuPage>()
            .add_systems(OnEnter(GameState::MainMenu), spawn_main_menu)
            .add_systems(
                Update,
                handle_main_menu_buttons.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu);
    }
}

fn spawn_main_menu(mut commands: Commands, mut page: ResMut<MainMenuPage>) {
    *page = MainMenuPage::Root;

    let camera = commands
        .spawn((
            MainMenuEntity,
            Camera2d,
            Camera {
                order: 100,
                ..default()
            },
            IsDefaultUiCamera,
        ))
        .id();

    commands
        .spawn((
            MainMenuEntity,
            MainMenuRoot,
            TargetCamera(camera),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.06, 0.07, 0.12)),
        ))
        .with_children(spawn_root_page);
}

fn spawn_root_page(parent: &mut ChildBuilder) {
    spawn_title(parent, "Celeste Rust");
    spawn_action_button(parent, "Start Game", MenuAction::StartGame);
    spawn_action_button(parent, "Select Chapter", MenuAction::SelectChapter);
    spawn_action_button(parent, "Quit", MenuAction::ExitGame);
}

fn spawn_chapter_select_page(parent: &mut ChildBuilder, chapters: &[ChapterEntry]) {
    spawn_title(parent, "Select Chapter");

    if chapters.is_empty() {
        parent.spawn((
            MainMenuEntity,
            Text::new("No maps found in assets/maps"),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.72, 0.8)),
            Node {
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            },
        ));
    } else {
        for chapter in chapters {
            spawn_chapter_button(parent, chapter);
        }
    }

    spawn_action_button(parent, "Return", MenuAction::ReturnToRoot);
}

fn spawn_title(parent: &mut ChildBuilder, label: &str) {
    parent.spawn((
        MainMenuEntity,
        Text::new(label.to_string()),
        TextFont {
            font_size: 56.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 1.0)),
        Node {
            margin: UiRect::bottom(Val::Px(28.0)),
            ..default()
        },
    ));
}

fn spawn_action_button(parent: &mut ChildBuilder, label: &str, action: MenuAction) {
    parent
        .spawn((
            MainMenuEntity,
            Button,
            MainMenuAction(action),
            Node {
                width: Val::Px(240.0),
                height: Val::Px(52.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.20, 0.34)),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_child((
            MainMenuEntity,
            Text::new(label.to_string()),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn spawn_chapter_button(parent: &mut ChildBuilder, chapter: &ChapterEntry) {
    parent
        .spawn((
            MainMenuEntity,
            Button,
            ChapterButton {
                path: chapter.path.clone(),
            },
            Node {
                width: Val::Px(220.0),
                height: Val::Px(38.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.16, 0.28)),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_child((
            MainMenuEntity,
            Text::new(chapter.label.clone()),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.88, 0.95)),
        ));
}

fn handle_main_menu_buttons(
    mut commands: Commands,
    root_query: Query<(Entity, Option<&Children>), With<MainMenuRoot>>,
    buttons: Query<
        (Entity, &Interaction, Option<&MainMenuAction>, Option<&ChapterButton>),
        (Changed<Interaction>, With<Button>),
    >,
    mut bg_query: Query<&mut BackgroundColor>,
    mut pending_map_path: ResMut<PendingMapPath>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit: EventWriter<AppExit>,
    mut page: ResMut<MainMenuPage>,
) {
    for (entity, interaction, action, chapter_button) in &buttons {
        match *interaction {
            Interaction::Hovered => {
                if let Ok(mut bg) = bg_query.get_mut(entity) {
                    *bg = BackgroundColor(Color::srgb(0.30, 0.34, 0.52));
                }
            }
            Interaction::None => {
                if let Ok(mut bg) = bg_query.get_mut(entity) {
                    *bg = if action.is_some() {
                        BackgroundColor(Color::srgb(0.18, 0.20, 0.34))
                    } else {
                        BackgroundColor(Color::srgb(0.14, 0.16, 0.28))
                    };
                }
            }
            Interaction::Pressed => {
                if let Ok(mut bg) = bg_query.get_mut(entity) {
                    *bg = BackgroundColor(Color::srgb(0.10, 0.12, 0.22));
                }

                if let Some(action) = action {
                    match action.0 {
                        MenuAction::StartGame => {
                            pending_map_path.path = Some(PathBuf::from(DEFAULT_MAP_PATH));
                            next_state.set(GameState::InGame);
                        }
                        MenuAction::SelectChapter => {
                            *page = MainMenuPage::ChapterSelect;
                            rebuild_main_menu_page(&mut commands, &root_query, MainMenuPage::ChapterSelect);
                        }
                        MenuAction::ReturnToRoot => {
                            *page = MainMenuPage::Root;
                            rebuild_main_menu_page(&mut commands, &root_query, MainMenuPage::Root);
                        }
                        MenuAction::ExitGame => {
                            app_exit.send(AppExit::Success);
                        }
                    }
                }

                if let Some(chapter_button) = chapter_button {
                    pending_map_path.path = Some(chapter_button.path.clone());
                    next_state.set(GameState::InGame);
                }
            }
        }
    }
}

fn rebuild_main_menu_page(
    commands: &mut Commands,
    root_query: &Query<(Entity, Option<&Children>), With<MainMenuRoot>>,
    page: MainMenuPage,
) {
    let Ok((root, children)) = root_query.get_single() else {
        return;
    };

    if let Some(children) = children {
        for child in children.iter() {
            commands.entity(*child).despawn_recursive();
        }
    }

    commands.entity(root).with_children(|parent| match page {
        MainMenuPage::Root => spawn_root_page(parent),
        MainMenuPage::ChapterSelect => {
            let chapters = scan_chapters();
            spawn_chapter_select_page(parent, &chapters);
        }
    });
}

fn cleanup_main_menu(
    mut commands: Commands,
    root_query: Query<Entity, With<MainMenuRoot>>,
    camera_query: Query<Entity, (With<MainMenuEntity>, With<Camera2d>)>,
    mut page: ResMut<MainMenuPage>,
) {
    for root in &root_query {
        commands.entity(root).despawn_recursive();
    }
    for camera in &camera_query {
        commands.entity(camera).despawn();
    }
    *page = MainMenuPage::Root;
}

fn scan_chapters() -> Vec<ChapterEntry> {
    let mut chapters = Vec::new();

    if let Ok(entries) = fs::read_dir("assets/maps") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "json") {
                let label = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                chapters.push(ChapterEntry { label, path });
            }
        }
    }

    chapters.sort_by(|a, b| a.label.cmp(&b.label));
    chapters
}
