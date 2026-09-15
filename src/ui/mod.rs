pub mod animation;
pub mod view;

pub use animation::*;
pub use view::*;

use bevy::prelude::*;
use crate::{
    battle::BattleState, party::PartyState, town::TownState, AppMode, CommandMenuState,
    PlayerInventory, TravelState,
};
use view::{format_left_window, format_right_window, format_status_header};

// ─────────────────────────────────────────────
// 罫線文字セット（単線）
// ─────────────────────────────────────────────
pub mod box_chars {
    pub const TOP_LEFT: char = '┌';
    pub const TOP_RIGHT: char = '┐';
    pub const BOTTOM_LEFT: char = '└';
    pub const BOTTOM_RIGHT: char = '┘';
    pub const HORIZONTAL: char = '─';
    pub const VERTICAL: char = '│';
}

// ターミナル風パレット
pub mod palette {
    use bevy::prelude::Color;
    pub const BG: Color = Color::srgb(0.02, 0.02, 0.02);
    pub const FRAME: Color = Color::srgb(0.15, 0.85, 0.35);
    pub const TEXT: Color = Color::srgb(0.55, 1.0, 0.65);
    pub const TEXT_HIGHLIGHT: Color = Color::srgb(1.0, 0.9, 0.3);
    pub const DAMAGE: Color = Color::srgb(1.0, 0.25, 0.25);
}

// フォント設定
pub const FONT_PATH: &str = "fonts/BizinGothic-Regular.ttf";
pub const CELL_PX: f32 = 18.0;

#[derive(Component)]
pub struct TypewriterMessage {
    pub full_text: String,
    pub shown_chars: usize,
    pub timer: Timer,
}

#[derive(Component, Default)]
pub struct LeftWindowTextNode;

#[derive(Component)]
pub struct MessageTextNode;

/// メッセージ本文と同じグリッドセルに重ねて描画する下線オーバーレイ（ADR-0012）。
#[derive(Component, Default)]
pub struct MessageUnderlineNode;

/// 「おぼえる」候補選択中、カーソルが指す対象語だけを重ね書きしてハイライト色に変えるオーバーレイ（ADR-0012）。
#[derive(Component, Default)]
pub struct MessageHighlightNode;

#[derive(Component, Default)]
pub struct RightWindowTextNode;

#[derive(Component)]
pub struct StatusHeaderNode;

#[derive(Component)]
pub struct TownMapImageNode;

#[derive(Component)]
pub struct BattleMonsterTextNode;

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    party: Res<PartyState>,
    town_mode: Res<AppMode>,
    inv: Res<PlayerInventory>,
    battle: Res<BattleState>,
) {
    let font = asset_server.load(FONT_PATH);
    let town = TownState::new(&mut images);

    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        })
        .with_children(|root| {
            // 上部：ステータス＆操作ガイドウィンドウ (4行)
            spawn_status_window(root, font.clone(), &party, *town_mode, &town, &inv);

            // 中央：ドット絵街マップ or 敵モンスターのウィンドウ (9行)
            spawn_center_window(root, font.clone(), &town);

            // 下部：ADR-0002準拠 三分割ウィンドウ
            spawn_tri_split_window(
                root,
                font.clone(),
                &inv,
                &battle,
                &party,
                "【王都アルカン 商業区】\n夜の冷たい風が石畳を抜けていく。[Z]キーでコマンドを開き、話したい相手や調べたい対象の方向を選ぼう。",
            );
        });

    commands.insert_resource(town);
}

pub fn spawn_status_window(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    party: &PartyState,
    mode: AppMode,
    town: &TownState,
    inv: &PlayerInventory,
) {
    let inner_cols = 46;
    let inner_rows = 4;
    let outer_cols = inner_cols + 2;
    let outer_rows = inner_rows + 2;

    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::px(outer_cols as u16, CELL_PX)],
            grid_template_rows: vec![RepeatedGridTrack::px(outer_rows as u16, CELL_PX)],
            ..default()
        })
        .with_children(|grid| {
            spawn_box_border(grid, font.clone(), outer_cols, outer_rows);

            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                Text::new(format_status_header(
                    party,
                    mode,
                    town,
                    inv,
                    &None,
                    &CommandMenuState::default(),
                    &TravelState::default(),
                )),
                TextFont {
                    font,
                    font_size: CELL_PX * 0.82,
                    ..default()
                },
                TextColor(palette::TEXT_HIGHLIGHT),
                StatusHeaderNode,
            ));
        });
}

pub fn spawn_center_window(parent: &mut ChildBuilder, font: Handle<Font>, town: &TownState) {
    let inner_cols = 46;
    let inner_rows = 13;
    let outer_cols = inner_cols + 2;
    let outer_rows = inner_rows + 2;

    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::px(outer_cols as u16, CELL_PX)],
            grid_template_rows: vec![RepeatedGridTrack::px(outer_rows as u16, CELL_PX)],
            ..default()
        })
        .with_children(|grid| {
            spawn_box_border(grid, font.clone(), outer_cols, outer_rows);

            // ドット絵街マップ描画ノード（枠の内側 828×234px に完全フィット）
            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    display: Display::Flex,
                    ..default()
                },
                ImageNode::new(town.texture_handle.clone()),
                TownMapImageNode,
            ));

            // 戦闘モンスター描画ノード（初期は非表示）
            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    padding: UiRect::all(Val::Px(4.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    display: Display::None,
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font,
                    font_size: CELL_PX * 0.85,
                    ..default()
                },
                TextColor(palette::TEXT),
                BattleMonsterTextNode,
            ));
        });
}

