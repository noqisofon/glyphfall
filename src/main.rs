use bevy::prelude::*;
use rand::{thread_rng, Rng};

mod battle;
mod event;
mod party;
mod town;

use battle::{create_default_monsters, BattleState, Monster};
use event::{
    try_trigger_sudden_event, SuddenEventCategory, SuddenEventHistory, SuddenEventRegistry,
};
use party::{
    diagnose_member, evaluate_command, ActionOutcome, Influence, MentalState, PartyCommand,
    PartyMember, Personality, PlayerInventory, PlayerSkills,
};
use town::{
    AreaId, CommandKind, DialogueLearnStage, DialoguePartner, DialogueSession, InteractOutcome,
    LearnableSpan, MoveOutcome, Position, TargetKind, TownState, SHOP_ITEMS,
};

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
const CELL_PX: f32 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Resource)]
pub enum AppMode {
    Town,
    Interact,
    Dialogue,
    Shop,
    Inn,
    Battle,
    /// ADR-0004/ADR-0013: 街道口で移動姿勢を選択している最中。
    Travel,
}

/// 旅シミュレーション（ADR-0004）の移動姿勢。基礎エンカウント率のみを扱う
/// 最小実装で、移動時間短縮などのメリットは今後の課題とする。
#[derive(Clone, Copy, Debug)]
enum TravelPosture {
    Cautious,
    Normal,
    Bold,
}

impl TravelPosture {
    fn label(&self) -> &'static str {
        match self {
            TravelPosture::Cautious => "慎重に",
            TravelPosture::Normal => "普通に",
            TravelPosture::Bold => "大胆に",
        }
    }

    /// ADR-0004の姿勢別エンカウント率表に対応する基礎確率。
    fn base_probability(&self) -> f32 {
        match self {
            TravelPosture::Cautious => 0.15,
            TravelPosture::Normal => 0.35,
            TravelPosture::Bold => 0.65,
        }
    }
}

/// 街道口に接触してから移動姿勢が確定するまでの間、行き先を保持しておくための状態。
#[derive(Resource)]
struct TravelState {
    destination: AreaId,
    spawn_pos: Position,
}

impl Default for TravelState {
    fn default() -> Self {
        Self {
            destination: AreaId::Town,
            spawn_pos: Position { x: 0, y: 0 },
        }
    }
}

fn in_mode(target: AppMode) -> impl Fn(Res<AppMode>) -> bool {
    move |mode: Res<AppMode>| *mode == target
}

#[derive(Event, Debug, Clone)]
pub struct ShowMessage(pub String);

/// ADR-0011: コマンド駆動インタラクトの進行段階。
/// 「どうぐ」以外はコマンド決定後に方向選択へ遷移する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum CommandMenuStage {
    #[default]
    ChoosingCommand,
    ChoosingDirection(CommandKind),
}

