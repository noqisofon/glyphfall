use bevy::prelude::*;

// ─────────────────────────────────────────────
// 罫線文字セット（単線）。将来的に二重線・太線バリエーションを
// 増やすならここに enum Style { Single, Double, Bold } を足す形。
// ─────────────────────────────────────────────
mod box_chars {
    pub const TOP_LEFT: char = '┌';
    pub const TOP_RIGHT: char = '┐';
    pub const BOTTOM_LEFT: char = '└';
    pub const BOTTOM_RIGHT: char = '┘';
    pub const HORIZONTAL: char = '─';
    pub const VERTICAL: char = '│';
}

// ターミナル風パレット。まずは緑モノクロ（琥珀にしたければ ✏️ で変更）。
mod palette {
    use bevy::prelude::Color;
    pub const BG: Color = Color::srgb(0.02, 0.02, 0.02);
    pub const FRAME: Color = Color::srgb(0.15, 0.85, 0.35);
    pub const TEXT: Color = Color::srgb(0.55, 1.0, 0.65);
    pub const TEXT_DIM: Color = Color::srgb(0.15, 0.45, 0.2);
}

const FONT_PATH: &str = "fonts/FiraMono-Regular.ttf"; // 手元の等幅フォントに差し替え可
const CELL_PX: f32 = 20.0; // 1文字セルの幅・高さの目安（フォントサイズと連動）

#[derive(Resource)]
struct TerminalFont(Handle<Font>);

// メッセージボックスの「1文字ずつ送り」演出用ステート
#[derive(Component)]
struct TypewriterMessage {
    full_text: String,
    shown_chars: usize,
    timer: Timer,
}

#[derive(Component)]
struct MessageTextNode;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Glyphfall".into(),
                resolution: (960.0, 640.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(palette::BG))
        .add_systems(Startup, setup)
        .add_systems(Update, typewriter_tick)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(FONT_PATH);
    commands.insert_resource(TerminalFont(font.clone()));

    commands.spawn(Camera2d);

    // 画面全体のルート。下部にメッセージウィンドウを固定配置する
    // DQ定番レイアウトの雛形（会話メモの「三分割」構想にも将来対応しやすい形）
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexEnd,
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        })
        .with_children(|root| {
            spawn_message_window(
                root,
                font.clone(),
                40, // 内側の横セル数（文字数の目安）
                5,  // 内側の縦セル数（行数）
                "せんしは　いてつくはどうを　うけた！\nさっきから　しゃべりかたが　へんだ　わるいむしでも　いるのでは。",
            );
        });
}

/// 罫線文字で囲んだウィンドウ枠 + 内側に等幅テキストを1つ生成する。
/// inner_cols / inner_rows は「中身のテキストエリア」の目安セル数。
/// 枠線ぶんは呼び出し側で気にしなくていいようにここで+2している。
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
                        _ => None, // 内側セル（テキストエリア）
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

            // テキストエリア本体：内側セル全体を1つのText領域として重ねる
            // （文字送り演出をする都合上、1セル1エンティティより
            //  複数行テキストノード1つの方が扱いやすい）
            grid.spawn((
                Node {
                    grid_column: GridPlacement::start_span(2, outer_cols as u16 - 2),
                    grid_row: GridPlacement::start_span(2, outer_rows as u16 - 2),
                    padding: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                Text::new(""),
                TextFont {
                    font,
                    font_size: CELL_PX * 0.8,
                    ..default()
                },
                TextColor(palette::TEXT),
                MessageTextNode,
                TypewriterMessage {
                    full_text: message.to_string(),
                    shown_chars: 0,
                    timer: Timer::from_seconds(0.04, TimerMode::Repeating),
                },
            ));
        });
}

/// DQ的な「1文字ずつメッセージが送られてくる」演出。
/// ターミナルUIとの相性がいいので最初から入れておく。
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
