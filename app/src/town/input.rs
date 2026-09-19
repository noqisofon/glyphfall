use super::{
    CommandKind, DialogueLearnStage, DialoguePartner, DialogueSession, InteractOutcome,
    MoveOutcome, TownStateRes, TravelPhase, TravelPosture, TravelSimulation, TravelStepOutcome,
    TravelTransport, SHOP_ITEMS,
};
use crate::event::{SuddenEventCategory, SuddenEventHistoryRes, SuddenEventRegistryRes};
use crate::party::{PlayerInventoryRes, CURRENCY_UNIT};
use crate::ActiveDialogue;
use crate::{AppMode, CommandMenuStage, CommandMenuState, PartyStateRes, ShowMessage, TravelState};
use bevy::prelude::*;
use rand::thread_rng;

pub fn handle_town_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownStateRes>,
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
        let mut rng = thread_rng();
        let outcome = town.move_player(dx, dy, &mut rng);

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
            MoveOutcome::TriggerBattle { message, .. } => {
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
                let from_area = town.current_area;
                travel_state.destination = destination;
                travel_state.spawn_pos = spawn_pos;
                travel_state.sim = Some(TravelSimulation::new(from_area, destination, spawn_pos));
                *mode = AppMode::Travel;
                msg_events.send(ShowMessage(format!(
                    "{}へ続く街道口に立った。\n旅の計画を立ててください。",
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

    // [B]: 戦闘テストへ突入
    if keyboard.just_pressed(KeyCode::KeyB) {
        *mode = AppMode::Battle;
        dialogue_res.0 = None;
        msg_events.send(ShowMessage(
            "地下迷宮の魔物とエンカウントした！\nあなたの行動を選択してください。([B]で街へ帰還)"
                .into(),
        ));
    }
}

pub fn handle_interact_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownStateRes>,
    mut inv: ResMut<PlayerInventoryRes>,
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
                    msg_events.send(ShowMessage(format!(
                        "『{}』する方向を選んでください。",
                        label
                    )));
                } else {
                    // 「どうぐ」は方向を選ばず、その場で所持品を確認する
                    let mut msg = format!("【どうぐ】所持金: {}{}\n", inv.gold, CURRENCY_UNIT);
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
            } else if keyboard.just_pressed(KeyCode::KeyS)
                || keyboard.just_pressed(KeyCode::ArrowDown)
            {
                dy += 1;
            } else if keyboard.just_pressed(KeyCode::KeyA)
                || keyboard.just_pressed(KeyCode::ArrowLeft)
            {
                dx -= 1;
            } else if keyboard.just_pressed(KeyCode::KeyD)
                || keyboard.just_pressed(KeyCode::ArrowRight)
            {
                dx += 1;
            }

            if dx != 0 || dy != 0 {
                town.face(dx, dy);
                let outcome = town.resolve_interact(command);
                match outcome {
                    InteractOutcome::Message(msg) => {
                        msg_events.send(ShowMessage(msg));
                    }
                    InteractOutcome::ChestOpened {
                        gold,
                        item,
                        message,
                    } => {
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

pub fn handle_dialogue_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut inv: ResMut<PlayerInventoryRes>,
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
                            msg_events.send(ShowMessage(
                                "新しく覚えられるキーワードは見当たらない。".into(),
                            ));
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

pub fn handle_shop_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut inv: ResMut<PlayerInventoryRes>,
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

pub fn handle_inn_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut party: ResMut<PartyStateRes>,
    mut inv: ResMut<PlayerInventoryRes>,
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

#[allow(clippy::too_many_arguments)]
pub fn handle_travel_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownStateRes>,
    mut travel_state: ResMut<TravelState>,
    mut inv: ResMut<PlayerInventoryRes>,
    event_registry: Res<SuddenEventRegistryRes>,
    mut event_history: ResMut<SuddenEventHistoryRes>,
    mut dialogue_res: ResMut<ActiveDialogue>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    let mut rng = thread_rng();

    let destination = travel_state.destination;
    let spawn_pos = travel_state.spawn_pos;
    let from_area = town.current_area;

    let travel = &mut *travel_state;
    if travel.sim.is_none() {
        travel.sim = Some(TravelSimulation::new(from_area, destination, spawn_pos));
    }
    let timer = &mut travel.step_timer;
    let sim = travel.sim.as_mut().unwrap();

    let mut reset_sim = false;

    match sim.phase {
        TravelPhase::ChoosingDestination => {
            // [1] または [Enter]: すずかけ村を選択して移動姿勢選択へ
            if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Enter) {
                sim.select_destination(destination, spawn_pos);
                msg_events.send(ShowMessage(format!(
                    "{}へ向かう旅を計画します。\n移動姿勢を選択してください。",
                    destination.name()
                )));
            } else if keyboard.just_pressed(KeyCode::Escape) {
                *mode = AppMode::Town;
                reset_sim = true;
                msg_events.send(ShowMessage("街道を進むのをやめた。".into()));
            }
        }
        TravelPhase::ChoosingPosture => {
            // [1] 慎重に, [2] 普通に, [3] 大胆に
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
                sim.select_posture(posture);
                msg_events.send(ShowMessage(format!(
                    "移動姿勢を「{}」に決定した。\n次に移動手段を選択してください。",
                    posture.label()
                )));
            } else if keyboard.just_pressed(KeyCode::Escape) {
                if !sim.back_phase() {
                    *mode = AppMode::Town;
                    reset_sim = true;
                    msg_events.send(ShowMessage("街道を進むのをやめた。".into()));
                } else {
                    msg_events.send(ShowMessage("移動先の選択に戻った。".into()));
                }
            }
        }
        TravelPhase::ChoosingTransport => {
            // [1] 徒歩 (0G), [2] 馬 (40G), [3] 馬車 (80G)
            let transport = if keyboard.just_pressed(KeyCode::Digit1) {
                Some(TravelTransport::Foot)
            } else if keyboard.just_pressed(KeyCode::Digit2) {
                Some(TravelTransport::Horse)
            } else if keyboard.just_pressed(KeyCode::Digit3) {
                Some(TravelTransport::Carriage)
            } else {
                None
            };

            if let Some(transport) = transport {
                let cost = transport.cost();
                if inv.gold < cost {
                    msg_events.send(ShowMessage(format!(
                        "所持金が足りません！（必要: {}{}, 所持: {}{}）",
                        cost, CURRENCY_UNIT, inv.gold, CURRENCY_UNIT
                    )));
                } else {
                    if cost > 0 {
                        inv.spend_gold(cost);
                    }
                    sim.select_transport(transport);
                    timer.reset();
                    let start_msg = sim.start_travel();
                    msg_events.send(ShowMessage(start_msg));
                }
            } else if keyboard.just_pressed(KeyCode::Escape) {
                sim.back_phase();
                msg_events.send(ShowMessage("移動姿勢の選択に戻った。".into()));
            }
        }
        TravelPhase::Traveling => {
            // タイマー自動進行（イベントがなければ自動で進む） ＆ [Space]/[Enter]手動送り
            timer.tick(time.delta());
            let manual_advance =
                keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Enter);
            let timer_advance = timer.just_finished();

            if manual_advance || timer_advance {
                timer.reset();
                let outcome = sim.advance_day(&event_registry, &mut event_history, &mut rng);
                match outcome {
                    TravelStepOutcome::Continued { message, .. } => {
                        msg_events.send(ShowMessage(message));
                    }
                    TravelStepOutcome::EventOccurred { message, .. } => {
                        msg_events.send(ShowMessage(message));
                    }
                    TravelStepOutcome::Arrived { message, .. } => {
                        msg_events.send(ShowMessage(message));
                    }
                }
            }
        }
        TravelPhase::EncounterEvent => {
            // [Space] または [Enter]: イベント解決 / 戦闘突入
            if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Enter) {
                let category = sim.pending_event.as_ref().map(|e| e.category);
                match category {
                    Some(SuddenEventCategory::Bandit) | Some(SuddenEventCategory::WildAnimal) => {
                        // 目的地の街へ切り替えてから戦闘へ突入
                        let dest = sim.plan.destination;
                        let spawn = sim.spawn_pos;
                        town.switch_area(dest, spawn, &mut rng);
                        *mode = AppMode::Battle;
                        dialogue_res.0 = None;
                        reset_sim = true;
                        msg_events.send(ShowMessage(
                            "魔物・山賊が襲いかかってきた！ 武器を構えろ！".into(),
                        ));
                    }
                    _ => {
                        // その他のイベントを解決して旅程再開
                        let outcome = sim.resolve_event_and_continue();
                        match outcome {
                            TravelStepOutcome::Continued { message, .. } => {
                                msg_events.send(ShowMessage(message));
                            }
                            TravelStepOutcome::Arrived { message, .. } => {
                                msg_events.send(ShowMessage(message));
                            }
                            TravelStepOutcome::EventOccurred { message, .. } => {
                                msg_events.send(ShowMessage(message));
                            }
                        }
                    }
                }
            }
        }
        TravelPhase::Arrived => {
            // [Space] または [Enter]: 目的地エリアへ切り替えて探索復帰
            if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Enter) {
                let dest = sim.plan.destination;
                let spawn = sim.spawn_pos;
                town.switch_area(dest, spawn, &mut rng);
                *mode = AppMode::Town;
                reset_sim = true;
                msg_events.send(ShowMessage(format!(
                    "{}に到着した。\n[WASD]で歩き回れる。",
                    dest.name()
                )));
            }
        }
    }

    if reset_sim {
        travel_state.sim = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::PlayerInventory;
    use crate::town::{AreaId, TownState};
    use crate::Position;

    fn create_test_travel_app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<Time>();
        app.insert_resource(AppMode::Travel);
        app.insert_resource(TownStateRes {
            state: TownState::new(),
            texture_handle: Handle::default(),
        });
        let travel_state = TravelState {
            sim: Some(TravelSimulation::new(
                AreaId::Town,
                AreaId::Village,
                Position { x: 20, y: 5 },
            )),
            destination: AreaId::Village,
            spawn_pos: Position { x: 20, y: 5 },
            step_timer: Timer::new(std::time::Duration::from_millis(800), TimerMode::Repeating),
        };
        app.insert_resource(travel_state);
        let inv = PlayerInventory {
            gold: 100,
            ..Default::default()
        };
        app.insert_resource(PlayerInventoryRes(inv));
        // イベント抽選をスキップさせるため空のレジストリを用意
        app.insert_resource(SuddenEventRegistryRes(
            glyphfall_core::event::SuddenEventRegistry { events: Vec::new() },
        ));
        app.init_resource::<SuddenEventHistoryRes>();
        app.insert_resource(ActiveDialogue(None));
        app.add_event::<ShowMessage>();
        app.add_systems(Update, handle_travel_input);
        app
    }

    fn press_key(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
    }

    #[test]
    fn test_travel_cancellation_at_destination_step() {
        let mut app = create_test_travel_app();

        // Esc キーを押して計画をキャンセル
        press_key(&mut app, KeyCode::Escape);

        assert_eq!(*app.world().resource::<AppMode>(), AppMode::Town);
        assert!(app.world().resource::<TravelState>().sim.is_none());
    }

    #[test]
    fn test_travel_three_step_planning_and_arrival_flow() {
        let mut app = create_test_travel_app();

        // Step 1: 移動先選択 ([1] すずかけ村)
        press_key(&mut app, KeyCode::Digit1);

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::ChoosingPosture);
        }

        // Step 2: 移動姿勢選択 ([2] 普通に)
        press_key(&mut app, KeyCode::Digit2);

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::ChoosingTransport);
            assert_eq!(sim.plan.posture, TravelPosture::Normal);
        }

        // Step 3: 移動手段選択 ([3] 乗合馬車, 費用80G)
        press_key(&mut app, KeyCode::Digit3);

        {
            let inv = app.world().resource::<PlayerInventoryRes>();
            assert_eq!(inv.gold, 20); // 100 - 80 = 20G

            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::Traveling);
            assert_eq!(sim.current_day, 1);
            assert_eq!(sim.total_days, 2); // 馬車: 2日
        }

        // 旅程進行: [Space] で 1日進める (Day 1 -> Day 2)
        press_key(&mut app, KeyCode::Space);

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::Traveling);
            assert_eq!(sim.current_day, 2);
        }

        // 旅程進行: [Space] でさらに進める (Day 2 >= Total 2 -> Arrived)
        press_key(&mut app, KeyCode::Space);

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::Arrived);
        }

        // 到着確定: [Space] で村へ入る
        press_key(&mut app, KeyCode::Space);

        assert_eq!(*app.world().resource::<AppMode>(), AppMode::Town);
        assert_eq!(
            app.world().resource::<TownStateRes>().current_area,
            AreaId::Village
        );
        assert!(app.world().resource::<TravelState>().sim.is_none());
    }

    #[test]
    fn test_travel_transport_insufficient_funds_blocks_progress() {
        let mut app = create_test_travel_app();
        app.world_mut().resource_mut::<PlayerInventoryRes>().gold = 10; // 10Gしか持っていない

        // Step 1: 行き先
        press_key(&mut app, KeyCode::Digit1);

        // Step 2: 姿勢
        press_key(&mut app, KeyCode::Digit1);

        // Step 3: 乗合馬車 (80G必要) を選択しようとする
        press_key(&mut app, KeyCode::Digit3);

        // 資金不足のためゴールドは消費されず、ChoosingTransport のまま
        assert_eq!(app.world().resource::<PlayerInventoryRes>().gold, 10);
        let travel = app.world().resource::<TravelState>();
        let sim = travel.sim.as_ref().unwrap();
        assert_eq!(sim.phase, TravelPhase::ChoosingTransport);
    }

    #[test]
    fn test_travel_automatic_timer_advancement_without_key_press() {
        let mut app = create_test_travel_app();

        // 旅計画: 徒歩で出発 (所要3日)
        press_key(&mut app, KeyCode::Digit1); // すずかけ村
        press_key(&mut app, KeyCode::Digit2); // 普通に
        press_key(&mut app, KeyCode::Digit1); // 徒歩 (0G)

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::Traveling);
            assert_eq!(sim.current_day, 1);
            assert_eq!(sim.total_days, 3);
        }

        // キーを押さず、時間を0.8秒進める -> Day 1 から Day 2 へ自動進行
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(800));
        app.update();

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::Traveling);
            assert_eq!(sim.current_day, 2);
        }

        // さらに0.8秒進める -> Day 2 から Day 3 へ自動進行
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(800));
        app.update();

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::Traveling);
            assert_eq!(sim.current_day, 3);
        }

        // さらに0.8秒進める -> 踏破して Arrived に自動遷移（ここで止まる）
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(800));
        app.update();

        {
            let travel = app.world().resource::<TravelState>();
            let sim = travel.sim.as_ref().unwrap();
            assert_eq!(sim.phase, TravelPhase::Arrived);

            let inv = app.world().resource::<PlayerInventoryRes>();
            let center_display = crate::ui::format_travel_center_display(travel, inv);
            // 旅が終わるまでメッセージログ（第1日目〜第3日目、および到着メッセージ）が画面に残っていること
            assert!(center_display.contains("第1日目"));
            assert!(center_display.contains("第2日目"));
            assert!(center_display.contains("第3日目"));
            assert!(center_display.contains("無事到着した"));
        }
    }
}