#[derive(Resource, Default)]
struct CommandMenuState {
    stage: CommandMenuStage,
    selected_index: usize,
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

/// メッセージ本文と同じグリッドセルに重ねて描画する下線オーバーレイ（ADR-0012）。
/// 表示完了後、対象語の桁位置に下線グリフを並べる。
#[derive(Component, Default)]
struct MessageUnderlineNode;

/// 「おぼえる」候補選択中、カーソルが指す対象語だけを重ね書きしてハイライト色に変える
/// オーバーレイ（ADR-0012）。それ以外は空文字列にしておく。
#[derive(Component, Default)]
struct MessageHighlightNode;

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
                resolution: (960.0_f32, 640.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(palette::BG))
        .insert_resource(AppMode::Town)
        .insert_resource(PlayerInventory::default())
        .insert_resource(ActiveDialogue::default())
        .insert_resource(CommandMenuState::default())
        .insert_resource(TravelState::default())
        .insert_resource(SuddenEventRegistry::travel_default())
        .insert_resource(SuddenEventHistory::default())
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
        .add_event::<ShowMessage>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (typewriter_tick, dialogue_underline_tick, monster_flash_tick),
                (
                    handle_common_input,
                    handle_town_input.run_if(in_mode(AppMode::Town)),
                    handle_interact_input.run_if(in_mode(AppMode::Interact)),
                    handle_dialogue_input.run_if(in_mode(AppMode::Dialogue)),
                    handle_shop_input.run_if(in_mode(AppMode::Shop)),
                    handle_inn_input.run_if(in_mode(AppMode::Inn)),
                    handle_battle_input.run_if(in_mode(AppMode::Battle)),
                    handle_travel_input.run_if(in_mode(AppMode::Travel)),
                ),
                (
                    update_message_window,
                    update_town_texture_system,
                ),
                (
                    update_status_header_system,
                    update_tri_split_windows_system,
                    update_battle_monster_display_system,
                    update_center_window_visibility_system,
                ),
            )
                .chain(),
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
                "【王都アルカン 商業区】\n夜の冷たい風が石畳を抜けていく。[Z]キーでコマンドを開き、話したい相手や調べたい対象の方向を選ぼう。",
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
    command_menu: &CommandMenuState,
    travel: &TravelState,
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
                let learn_stage = dialogue.as_ref().map(|s| s.learn_stage).unwrap_or_default();
                match learn_stage {
                    DialogueLearnStage::Talking => {
                        header.push_str(&format!(
                            "  [会話中: {}] 所持金: {}G | [W/S]で話題選択 | [1]たずねる | [2]おぼえる | [3]はなれる\n",
                            partner_name, inv.gold
                        ));
                    }
                    DialogueLearnStage::ChoosingLearnTarget { .. } => {
                        header.push_str(&format!(
                            "  [会話中: {}] 覚える言葉を選択中 | [W/S]候補選択 | [1]決定 | [3]やめる\n",
                            partner_name
                        ));
                    }
                }
            }
            AppMode::Interact => {
                let hint = match command_menu.stage {
                    CommandMenuStage::ChoosingCommand => "コマンドを選択してください",
                    CommandMenuStage::ChoosingDirection(_) => "方向キーで対象を選択してください",
                };
                header.push_str(&format!(
                    "  [コマンド選択中] 所持金: {}G | {}\n",
                    inv.gold, hint
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
            AppMode::Travel => {
                header.push_str(&format!(
                    "  [街道・移動姿勢選択中] 行き先: {} | 所持金: {}G\n",
                    travel.destination.name(),
                    inv.gold
                ));
            }
        }
    }

    match mode {
        AppMode::Town => {
            header.push_str("操作: [WASD]移動 | [Z]コマンド | [L]松明切替 | [B]戦闘切替 | [Tab]仲間切替 | [Space]スキップ");
        }
        AppMode::Interact => match command_menu.stage {
            CommandMenuStage::ChoosingCommand => {
                header.push_str("操作: [W/S]コマンド選択 | [1/Enter]決定 | [3/Esc]やめる");
            }
            CommandMenuStage::ChoosingDirection(_) => {
                header.push_str("操作: [WASD]方向を選択 | [Esc]やめる");
            }
        },
        AppMode::Dialogue => {
            match dialogue.as_ref().map(|s| s.learn_stage).unwrap_or_default() {
                DialogueLearnStage::Talking => {
                    header.push_str("操作: [W/S]話題選択 | [1/Enter]たずねる | [2]おぼえる | [3/Esc]はなれる");
                }
                DialogueLearnStage::ChoosingLearnTarget { .. } => {
                    header.push_str("操作: [W/S]候補選択 | [1/Enter]決定 | [3/Esc]やめる");
                }
            }
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
        AppMode::Travel => {
            header.push_str("操作: [1]慎重に | [2]普通に | [3]大胆に | [Esc]やめる");
        }
    }

    header
}

fn format_left_window(
    mode: AppMode,
    inv: &PlayerInventory,
    dialogue: &Option<DialogueSession>,
    command_menu: &CommandMenuState,
    facing_target: TargetKind,
    travel: &TravelState,
) -> String {
    match mode {
        AppMode::Interact => match command_menu.stage {
            CommandMenuStage::ChoosingCommand => {
                let mut out = "【コマンド】\n".to_string();
                for (idx, cmd) in CommandKind::ALL.iter().enumerate() {
                    let cursor = if idx == command_menu.selected_index {
                        "▶"
                    } else {
                        " "
                    };
                    out.push_str(&format!("{} {}\n", cursor, cmd.dynamic_label(facing_target)));
                }
                out
            }
            CommandMenuStage::ChoosingDirection(cmd) => {
                format!(
                    "【{}】\n方向を選んで\n対象を指定\nしてください",
                    cmd.dynamic_label(facing_target)
                )
            }
        },
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
        AppMode::Travel => {
            format!(
                "【街道】\n行き先:\n{}\n\nどのように\n進みますか？",
                travel.destination.name()
            )
        }
    }
}

fn format_right_window(
    mode: AppMode,
    dialogue: &Option<DialogueSession>,
    command_menu: &CommandMenuState,
) -> String {
    match mode {
        AppMode::Interact => match command_menu.stage {
            CommandMenuStage::ChoosingCommand => "[W/S]選択\n[1/Enter]決定\n[3/Esc]やめる".into(),
            CommandMenuStage::ChoosingDirection(_) => "[WASD]方向選択\n[Esc]やめる".into(),
        },
        AppMode::Dialogue => {
            match dialogue.as_ref().map(|s| s.learn_stage).unwrap_or_default() {
                DialogueLearnStage::Talking => {
                    let has_learnable = dialogue
                        .as_ref()
                        .map(|s| !s.learnable_spans.is_empty())
                        .unwrap_or(false);
                    let learn_str = if has_learnable {
                        "[2]おぼえる★"
                    } else {
                        "[2]おぼえる"
                    };
                    format!("[1]たずねる\n{}\n[3]はなれる\n(W/S:選択)", learn_str)
                }
                DialogueLearnStage::ChoosingLearnTarget { .. } => {
                    "[1]決定\n[3]やめる\n\n(W/S:候補選択)".into()
                }
            }
        }
        AppMode::Shop => "[1]かう\n[3]みせをでる\n(W/S:商品選)\n(所持金消費)".into(),
        AppMode::Inn => "[1]とまる(50G)\n[3]やめる\n\n(HP/MP全回復)".into(),
        AppMode::Town => "[探索操作]\nWASD:移動\nZ   :コマンド\nTab :仲間\nB   :戦闘".into(),
        AppMode::Battle => "[1]たたかう\n[2]みをまもる\n[3]すてみ\n[4]観察\n[B]街へ帰還".into(),
        AppMode::Travel => "[1]慎重に\n[2]普通に\n[3]大胆に\n[Esc]やめる".into(),
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

fn spawn_center_window(parent: &mut ChildBuilder, font: Handle<Font>, town: &TownState) {
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

fn spawn_tri_split_window(
    parent: &mut ChildBuilder,
    font: Handle<Font>,
    inv: &PlayerInventory,
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
                    TargetKind::Nothing,
                    &TravelState::default(),
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
                &format_right_window(AppMode::Town, &None, &CommandMenuState::default()),
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

            // 下線オーバーレイ：メッセージ本文と全く同じグリッドセルに重ね、
            // 対象語の桁位置だけ`_`を並べることで等幅フォント越しに下線を表現する（ADR-0012）。
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

            // ハイライトオーバーレイ：候補選択中のカーソル位置の語だけを重ね書きし、
            // ハイライト色で「色替え」したように見せる（ADR-0012）。
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

/// テキスト中の各`LearnableSpan`が占める桁位置にだけ`_`を並べた文字列を作る。
/// 改行はそのまま維持し、行ごとの文字数が本文と一致するようにする
/// （同一グリッドセルに重ね描きしたときに、対象語の真下に揃うようにするため）。
fn build_underline_text(text: &str, spans: &[LearnableSpan]) -> String {
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            if c == '\n' {
                '\n'
            } else if spans.iter().any(|s| i >= s.start && i < s.end) {
                '_'
            } else {
                ' '
            }
        })
        .collect()
}

/// `span`が指す対象語の文字だけをそのまま残し、それ以外を空白に置き換えた文字列を作る。
/// 本文と同じグリッドセルに重ねて別色で描画することで、対象語だけ色替えしたように見せる。
fn build_highlight_text(text: &str, span: &LearnableSpan) -> String {
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            if c == '\n' {
                '\n'
            } else if i >= span.start && i < span.end {
                c
            } else {
                ' '
            }
        })
        .collect()
}

/// メッセージの文字送りが完了した後にだけ、下線と候補ハイライトのオーバーレイを更新する（ADR-0012）。
fn dialogue_underline_tick(
    mode: Res<AppMode>,
    dialogue_res: Res<ActiveDialogue>,
    typewriter_query: Query<&TypewriterMessage, With<MessageTextNode>>,
    mut underline_query: Query<
        &mut Text,
        (With<MessageUnderlineNode>, Without<MessageHighlightNode>),
    >,
    mut highlight_query: Query<
        &mut Text,
        (With<MessageHighlightNode>, Without<MessageUnderlineNode>),
    >,
) {
    let (Ok(mut underline_text), Ok(mut highlight_text)) =
        (underline_query.get_single_mut(), highlight_query.get_single_mut())
    else {
        return;
    };

    let session = (*mode == AppMode::Dialogue).then(|| dialogue_res.0.as_ref()).flatten();
    let Some(session) = session else {
        *underline_text = Text::new("");
        *highlight_text = Text::new("");
        return;
    };

    let fully_shown = typewriter_query
        .get_single()
        .map(|t| t.shown_chars >= t.full_text.chars().count())
        .unwrap_or(false);

    if fully_shown && !session.learnable_spans.is_empty() {
        *underline_text = Text::new(build_underline_text(&session.current_text, &session.learnable_spans));
    } else {
        *underline_text = Text::new("");
    }

    *highlight_text = match session.learn_stage {
        DialogueLearnStage::ChoosingLearnTarget { cursor } if fully_shown => session
            .learnable_spans
            .get(cursor)
            .map(|span| Text::new(build_highlight_text(&session.current_text, span)))
            .unwrap_or_else(|| Text::new("")),
        _ => Text::new(""),
    };
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

fn handle_common_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut party: ResMut<PartyState>,
    mut town: ResMut<TownState>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut msg_events: EventWriter<ShowMessage>,
    mut message_query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
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
    }

    // [Tab]: 注目する仲間切り替え
    if keyboard.just_pressed(KeyCode::Tab) {
        party.selected_index = (party.selected_index + 1) % party.members.len();
        let member = &party.members[party.selected_index];
        msg_events.send(ShowMessage(format!("{}に　ちゅうもくした。", member.name)));
    }

    // [B]: モード切替（街探索 ↔ 戦闘テスト）
    if keyboard.just_pressed(KeyCode::KeyB) {
        *mode = match *mode {
            AppMode::Battle => {
                msg_events.send(ShowMessage(
                    "王都アルカンの街並みへ生還した。\n[WASD]で街を歩き回れる。".into(),
                ));
                AppMode::Town
            }
            _ => {
                msg_events.send(ShowMessage(
                    "地下迷宮の魔物とエンカウントした！\n[1]たたかう や [2]みをまもる で指示を出そう。[B]で街へ戻る。".into(),
                ));
                AppMode::Battle
            }
        };
        dialogue_res.0 = None;
    }
}

