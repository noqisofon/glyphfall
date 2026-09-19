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
pub use town::{AreaId, CommandKind, DialogueSession, Position, TownStateRes, SHOP_ITEMS};
pub use ui::palette;

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
pub enum TravelPosture {
    Cautious,
    Normal,
    Bold,
}

impl TravelPosture {
    pub fn label(&self) -> &'static str {
        match self {
            TravelPosture::Cautious => "慎重に",
            TravelPosture::Normal => "普通に",
            TravelPosture::Bold => "大胆に",
        }
    }

    /// ADR-0004の姿勢別エンカウント率表に対応する基礎確率。
    pub fn base_probability(&self) -> f32 {
        match self {
            TravelPosture::Cautious => 0.15,
            TravelPosture::Normal => 0.35,
            TravelPosture::Bold => 0.65,
        }
    }
}

/// 街道口に接触してから移動姿勢が確定するまでの間、行き先を保持しておくための状態。
#[derive(Resource)]
pub struct TravelState {
    pub destination: AreaId,
    pub spawn_pos: Position,
}

impl Default for TravelState {
    fn default() -> Self {
        Self {
            destination: AreaId::Town,
            spawn_pos: Position { x: 0, y: 0 },
        }
    }
}

pub fn in_mode(target: AppMode) -> impl Fn(Res<AppMode>) -> bool {
    move |mode: Res<AppMode>| *mode == target
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
        .add_systems(OnEnter(AppScreen::Playing), ui::setup_playing_screen)
        .add_systems(
            Update,
            (
                (
                    ui::typewriter_tick,
                    ui::dialogue_underline_tick,
                    ui::monster_flash_tick,
                ),
                (
                    handle_common_input,
                    town::handle_town_input.run_if(in_mode(AppMode::Town)),
                    town::handle_interact_input.run_if(in_mode(AppMode::Interact)),
                    town::handle_dialogue_input.run_if(in_mode(AppMode::Dialogue)),
                    town::handle_shop_input.run_if(in_mode(AppMode::Shop)),
                    town::handle_inn_input.run_if(in_mode(AppMode::Inn)),
                    battle::handle_battle_input.run_if(in_mode(AppMode::Battle)),
                    town::handle_travel_input.run_if(in_mode(AppMode::Travel)),
                ),
                (ui::update_message_window, ui::update_town_texture_system),
                (
                    ui::update_status_header_system,
                    ui::update_tri_split_windows_system,
                    ui::update_battle_monster_display_system,
                    ui::update_center_window_visibility_system,
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
}
