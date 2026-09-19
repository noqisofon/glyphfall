//! 最小限のゲーム開始フロー:
//!
//! タイトル画面 → 世界作成画面 → キャラクリ画面 → 世界プロシージャル生成待機画面 → ゲーム画面
//!
//! 「ゲーム画面」自体は `crate::ui::setup_playing_screen`（`AppScreen::Playing`突入時）が担う。

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;

use crate::party::{
    CharacterBuild, EloperPartnerCandidate, MotiveKind, OriginKind, PartyStateRes,
    PlayerInventoryRes, PlayerResourceRes, ReserveRosterRes,
};
use crate::town::TownStateRes;
use crate::ui::{palette, CELL_PX, FONT_PATH};

const MAX_NAME_LEN: usize = 20;
const DEFAULT_WORLD_NAME: &str = "名もなき世界";
const DEFAULT_CHARACTER_NAME: &str = "あなた";
const GENERATING_MIN_SECONDS: f32 = 0.8;

#[derive(States, Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
pub enum AppScreen {
    #[default]
    Title,
    WorldCreation,
    CharacterCreation,
    /// ADR-0022/0023: 出自（属性軸）選択
    OriginSelection,
    /// ADR-0022/0023: 動機（動機軸）選択
    MotiveSelection,
    /// ADR-0022: 駆け落ち相手選択
    PartnerSelection,
    Generating,
    Playing,
}

/// 世界作成・キャラクリで決定された設定。ゲーム画面側（`ui::setup_playing_screen`）
/// でも読み取るため`pub`。
#[derive(Resource, Default)]
pub struct NewGameConfig {
    pub world_name: String,
    pub character_name: String,
    pub build: CharacterBuild,
}

#[derive(Resource, Default)]
struct NameEntryState {
    buffer: String,
}

#[derive(Resource, Default)]
pub struct SelectionState {
    pub cursor: usize,
}

#[derive(Resource)]
struct GeneratingTimer(Timer);

#[derive(Component)]
struct TitleScreenRoot;

#[derive(Component)]
struct TitleNewGameButton;

#[derive(Component)]
struct FlowScreenRoot;

#[derive(Component)]
struct GeneratingRoot;

#[derive(Component)]
struct NameValueText;

#[derive(Component)]
struct CreateButton;

#[derive(Component)]
struct SelectionScreenRoot;

#[derive(Component)]
struct SelectionItemButton(usize);

#[derive(Component)]
struct SelectionItemText(usize);

#[derive(Component)]
struct SelectionDetailTitle;

#[derive(Component)]
struct SelectionDetailDesc;

#[derive(Component)]
struct SelectionDetailBonus;

#[derive(Component)]
struct SelectionConfirmButton;