pub fn spawn_tri_split_window(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    inv: &PlayerInventory,
    battle: &BattleState,
    party: &PartyState,
    default_msg: &str,
) {
    let total_width = (15 + 22 + 11) as f32 * CELL_PX; // 48列 = 864.0 px

    parent
        .spawn(Node {
            width: Val::Px(total_width),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexStart,
            ..default()
        })
        .with_children(|row| {
            // 左ウィンドウ: 13列 + 枠2列 = 15列 (270px)
            spawn_sub_window::<LeftWindowTextNode>(
                row,
                font.clone(),
                13,
                5,
                &format_left_window(
                    AppMode::Town,
                    inv,
                    &None,
                    &CommandMenuState::default(),
                    crate::town::TargetKind::Nothing,
                    &TravelState::default(),
                    battle,
                    party,
                ),
            );

            // 中央メッセージウィンドウ: 20列 + 枠2列 = 22列 (396px)
            spawn_sub_message_window(row, font.clone(), 20, 5, default_msg);

            // 右行動ウィンドウ: 9列 + 枠2列 = 11列 (198px)
            spawn_sub_window::<RightWindowTextNode>(
                row,
                font.clone(),
                9,
                5,
                &format_right_window(AppMode::Town, &None, &CommandMenuState::default(), battle),
            );
        });
}

pub fn spawn_sub_window<M: Component + Default>(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    inner_cols: usize,
    inner_rows: usize,
    initial_text: &str,
) {
    let outer_cols = inner_cols + 2;
    let outer_rows = inner_rows + 2;

    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::px(outer_cols as u16, CELL_PX)],
            grid_template_rows: vec![RepeatedGridTrack::px(outer_rows as u16, CELL_PX)],
            ..default()
        })
        .with_children(|grid| {
            spawn_box_border(grid, font.clone(), outer_cols, outer_rows);

            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                Text::new(initial_text),
                TextFont {
                    font,
                    font_size: CELL_PX * 0.82,
                    ..default()
                },
                TextColor(palette::TEXT),
                M::default(),
            ));
        });
}

pub fn spawn_sub_message_window(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    inner_cols: usize,
    inner_rows: usize,
    message: &str,
) {
    let outer_cols = inner_cols + 2;
    let outer_rows = inner_rows + 2;

    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::px(outer_cols as u16, CELL_PX)],
            grid_template_rows: vec![RepeatedGridTrack::px(outer_rows as u16, CELL_PX)],
            ..default()
        })
        .with_children(|grid| {
            spawn_box_border(grid, font.clone(), outer_cols, outer_rows);

            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: CELL_PX * 0.82,
                    ..default()
                },
                TextColor(palette::TEXT),
                MessageTextNode,
                TypewriterMessage {
                    full_text: message.to_string(),
                    shown_chars: 0,
                    timer: Timer::from_seconds(0.02, TimerMode::Repeating),
                },
            ));

            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font: font.clone(),
                    font_size: CELL_PX * 0.82,
                    ..default()
                },
                TextColor(palette::FRAME),
                MessageUnderlineNode,
            ));

            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font,
                    font_size: CELL_PX * 0.82,
                    ..default()
                },
                TextColor(palette::TEXT_HIGHLIGHT),
                MessageHighlightNode,
            ));
        });
}

pub fn spawn_box_border(
    grid: &mut ChildBuilder,
    font: Handle<Font>,
    outer_cols: usize,
    outer_rows: usize,
) {
    for row in 0..outer_rows {
        for col in 0..outer_cols {
            let is_top = row == 0;
            let is_bottom = row == outer_rows - 1;
            let is_left = col == 0;
            let is_right = col == outer_cols - 1;

            let ch = match (is_top, is_bottom, is_left, is_right) {
                (true, _, true, _) => Some(box_chars::TOP_LEFT),
                (true, _, _, true) => Some(box_chars::TOP_RIGHT),
                (_, true, true, _) => Some(box_chars::BOTTOM_LEFT),
                (_, true, _, true) => Some(box_chars::BOTTOM_RIGHT),
                (true, false, false, false) => Some(box_chars::HORIZONTAL),
                (false, true, false, false) => Some(box_chars::HORIZONTAL),
                (false, false, true, false) => Some(box_chars::VERTICAL),
                (false, false, false, true) => Some(box_chars::VERTICAL),
                _ => None,
            };

            if let Some(c) = ch {
                grid.spawn((
                    Node {
                        grid_column: GridPlacement::start((col + 1) as i16),
                        grid_row: GridPlacement::start((row + 1) as i16),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Text::new(c.to_string()),
                    TextFont {
                        font: font.clone(),
                        font_size: CELL_PX,
                        ..default()
                    },
                    TextColor(palette::FRAME),
                ));
            }
        }
    }
}