fn handle_town_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownState>,
    mut command_menu: ResMut<CommandMenuState>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut travel_state: ResMut<TravelState>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    // [L]: 松明のON/OFF切り替え
    if keyboard.just_pressed(KeyCode::KeyL) {
        town.torch_active = !town.torch_active;
        town.recompute_fov();
        let status = if town.torch_active {
            "松明に火を灯した。周囲が明るくなった！（視界半径6マス）"
        } else {
            "松明の火を消した。月明かりだけが頼りだ……（視界半径2マス）"
        };
        msg_events.send(ShowMessage(status.into()));
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
                    msg_events.send(ShowMessage(msg));
                }
            }
            MoveOutcome::Blocked { message } => {
                if let Some(msg) = message {
                    msg_events.send(ShowMessage(msg));
                }
            }
            MoveOutcome::TriggerBattle { message } => {
                msg_events.send(ShowMessage(message));
                *mode = AppMode::Battle;
                dialogue_res.0 = None;
            }
            MoveOutcome::ChangeArea { message, .. } => {
                msg_events.send(ShowMessage(message));
            }
            MoveOutcome::RequestTravel {
                destination,
                spawn_pos,
            } => {
                travel_state.destination = destination;
                travel_state.spawn_pos = spawn_pos;
                *mode = AppMode::Travel;
                msg_events.send(ShowMessage(format!(
                    "{}へ続く街道だ。どのように進みますか？\n[1]慎重に [2]普通に [3]大胆に（[Esc]でやめる）",
                    destination.name()
                )));
            }
        }
    }

    // [Z]: コマンドウィンドウを開く（ADR-0011: コマンド駆動インタラクト）
    if keyboard.just_pressed(KeyCode::KeyZ) {
        command_menu.stage = CommandMenuStage::ChoosingCommand;
        command_menu.selected_index = 0;
        *mode = AppMode::Interact;
        msg_events.send(ShowMessage("コマンドを選んでください。".into()));
    }
}

