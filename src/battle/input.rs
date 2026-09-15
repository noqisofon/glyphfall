use bevy::prelude::*;
use rand::thread_rng;
use crate::{AppMode, PartyState, PlayerResource, ShowMessage};
use crate::party::{PartyCommand, PlayerBattleAction};
use super::{BattlePhase, BattleState};

pub fn handle_battle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut party: ResMut<PartyState>,
    player: Res<PlayerResource>,
    mut battle: ResMut<BattleState>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    let mut rng = thread_rng();

    // [B]: いつでも戦闘離脱（街へ帰還）
    if keyboard.just_pressed(KeyCode::KeyB) {
        *mode = AppMode::Town;
        battle.reset_turn_commands();
        msg_events.send(ShowMessage("戦闘を離脱し、王都アルカンの街並みへ戻った。\n[WASD]で街を歩き回れる。".into()));
        return;
    }

    // [N]: モンスター切り替え（テスト用）
    if keyboard.just_pressed(KeyCode::KeyN) {
        battle.next_monster();
        let mon = battle.current_monster();
        msg_events.send(ShowMessage(format!(
            "あらたな　魔物【{}】が　あらわれた！\nあなたの行動を選択してください。",
            mon.name
        )));
        return;
    }

    match battle.phase.clone() {
        BattlePhase::CommandInput { member_cursor } => {
            if member_cursor == 0 {
                // あなたの行動選択
                let chosen = if keyboard.just_pressed(KeyCode::Digit1) {
                    Some(PlayerBattleAction::Attack)
                } else if keyboard.just_pressed(KeyCode::Digit2) {
                    Some(PlayerBattleAction::Defend)
                } else if keyboard.just_pressed(KeyCode::Digit3) {
                    Some(PlayerBattleAction::UseItem)
                } else if keyboard.just_pressed(KeyCode::Digit4) {
                    Some(PlayerBattleAction::Diagnose)
                } else if keyboard.just_pressed(KeyCode::Digit5) {
                    Some(PlayerBattleAction::Flee)
                } else {
                    None
                };

                if let Some(action) = chosen {
                    battle.player_action = Some(action);
                    battle.phase = BattlePhase::CommandInput { member_cursor: 1 };
                    let next_name = &party.members[1].name;
                    msg_events.send(ShowMessage(format!(
                        "あなた:「{}」を選択した。\n続いて、{} への指示を選択してください。",
                        action.name(),
                        next_name
                    )));
                }
            } else if member_cursor < party.members.len() {
                // 仲間への指示選択
                let chosen = if keyboard.just_pressed(KeyCode::Digit1) {
                    Some(PartyCommand::Attack)
                } else if keyboard.just_pressed(KeyCode::Digit2) {
                    Some(PartyCommand::Defend)
                } else if keyboard.just_pressed(KeyCode::Digit3) {
                    Some(PartyCommand::DesperateAttack)
                } else if keyboard.just_pressed(KeyCode::Digit4) {
                    Some(PartyCommand::CastSpell)
                } else {
                    None
                };

                if let Some(cmd) = chosen {
                    battle.party_commands[member_cursor] = Some(cmd);
                    let current_name = party.members[member_cursor].name.clone();

                    if member_cursor + 1 < party.members.len() {
                        let next_idx = member_cursor + 1;
                        battle.phase = BattlePhase::CommandInput {
                            member_cursor: next_idx,
                        };
                        let next_name = &party.members[next_idx].name;
                        msg_events.send(ShowMessage(format!(
                            "{}:「{}」を指示した。\n続いて、{} への指示を選択してください。",
                            current_name,
                            cmd.name(),
                            next_name
                        )));
                    } else {
                        // 全員の指示が決定！ターン解決を実行
                        battle.build_turn_resolution(&mut party.members, &player.skills, &mut rng);
                        if let Some(first_step) = battle.turn_steps.first() {
                            let step_msg = first_step.message.clone();
                            if first_step.monster_damage.is_some() {
                                battle.is_flashing = true;
                                battle.flash_timer.reset();
                            }
                            msg_events.send(ShowMessage(format!(
                                "【ターン開始！】全員の指示が揃った！\n\n{}\n([Space]で次へ)",
                                step_msg
                            )));
                        }
                    }
                } else if keyboard.just_pressed(KeyCode::Escape) {
                    // 1つ前のメンバーに戻る
                    let prev_idx = member_cursor - 1;
                    battle.phase = BattlePhase::CommandInput {
                        member_cursor: prev_idx,
                    };
                    let name = &party.members[prev_idx].name;
                    msg_events.send(ShowMessage(format!("{} の行動選択に戻りました。", name)));
                }
            }
        }
        BattlePhase::TurnResolving { step_cursor } => {
            if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::Enter) {
                let current_step = &battle.turn_steps[step_cursor];
                if current_step.monster_defeated {
                    let mon_name = battle.current_monster().name.clone();
                    battle.next_monster();
                    *mode = AppMode::Town;
                    msg_events.send(ShowMessage(format!(
                        "魔物【{}】を撃破した！\n戦闘に勝利し、王都アルカンの街並みへ戻った。\n[WASD]で街を歩き回れる。",
                        mon_name
                    )));
                    return;
                }
                if current_step.player_defeated {
                    battle.phase = BattlePhase::Defeat;
                    msg_events.send(ShowMessage(
                        "あなた（一般人）は力尽きてしまった……！\nパーティは壊滅した。\n[Space] または [B] で宿屋へ戻る。".into(),
                    ));
                    return;
                }
                if current_step.flee_success {
                    *mode = AppMode::Town;
                    battle.reset_turn_commands();
                    msg_events.send(ShowMessage("脱出に成功し、街へ逃げ帰った！".into()));
                    return;
                }

                let next_cursor = step_cursor + 1;
                if next_cursor < battle.turn_steps.len() {
                    let has_damage = battle.turn_steps[next_cursor].monster_damage.is_some();
                    let step_msg = battle.turn_steps[next_cursor].message.clone();
                    let is_defeated = battle.turn_steps[next_cursor].monster_defeated;
                    battle.phase = BattlePhase::TurnResolving {
                        step_cursor: next_cursor,
                    };
                    if has_damage {
                        battle.is_flashing = true;
                        battle.flash_timer.reset();
                    }
                    if is_defeated {
                        msg_events.send(ShowMessage(format!(
                            "{}\n魔物をたおした！([Space]で街へ帰還)",
                            step_msg
                        )));
                    } else {
                        msg_events.send(ShowMessage(format!("{}\n([Space]で次へ)", step_msg)));
                    }
                } else {
                    // 全ステップ再生完了
                    if battle.current_monster().is_dead() {
                        let mon_name = battle.current_monster().name.clone();
                        battle.next_monster();
                        *mode = AppMode::Town;
                        msg_events.send(ShowMessage(format!(
                            "魔物【{}】を撃破した！\n戦闘に勝利し、王都アルカンの街並みへ戻った。\n[WASD]で街を歩き回れる。",
                            mon_name
                        )));
                    } else if party.members[0].hp <= 0 {
                        battle.phase = BattlePhase::Defeat;
                        msg_events.send(ShowMessage(
                            "あなた（一般人）は力尽きてしまった……！\nパーティは壊滅した。\n[Space] または [B] で宿屋へ戻る。".into(),
                        ));
                    } else {
                        // 次のターンへ
                        battle.reset_turn_commands();
                        msg_events.send(ShowMessage(
                            "次のターン！ あなたの行動を選択してください。".into(),
                        ));
                    }
                }
            }
        }
        BattlePhase::Victory => {
            if keyboard.just_pressed(KeyCode::Space)
                || keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::KeyB)
            {
                let mon_name = battle.current_monster().name.clone();
                battle.next_monster();
                *mode = AppMode::Town;
                msg_events.send(ShowMessage(format!(
                    "魔物【{}】を撃破し、王都アルカンの街並みへ戻った。\n[WASD]で街を歩き回れる。",
                    mon_name
                )));
            }
        }
        BattlePhase::Defeat => {
            if keyboard.just_pressed(KeyCode::Space)
                || keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::KeyB)
            {
                // 宿屋で全員全快して街へ戻る
                for m in &mut party.members {
                    m.hp = m.max_hp;
                    m.mp = m.max_mp;
                }
                *mode = AppMode::Town;
                battle.reset_turn_commands();
                msg_events.send(ShowMessage(
                    "教会の神父に助け出され、宿屋で目覚めた……\n（HP/MP全回復）".into(),
                ));
            }
        }
    }
}
