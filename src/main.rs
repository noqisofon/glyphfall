use bevy::prelude::*;
use rand::{thread_rng, Rng};

mod battle;
mod party;
mod town;

use battle::{create_default_monsters, BattleState, Monster};
use party::{
    diagnose_member, evaluate_command, ActionOutcome, Influence, MentalState, PartyCommand,
    PartyMember, Personality, PlayerInventory, PlayerSkills,
};
use town::{DialoguePartner, DialogueSession, MoveOutcome, TownState, SHOP_ITEMS};

// ─────────────────────────────────────────────
// 罫線文字セット（単線）
// ─────────────────────────────────────────────
mod box_chars {
    pub const TOP_LEFT: char = '┌';
    pub const TOP_RIGHT: char = '┐';
    pub const BOTTOM_LEFT: char = '└';
    pub const BOTTOM_RIGHT: char = '┘';
    pub const HORIZONTAL: char = '─';
    pub const VERTICAL: char = '│';
}

// ターミナル風パレット
mod palette {
    use bevy::prelude::Color;
    pub const BG: Color = Color::srgb(0.02, 0.02, 0.02);
    pub const FRAME: Color = Color::srgb(0.15, 0.85, 0.35);
    pub const TEXT: Color = Color::srgb(0.55, 1.0, 0.65);
    pub const TEXT_HIGHLIGHT: Color = Color::srgb(1.0, 0.9, 0.3);
    pub const DAMAGE: Color = Color::srgb(1.0, 0.25, 0.25);
}

// フォント設定（Bizin Gothic / Mint Mono どちらも assets/fonts に配置済み）
const FONT_PATH: &str = "fonts/BizinGothic-Regular.ttf";
// const FONT_PATH: &str = "fonts/MintMono-Regular.ttf";
const CELL_PX: f32 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Resource)]
pub enum AppMode {
    Town,
    Dialogue,
    Shop,
    Inn,
    Battle,
}

#[derive(Component)]
struct TypewriterMessage {
    full_text: String,
    shown_chars: usize,
    timer: Timer,
}

#[derive(Component, Default)]
struct LeftWindowTextNode;

#[derive(Component)]
struct MessageTextNode;

#[derive(Component, Default)]
struct RightWindowTextNode;

#[derive(Component)]
struct StatusHeaderNode;

#[derive(Component)]
struct TownMapImageNode;

#[derive(Component)]
struct BattleMonsterTextNode;

#[derive(Resource)]
struct PartyState {
    members: Vec<PartyMember>,
    selected_index: usize,
    debug_mode: bool,
}

#[derive(Resource)]
struct PlayerResource {
    skills: PlayerSkills,
}

#[derive(Resource, Default)]
struct ActiveDialogue(pub Option<DialogueSession>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Glyphfall - Town, Dungeon & Tri-Split Conversation Prototype".into(),
                resolution: (1000.0_f32, 640.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(palette::BG))
        .insert_resource(AppMode::Town)
        .insert_resource(PlayerInventory::default())
        .insert_resource(ActiveDialogue::default())
        .insert_resource(PlayerResource {
            skills: PlayerSkills {
                magic_knowledge: 45,
                keen_eye: 55,
            },
        })
        .insert_resource(BattleState {
            current_index: 0,
            monsters: create_default_monsters(),
            flash_timer: Timer::from_seconds(0.18, TimerMode::Once),
            is_flashing: false,
        })
        .insert_resource(PartyState {
            selected_index: 0,
            debug_mode: false,
            members: vec![
                PartyMember::new(
                    "戦士ガルツ",
                    "せんし",
                    Influence::new_natural(82),
                    MentalState::Normal,
                    Personality::Loyal,
                ),
                PartyMember::new(
                    "遊び人ロロ",
                    "あそびにん",
                    Influence::new_natural(45),
                    MentalState::Normal,
                    Personality::Slacker,
                ),
                PartyMember::new(
                    "魔法使いミレイ",
                    "まほうつかい",
                    Influence::new_natural(88), // 自然上限寸前の素の執着
                    MentalState::Normal,
                    Personality::Yandere,
                ),
                PartyMember::new(
                    "騎士アルヴィン",
                    "きし",
                    Influence::new_supernatural(115), // 魅了による異常値
                    MentalState::Charmed,
                    Personality::Loyal,
                ),
            ],
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (typewriter_tick, handle_input, monster_flash_tick),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    party: Res<PartyState>,
    town_mode: Res<AppMode>,
    inv: Res<PlayerInventory>,
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

            // 下部：ADR-0002準拠 三分割ウィンドウ（左：話題/商品 13列, 中：メッセージ 20列, 右：行動 9列）
            spawn_tri_split_window(
                root,
                font.clone(),
                &inv,
                "【王都アルカン 商業区】\n夜の冷たい風が石畳を抜けていく。住人やお店に接触すると会話・取引ができます。",
            );
        });

    commands.insert_resource(town);
}