fn handle_interact_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownState>,
    mut inv: ResMut<PlayerInventory>,
    mut command_menu: ResMut<CommandMenuState>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    match command_menu.stage {
        CommandMenuStage::ChoosingCommand => {
            let len = CommandKind::ALL.len();
            if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
                command_menu.selected_index = (command_menu.selected_index + len - 1) % len;
            }
            if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
                command_menu.selected_index = (command_menu.selected_index + 1) % len;
            }

            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
                let command = CommandKind::ALL[command_menu.selected_index];
                if command.needs_direction() {
                    let label = command.dynamic_label(town.facing_target_kind());
                    command_menu.stage = CommandMenuStage::ChoosingDirection(command);
                    msg_events.send(ShowMessage(format!("『{}』する方向を選んでください。", label)));
                } else {
                    // 「どうぐ」は方向を選ばず、その場で所持品を確認する
                    let mut msg = format!("【どうぐ】所持金: {}G\n", inv.gold);
                    if inv.items.is_empty() {
                        msg.push_str("何も持っていない。");
                    } else {
                        for item in &inv.items {
                            msg.push_str(&format!("・{}\n", item));
                        }
                    }
                    msg_events.send(ShowMessage(msg));
                    *mode = AppMode::Town;
                }
            }

            if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
                *mode = AppMode::Town;
                msg_events.send(ShowMessage("コマンドをやめた。".into()));
            }
        }
        CommandMenuStage::ChoosingDirection(command) => {
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
                town.face(dx, dy);
                let outcome = town.resolve_interact(command);
                match outcome {
                    InteractOutcome::Message(msg) => {
                        msg_events.send(ShowMessage(msg));
                    }
                    InteractOutcome::ChestOpened { gold, item, message } => {
                        inv.add_gold(gold);
                        inv.add_item(&item);
                        msg_events.send(ShowMessage(message));
                    }
                    InteractOutcome::StartDialogue(partner) => {
                        let session = DialogueSession::start(partner);
                        msg_events.send(ShowMessage(session.current_text.clone()));
                        *mode = match partner {
                            DialoguePartner::Shop => AppMode::Shop,
                            DialoguePartner::Inn => AppMode::Inn,
                            _ => AppMode::Dialogue,
                        };
                        dialogue_res.0 = Some(session);
                    }
                }

                if *mode == AppMode::Interact {
                    *mode = AppMode::Town;
                }
                command_menu.stage = CommandMenuStage::ChoosingCommand;
            }

            if keyboard.just_pressed(KeyCode::Escape) {
                *mode = AppMode::Town;
                command_menu.stage = CommandMenuStage::ChoosingCommand;
                msg_events.send(ShowMessage("コマンドをやめた。".into()));
            }
        }
    }
}

