use bevy::prelude::*;

mod battle;
mod event;
mod flow;
mod macros;
mod party;
mod town;
mod ui;

use battle::{create_default_monsters, BattleState, BattleStateRes};
use event::{SuddenEventHistoryRes, SuddenEventRegistryRes};
use flow::AppScreen;
pub use party::{
    Influence, MentalState, PartyMember, PartyStateRes, Personality, PlayerInventoryRes,
    PlayerResourceRes, PlayerSkills, ReserveRosterRes,
};
pub use town::{
    AreaId, CommandKind, DialogueSession, Position, TownStateRes, TravelPhase, TravelPlan,
    TravelPosture, TravelSimulation, TravelStepOutcome, TravelTransport, SHOP_ITEMS,
};
pub use ui::palette;

/// デバッグ用キー（戦闘開始[B]・戦闘離脱[B]・敵切替[N]）を有効にするか。
/// リリースビルドでは無効（戦闘を無条件に離脱できると「にげる」や敗北が無意味になるため）。
pub const DEBUG_KEYS: bool = cfg!(debug_assertions);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Resource)]
pub enum AppMode {
    Town,
    Interact,
    Dialogue,
    Shop,
    Inn,
    Battle,
    /// ADR-0004/ADR-0013/ADR-0026: 街道口での旅計画および暗転旅シミュレーション中。
    Travel,
}

/// 街道口に接触してから移動姿勢が確定するまでの間、行き先を保持しておくための状態。
#[derive(Resource)]
pub struct TravelState {
    pub sim: Option<TravelSimulation>,
    pub destination: AreaId,
    pub spawn_pos: Position,
    pub step_timer: Timer,
}

impl Default for TravelState {
    fn default() -> Self {
        Self {
            sim: None,
            destination: AreaId::default(),
            spawn_pos: Position::default(),
            step_timer: Timer::new(std::time::Duration::from_millis(800), TimerMode::Repeating),
        }
    }
}

/// フレーム開始時点の`AppMode`のスナップショット。
///
/// 入力ハンドラは同じフレーム内で順に走るため、先行ハンドラがモードを切り替えると、
/// 後続ハンドラが同じキー入力を新しいモードの操作として拾ってしまう
/// （例: 話しかけの方向キー[S]が、遷移直後の道具屋で商品カーソル移動としても処理される）。
/// `in_mode`はこのスナップショットと現在値の両方が一致するときだけ真にして、
/// 遷移したフレームでは新モードのハンドラを動かさない。
#[derive(Resource, Clone, Copy, Debug)]
pub struct FrameMode(pub AppMode);

pub fn snapshot_frame_mode(mode: Res<AppMode>, mut frame_mode: ResMut<FrameMode>) {
    if frame_mode.0 != *mode {
        frame_mode.0 = *mode;
    }
}

pub fn in_mode(target: AppMode) -> impl Fn(Res<AppMode>, Res<FrameMode>) -> bool {
    move |mode: Res<AppMode>, frame_mode: Res<FrameMode>| *mode == target && frame_mode.0 == target
}

#[derive(Event, Debug, Clone)]
pub struct ShowMessage(pub String);

/// ADR-0011: コマンド駆動インタラクトの進行段階。
/// 「どうぐ」以外はコマンド決定後に方向選択へ遷移する。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CommandMenuStage {
    #[default]
    ChoosingCommand,
    ChoosingDirection(CommandKind),
}

#[derive(Resource, Default)]
pub struct CommandMenuState {
    pub stage: CommandMenuStage,
    pub selected_index: usize,
}