fn format_status_header(
    party: &PartyState,
    mode: AppMode,
    town: &TownState,
    inv: &PlayerInventory,
    dialogue: &Option<DialogueSession>,
) -> String {
    let member = &party.members[party.selected_index];
    let mut header = format!(
        "▼ 注目中の仲間 [No.{}/{}]: {} ({})  HP: {}/{}  MP: {}/{}\n",
        party.selected_index + 1,
        party.members.len(),
        member.name,
        member.job,
        member.hp,
        member.max_hp,
        member.mp,
        member.max_mp
    );

    if party.debug_mode {
        let abnormal_str = if member.influence.is_abnormal() {
            "【魔術介入/異常】"
        } else {
            "（自然上限内）"
        };
        let mental_str = match member.mental_state {
            MentalState::Normal => "正常",
            MentalState::Charmed => "魅了(Charmed)",
            MentalState::Cursed => "呪縛(Cursed)",
        };
        let personality_str = match member.personality {
            Personality::Loyal => "忠実(Loyal)",
            Personality::Slacker => "遊び人(Slacker)",
            Personality::Coward => "臆病(Coward)",
            Personality::Yandere => "ヤンデレ(Yandere)",
        };
        header.push_str(&format!(
            "  [DEBUG] 影響度: {} {} | 精神: {} | 性格: {} | 全視界: {}\n",
            member.influence.raw_value, abnormal_str, mental_str, personality_str, town.debug_see_all
        ));
    } else {
        let torch_str = if town.torch_active { "点灯中(半径6)" } else { "消灯(半径2)" };
        match mode {
            AppMode::Town => {
                header.push_str(&format!(
                    "  [{}] 所持金: {}G | 松明: {} | 仲間列: 主人公(青) 戦士(赤) 遊び人(黄) 魔法使い(紫) 騎士(白)\n",
                    town.current_area.name(),
                    inv.gold,
                    torch_str
                ));
            }
            AppMode::Dialogue => {
                let partner_name = dialogue.as_ref().map(|s| s.partner.name()).unwrap_or("相手");
                header.push_str(&format!(
                    "  [会話中: {}] 所持金: {}G | [W/S]で話題選択 | [1]たずねる | [2]おぼえる | [3]はなれる\n",
                    partner_name, inv.gold
                ));
            }
            AppMode::Shop => {
                header.push_str(&format!(
                    "  [道具屋・取引中: 道具屋の店主] 所持金: {}G | [W/S]で商品選択 | [1]購入 | [3]店を出る\n",
                    inv.gold
                ));
            }
            AppMode::Inn => {
                header.push_str(&format!(
                    "  [宿屋・受付: 宿屋の主人] 所持金: {}G | [1]宿泊(50G)で全快 | [3]宿を出る\n",
                    inv.gold
                ));
            }
            AppMode::Battle => {
                header.push_str("  [地下封鎖迷宮・戦闘交戦中] (F1キーで開発用隠しパラメータを表示)\n");
            }
        }
    }

    match mode {
        AppMode::Town => {
            header.push_str("操作: [WASD]移動 | [L]松明切替 | [B]戦闘切替 | [Tab]仲間切替 | [Space]スキップ");
        }
        AppMode::Dialogue => {
            header.push_str("操作: [W/S]話題選択 | [1/Enter]たずねる | [2]おぼえる | [3/Esc]はなれる");
        }
        AppMode::Shop => {
            header.push_str("操作: [W/S]商品選択 | [1/Enter]かう | [3/Esc]店を出る");
        }
        AppMode::Inn => {
            header.push_str("操作: [1/Enter]とまる(50G) | [3/Esc]やめる");
        }
        AppMode::Battle => {
            header.push_str("操作: [1]たたかう | [2]みをまもる | [3]すてみ | [4]観察 | [N]敵切替 | [B]街へ帰還");
        }
    }

    header
}

