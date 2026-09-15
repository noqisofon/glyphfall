use bevy::prelude::*;
use rand::thread_rng;
use crate::{
    AppMode, CommandMenuStage, CommandMenuState, PartyState, ShowMessage, TravelPosture,
    TravelState,
};
use crate::event::{
    try_trigger_sudden_event, SuddenEventCategory, SuddenEventHistory, SuddenEventRegistry,
};
use crate::party::PlayerInventory;
use super::{
    CommandKind, DialogueLearnStage, DialoguePartner, DialogueSession, InteractOutcome,
    MoveOutcome, TownState, SHOP_ITEMS,
};
use crate::ActiveDialogue;

pub fn handle_town_input(
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

    // [B]: 戦闘テストへ突入
    if keyboard.just_pressed(KeyCode::KeyB) {
        *mode = AppMode::Battle;
        dialogue_res.0 = None;
        msg_events.send(ShowMessage(
            "地下迷宮の魔物とエンカウントした！\nあなたの行動を選択してください。([B]で街へ帰還)".into(),
        ));
    }
}

pub fn handle_interact_input(
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

pub fn handle_dialogue_input(
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

pub fn handle_shop_input(
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

pub fn handle_inn_input(
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

pub fn handle_travel_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut town: ResMut<TownState>,
    travel_state: Res<TravelState>,
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
