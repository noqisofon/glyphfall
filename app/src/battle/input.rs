use super::{BattlePhase, BattleStateRes};
use crate::party::{PartyCommand, PlayerBattleAction};
use crate::{
    AppMode, PartyStateRes, PlayerInventoryRes, PlayerResourceRes, ShowMessage, TownStateRes,
};
use bevy::prelude::*;
use glyphfall_core::party::{PartyMember, PlayerSkills};
use rand::thread_rng;

fn next_alive_member(members: &[PartyMember], start_idx: usize) -> Option<usize> {
    (start_idx..members.len()).find(|&i| members[i].hp > 0)
}

fn start_turn_resolution(
    battle: &mut BattleStateRes,
    members: &mut [PartyMember],
    skills: &PlayerSkills,
    rng: &mut impl rand::Rng,
    msg_events: &mut EventWriter<ShowMessage>,
) {
    battle.build_turn_resolution(members, skills, rng);
    battle.apply_step_effects(0, members);
    if let Some(first_step) = battle.turn_steps.first() {
        let step_msg = first_step.message.clone();
        let is_defeated = first_step.monster_defeated;
        if is_defeated {
            msg_events.send(ShowMessage(format!(
                "【ターン開始！】\n\n{}\n魔物をたおした！([Space]で探索へ復帰)",
                step_msg
            )));
        } else {
            msg_events.send(ShowMessage(format!(
                "【ターン開始！】\n\n{}\n([Space]で次へ)",
                step_msg
            )));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_battle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut party: ResMut<PartyStateRes>,
    mut inv: ResMut<PlayerInventoryRes>,
    player: Res<PlayerResourceRes>,
    mut battle: ResMut<BattleStateRes>,
    mut town: ResMut<TownStateRes>,
    mut msg_events: EventWriter<ShowMessage>,
) {
    let mut rng = thread_rng();

    // [B]: いつでも戦闘離脱（街・探索へ復帰）
    if keyboard.just_pressed(KeyCode::KeyB) {
        *mode = AppMode::Town;
        town.last_encounter_pos = None;
        battle.reset_turn_commands(party.members.len());
        let area_name = town.current_area.name();
        msg_events.send(ShowMessage(format!(
            "戦闘を離脱し、{}へ戻った。\n[WASD]で歩き回れる。",
            area_name
        )));
        return;
    }

    // [N]: モンスター切り替え（テスト用）
    if keyboard.just_pressed(KeyCode::KeyN) {
        battle.next_monster(party.members.len());
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
                    if inv.remove_item("やくそう") {
                        Some(PlayerBattleAction::UseItem)
                    } else {
                        msg_events.send(ShowMessage(
                            "やくそうを持っていません！\n別の行動を選択してください。".into(),
                        ));
                        None
                    }
                } else if keyboard.just_pressed(KeyCode::Digit4) {
                    Some(PlayerBattleAction::Diagnose)
                } else if keyboard.just_pressed(KeyCode::Digit5) {
                    Some(PlayerBattleAction::Flee)
                } else {
                    None
                };

                if let Some(action) = chosen {
                    battle.player_action = Some(action);
                    if let Some(next_idx) = next_alive_member(&party.members, 1) {
                        battle.phase = BattlePhase::CommandInput {
                            member_cursor: next_idx,
                        };
                        let next_name = &party.members[next_idx].name;
                        msg_events.send(ShowMessage(format!(
                            "あなた:「{}」を選択した。\n続いて、{} への指示を選択してください。",
                            action.name(),
                            next_name
                        )));
                    } else {
                        // 生存している仲間がいない場合、全員の指示決定としてターン解決を実行
                        start_turn_resolution(
                            &mut battle,
                            &mut party.members,
                            &player.skills,
                            &mut rng,
                            &mut msg_events,
                        );
                    }
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
                    battle.set_party_command(member_cursor, cmd);
                    let current_name = party.members[member_cursor].name.clone();

                    if let Some(next_idx) = next_alive_member(&party.members, member_cursor + 1) {
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
                        start_turn_resolution(
                            &mut battle,
                            &mut party.members,
                            &player.skills,
                            &mut rng,
                            &mut msg_events,
                        );
                    }
                } else if keyboard.just_pressed(KeyCode::Escape) {
                    // 1つ前の生存メンバーに戻る
                    let prev_idx = (0..member_cursor)
                        .rev()
                        .find(|&i| i == 0 || party.members[i].hp > 0)
                        .unwrap_or(0);
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
                    battle.next_monster(party.members.len());
                    town.clear_defeated_monster();
                    *mode = AppMode::Town;
                    let area_name = town.current_area.name();
                    msg_events.send(ShowMessage(format!(
                        "魔物【{}】を撃破した！\n戦闘に勝利し、{}の探索に戻った。\n[WASD]で歩き回れる。",
                        mon_name, area_name
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
                    town.last_encounter_pos = None;
                    battle.reset_turn_commands(party.members.len());
                    let area_name = town.current_area.name();
                    msg_events.send(ShowMessage(format!(
                        "脱出に成功し、{}の安全な場所へ逃げ戻った！",
                        area_name
                    )));
                    return;
                }

                let next_cursor = step_cursor + 1;
                if next_cursor < battle.turn_steps.len() {
                    battle.phase = BattlePhase::TurnResolving {
                        step_cursor: next_cursor,
                    };
                    battle.apply_step_effects(next_cursor, &mut party.members);
                    let step_msg = battle.turn_steps[next_cursor].message.clone();
                    let is_defeated = battle.turn_steps[next_cursor].monster_defeated;
                    if is_defeated {
                        msg_events.send(ShowMessage(format!(
                            "{}\n魔物をたおした！([Space]で探索へ復帰)",
                            step_msg
                        )));
                    } else {
                        msg_events.send(ShowMessage(format!("{}\n([Space]で次へ)", step_msg)));
                    }
                } else {
                    // 全ステップ再生完了
                    if battle.current_monster().is_dead() {
                        let mon_name = battle.current_monster().name.clone();
                        battle.next_monster(party.members.len());
                        town.clear_defeated_monster();
                        *mode = AppMode::Town;
                        let area_name = town.current_area.name();
                        msg_events.send(ShowMessage(format!(
                            "魔物【{}】を撃破した！\n戦闘に勝利し、{}の探索に戻った。\n[WASD]で歩き回れる。",
                            mon_name, area_name
                        )));
                    } else if party.members[0].hp <= 0 {
                        battle.phase = BattlePhase::Defeat;
                        msg_events.send(ShowMessage(
                            "あなた（一般人）は力尽きてしまった……！\nパーティは壊滅した。\n[Space] または [B] で宿屋へ戻る。".into(),
                        ));
                    } else {
                        // 次のターンへ
                        battle.reset_turn_commands(party.members.len());
                        msg_events.send(ShowMessage(
                            "次のターン！ あなたの行動を選択してください。".into(),
                        ));
                    }
                }
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
                town.last_encounter_pos = None;
                // 王都アルカンの宿屋前 (3, 4) に帰還
                town.switch_area(
                    crate::AreaId::Town,
                    crate::Position { x: 3, y: 4 },
                    &mut rng,
                );
                *mode = AppMode::Town;
                battle.reset_turn_commands(party.members.len());
                msg_events.send(ShowMessage(
                    "教会の神父に助け出され、王都の宿屋で目覚めた……\n（HP/MP全回復）".into(),
                ));
            }
        }
    }
}
