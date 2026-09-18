//! 最小限のゲーム開始フロー:
//!
//! 世界作成画面 → キャラクリ画面 → 世界プロシージャル生成待機画面 → ゲーム画面
//!
//! 各画面は名前入力＋作成ボタンのみを持つ最小実装。「ゲーム画面」自体は
//! `crate::ui::setup_playing_screen`（`AppScreen::Playing`突入時）が担う。

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::party::PartyStateRes;
use crate::town::TownStateRes;
use crate::ui::{palette, CELL_PX, FONT_PATH};

const MAX_NAME_LEN: usize = 20;
const DEFAULT_WORLD_NAME: &str = "名もなき世界";
const DEFAULT_CHARACTER_NAME: &str = "あなた";
const GENERATING_MIN_SECONDS: f32 = 0.8;

#[derive(States, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub enum AppScreen {
    #[default]
    WorldCreation,
    CharacterCreation,
    Generating,
    Playing,
}

/// 世界作成・キャラクリで入力された名前。ゲーム画面側（`ui::setup_playing_screen`）
/// でも読み取るため`pub`。
#[derive(Resource, Default)]
pub struct NewGameConfig {
    pub world_name: String,
    pub character_name: String,
}

#[derive(Resource, Default)]
struct NameEntryState {
    buffer: String,
}

#[derive(Resource)]
struct GeneratingTimer(Timer);

#[derive(Component)]
struct FlowScreenRoot;

#[derive(Component)]
struct GeneratingRoot;

#[derive(Component)]
struct NameValueText;

#[derive(Component)]
struct CreateButton;

/// 関数プラグイン。世界作成〜世界生成待機までの3画面分の状態・システムを登録する。
/// 「ゲーム画面」（`AppScreen::Playing`突入時の処理）は既存の`ui`モジュールが持つため
/// ここでは登録しない（呼び出し側の`main.rs`で配線する）。
pub fn screens_plugin(app: &mut App) {
    app.init_state::<AppScreen>()
        .init_resource::<NewGameConfig>()
        .init_resource::<NameEntryState>()
        .add_systems(OnEnter(AppScreen::WorldCreation), setup_world_creation)
        .add_systems(OnExit(AppScreen::WorldCreation), teardown_name_entry_screen)
        .add_systems(OnEnter(AppScreen::CharacterCreation), setup_character_creation)
        .add_systems(OnExit(AppScreen::CharacterCreation), teardown_name_entry_screen)
        .add_systems(
            Update,
            (
                name_entry_keyboard_system,
                name_entry_confirm_system,
                update_name_display_system,
                button_visual_feedback_system,
            )
                .chain()
                .run_if(in_name_entry_screen),
        )
        .add_systems(OnEnter(AppScreen::Generating), setup_generating_screen)
        .add_systems(OnExit(AppScreen::Generating), teardown_generating_screen)
        .add_systems(
            Update,
            tick_generating_system.run_if(in_state(AppScreen::Generating)),
        );
}

fn in_name_entry_screen(state: Res<State<AppScreen>>) -> bool {
    matches!(
        state.get(),
        AppScreen::WorldCreation | AppScreen::CharacterCreation
    )
}

fn setup_world_creation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut name_state: ResMut<NameEntryState>,
) {
    name_state.buffer.clear();
    spawn_name_entry_screen(
        &mut commands,
        asset_server.load(FONT_PATH),
        "世界を作成する",
        "新しく歩む世界の名前を入力してください。",
    );
}

fn setup_character_creation(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut name_state: ResMut<NameEntryState>,
) {
    name_state.buffer.clear();
    spawn_name_entry_screen(
        &mut commands,
        asset_server.load(FONT_PATH),
        "キャラクターを作成する",
        "あなたの名前を入力してください。",
    );
}

fn spawn_name_entry_screen(commands: &mut Commands, font: Handle<Font>, heading: &str, prompt: &str) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            FlowScreenRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(14.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor(palette::FRAME),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(heading),
                    TextFont {
                        font: font.clone(),
                        font_size: CELL_PX * 1.3,
                        ..default()
                    },
                    TextColor(palette::TEXT_HIGHLIGHT),
                ));
                panel.spawn((
                    Text::new(prompt),
                    TextFont {
                        font: font.clone(),
                        font_size: CELL_PX * 0.85,
                        ..default()
                    },
                    TextColor(palette::TEXT),
                ));
                panel
                    .spawn((
                        Node {
                            min_width: Val::Px(340.0),
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor(palette::TEXT),
                    ))
                    .with_children(|input_box| {
                        input_box.spawn((
                            Text::new("> _"),
                            TextFont {
                                font: font.clone(),
                                font_size: CELL_PX,
                                ..default()
                            },
                            TextColor(palette::TEXT_HIGHLIGHT),
                            NameValueText,
                        ));
                    });
                panel.spawn((
                    Text::new("[Enter] または下のボタンで決定（未入力なら仮の名前になります）"),
                    TextFont {
                        font: font.clone(),
                        font_size: CELL_PX * 0.65,
                        ..default()
                    },
                    TextColor(palette::TEXT),
                ));
                panel
                    .spawn((
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(18.0), Val::Px(8.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BorderColor(palette::FRAME),
                        BackgroundColor(Color::NONE),
                        CreateButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("▶ 作成する"),
                            TextFont {
                                font,
                                font_size: CELL_PX,
                                ..default()
                            },
                            TextColor(palette::TEXT_HIGHLIGHT),
                        ));
                    });
            });
        });
}