fn handle_dialogue_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut inv: ResMut<PlayerInventory>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    let learn_stage = dialogue_res
        .0
        .as_ref()
        .map(|s| s.learn_stage)
        .unwrap_or_default();

    match learn_stage {
        DialogueLearnStage::Talking => {
            // [W/S]: 話題選択
            if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    session.selected_topic_index = session.selected_topic_index.saturating_sub(1);
                }
            }
            if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    let max_idx = inv.topics.len().saturating_sub(1);
                    if session.selected_topic_index < max_idx {
                        session.selected_topic_index += 1;
                    }
                }
            }

            // [1] / [Enter]: 話題を振る
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    if let Some(topic) = inv.topics.get(session.selected_topic_index).cloned() {
                        session.ask_topic(&topic);
                        msg_events.send(ShowMessage(session.current_text.clone()));
                    }
                }
            }

            // [2]: 「おぼえる」。下線候補の数に応じて挙動が変わる（ADR-0012）。
            if keyboard.just_pressed(KeyCode::Digit2) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    match session.learnable_spans.len() {
                        0 => {
                            msg_events.send(ShowMessage("新しく覚えられるキーワードは見当たらない。".into()));
                        }
                        1 => {
                            let word = session.learnable_spans[0].slice(&session.current_text);
                            let learned = inv.learn_topic(&word);
                            let msg = if learned {
                                format!(
                                    "【{}】を手帳に覚えた！\n（話題リストに追加されました）",
                                    word
                                )
                            } else {
                                format!("【{}】は既に覚えている。", word)
                            };
                            msg_events.send(ShowMessage(msg));
                        }
                        _ => {
                            session.learn_stage =
                                DialogueLearnStage::ChoosingLearnTarget { cursor: 0 };
                        }
                    }
                }
            }

            // [3] / [Esc]: 会話を終える
            if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
                dialogue_res.0 = None;
                *mode = AppMode::Town;
                msg_events.send(ShowMessage("会話を終えて、再び歩き出した。".into()));
            }
        }
        DialogueLearnStage::ChoosingLearnTarget { cursor } => {
            // [W/S]: 下線候補のカーソル移動（本文中の出現順を循環）
            if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    let len = session.learnable_spans.len().max(1);
                    session.learn_stage = DialogueLearnStage::ChoosingLearnTarget {
                        cursor: (cursor + len - 1) % len,
                    };
                }
            }
            if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    let len = session.learnable_spans.len().max(1);
                    session.learn_stage = DialogueLearnStage::ChoosingLearnTarget {
                        cursor: (cursor + 1) % len,
                    };
                }
            }

            // [1] / [Enter]: カーソル位置の候補を確定して覚える
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    if let Some(span) = session.learnable_spans.get(cursor).copied() {
                        let word = span.slice(&session.current_text);
                        let learned = inv.learn_topic(&word);
                        let msg = if learned {
                            format!(
                                "【{}】を手帳に覚えた！\n（話題リストに追加されました）",
                                word
                            )
                        } else {
                            format!("【{}】は既に覚えている。", word)
                        };
                        msg_events.send(ShowMessage(msg));
                    }
                    session.learn_stage = DialogueLearnStage::Talking;
                }
            }

            // [3] / [Esc]: 何も覚えずに選択をやめる（会話自体は終えない）
            if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
                if let Some(session) = dialogue_res.0.as_mut() {
                    session.learn_stage = DialogueLearnStage::Talking;
                }
            }
        }
    }
}