/// 関数プラグイン。タイトル画面〜世界生成待機までの状態・システムを登録する。
/// 「ゲーム画面」（`AppScreen::Playing`突入時の処理）は既存の`ui`モジュールが持つため
/// ここでは登録しない（呼び出し側の`main.rs`で配線する）。
pub fn screens_plugin(app: &mut App) {
    app.init_state::<AppScreen>()
        .init_resource::<NewGameConfig>()
        .init_resource::<NameEntryState>()
        .init_resource::<SelectionState>()
        .add_systems(OnEnter(AppScreen::Title), setup_title_screen)
        .add_systems(OnExit(AppScreen::Title), teardown_title_screen)
        .add_systems(
            Update,
            (title_input_system, title_button_visual_feedback_system)
                .chain()
                .run_if(in_state(AppScreen::Title)),
        )
        .add_systems(OnEnter(AppScreen::WorldCreation), setup_world_creation)
        .add_systems(OnExit(AppScreen::WorldCreation), teardown_name_entry_screen)
        .add_systems(
            OnEnter(AppScreen::CharacterCreation),
            setup_character_creation,
        )
        .add_systems(
            OnExit(AppScreen::CharacterCreation),
            teardown_name_entry_screen,
        )
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
        .add_systems(OnEnter(AppScreen::OriginSelection), setup_origin_selection)
        .add_systems(
            OnExit(AppScreen::OriginSelection),
            teardown_selection_screen,
        )
        .add_systems(OnEnter(AppScreen::MotiveSelection), setup_motive_selection)
        .add_systems(
            OnExit(AppScreen::MotiveSelection),
            teardown_selection_screen,
        )
        .add_systems(
            OnEnter(AppScreen::PartnerSelection),
            setup_partner_selection,
        )
        .add_systems(
            OnExit(AppScreen::PartnerSelection),
            teardown_selection_screen,
        )
        .add_systems(
            Update,
            (
                selection_input_system,
                selection_button_interaction_system,
                update_selection_visual_system,
                confirm_button_visual_feedback_system,
            )
                .chain()
                .run_if(in_selection_screen),
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

fn setup_title_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(FONT_PATH);

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
            TitleScreenRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(24.0),
                    padding: UiRect::all(Val::Px(36.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor(palette::FRAME),
            ))
            .with_children(|panel| {
                // タイトルロゴ部
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn((
                            Text::new("G L Y P H F A L L"),
                            TextFont {
                                font: font.clone(),
                                font_size: CELL_PX * 2.0,
                                ..default()
                            },
                            TextColor(palette::TEXT_HIGHLIGHT),
                        ));
                        header.spawn((
                            Text::new("― 街と迷宮、言葉を紡ぐ旅 ―"),
                            TextFont {
                                font: font.clone(),
                                font_size: CELL_PX * 0.85,
                                ..default()
                            },
                            TextColor(palette::TEXT),
                        ));
                    });

                // メニュー部
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(12.0),
                        min_width: Val::Px(320.0),
                        ..default()
                    })
                    .with_children(|menu| {
                        // New Game ボタン
                        menu.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BorderColor(palette::FRAME),
                            BackgroundColor(Color::NONE),
                            TitleNewGameButton,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("▶ はじめる (New Game)"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: CELL_PX * 1.05,
                                    ..default()
                                },
                                TextColor(palette::TEXT_HIGHLIGHT),
                            ));
                        });

                        // Continue 表示（非活性）
                        menu.spawn(Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .with_children(|item| {
                            item.spawn((
                                Text::new("  つづきから (Continue - 未実装)"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: CELL_PX * 0.85,
                                    ..default()
                                },
                                TextColor(palette::FRAME),
                            ));
                        });
                    });

                // ガイド＆バージョン情報
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|footer| {
                        footer.spawn((
                            Text::new("[Enter] / [Space] キー または クリック で開始"),
                            TextFont {
                                font: font.clone(),
                                font_size: CELL_PX * 0.7,
                                ..default()
                            },
                            TextColor(palette::TEXT),
                        ));
                        footer.spawn((
                            Text::new("v0.1.0 - Prototype"),
                            TextFont {
                                font,
                                font_size: CELL_PX * 0.6,
                                ..default()
                            },
                            TextColor(palette::FRAME),
                        ));
                    });
            });
        });
}

fn teardown_title_screen(mut commands: Commands, query: Query<Entity, With<TitleScreenRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn title_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<TitleNewGameButton>)>,
    mut next_screen: ResMut<NextState<AppScreen>>,
) {
    let button_pressed = mouse_buttons.just_pressed(MouseButton::Left)
        && button_query.iter().any(|i| *i == Interaction::Pressed);
    let key_pressed = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::NumpadEnter)
        || keyboard.just_pressed(KeyCode::Space);

    if button_pressed || key_pressed {
        next_screen.set(AppScreen::WorldCreation);
    }
}

type TitleButtonInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static mut BackgroundColor),
    (Changed<Interaction>, With<TitleNewGameButton>),
>;

fn title_button_visual_feedback_system(mut query: TitleButtonInteractionQuery) {
    for (interaction, mut bg) in &mut query {
        *bg = match interaction {
            Interaction::Pressed => BackgroundColor(Color::srgba(0.15, 0.85, 0.35, 0.35)),
            Interaction::Hovered => BackgroundColor(Color::srgba(0.15, 0.85, 0.35, 0.15)),
            Interaction::None => BackgroundColor(Color::NONE),
        };
    }
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
        "新しく歩む世界の名前を入力してください（半角英数）。",
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
        "あなたの名前を入力してください（半角英数）。",
    );
}

