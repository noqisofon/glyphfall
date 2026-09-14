use bevy::prelude::*;
use rand::{thread_rng, Rng};

mod battle;
mod party;
mod town;

use battle::{create_default_monsters, BattleState, Monster};
use party::{
    diagnose_member, evaluate_command, ActionOutcome, Influence, MentalState, PartyCommand,
    PartyMember, Personality, PlayerSkills,
};
use town::{MoveOutcome, TownState};

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

const FONT_PATH: &str = "fonts/ZenKakuGothicNew-Regular.ttf";
const CELL_PX: f32 = 18.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Resource)]
pub enum AppMode {
    Town,
    Battle,
}

#[derive(Component)]
struct TypewriterMessage {
    full_text: String,
    shown_chars: usize,
    timer: Timer,
}

#[derive(Component)]
struct MessageTextNode;

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Glyphfall - 2D Pixel Art Town & Battle Prototype".into(),
                resolution: (960.0_f32, 640.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(palette::BG))
        .insert_resource(AppMode::Town)
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
    mode: Res<AppMode>,
) {
    let font = asset_server.load(FONT_PATH);
    let town = TownState::new(&mut images);

    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        })
        .with_children(|root| {
            // 上部：ステータス＆操作ガイドウィンドウ (行数 4)
            spawn_status_window(root, font.clone(), &party, *mode, &town);

            // 中央：ドット絵街マップ or 敵モンスターのウィンドウ (行数 9)
            spawn_center_window(root, font.clone(), &town);

            // 下部：メッセージウィンドウ (行数 5)
            spawn_message_window(
                root,
                font.clone(),
                46,
                5,
                "【王都アルカン 商業区】\n夜の冷たい風が石畳を抜けていく。[WASD]で移動し仲間と街を探索しよう。\n[L]松明切替 | [B]地下迷宮(戦闘) | [Tab]仲間切替",
            );
        });

    commands.insert_resource(town);
}

fn format_status_header(party: &PartyState, mode: AppMode, town: &TownState) -> String {
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
        match mode {
            AppMode::Town => {
                let torch_str = if town.torch_active { "点灯中(半径6)" } else { "消灯(半径2)" };
                header.push_str(&format!(
                    "  [王都アルカン・商業区] 松明: {} | 仲間列: 主人公(青) 戦士(赤) 遊び人(黄) 魔法使い(紫) 騎士(白)\n",
                    torch_str
                ));
            }
            AppMode::Battle => {
                header.push_str("  [地下封鎖迷宮・戦闘交戦中] (F1キーで開発用隠しパラメータを表示)\n");
            }
        }
    }

    match mode {
        AppMode::Town => {
            header.push_str("操作: [WASD]移動 | [L]松明切替 | [B]地下迷宮(戦闘) | [Tab]仲間切替 | [Space]スキップ");
        }
        AppMode::Battle => {
            header.push_str("操作: [1]たたかう | [2]みをまもる | [3]すてみ | [4]観察 | [N]敵切替 | [B]街へ帰還");
        }
    }

    header
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
            margin: UiRect::bottom(Val::Px(4.0)),
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
                Text::new(format_status_header(party, mode, town)),
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
            margin: UiRect::vertical(Val::Px(4.0)),
            ..default()
        })
        .with_children(|grid| {
            spawn_box_border(grid, font.clone(), outer_cols, outer_rows);

            // ドット絵街マップ描画ノード（736×144px）
            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    width: Val::Px(736.0),
                    height: Val::Px(144.0),
                    align_self: AlignSelf::Center,
                    justify_self: JustifySelf::Center,
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

fn spawn_message_window(
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
            margin: UiRect::top(Val::Px(4.0)),
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
                    timer: Timer::from_seconds(0.025, TimerMode::Repeating),
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
    mut battle: ResMut<BattleState>,
    player: Res<PlayerResource>,
    mut header_query: Query<&mut Text, (With<StatusHeaderNode>, Without<MessageTextNode>, Without<BattleMonsterTextNode>)>,
    mut battle_monster_query: Query<(&mut Text, &mut Node), (With<BattleMonsterTextNode>, Without<MessageTextNode>, Without<StatusHeaderNode>, Without<TownMapImageNode>)>,
    mut town_image_query: Query<&mut Node, (With<TownMapImageNode>, Without<BattleMonsterTextNode>)>,
    mut message_query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
    let mut rng = thread_rng();
    let mut new_message = None;
    let mut update_header = false;
    let mut mode_changed = false;
    let mut update_monster_display = false;

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
            AppMode::Town => {
                new_message = Some("地下迷宮の魔物とエンカウントした！\n[1]たたかう や [2]みをまもる で指示を出そう。[B]で街へ戻る。".into());
                AppMode::Battle
            }
            AppMode::Battle => {
                new_message = Some("王都アルカンの街並みへ生還した。\n[WASD]で街を歩き回れる。".into());
                AppMode::Town
            }
        };
        update_header = true;
        mode_changed = true;
        update_monster_display = true;
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
                        mode_changed = true;
                        update_header = true;
                        update_monster_display = true;
                    }
                }
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

    // モード切替に伴うノード表示・非表示の更新
    if mode_changed {
        if let Ok(mut town_node) = town_image_query.get_single_mut() {
            town_node.display = match *mode {
                AppMode::Town => Display::Flex,
                AppMode::Battle => Display::None,
            };
        }
        if let Ok((_, mut battle_node)) = battle_monster_query.get_single_mut() {
            battle_node.display = match *mode {
                AppMode::Town => Display::None,
                AppMode::Battle => Display::Flex,
            };
        }
    }

    // 戦闘モンスター表示の更新
    if update_monster_display && *mode == AppMode::Battle {
        if let Ok((mut text, _)) = battle_monster_query.get_single_mut() {
            *text = Text::new(format_monster_display(battle.current_monster()));
        }
    }

    // ヘッダーUIの更新
    if update_header {
        if let Ok(mut text) = header_query.get_single_mut() {
            *text = Text::new(format_status_header(&party, *mode, &town));
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