fn handle_shop_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut inv: ResMut<PlayerInventory>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    // [W/S]: 商品選択
    if keyboard.just_pressed(KeyCode::KeyW) || keyboard.just_pressed(KeyCode::ArrowUp) {
        if let Some(session) = dialogue_res.0.as_mut() {
            session.selected_shop_index = session.selected_shop_index.saturating_sub(1);
            let item = &SHOP_ITEMS[session.selected_shop_index];
            msg_events.send(ShowMessage(format!(
                "【{}】({}G)\n{}",
                item.name, item.price, item.description
            )));
        }
    }
    if keyboard.just_pressed(KeyCode::KeyS) || keyboard.just_pressed(KeyCode::ArrowDown) {
        if let Some(session) = dialogue_res.0.as_mut() {
            if session.selected_shop_index + 1 < SHOP_ITEMS.len() {
                session.selected_shop_index += 1;
                let item = &SHOP_ITEMS[session.selected_shop_index];
                msg_events.send(ShowMessage(format!(
                    "【{}】({}G)\n{}",
                    item.name, item.price, item.description
                )));
            }
        }
    }

    // [1] / [Enter]: 商品購入
    if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
        if let Some(session) = dialogue_res.0.as_mut() {
            session.buy_item(&mut inv);
            msg_events.send(ShowMessage(session.current_text.clone()));
        }
    }

    // [3] / [Esc]: 店を出る
    if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
        dialogue_res.0 = None;
        *mode = AppMode::Town;
        msg_events.send(ShowMessage("道具屋を出た。".into()));
    }
}