fn teardown_name_entry_screen(mut commands: Commands, query: Query<Entity, With<FlowScreenRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn name_entry_keyboard_system(
    mut events: EventReader<KeyboardInput>,
    mut name_state: ResMut<NameEntryState>,
) {
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match &ev.logical_key {
            Key::Character(s) => {
                if name_state.buffer.chars().count() < MAX_NAME_LEN {
                    name_state.buffer.push_str(s);
                }
            }
            Key::Space => {
                if name_state.buffer.chars().count() < MAX_NAME_LEN {
                    name_state.buffer.push(' ');
                }
            }
            Key::Backspace => {
                name_state.buffer.pop();
            }
            _ => {}
        }
    }
}

fn update_name_display_system(
    name_state: Res<NameEntryState>,
    mut query: Query<&mut Text, With<NameValueText>>,
) {
    if let Ok(mut text) = query.get_single_mut() {
        *text = Text::new(format!("> {}_", name_state.buffer));
    }
}

fn button_visual_feedback_system(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<CreateButton>)>,
) {
    for (interaction, mut bg) in &mut query {
        *bg = match interaction {
            Interaction::Pressed => BackgroundColor(Color::srgba(0.15, 0.85, 0.35, 0.35)),
            Interaction::Hovered => BackgroundColor(Color::srgba(0.15, 0.85, 0.35, 0.15)),
            Interaction::None => BackgroundColor(Color::NONE),
        };
    }
}

fn name_entry_confirm_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<CreateButton>)>,
    screen: Res<State<AppScreen>>,
    mut next_screen: ResMut<NextState<AppScreen>>,
    mut config: ResMut<NewGameConfig>,
    name_state: Res<NameEntryState>,
) {
    let button_pressed = button_query.iter().any(|i| *i == Interaction::Pressed);
    let enter_pressed =
        keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::NumpadEnter);
    if !button_pressed && !enter_pressed {
        return;
    }

    let name = name_state.buffer.trim();
    match screen.get() {
        AppScreen::WorldCreation => {
            config.world_name = if name.is_empty() {
                DEFAULT_WORLD_NAME.to_string()
            } else {
                name.to_string()
            };
            next_screen.set(AppScreen::CharacterCreation);
        }
        AppScreen::CharacterCreation => {
            config.character_name = if name.is_empty() {
                DEFAULT_CHARACTER_NAME.to_string()
            } else {
                name.to_string()
            };
            next_screen.set(AppScreen::Generating);
        }
        AppScreen::Generating | AppScreen::Playing => {}
    }
}

fn setup_generating_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut party: ResMut<PartyStateRes>,
    config: Res<NewGameConfig>,
) {
    // ADR-0022以前の最小実装: 街1つ（アルカン王都）のみを「世界」として生成する。
    if let Some(player) = party.members.iter_mut().find(|m| m.is_player) {
        player.name = config.character_name.clone();
    }
    let town = TownStateRes::new(&mut images, &party.members);
    commands.insert_resource(town);
    commands.insert_resource(GeneratingTimer(Timer::from_seconds(
        GENERATING_MIN_SECONDS,
        TimerMode::Once,
    )));

    let font = asset_server.load(FONT_PATH);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            GeneratingRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!("『{}』を生成しています…", config.world_name)),
                TextFont {
                    font: font.clone(),
                    font_size: CELL_PX * 1.1,
                    ..default()
                },
                TextColor(palette::TEXT_HIGHLIGHT),
            ));
            root.spawn((
                Text::new("街を組み立てています。少々お待ちください。"),
                TextFont {
                    font,
                    font_size: CELL_PX * 0.8,
                    ..default()
                },
                TextColor(palette::TEXT),
            ));
        });
}

fn tick_generating_system(
    time: Res<Time>,
    mut timer: ResMut<GeneratingTimer>,
    mut next_screen: ResMut<NextState<AppScreen>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next_screen.set(AppScreen::Playing);
    }
}

fn teardown_generating_screen(mut commands: Commands, query: Query<Entity, With<GeneratingRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