fn spawn_name_entry_screen(
    commands: &mut Commands,
    font: Handle<Font>,
    heading: &str,
    prompt: &str,
) {
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
                let remaining = MAX_NAME_LEN.saturating_sub(name_state.buffer.chars().count());
                for c in s.chars().filter(|c| !c.is_control()).take(remaining) {
                    name_state.buffer.push(c);
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

type ButtonInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static mut BackgroundColor),
    (Changed<Interaction>, With<CreateButton>),
>;

fn button_visual_feedback_system(mut query: ButtonInteractionQuery) {
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
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    button_query: Query<&Interaction, (Changed<Interaction>, With<CreateButton>)>,
    screen: Res<State<AppScreen>>,
    mut next_screen: ResMut<NextState<AppScreen>>,
    mut config: ResMut<NewGameConfig>,
    name_state: Res<NameEntryState>,
) {
    let button_pressed = mouse_buttons.just_pressed(MouseButton::Left)
        && button_query.iter().any(|i| *i == Interaction::Pressed);
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
            next_screen.set(AppScreen::OriginSelection);
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn setup_generating_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut party: ResMut<PartyStateRes>,
    mut player_resource: ResMut<PlayerResourceRes>,
    mut inventory: ResMut<PlayerInventoryRes>,
    mut reserve_roster: ResMut<ReserveRosterRes>,
    config: Res<NewGameConfig>,
) {
    // ADR-0022/ADR-0023: キャラクリ（出自×動機）による各種リソースの初期化
    let party_members = config.build.create_party_members(&config.character_name);
    party.members = party_members;
    party.selected_index = 0;

    player_resource.0.skills = config.build.calculate_player_skills();
    inventory.0 = config.build.create_inventory();
    reserve_roster.0 = config.build.create_reserve_roster();

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

fn in_selection_screen(state: Res<State<AppScreen>>) -> bool {
    matches!(
        state.get(),
        AppScreen::OriginSelection | AppScreen::MotiveSelection | AppScreen::PartnerSelection
    )
}

struct SelectionDisplayData {
    items: Vec<String>,
    detail_title: String,
    detail_desc: String,
    detail_bonus: String,
}

fn get_selection_display_data(screen: AppScreen, cursor: usize) -> SelectionDisplayData {
    match screen {
        AppScreen::OriginSelection => {
            let origins = OriginKind::ALL;
            let idx = cursor.min(origins.len().saturating_sub(1));
            let current = origins[idx];
            SelectionDisplayData {
                items: origins
                    .iter()
                    .map(|o| format!("{} [{}]", o.title(), o.category()))
                    .collect(),
                detail_title: format!("【 {} 】- {}", current.title(), current.category()),
                detail_desc: current.description().to_string(),
                detail_bonus: current.bonus_summary().to_string(),
            }
        }
        AppScreen::MotiveSelection => {
            let motives = MotiveKind::ALL;
            let idx = cursor.min(motives.len().saturating_sub(1));
            let current = motives[idx];
            SelectionDisplayData {
                items: motives.iter().map(|m| m.title().to_string()).collect(),
                detail_title: format!("【 {} 】", current.title()),
                detail_desc: current.description().to_string(),
                detail_bonus: current.bonus_summary().to_string(),
            }
        }
        AppScreen::PartnerSelection => {
            let candidates = EloperPartnerCandidate::default_candidates();
            let idx = cursor.min(candidates.len().saturating_sub(1));
            let current = &candidates[idx];
            SelectionDisplayData {
                items: candidates
                    .iter()
                    .map(|c| format!("{}（{}）", c.name, c.job))
                    .collect(),
                detail_title: format!(
                    "【 {} 】 職業: {} / 性格: {}",
                    current.name,
                    current.job,
                    current.personality.label()
                ),
                detail_desc: current.backstory.clone(),
                detail_bonus: format!(
                    "HP: {} / MP: {} / 初期影響度: 88（自然上限寸前）",
                    current.hp, current.mp
                ),
            }
        }
        _ => SelectionDisplayData {
            items: Vec::new(),
            detail_title: String::new(),
            detail_desc: String::new(),
            detail_bonus: String::new(),
        },
    }
}

fn get_item_count(screen: AppScreen) -> usize {
    match screen {
        AppScreen::OriginSelection => OriginKind::ALL.len(),
        AppScreen::MotiveSelection => MotiveKind::ALL.len(),
        AppScreen::PartnerSelection => EloperPartnerCandidate::default_candidates().len(),
        _ => 0,
    }
}

fn advance_selection(
    screen: AppScreen,
    cursor: usize,
    config: &mut NewGameConfig,
    next_screen: &mut NextState<AppScreen>,
) {
    match screen {
        AppScreen::OriginSelection => {
            let origins = OriginKind::ALL;
            let idx = cursor.min(origins.len().saturating_sub(1));
            config.build.origin = origins[idx];
            next_screen.set(AppScreen::MotiveSelection);
        }
        AppScreen::MotiveSelection => {
            let motives = MotiveKind::ALL;
            let idx = cursor.min(motives.len().saturating_sub(1));
            let motive = motives[idx];
            config.build.motive = motive;
            if motive == MotiveKind::Eloper {
                next_screen.set(AppScreen::PartnerSelection);
            } else {
                config.build.partner = None;
                next_screen.set(AppScreen::Generating);
            }
        }
        AppScreen::PartnerSelection => {
            let mut candidates = EloperPartnerCandidate::default_candidates();
            let idx = cursor.min(candidates.len().saturating_sub(1));
            config.build.partner = Some(candidates.remove(idx));
            next_screen.set(AppScreen::Generating);
        }
        _ => {}
    }
}

fn setup_origin_selection(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut selection_state: ResMut<SelectionState>,
) {
    selection_state.cursor = 0;
    spawn_selection_screen_layout(
        &mut commands,
        asset_server.load(FONT_PATH),
        AppScreen::OriginSelection,
        "▼ キャラクター作成（1/2）: 出自（属性）の選択",
        "あなたがどのような社会的背景を持ち、何を身につけてきたかを決定します。",
    );
}

fn setup_motive_selection(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut selection_state: ResMut<SelectionState>,
) {
    selection_state.cursor = 0;
    spawn_selection_screen_layout(
        &mut commands,
        asset_server.load(FONT_PATH),
        AppScreen::MotiveSelection,
        "▼ キャラクター作成（2/2）: 旅立ちの動機の選択",
        "なぜ今この瞬間に旅に出たのか、その起点となる目的・状況を決定します。",
    );
}

fn setup_partner_selection(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut selection_state: ResMut<SelectionState>,
) {
    selection_state.cursor = 0;
    spawn_selection_screen_layout(
        &mut commands,
        asset_server.load(FONT_PATH),
        AppScreen::PartnerSelection,
        "▼ 駆け落ち相手の選択（相棒プール）",
        "共に手を取り、家を捨てて旅立った大切なパートナーを決定します。",
    );
}

fn spawn_selection_screen_layout(
    commands: &mut Commands,
    font: Handle<Font>,
    screen: AppScreen,
    heading: &str,
    subheading: &str,
) {
    let data = get_selection_display_data(screen, 0);

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
            SelectionScreenRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    min_width: Val::Px(780.0),
                    min_height: Val::Px(450.0),
                    ..default()
                },
                BorderColor(palette::FRAME),
                BackgroundColor(palette::BG),
            ))
            .with_children(|panel| {
                // ヘッダー部
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn((
                            Text::new(heading),
                            TextFont {
                                font: font.clone(),
                                font_size: CELL_PX * 1.25,
                                ..default()
                            },
                            TextColor(palette::TEXT_HIGHLIGHT),
                        ));
                        header.spawn((
                            Text::new(subheading),
                            TextFont {
                                font: font.clone(),
                                font_size: CELL_PX * 0.8,
                                ..default()
                            },
                            TextColor(palette::TEXT),
                        ));
                    });

                // メインボディ部（横2カラム）
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(18.0),
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|body| {
                        // 左カラム（選択肢一覧）
                        body.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            width: Val::Px(310.0),
                            ..default()
                        })
                        .with_children(|list| {
                            for (i, label) in data.items.iter().enumerate() {
                                let prefix = if i == 0 { "▶ " } else { "   " };
                                let is_active = i == 0;
                                list.spawn((
                                    Button,
                                    Node {
                                        width: Val::Percent(100.0),
                                        padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                                        border: UiRect::all(Val::Px(1.0)),
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BorderColor(if is_active {
                                        palette::TEXT_HIGHLIGHT
                                    } else {
                                        palette::FRAME
                                    }),
                                    BackgroundColor(if is_active {
                                        Color::srgba(0.15, 0.85, 0.35, 0.25)
                                    } else {
                                        Color::NONE
                                    }),
                                    SelectionItemButton(i),
                                ))
                                .with_children(|btn| {
                                    btn.spawn((
                                        Text::new(format!("{}{}. {}", prefix, i + 1, label)),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: CELL_PX * 0.9,
                                            ..default()
                                        },
                                        TextColor(if is_active {
                                            palette::TEXT_HIGHLIGHT
                                        } else {
                                            palette::TEXT
                                        }),
                                        SelectionItemText(i),
                                    ));
                                });
                            }
                        });

                        // 右カラム（詳細プレビュー枠）
                        body.spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(10.0),
                                width: Val::Px(420.0),
                                min_height: Val::Px(240.0),
                                padding: UiRect::all(Val::Px(16.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor(palette::FRAME),
                        ))
                        .with_children(|detail| {
                            detail.spawn((
                                Text::new(&data.detail_title),
                                TextFont {
                                    font: font.clone(),
                                    font_size: CELL_PX * 1.05,
                                    ..default()
                                },
                                TextColor(palette::TEXT_HIGHLIGHT),
                                SelectionDetailTitle,
                            ));
                            detail.spawn((
                                Text::new("─".repeat(34)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: CELL_PX * 0.65,
                                    ..default()
                                },
                                TextColor(palette::FRAME),
                            ));
                            detail.spawn((
                                Text::new(&data.detail_desc),
                                TextFont {
                                    font: font.clone(),
                                    font_size: CELL_PX * 0.8,
                                    ..default()
                                },
                                TextColor(palette::TEXT),
                                SelectionDetailDesc,
                            ));
                            detail.spawn((
                                Text::new("─".repeat(34)),
                                TextFont {
                                    font: font.clone(),
                                    font_size: CELL_PX * 0.65,
                                    ..default()
                                },
                                TextColor(palette::FRAME),
                            ));
                            detail.spawn((
                                Text::new(&data.detail_bonus),
                                TextFont {
                                    font: font.clone(),
                                    font_size: CELL_PX * 0.78,
                                    ..default()
                                },
                                TextColor(palette::TEXT_HIGHLIGHT),
                                SelectionDetailBonus,
                            ));
                        });
                    });

                // フッター部
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        ..default()
                    })
                    .with_children(|footer| {
                        footer.spawn((
                            Text::new(
                                "操作: [↑/↓] または [W/S] で選択 | [1-5] 直接選択 | [Enter] で決定",
                            ),
                            TextFont {
                                font: font.clone(),
                                font_size: CELL_PX * 0.7,
                                ..default()
                            },
                            TextColor(palette::TEXT),
                        ));
                        footer
                            .spawn((
                                Button,
                                Node {
                                    padding: UiRect::axes(Val::Px(24.0), Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BorderColor(palette::FRAME),
                                BackgroundColor(Color::NONE),
                                SelectionConfirmButton,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new("▶ この選択で決定する"),
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
        });
}

fn teardown_selection_screen(
    mut commands: Commands,
    query: Query<Entity, With<SelectionScreenRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn selection_input_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    screen: Res<State<AppScreen>>,
    mut next_screen: ResMut<NextState<AppScreen>>,
    mut config: ResMut<NewGameConfig>,
    mut selection_state: ResMut<SelectionState>,
) {
    let count = get_item_count(*screen.get());
    if count == 0 {
        return;
    }

    if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
        selection_state.cursor = (selection_state.cursor + count - 1) % count;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
        selection_state.cursor = (selection_state.cursor + 1) % count;
    }

    let digit_keys = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Numpad1, 0),
        (KeyCode::Numpad2, 1),
        (KeyCode::Numpad3, 2),
        (KeyCode::Numpad4, 3),
        (KeyCode::Numpad5, 4),
    ];
    for (key, idx) in digit_keys {
        if keyboard.just_pressed(key) && idx < count {
            selection_state.cursor = idx;
        }
    }

    let enter_pressed = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::NumpadEnter)
        || keyboard.just_pressed(KeyCode::Space);
    if enter_pressed {
        advance_selection(
            *screen.get(),
            selection_state.cursor,
            &mut config,
            &mut next_screen,
        );
    }
}