fn handle_inn_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut party: ResMut<PartyState>,
    mut inv: ResMut<PlayerInventory>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    // [1] / [Enter]: 宿泊
    if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
        if let Some(session) = dialogue_res.0.as_mut() {
            session.rest_at_inn(&mut inv, &mut party.members);
            msg_events.send(ShowMessage(session.current_text.clone()));
        }
    }

    // [3] / [Esc]: 宿を出る
    if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Escape) {
        dialogue_res.0 = None;
        *mode = AppMode::Town;
        msg_events.send(ShowMessage("宿屋を出た。".into()));
    }
}

fn handle_battle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    party: Res<PartyState>,
    player: Res<PlayerResource>,
    mut battle: ResMut<BattleState>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    let mut rng = thread_rng();

    // [N]: モンスター切り替え（テスト用）
    if keyboard.just_pressed(KeyCode::KeyN) {
        battle.next_monster();
        let mon = battle.current_monster();
        msg_events.send(ShowMessage(format!("あらたな　魔物【{}】が　あらわれた！", mon.name)));
    }

    // [1]: 「たたかう」
    if keyboard.just_pressed(KeyCode::Digit1) {
        let member = &party.members[party.selected_index];
        let outcome = evaluate_command(member, PartyCommand::Attack, &mut rng);
        match outcome {
            ActionOutcome::Obeyed { action_msg } => {
                let damage: i32 = rng.gen_range(8..=14);
                let (mon_name, is_dead) = battle.apply_damage(damage);

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
                msg_events.send(ShowMessage(msg));
            }
            ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                if member.personality == Personality::Yandere {
                    let damage: i32 = rng.gen_range(16..=22);
                    let (mon_name, is_dead) = battle.apply_damage(damage);

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
                    msg_events.send(ShowMessage(msg));
                } else {
                    msg_events.send(ShowMessage(format!(
                        "{}に「たたかう」よう　指示した！\n{}\n{}",
                        member.name, reason_msg, action_msg
                    )));
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
        msg_events.send(ShowMessage(msg));
    }

    // [3]: 「すてみ」
    if keyboard.just_pressed(KeyCode::Digit3) {
        let member = &party.members[party.selected_index];
        let outcome = evaluate_command(member, PartyCommand::DesperateAttack, &mut rng);
        match outcome {
            ActionOutcome::Obeyed { action_msg } => {
                let damage: i32 = rng.gen_range(25..=35);
                let (mon_name, is_dead) = battle.apply_damage(damage);

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
                msg_events.send(ShowMessage(msg));
            }
            ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                msg_events.send(ShowMessage(format!(
                    "{}に「すてみ」を　命じた！\n{}\n{}",
                    member.name, reason_msg, action_msg
                )));
            }
        };
    }

    // [4]: 「観察する」
    if keyboard.just_pressed(KeyCode::Digit4) {
        let member = &party.members[party.selected_index];
        let report = diagnose_member(&player.skills, member, &mut rng);
        let msg = format!("{}\n{}", report.observation_msg, report.conclusion_msg);
        msg_events.send(ShowMessage(msg));
    }
}

fn handle_travel_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownState>,
    mut travel_state: ResMut<TravelState>,
    event_registry: Res<SuddenEventRegistry>,
    mut event_history: ResMut<SuddenEventHistory>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    // ADR-0004: 移動姿勢（慎重に／普通に／大胆に）を選択し、旅シミュレーションを実行する。
    // ADR-0013の発生エンジンで道中の突発イベントを2段階抽選する。
    let posture = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(TravelPosture::Cautious)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(TravelPosture::Normal)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(TravelPosture::Bold)
    } else {
        None
    };

    if let Some(posture) = posture {
        let mut rng = thread_rng();
        let triggered = try_trigger_sudden_event(
            &event_registry,
            &mut event_history,
            posture.base_probability(),
            &mut rng,
        );

        let destination = travel_state.destination;
        let spawn_pos = travel_state.spawn_pos;

        let mut msg = format!(
            "{}進み、{}へ向かった。\n",
            posture.label(),
            destination.name()
        );
        match triggered {
            Some(evt) => msg.push_str(evt.message),
            None => msg.push_str("道中、特に何も起こらなかった。"),
        }

        town.switch_area(destination, spawn_pos);

        match triggered.map(|evt| evt.category) {
            Some(SuddenEventCategory::Bandit) | Some(SuddenEventCategory::WildAnimal) => {
                *mode = AppMode::Battle;
                dialogue_res.0 = None;
            }
            _ => {
                *mode = AppMode::Town;
            }
        }

        msg_events.send(ShowMessage(msg));
    } else if keyboard.just_pressed(KeyCode::Escape) {
        *mode = AppMode::Town;
        msg_events.send(ShowMessage("街道を進むのをやめた。".into()));
    }
}

