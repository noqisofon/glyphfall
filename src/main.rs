use bevy::prelude::*;
use rand::thread_rng;

mod party;
use party::{
    diagnose_member, evaluate_command, ActionOutcome, Influence, MentalState, PartyCommand,
    PartyMember, Personality, PlayerSkills,
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
}

const FONT_PATH: &str = "fonts/ZenKakuGothicNew-Regular.ttf";
const CELL_PX: f32 = 18.0;


// メッセージボックスの「1文字ずつ送り」演出用ステート
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
                title: "Glyphfall - Party & Influence Prototype".into(),
                resolution: (960.0_f32, 640.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(palette::BG))
        .insert_resource(PlayerResource {
            skills: PlayerSkills {
                magic_knowledge: 45,
                keen_eye: 55,
            },
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
        .add_systems(Update, (typewriter_tick, handle_input))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, party: Res<PartyState>) {
    let font = asset_server.load(FONT_PATH);

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
            // 上部：ステータス＆操作ガイドウィンドウ
            spawn_status_window(root, font.clone(), &party);

            // 下部：メッセージウィンドウ
            spawn_message_window(
                root,
                font.clone(),
                46,
                6,
                "パーティのなかまが　あなたの指示を待っている。\n[Tab]で仲間を選び、[1]〜[4]で指示や観察を行おう。",
            );
        });
}

fn format_status_header(party: &PartyState) -> String {
    let member = &party.members[party.selected_index];
    let mut header = format!(
        "▼ 選択中の仲間 [No.{}/{}]: {} ({})  HP: {}/{}  MP: {}/{}\n",
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
            "  [DEBUG] 隠し影響度: {} {} | 精神: {} | 性格: {}\n",
            member.influence.raw_value, abnormal_str, mental_str, personality_str
        ));
    } else {
        header.push_str("  [ステータス: 良好] (F1キーで開発用隠しパラメータを表示)\n");
    }

    header.push_str("操作: [Tab]仲間切替 | [1]たたかう | [2]みをまもる | [3]すてみ | [4]観察(目星+魔術) | [Space]スキップ");
    header
}


fn spawn_status_window(parent: &mut ChildBuilder, font: Handle<Font>, party: &PartyState) {
    let inner_cols = 46;
    let inner_rows = 4;
    let outer_cols = inner_cols + 2;
    let outer_rows = inner_rows + 2;

    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::px(outer_cols as u16, CELL_PX)],
            grid_template_rows: vec![RepeatedGridTrack::px(outer_rows as u16, CELL_PX)],
            margin: UiRect::bottom(Val::Px(8.0)),
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
                Text::new(format_status_header(party)),
                TextFont {
                    font,
                    font_size: CELL_PX * 0.85,
                    ..default()
                },
                TextColor(palette::TEXT_HIGHLIGHT),
                StatusHeaderNode,
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
                    font_size: CELL_PX * 0.85,
                    ..default()
                },
                TextColor(palette::TEXT),
                MessageTextNode,
                TypewriterMessage {
                    full_text: message.to_string(),
                    shown_chars: 0,
                    timer: Timer::from_seconds(0.03, TimerMode::Repeating),
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

fn handle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut party: ResMut<PartyState>,
    player: Res<PlayerResource>,
    mut header_query: Query<&mut Text, (With<StatusHeaderNode>, Without<MessageTextNode>)>,
    mut message_query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
    let mut rng = thread_rng();
    let mut new_message = None;
    let mut update_header = false;

    // [Space]: タイプライターの文字送りをスキップ
    if keyboard.just_pressed(KeyCode::Space) {
        if let Ok((mut type_msg, mut text)) = message_query.get_single_mut() {
            type_msg.shown_chars = type_msg.full_text.chars().count();
            *text = Text::new(type_msg.full_text.clone());
        }
    }

    // [F1]: デバッグ表示切替
    if keyboard.just_pressed(KeyCode::F1) {
        party.debug_mode = !party.debug_mode;
        update_header = true;
    }

    // [Tab]: 仲間切り替え
    if keyboard.just_pressed(KeyCode::Tab) {
        party.selected_index = (party.selected_index + 1) % party.members.len();
        let member = &party.members[party.selected_index];
        new_message = Some(format!("{}に　ちゅうもくした。", member.name));
        update_header = true;
    }

    // [1]: 「たたかう」
    if keyboard.just_pressed(KeyCode::Digit1) {
        let member = &party.members[party.selected_index];
        let outcome = evaluate_command(member, PartyCommand::Attack, &mut rng);
        let msg = match outcome {
            ActionOutcome::Obeyed { action_msg } => {
                format!("{}に「たたかう」よう　指示した。\n{}", member.name, action_msg)
            }
            ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                format!(
                    "{}に「たたかう」よう　指示した！\n{}\n{}",
                    member.name, reason_msg, action_msg
                )
            }
        };
        new_message = Some(msg);
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

    // [3]: 「すてみ」（理不尽・危険な命令）
    if keyboard.just_pressed(KeyCode::Digit3) {
        let member = &party.members[party.selected_index];
        let outcome = evaluate_command(member, PartyCommand::DesperateAttack, &mut rng);
        let msg = match outcome {
            ActionOutcome::Obeyed { action_msg } => {
                format!("{}に「すてみ」を　命じた！\n{}", member.name, action_msg)
            }
            ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                format!(
                    "{}に「すてみ」を　命じた！\n{}\n{}",
                    member.name, reason_msg, action_msg
                )
            }
        };
        new_message = Some(msg);
    }

    // [4]: 「観察する（魔術知識＋目星）」
    if keyboard.just_pressed(KeyCode::Digit4) {
        let member = &party.members[party.selected_index];
        let report = diagnose_member(&player.skills, member, &mut rng);
        let msg = format!("{}\n{}", report.observation_msg, report.conclusion_msg);
        new_message = Some(msg);
    }


    // ヘッダーUIの更新
    if update_header {
        if let Ok(mut text) = header_query.get_single_mut() {
            *text = Text::new(format_status_header(&party));
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