#[derive(Resource, Default)]
pub struct ActiveDialogue(pub Option<DialogueSession>);

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
        .add_plugins(flow::screens_plugin)
        .insert_resource(AppMode::Town)
        .insert_resource(FrameMode(AppMode::Town))
        .insert_resource(PlayerInventoryRes::default())
        .insert_resource(ActiveDialogue::default())
        .insert_resource(CommandMenuState::default())
        .insert_resource(TravelState::default())
        .insert_resource(SuddenEventRegistryRes(
            glyphfall_core::event::SuddenEventRegistry::travel_default(),
        ))
        .insert_resource(SuddenEventHistoryRes::default())
        .insert_resource(PlayerResourceRes(glyphfall_core::party::PlayerResource {
            skills: PlayerSkills {
                magic_knowledge: 45,
                keen_eye: 55,
            },
        }))
        .insert_resource(BattleStateRes(BattleState::new(create_default_monsters())))
        .insert_resource(ReserveRosterRes(glyphfall_core::party::ReserveRoster::new(
            vec![PartyMember::new(
                "騎士アルヴィン",
                "きし",
                Influence::new_supernatural(115), // 魅了による異常値
                MentalState::Charmed,
                Personality::Loyal,
            )
            .with_stats(40, 15)],
        )))
        .insert_resource(PartyStateRes(glyphfall_core::party::PartyState {
            selected_index: 0,
            debug_mode: false,
            members: vec![
                PartyMember::new_player("あなた").with_stats(20, 0),
                PartyMember::new(
                    "戦士ガルツ",
                    "せんし",
                    Influence::new_natural(82),
                    MentalState::Normal,
                    Personality::Loyal,
                )
                .with_stats(35, 10),
                PartyMember::new(
                    "遊び人ロロ",
                    "あそびにん",
                    Influence::new_natural(45),
                    MentalState::Normal,
                    Personality::Slacker,
                )
                .with_stats(25, 10),
                PartyMember::new(
                    "魔法使いミレイ",
                    "まほうつかい",
                    Influence::new_natural(88), // 自然上限寸前の素の執着
                    MentalState::Normal,
                    Personality::Yandere,
                )
                .with_stats(22, 25),
            ],
        }))
        .add_event::<ShowMessage>()
        .add_systems(Startup, spawn_camera)
        .add_systems(Update, ui::update_ui_scale_system)
        .add_systems(OnEnter(AppScreen::Playing), ui::setup_playing_screen)
        .add_systems(
            Update,
            (
                (
                    ui::typewriter_tick,
                    ui::dialogue_underline_tick,
                    ui::monster_flash_tick,
                ),
                // 入力ハンドラは実行順を固定（chain）し、フレーム開始時のモードで
                // 振り分ける（`FrameMode`参照）。順序が曖昧だと同一フレームのキー入力が
                // 遷移先ハンドラにも漏れ、実行順次第で挙動が変わってしまう。
                (
                    snapshot_frame_mode,
                    handle_common_input,
                    town::handle_town_input.run_if(in_mode(AppMode::Town)),
                    town::handle_interact_input.run_if(in_mode(AppMode::Interact)),
                    town::handle_dialogue_input.run_if(in_mode(AppMode::Dialogue)),
                    town::handle_shop_input.run_if(in_mode(AppMode::Shop)),
                    town::handle_inn_input.run_if(in_mode(AppMode::Inn)),
                    battle::handle_battle_input.run_if(in_mode(AppMode::Battle)),
                    town::handle_travel_input.run_if(in_mode(AppMode::Travel)),
                )
                    .chain(),
                (ui::update_message_window, ui::update_town_texture_system),
                (
                    ui::update_status_header_system,
                    ui::update_tri_split_windows_system,
                    ui::update_battle_monster_display_system,
                    ui::update_travel_simulation_display_system,
                    ui::update_center_window_visibility_system,
                    ui::update_bottom_window_layout_system,
                ),
            )
                .chain()
                .run_if(in_state(AppScreen::Playing)),
        )
        .run();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn handle_common_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut party: ResMut<PartyStateRes>,
    mut town: ResMut<TownStateRes>,
    mut msg_events: EventWriter<ShowMessage>,
    mut message_query: Query<(&mut ui::TypewriterMessage, &mut Text), With<ui::MessageTextNode>>,
) {
    // [Space]: タイプライターの文字送りをスキップ
    if keyboard.just_pressed(KeyCode::Space) {
        for (mut type_msg, mut text) in &mut message_query {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use glyphfall_core::party::{PartyState, PlayerInventory, PlayerResource};
    use glyphfall_core::town::{TileType, TownState};

    fn input_app(mode: AppMode) -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<Time>();
        app.insert_resource(mode);
        app.insert_resource(FrameMode(mode));
        app.insert_resource(TownStateRes {
            state: TownState::new(),
            texture_handle: Handle::default(),
        });
        app.insert_resource(PlayerInventoryRes(PlayerInventory::default()));
        app.insert_resource(ActiveDialogue::default());
        app.insert_resource(CommandMenuState::default());
        app.insert_resource(TravelState::default());
        app.insert_resource(PartyStateRes(PartyState {
            selected_index: 0,
            debug_mode: false,
            members: vec![PartyMember::new_player("テスト旅人").with_stats(20, 0)],
        }));
        app.insert_resource(PlayerResourceRes(PlayerResource {
            skills: PlayerSkills::default(),
        }));
        app.insert_resource(BattleStateRes(BattleState::new(create_default_monsters())));
        app.add_event::<ShowMessage>();
        // 本番(main)と同じ並び・同じ run_if で登録する
        app.add_systems(
            Update,
            (
                snapshot_frame_mode,
                town::handle_town_input.run_if(in_mode(AppMode::Town)),
                town::handle_interact_input.run_if(in_mode(AppMode::Interact)),
                town::handle_shop_input.run_if(in_mode(AppMode::Shop)),
                battle::handle_battle_input.run_if(in_mode(AppMode::Battle)),
            )
                .chain(),
        );
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
    }

    fn drain_messages(app: &mut App) -> Vec<String> {
        app.world_mut()
            .resource_mut::<Events<ShowMessage>>()
            .drain()
            .map(|e| e.0)
            .collect()
    }

    /// 話しかけの方向キー[S]が、遷移直後の道具屋の商品カーソル移動としても
    /// 処理され、挨拶が上書きされていた不具合の回帰テスト。
    #[test]
    fn direction_key_that_opens_shop_is_not_reused_by_shop_handler() {
        let mut app = input_app(AppMode::Interact);
        {
            let mut town = app.world_mut().resource_mut::<TownStateRes>();
            let (x, y) = (town.player_pos.x, town.player_pos.y);
            town.map.set(x, y + 1, TileType::Shop);
        }
        app.world_mut().resource_mut::<CommandMenuState>().stage =
            CommandMenuStage::ChoosingDirection(CommandKind::Talk);

        press(&mut app, KeyCode::KeyS);

        assert_eq!(*app.world().resource::<AppMode>(), AppMode::Shop);
        let index = app
            .world()
            .resource::<ActiveDialogue>()
            .0
            .as_ref()
            .map(|s| s.selected_shop_index);
        assert_eq!(index, Some(0));
        let messages = drain_messages(&mut app);
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(messages[0].contains("へいらっしゃい"), "{messages:?}");
    }

    /// 探索中の[B]でバトルに入ったフレームで、同じ[B]が戦闘離脱として
    /// 処理されないこと。
    #[test]
    fn debug_battle_key_does_not_bounce_back_in_same_frame() {
        let mut app = input_app(AppMode::Town);
        press(&mut app, KeyCode::KeyB);
        assert_eq!(*app.world().resource::<AppMode>(), AppMode::Battle);
    }

    /// 関所の番人・騎士団長の正面で「はなす」を選ぶと、実際に会話モードへ入ること。
    #[test]
    fn talking_to_checkpoint_guard_and_knight_commander_starts_dialogue() {
        use glyphfall_core::town::DialoguePartner;

        for (tile, expected) in [
            (
                TileType::NpcCheckpointGuard,
                DialoguePartner::CheckpointGuard,
            ),
            (
                TileType::NpcKnightCommander,
                DialoguePartner::KnightCommander,
            ),
        ] {
            let mut app = input_app(AppMode::Interact);
            {
                let mut town = app.world_mut().resource_mut::<TownStateRes>();
                let (x, y) = (town.player_pos.x, town.player_pos.y);
                town.map.set(x, y + 1, tile);
            }
            app.world_mut().resource_mut::<CommandMenuState>().stage =
                CommandMenuStage::ChoosingDirection(CommandKind::Talk);

            press(&mut app, KeyCode::KeyS);

            assert_eq!(*app.world().resource::<AppMode>(), AppMode::Dialogue);
            let partner = app
                .world()
                .resource::<ActiveDialogue>()
                .0
                .as_ref()
                .map(|s| s.partner);
            assert_eq!(partner, Some(expected));
        }
    }
}