fn update_message_window(
    mut events: EventReader<ShowMessage>,
    mut query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
    for event in events.read() {
        if let Ok((mut type_msg, mut text)) = query.get_single_mut() {
            type_msg.full_text = event.0.clone();
            type_msg.shown_chars = 0;
            type_msg.timer.reset();
            *text = Text::new("");
        }
    }
}

fn update_town_texture_system(
    mut town: ResMut<TownState>,
    mut images: ResMut<Assets<Image>>,
) {
    town.update_texture(&mut images);
}

fn update_center_window_visibility_system(
    mode: Res<AppMode>,
    mut town_image_query: Query<&mut Node, (With<TownMapImageNode>, Without<BattleMonsterTextNode>)>,
    mut battle_monster_query: Query<&mut Node, (With<BattleMonsterTextNode>, Without<TownMapImageNode>)>,
) {
    if !mode.is_changed() {
        return;
    }
    if let Ok(mut town_node) = town_image_query.get_single_mut() {
        town_node.display = match *mode {
            AppMode::Battle => Display::None,
            _ => Display::Flex,
        };
    }
    if let Ok(mut battle_node) = battle_monster_query.get_single_mut() {
        battle_node.display = match *mode {
            AppMode::Battle => Display::Flex,
            _ => Display::None,
        };
    }
}

fn update_battle_monster_display_system(
    mode: Res<AppMode>,
    battle: Res<BattleState>,
    mut query: Query<&mut Text, With<BattleMonsterTextNode>>,
) {
    if *mode != AppMode::Battle {
        return;
    }
    if mode.is_changed() || battle.is_changed() {
        if let Ok(mut text) = query.get_single_mut() {
            *text = Text::new(format_monster_display(battle.current_monster()));
        }
    }
}

fn update_status_header_system(
    party: Res<PartyState>,
    mode: Res<AppMode>,
    town: Res<TownState>,
    inv: Res<PlayerInventory>,
    dialogue: Res<ActiveDialogue>,
    command_menu: Res<CommandMenuState>,
    travel: Res<TravelState>,
    mut query: Query<&mut Text, With<StatusHeaderNode>>,
) {
    if party.is_changed()
        || mode.is_changed()
        || town.is_changed()
        || inv.is_changed()
        || dialogue.is_changed()
        || command_menu.is_changed()
        || travel.is_changed()
    {
        if let Ok(mut text) = query.get_single_mut() {
            *text = Text::new(format_status_header(
                &party,
                *mode,
                &town,
                &inv,
                &dialogue.0,
                &command_menu,
                &travel,
            ));
        }
    }
}

fn update_tri_split_windows_system(
    mode: Res<AppMode>,
    inv: Res<PlayerInventory>,
    dialogue: Res<ActiveDialogue>,
    command_menu: Res<CommandMenuState>,
    town: Res<TownState>,
    travel: Res<TravelState>,
    mut left_query: Query<&mut Text, (With<LeftWindowTextNode>, Without<RightWindowTextNode>)>,
    mut right_query: Query<&mut Text, (With<RightWindowTextNode>, Without<LeftWindowTextNode>)>,
) {
    if mode.is_changed()
        || inv.is_changed()
        || dialogue.is_changed()
        || command_menu.is_changed()
        || town.is_changed()
        || travel.is_changed()
    {
        let facing_target = town.facing_target_kind();
        if let Ok(mut text) = left_query.get_single_mut() {
            *text = Text::new(format_left_window(
                *mode,
                &inv,
                &dialogue.0,
                &command_menu,
                facing_target,
                &travel,
            ));
        }
        if let Ok(mut text) = right_query.get_single_mut() {
            *text = Text::new(format_right_window(*mode, &dialogue.0, &command_menu));
        }
    }
}