fn format_left_window(
    mode: AppMode,
    inv: &PlayerInventory,
    dialogue: &Option<DialogueSession>,
) -> String {
    match mode {
        AppMode::Dialogue => {
            let session = dialogue.as_ref();
            let selected = session.map(|s| s.selected_topic_index).unwrap_or(0);
            let mut out = "【話題を振る】\n".to_string();
            for (idx, topic) in inv.topics.iter().enumerate().take(4) {
                let cursor = if idx == selected { "▶" } else { " " };
                out.push_str(&format!("{} {}\n", cursor, topic));
            }
            out
        }
        AppMode::Shop => {
            let session = dialogue.as_ref();
            let selected = session.map(|s| s.selected_shop_index).unwrap_or(0);
            let mut out = format!("【品物】(金:{}G)\n", inv.gold);
            for (idx, item) in SHOP_ITEMS.iter().enumerate() {
                let cursor = if idx == selected { "▶" } else { " " };
                out.push_str(&format!("{}{} {:2}G\n", cursor, item.name, item.price));
            }
            out
        }
        AppMode::Inn => {
            format!(
                "【宿屋・宿泊】\n一泊料金: 50G\n所持金  : {}G\n全員のHP/MP全快",
                inv.gold
            )
        }
        AppMode::Town => {
            let mut out = format!("【手帳】金:{}G\n[覚えた話題]\n", inv.gold);
            for topic in inv.topics.iter().take(3) {
                out.push_str(&format!("・{}\n", topic));
            }
            out
        }
        AppMode::Battle => {
            format!(
                "【所持アイテム】\n・やくそう\n・特やくそう\n所持金: {}G",
                inv.gold
            )
        }
    }
}

fn format_right_window(mode: AppMode, dialogue: &Option<DialogueSession>) -> String {
    match mode {
        AppMode::Dialogue => {
            let has_learnable = dialogue
                .as_ref()
                .and_then(|s| s.learnable_topic.as_ref())
                .is_some();
            let learn_str = if has_learnable {
                "[2]おぼえる★"
            } else {
                "[2]おぼえる"
            };
            format!("[1]たずねる\n{}\n[3]はなれる\n(W/S:選択)", learn_str)
        }
        AppMode::Shop => "[1]かう\n[3]みせをでる\n(W/S:商品選)\n(所持金消費)".into(),
        AppMode::Inn => "[1]とまる(50G)\n[3]やめる\n\n(HP/MP全回復)".into(),
        AppMode::Town => "[探索操作]\nWASD:移動\nL   :松明\nTab :仲間\nB   :戦闘".into(),
        AppMode::Battle => "[1]たたかう\n[2]みをまもる\n[3]すてみ\n[4]観察\n[B]街へ帰還".into(),
    }
}

fn format_hp_bar(hp: i32, max_hp: i32, length: usize) -> String {
    let ratio = (hp as f32 / max_hp as f32).clamp(0.0, 1.0);
    let filled = (ratio * length as f32).round() as usize;
    let empty = length.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), " ".repeat(empty))
}

fn format_monster_display(monster: &Monster) -> String {
    let hp_bar = format_hp_bar(monster.hp, monster.max_hp, 14);
    let clean_art = monster.glyph_art.trim_matches('\n');
    format!(
        "{}\n           【 {} 】  HP: {:2}/{}  {}\n",
        clean_art, monster.name, monster.hp, monster.max_hp, hp_bar
    )
}

fn spawn_status_window(
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
                Text::new(format_status_header(party, mode, town, inv, &None)),
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

fn spawn_center_window(parent: &mut ChildBuilder, font: Handle<Font>, town: &TownState) {
    let inner_cols = 46;
    let inner_rows = 9;
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

            // ドット絵街マップ描画ノード（枠の内側 920×180px に完全フィット）
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

fn spawn_tri_split_window(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    inv: &PlayerInventory,
    default_msg: &str,
) {
    let total_width = (15 + 22 + 11) as f32 * CELL_PX; // 48列 = 960.0 px

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
                &format_left_window(AppMode::Town, inv, &None),
            );

            // 中央メッセージウィンドウ: 20列 + 枠2列 = 22列 (396px)
            spawn_sub_message_window(row, font.clone(), 20, 5, default_msg);

            // 右行動ウィンドウ: 9列 + 枠2列 = 11列 (198px)
            spawn_sub_window::<RightWindowTextNode>(
                row,
                font.clone(),
                9,
                5,
                &format_right_window(AppMode::Town, &None),
            );
        });
}