type SelectionItemInteractionQuery<'w, 's> =
    Query<'w, 's, (&'static Interaction, &'static SelectionItemButton), Changed<Interaction>>;

type SelectionConfirmInteractionQuery<'w, 's> =
    Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<SelectionConfirmButton>)>;

fn selection_button_interaction_system(
    item_query: SelectionItemInteractionQuery,
    confirm_query: SelectionConfirmInteractionQuery,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    screen: Res<State<AppScreen>>,
    mut next_screen: ResMut<NextState<AppScreen>>,
    mut config: ResMut<NewGameConfig>,
    mut selection_state: ResMut<SelectionState>,
) {
    let mouse_clicked = mouse_buttons.just_pressed(MouseButton::Left);

    for (interaction, item_btn) in &item_query {
        match *interaction {
            Interaction::Hovered => {
                selection_state.cursor = item_btn.0;
            }
            Interaction::Pressed if mouse_clicked => {
                selection_state.cursor = item_btn.0;
                advance_selection(
                    *screen.get(),
                    selection_state.cursor,
                    &mut config,
                    &mut next_screen,
                );
                return;
            }
            _ => {}
        }
    }

    if mouse_clicked && confirm_query.iter().any(|i| *i == Interaction::Pressed) {
        advance_selection(
            *screen.get(),
            selection_state.cursor,
            &mut config,
            &mut next_screen,
        );
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_selection_visual_system(
    selection_state: Res<SelectionState>,
    screen: Res<State<AppScreen>>,
    mut text_query: Query<(&SelectionItemText, &mut Text, &mut TextColor)>,
    mut btn_query: Query<(&SelectionItemButton, &mut BackgroundColor, &mut BorderColor)>,
    mut title_query: Query<
        &mut Text,
        (
            With<SelectionDetailTitle>,
            Without<SelectionItemText>,
            Without<SelectionDetailDesc>,
            Without<SelectionDetailBonus>,
        ),
    >,
    mut desc_query: Query<
        &mut Text,
        (
            With<SelectionDetailDesc>,
            Without<SelectionItemText>,
            Without<SelectionDetailTitle>,
            Without<SelectionDetailBonus>,
        ),
    >,
    mut bonus_query: Query<
        &mut Text,
        (
            With<SelectionDetailBonus>,
            Without<SelectionItemText>,
            Without<SelectionDetailTitle>,
            Without<SelectionDetailDesc>,
        ),
    >,
) {
    if !selection_state.is_changed() && !screen.is_changed() {
        return;
    }

    let data = get_selection_display_data(*screen.get(), selection_state.cursor);

    for (item_text, mut text, mut color) in &mut text_query {
        let is_selected = item_text.0 == selection_state.cursor;
        let prefix = if is_selected { "▶ " } else { "   " };
        let label = data
            .items
            .get(item_text.0)
            .map(|s| s.as_str())
            .unwrap_or("");
        *text = Text::new(format!("{}{}. {}", prefix, item_text.0 + 1, label));
        *color = TextColor(if is_selected {
            palette::TEXT_HIGHLIGHT
        } else {
            palette::TEXT
        });
    }

    for (item_btn, mut bg, mut border) in &mut btn_query {
        let is_selected = item_btn.0 == selection_state.cursor;
        *bg = BackgroundColor(if is_selected {
            Color::srgba(0.15, 0.85, 0.35, 0.25)
        } else {
            Color::NONE
        });
        *border = BorderColor(if is_selected {
            palette::TEXT_HIGHLIGHT
        } else {
            palette::FRAME
        });
    }

    if let Ok(mut title) = title_query.get_single_mut() {
        *title = Text::new(&data.detail_title);
    }
    if let Ok(mut desc) = desc_query.get_single_mut() {
        *desc = Text::new(&data.detail_desc);
    }
    if let Ok(mut bonus) = bonus_query.get_single_mut() {
        *bonus = Text::new(&data.detail_bonus);
    }
}

type ConfirmButtonInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static mut BackgroundColor),
    (Changed<Interaction>, With<SelectionConfirmButton>),
>;

fn confirm_button_visual_feedback_system(mut query: ConfirmButtonInteractionQuery) {
    for (interaction, mut bg) in &mut query {
        *bg = match interaction {
            Interaction::Pressed => BackgroundColor(Color::srgba(0.15, 0.85, 0.35, 0.35)),
            Interaction::Hovered => BackgroundColor(Color::srgba(0.15, 0.85, 0.35, 0.15)),
            Interaction::None => BackgroundColor(Color::NONE),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_input_sanitization_and_limit() {
        let mut buffer = String::new();
        let input = "Hello\r\n\tWorld! 1234567890 extra characters";
        let remaining = MAX_NAME_LEN.saturating_sub(buffer.chars().count());
        for c in input.chars().filter(|c| !c.is_control()).take(remaining) {
            buffer.push(c);
        }
        assert_eq!(buffer.chars().count(), MAX_NAME_LEN);
        assert!(!buffer.contains('\r'));
        assert!(!buffer.contains('\n'));
        assert!(!buffer.contains('\t'));
        assert_eq!(buffer, "HelloWorld! 12345678");
    }

    fn create_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppScreen>()
            .init_resource::<NewGameConfig>()
            .init_resource::<NameEntryState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, name_entry_confirm_system);
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::WorldCreation);
        app.update();
        app
    }

    #[test]
    fn test_name_entry_confirm_world_creation_default() {
        let mut app = create_test_app();

        // Enterキー押下で決定
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);

        app.update();

        let config = app.world().resource::<NewGameConfig>();
        assert_eq!(config.world_name, DEFAULT_WORLD_NAME);
    }

    #[test]
    fn test_name_entry_confirm_custom_name_trimmed() {
        let mut app = create_test_app();

        // トリム対象の文字列を設定
        app.world_mut().resource_mut::<NameEntryState>().buffer = "  Eldoria  ".to_string();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);

        app.update();

        let config = app.world().resource::<NewGameConfig>();
        assert_eq!(config.world_name, "Eldoria");
    }

    #[test]
    fn test_button_pressed_without_mouse_just_pressed_ignored() {
        let mut app = create_test_app();

        // CreateButton エンティティを Interaction::Pressed で作成するが、
        // mouse_buttons には just_pressed を入れない（長押し押しっぱなしをシミュレート）
        app.world_mut().spawn((CreateButton, Interaction::Pressed));

        app.update();

        // 決定されず、ワールド名は空（未確定）のままであること
        let config = app.world().resource::<NewGameConfig>();
        assert_eq!(config.world_name, "");
    }

    #[test]
    fn test_button_pressed_with_mouse_just_pressed_confirms() {
        let mut app = create_test_app();

        // CreateButton エンティティを Interaction::Pressed で作成
        app.world_mut().spawn((CreateButton, Interaction::Pressed));
        // マウス左クリックの just_pressed を送信
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        app.update();

        // 決定されてデフォルト名がセットされる
        let config = app.world().resource::<NewGameConfig>();
        assert_eq!(config.world_name, DEFAULT_WORLD_NAME);
    }

    fn create_title_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppScreen>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, title_input_system);
        app
    }

    #[test]
    fn test_title_screen_transitions_to_world_creation_on_enter() {
        let mut app = create_title_test_app();
        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::Title
        );

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);

        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::WorldCreation
        );
    }

    #[test]
    fn test_title_screen_transitions_to_world_creation_on_space() {
        let mut app = create_title_test_app();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Space);

        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::WorldCreation
        );
    }

    #[test]
    fn test_title_screen_transitions_on_button_click() {
        let mut app = create_title_test_app();

        app.world_mut()
            .spawn((TitleNewGameButton, Interaction::Pressed));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);

        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::WorldCreation
        );
    }

    #[test]
    fn test_title_screen_button_without_mouse_just_pressed_ignored() {
        let mut app = create_title_test_app();

        app.world_mut()
            .spawn((TitleNewGameButton, Interaction::Pressed));

        app.update();

        // 状態はTitleのままであること
        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::Title
        );
    }

    #[test]
    fn test_character_creation_transitions_to_origin_selection() {
        let mut app = create_test_app();
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(AppScreen::CharacterCreation);
        app.update();
        app.update();

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::OriginSelection
        );
    }

    fn create_selection_test_app(initial_screen: AppScreen) -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppScreen>()
            .init_resource::<NewGameConfig>()
            .init_resource::<SelectionState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, selection_input_system.run_if(in_selection_screen));
        app.world_mut()
            .resource_mut::<NextState<AppScreen>>()
            .set(initial_screen);
        app.update();
        app.update();
        app
    }

    #[test]
    fn test_origin_selection_keyboard_navigation_and_confirm() {
        let mut app = create_selection_test_app(AppScreen::OriginSelection);

        // 最初は cursor 0 (MerchantSon)
        assert_eq!(app.world().resource::<SelectionState>().cursor, 0);

        // 下キーで cursor 1 (FormerMercenary) に移動
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ArrowDown);
        app.update();
        assert_eq!(app.world().resource::<SelectionState>().cursor, 1);

        // 前の入力をクリアして Enter で決定
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::MotiveSelection
        );
        assert_eq!(
            app.world().resource::<NewGameConfig>().build.origin,
            OriginKind::FormerMercenary
        );
    }

    #[test]
    fn test_motive_selection_aspiring_adventurer_transitions_to_generating() {
        let mut app = create_selection_test_app(AppScreen::MotiveSelection);

        // デフォルト cursor 0 (AspiringAdventurer) で決定
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::Generating
        );
        assert_eq!(
            app.world().resource::<NewGameConfig>().build.motive,
            MotiveKind::AspiringAdventurer
        );
        assert!(app
            .world()
            .resource::<NewGameConfig>()
            .build
            .partner
            .is_none());
    }

    #[test]
    fn test_motive_selection_eloper_transitions_to_partner_selection() {
        let mut app = create_selection_test_app(AppScreen::MotiveSelection);

        // '2' キーで直接 Eloper (index 1) を選択
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit2);
        app.update();
        assert_eq!(app.world().resource::<SelectionState>().cursor, 1);

        // 前の入力をクリアして Enter で決定
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::PartnerSelection
        );
        assert_eq!(
            app.world().resource::<NewGameConfig>().build.motive,
            MotiveKind::Eloper
        );
    }

    #[test]
    fn test_partner_selection_confirm_transitions_to_generating() {
        let mut app = create_selection_test_app(AppScreen::PartnerSelection);

        // 下キーで2番目の相手（クラリス）を選択
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyS);
        app.update();
        assert_eq!(app.world().resource::<SelectionState>().cursor, 1);

        // 前の入力をクリアして Enter で決定
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppScreen>>().get(),
            AppScreen::Generating
        );
        let config = app.world().resource::<NewGameConfig>();
        assert!(config.build.partner.is_some());
        assert_eq!(config.build.partner.as_ref().unwrap().name, "クラリス");
    }
}