fn spawn_sub_window<M: Component + Default>(
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

fn spawn_sub_message_window(
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
                    font,
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
        });
}

fn spawn_box_border(
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

fn typewriter_tick(
    time: Res<Time>,
    mut query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
    for (mut msg, mut text) in &mut query {
        if msg.shown_chars >= msg.full_text.chars().count() {
            continue;
        }
        msg.timer.tick(time.delta());
        if msg.timer.just_finished() {
            msg.shown_chars += 1;
            let shown: String = msg.full_text.chars().take(msg.shown_chars).collect();
            *text = Text::new(shown);
        }
    }
}

fn monster_flash_tick(
    time: Res<Time>,
    mode: Res<AppMode>,
    mut battle: ResMut<BattleState>,
    mut monster_query: Query<&mut TextColor, With<BattleMonsterTextNode>>,
) {
    if *mode == AppMode::Battle && battle.is_flashing {
        battle.flash_timer.tick(time.delta());
        if let Ok(mut color) = monster_query.get_single_mut() {
            *color = TextColor(palette::DAMAGE);
        }
        if battle.flash_timer.finished() {
            battle.is_flashing = false;
            battle.flash_timer.reset();
            if let Ok(mut color) = monster_query.get_single_mut() {
                *color = TextColor(palette::TEXT);
            }
        }
    }
}

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownState>,
    mut images: ResMut<Assets<Image>>,
    mut party: ResMut<PartyState>,
    mut inv: ResMut<PlayerInventory>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut battle: ResMut<BattleState>,
    player: Res<PlayerResource>,
    mut header_query: Query<&mut Text, (With<StatusHeaderNode>, Without<MessageTextNode>, Without<BattleMonsterTextNode>, Without<LeftWindowTextNode>, Without<RightWindowTextNode>)>,
    mut left_window_query: Query<&mut Text, (With<LeftWindowTextNode>, Without<MessageTextNode>, Without<StatusHeaderNode>, Without<BattleMonsterTextNode>, Without<RightWindowTextNode>)>,
    mut right_window_query: Query<&mut Text, (With<RightWindowTextNode>, Without<MessageTextNode>, Without<StatusHeaderNode>, Without<BattleMonsterTextNode>, Without<LeftWindowTextNode>)>,
    mut battle_monster_query: Query<(&mut Text, &mut Node), (With<BattleMonsterTextNode>, Without<MessageTextNode>, Without<StatusHeaderNode>, Without<TownMapImageNode>, Without<LeftWindowTextNode>, Without<RightWindowTextNode>)>,
    mut town_image_query: Query<&mut Node, (With<TownMapImageNode>, Without<BattleMonsterTextNode>)>,
    mut message_query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
    let mut rng = thread_rng();
    let mut new_message = None;
    let mut update_header = false;
    let mut mode_changed = false;
    let mut update_monster_display = false;
    let mut update_tri_windows = false;

    // [Space]: タイプライターの文字送りをスキップ
    if keyboard.just_pressed(KeyCode::Space) {
        if let Ok((mut type_msg, mut text)) = message_query.get_single_mut() {
            type_msg.shown_chars = type_msg.full_text.chars().count();
            *text = Text::new(type_msg.full_text.clone());
        }
    }

    // [F1]: デバッグ表示切替（街探索時は全視界可視化もトグル）
    if keyboard.just_pressed(KeyCode::F1) {
        party.debug_mode = !party.debug_mode;
        town.debug_see_all = party.debug_mode;
        town.recompute_fov();
        update_header = true;
    }

    // [Tab]: 注目する仲間切り替え
    if keyboard.just_pressed(KeyCode::Tab) {
        party.selected_index = (party.selected_index + 1) % party.members.len();
        let member = &party.members[party.selected_index];
        new_message = Some(format!("{}に　ちゅうもくした。", member.name));
        update_header = true;
    }

    // [B]: モード切替（街探索 ↔ 戦闘テスト）
    if keyboard.just_pressed(KeyCode::KeyB) {
        *mode = match *mode {
            AppMode::Battle => {
                new_message = Some("王都アルカンの街並みへ生還した。\n[WASD]で街を歩き回れる。".into());
                AppMode::Town
            }
            _ => {
                new_message = Some("地下迷宮の魔物とエンカウントした！\n[1]たたかう や [2]みをまもる で指示を出そう。[B]で街へ戻る。".into());
                AppMode::Battle
            }
        };
        dialogue_res.0 = None;
        update_header = true;
        mode_changed = true;
        update_monster_display = true;
        update_tri_windows = true;
    }

    match *mode {
        AppMode::Town => {
            // [L]: 松明のON/OFF切り替え
            if keyboard.just_pressed(KeyCode::KeyL) {
                town.torch_active = !town.torch_active;
                town.recompute_fov();
                update_header = true;
                let status = if town.torch_active {
                    "松明に火を灯した。周囲が明るくなった！（視界半径6マス）"
                } else {
                    "松明の火を消した。月明かりだけが頼りだ……（視界半径2マス）"
                };
                new_message = Some(status.into());
            }

            // 移動入力（WASD / 矢印キー）
            let mut dx = 0;
            let mut dy = 0;

            if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
                dy -= 1;
            } else if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
                dy += 1;
            } else if keyboard.just_pressed(KeyCode::KeyA) || keyboard.just_pressed(KeyCode::ArrowLeft) {
                dx -= 1;
            } else if keyboard.just_pressed(KeyCode::KeyD) || keyboard.just_pressed(KeyCode::ArrowRight) {
                dx += 1;
            }

            if dx != 0 || dy != 0 {
                let outcome = town.move_player(dx, dy);

                match outcome {
                    MoveOutcome::Moved { message } => {
                        if let Some(msg) = message {
                            new_message = Some(msg);
                        }
                    }
                    MoveOutcome::Blocked { message } => {
                        if let Some(msg) = message {
                            new_message = Some(msg);
                        }
                    }
                    MoveOutcome::TriggerBattle { message } => {
                        new_message = Some(message);
                        *mode = AppMode::Battle;
                        dialogue_res.0 = None;
                        mode_changed = true;
                        update_header = true;
                        update_monster_display = true;
                        update_tri_windows = true;
                    }
                    MoveOutcome::ChangeArea { message, .. } => {
                        new_message = Some(message);
                        update_header = true;
                    }
                    MoveOutcome::ChestOpened { gold, item, message } => {
                        inv.add_gold(gold);
                        inv.add_item(&item);
                        new_message = Some(message);
                        update_header = true;
                        update_tri_windows = true;
                    }
                    MoveOutcome::StartDialogue(partner) => {
                        let session = DialogueSession::start(partner);
                        new_message = Some(session.current_text.clone());
                        *mode = match partner {
                            DialoguePartner::Shop => AppMode::Shop,
                            DialoguePartner::Inn => AppMode::Inn,
                            _ => AppMode::Dialogue,
                        };
                        dialogue_res.0 = Some(session);
                        mode_changed = true;
                        update_header = true;
                        update_tri_windows = true;
                    }
                }
            }
        }
        AppMode::Dialogue => {
            // [W/S]: 話題選択
            if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    session.selected_topic_index = session.selected_topic_index.saturating_sub(1);
                    update_tri_windows = true;
                }
            }
            if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    let max_idx = inv.topics.len().saturating_sub(1);
                    if session.selected_topic_index < max_idx {
                        session.selected_topic_index += 1;
                        update_tri_windows = true;
                    }
                }
            }

            // [1] / [Enter]: 話題を振る
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    if let Some(topic) = inv.topics.get(session.selected_topic_index).cloned() {
                        session.ask_topic(&topic);
                        new_message = Some(session.current_text.clone());
                        update_tri_windows = true;
                    }
                }
            }

            // [2]: 「おぼえる」キーワードを手帳にストック
            if keyboard.just_pressed(KeyCode::Digit2) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    if let Some(new_topic) = session.learnable_topic.take() {
                        let learned = inv.learn_topic(&new_topic);
                        if learned {
                            new_message = Some(format!(
                                "【{}】を手帳に覚えた！\n（話題リストに追加されました）",
                                new_topic
                            ));
                        } else {
                            new_message = Some(format!("【{}】は既に覚えている。", new_topic));
                        }
                        update_tri_windows = true;
                    } else {
                        new_message = Some("新しく覚えられるキーワードは見当たらない。".into());
                    }
                }
            }

            // [3] / [Esc]: 会話を終える
            if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
                dialogue_res.0 = None;
                *mode = AppMode::Town;
                new_message = Some("会話を終えて、再び歩き出した。".into());
                mode_changed = true;
                update_header = true;
                update_tri_windows = true;
            }
        }
        AppMode::Shop => {
            // [W/S]: 商品選択
            if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    session.selected_shop_index = session.selected_shop_index.saturating_sub(1);
                    let item = &SHOP_ITEMS[session.selected_shop_index];
                    new_message = Some(format!("【{}】({}G)\n{}", item.name, item.price, item.description));
                    update_tri_windows = true;
                }
            }
            if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    if session.selected_shop_index + 1 < SHOP_ITEMS.len() {
                        session.selected_shop_index += 1;
                        let item = &SHOP_ITEMS[session.selected_shop_index];
                        new_message = Some(format!("【{}】({}G)\n{}", item.name, item.price, item.description));
                        update_tri_windows = true;
                    }
                }
            }

            // [1] / [Enter]: 商品購入
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    session.buy_item(&mut inv);
                    new_message = Some(session.current_text.clone());
                    update_header = true;
                    update_tri_windows = true;
                }
            }

            // [3] / [Esc]: 店を出る
            if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
                dialogue_res.0 = None;
                *mode = AppMode::Town;
                new_message = Some("道具屋を出た。".into());
                mode_changed = true;
                update_header = true;
                update_tri_windows = true;
            }
        }
        AppMode::Inn => {
            // [1] / [Enter]: 宿泊
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    session.rest_at_inn(&mut inv, &mut party.members);
                    new_message = Some(session.current_text.clone());
                    update_header = true;
                    update_tri_windows = true;
                }
            }

            // [3] / [Esc]: 宿を出る
            if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
                dialogue_res.0 = None;
                *mode = AppMode::Town;
                new_message = Some("宿屋を出た。".into());
                mode_changed = true;
                update_header = true;
                update_tri_windows = true;
            }
        }
        AppMode::Battle => {
            // [N]: モンスター切り替え（テスト用）
            if keyboard.just_pressed(KeyCode::KeyN) {
                battle.next_monster();
                let mon = battle.current_monster();
                new_message = Some(format!("あらたな　魔物【{}】が　あらわれた！", mon.name));
                update_monster_display = true;
            }

            // [1]: 「たたかう」
            if keyboard.just_pressed(KeyCode::Digit1) {
                let member = &party.members[party.selected_index];
                let outcome = evaluate_command(member, PartyCommand::Attack, &mut rng);
                match outcome {
                    ActionOutcome::Obeyed { action_msg } => {
                        let damage: i32 = rng.gen_range(8..=14);
                        let (mon_name, is_dead) = battle.apply_damage(damage);
                        update_monster_display = true;

                        let mut msg = format!(
                            "{}に「たたかう」よう　指示した！\n{}\n{}に {}の ダメージを与えた！",
                            member.name, action_msg, mon_name, damage
                        );

                        if is_dead {
                            msg.push_str(&format!("\n{}を　たおした！", mon_name));
                            battle.next_monster();
                            let next_mon = battle.current_monster();
                            msg.push_str(&format!("\n続いて　{}が　あらわれた！", next_mon.name));
                        }
                        new_message = Some(msg);
                    }
                    ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                        if member.personality == Personality::Yandere {
                            let damage: i32 = rng.gen_range(16..=22);
                            let (mon_name, is_dead) = battle.apply_damage(damage);
                            update_monster_display = true;

                            let mut msg = format!(
                                "{}に「たたかう」よう　指示した！\n{}\n{}\nなんと　{}に {}の 大ダメージ！",
                                member.name, reason_msg, action_msg, mon_name, damage
                            );
                            if is_dead {
                                msg.push_str(&format!("\n{}を　たおした！", mon_name));
                                battle.next_monster();
                                let next_mon = battle.current_monster();
                                msg.push_str(&format!("\n続いて　{}が　あらわれた！", next_mon.name));
                            }
                            new_message = Some(msg);
                        } else {
                            new_message = Some(format!(
                                "{}に「たたかう」よう　指示した！\n{}\n{}",
                                member.name, reason_msg, action_msg
                            ));
                        }
                    }
                };
            }

            // [2]: 「みをまもる」
            if keyboard.just_pressed(KeyCode::Digit2) {
                let member = &party.members[party.selected_index];
                let outcome = evaluate_command(member, PartyCommand::Defend, &mut rng);
                let msg = match outcome {
                    ActionOutcome::Obeyed { action_msg } => {
                        format!("{}に「みをまもる」よう　指示した。\n{}", member.name, action_msg)
                    }
                    ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                        format!(
                            "{}に「みをまもる」よう　指示した！\n{}\n{}",
                            member.name, reason_msg, action_msg
                        )
                    }
                };
                new_message = Some(msg);
            }

            // [3]: 「すてみ」
            if keyboard.just_pressed(KeyCode::Digit3) {
                let member = &party.members[party.selected_index];
                let outcome = evaluate_command(member, PartyCommand::DesperateAttack, &mut rng);
                match outcome {
                    ActionOutcome::Obeyed { action_msg } => {
                        let damage: i32 = rng.gen_range(25..=35);
                        let (mon_name, is_dead) = battle.apply_damage(damage);
                        update_monster_display = true;

                        let mut msg = format!(
                            "{}に「すてみ」を　命じた！\n{}\n会心の一撃！　{}に {}の 痛恨のダメージ！",
                            member.name, action_msg, mon_name, damage
                        );

                        if is_dead {
                            msg.push_str(&format!("\n{}を　たおした！", mon_name));
                            battle.next_monster();
                            let next_mon = battle.current_monster();
                            msg.push_str(&format!("\n続いて　{}が　あらわれた！", next_mon.name));
                        }
                        new_message = Some(msg);
                    }
                    ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                        new_message = Some(format!(
                            "{}に「すてみ」を　命じた！\n{}\n{}",
                            member.name, reason_msg, action_msg
                        ));
                    }
                };
            }

            // [4]: 「観察する」
            if keyboard.just_pressed(KeyCode::Digit4) {
                let member = &party.members[party.selected_index];
                let report = diagnose_member(&player.skills, member, &mut rng);
                let msg = format!("{}\n{}", report.observation_msg, report.conclusion_msg);
                new_message = Some(msg);
            }
        }
    }

    // テクスチャの更新（移動やFOV変化があった場合）
    town.update_texture(&mut images);

    // モード切替に伴う中央ノード表示・非表示の更新
    if mode_changed {
        if let Ok(mut town_node) = town_image_query.get_single_mut() {
            town_node.display = match *mode {
                AppMode::Battle => Display::None,
                _ => Display::Flex,
            };
        }
        if let Ok((_, mut battle_node)) = battle_monster_query.get_single_mut() {
            battle_node.display = match *mode {
                AppMode::Battle => Display::Flex,
                _ => Display::None,
            };
        }
    }

    // 戦闘モンスター表示の更新
    if update_monster_display && *mode == AppMode::Battle {
        if let Ok((mut text, _)) = battle_monster_query.get_single_mut() {
            *text = Text::new(format_monster_display(battle.current_monster()));
        }
    }

    // 三分割ウィンドウ（左・右）の更新
    if update_tri_windows {
        if let Ok(mut text) = left_window_query.get_single_mut() {
            *text = Text::new(format_left_window(*mode, &inv, &dialogue_res.0));
        }
        if let Ok(mut text) = right_window_query.get_single_mut() {
            *text = Text::new(format_right_window(*mode, &dialogue_res.0));
        }
    }

    // ヘッダーUIの更新
    if update_header {
        if let Ok(mut text) = header_query.get_single_mut() {
            *text = Text::new(format_status_header(&party, *mode, &town, &inv, &dialogue_res.0));
        }
    }

    // メッセージウィンドウの更新
    if let Some(msg) = new_message {
        if let Ok((mut type_msg, mut text)) = message_query.get_single_mut() {
            type_msg.full_text = msg;
            type_msg.shown_chars = 0;
            type_msg.timer.reset();
            *text = Text::new("");
        }
    }
}
