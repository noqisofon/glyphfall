use super::{
    BattleMonsterTextNode, LeftWindowTextNode, RightWindowTextNode, StatusHeaderNode,
    TownMapImageNode, TravelSimulationTextNode,
};
use crate::{
    battle::{BattlePhase, BattleState, BattleStateRes, Monster},
    event::SuddenEventCategory,
    party::{MentalState, PartyState, PartyStateRes, Personality, PlayerInventory, CURRENCY_UNIT},
    town::{
        DialogueSession, DialogueStage, TargetKind, TownState, TownStateRes, TravelPhase, INN_COST,
        QUESTION_TYPES,
    },
    ActiveDialogue, AppMode, CommandKind, CommandMenuStage, CommandMenuState, PlayerInventoryRes,
    TravelState, DEBUG_KEYS, SHOP_ITEMS,
};
use bevy::prelude::*;

pub fn is_dual_split_mode(mode: AppMode, dialogue: &Option<DialogueSession>) -> bool {
    match mode {
        AppMode::Inn => {
            let stage = dialogue.as_ref().map(|s| s.stage).unwrap_or_default();
            !matches!(stage, DialogueStage::ChoosingLearnTarget { .. })
        }
        _ => false,
    }
}

pub fn format_status_header(
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
            Personality::Player => "プレイヤー(Player)",
            Personality::Loyal => "忠実(Loyal)",
            Personality::Slacker => "遊び人(Slacker)",
            Personality::Coward => "臆病(Coward)",
            Personality::Yandere => "ヤンデレ(Yandere)",
        };
        header.push_str(&format!(
            "  [DEBUG] 影響度: {} {} | 精神: {} | 性格: {} | 全視界: {}\n",
            member.influence.raw_value,
            abnormal_str,
            mental_str,
            personality_str,
            town.debug_see_all
        ));
    } else {
        let torch_str = if town.torch_active {
            "点灯中(半径6)"
        } else {
            "消灯(半径2)"
        };
        match mode {
            AppMode::Town => {
                header.push_str(&format!(
                    "  [{}] 所持金: {}{} | 松明: {} | 仲間列: 主人公(青) 戦士(赤) 遊び人(黄) 魔法使い(紫)\n",
                    town.current_area.name(),
                    inv.gold,
                    CURRENCY_UNIT,
                    torch_str
                ));
            }
            AppMode::Dialogue => {
                let partner_name = dialogue
                    .as_ref()
                    .map(|s| s.partner.name())
                    .unwrap_or("相手");
                let stage = dialogue.as_ref().map(|s| s.stage).unwrap_or_default();
                match stage {
                    DialogueStage::Talking => {
                        header.push_str(&format!(
                            "  [会話中: {}] 所持金: {}{}\n",
                            partner_name, inv.gold, CURRENCY_UNIT
                        ));
                    }
                    DialogueStage::SelectingSubject { .. } => {
                        header.push_str(&format!(
                            "  [会話中: {}] たずねる対象を選択中\n",
                            partner_name
                        ));
                    }
                    DialogueStage::SelectingQuestion { .. } => {
                        header.push_str(&format!("  [会話中: {}] 聞き方を選択中\n", partner_name));
                    }
                    DialogueStage::ChoosingLearnTarget { .. } => {
                        header.push_str(&format!(
                            "  [会話中: {}] 覚える言葉を選択中\n",
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
                    "  [コマンド選択中] 所持金: {}{} | {}\n",
                    inv.gold, CURRENCY_UNIT, hint
                ));
            }
            AppMode::Shop => {
                header.push_str(&format!(
                    "  [道具屋・取引中: 道具屋の店主] 所持金: {}{}\n",
                    inv.gold, CURRENCY_UNIT
                ));
            }
            AppMode::Inn => {
                header.push_str(&format!(
                    "  [宿屋・受付: 宿屋の主人] 所持金: {}{}\n",
                    inv.gold, CURRENCY_UNIT
                ));
            }
            AppMode::Battle => {
                let mut party_summary = String::new();
                for (i, m) in party.members.iter().enumerate() {
                    let sep = if i > 0 { " | " } else { "" };
                    party_summary.push_str(&format!("{}{}:{}/{}", sep, m.name, m.hp, m.max_hp));
                }
                header.push_str(&format!("  [戦闘交戦中] {}\n", party_summary));
            }
            AppMode::Travel => {
                if let Some(sim) = &travel.sim {
                    match sim.phase {
                        TravelPhase::ChoosingDestination => {
                            header.push_str(&format!(
                                "  [街道口・移動先選択] 行き先を選択してください | 所持金: {}{}\n",
                                inv.gold, CURRENCY_UNIT
                            ));
                        }
                        TravelPhase::ChoosingPosture => {
                            header.push_str(&format!(
                                "  [街道口・移動姿勢選択] 行き先: {} | 所持金: {}{}\n",
                                sim.plan.destination.name(),
                                inv.gold,
                                CURRENCY_UNIT
                            ));
                        }
                        TravelPhase::ChoosingTransport => {
                            header.push_str(&format!(
                                "  [街道口・移動手段選択] 行き先: {} (姿勢: {}) | 所持金: {}{}\n",
                                sim.plan.destination.name(),
                                sim.plan.posture.label(),
                                inv.gold,
                                CURRENCY_UNIT
                            ));
                        }
                        TravelPhase::Traveling => {
                            header.push_str(&format!(
                                "  [街道旅程進行中: 第{}/{}日] 行き先: {} (手段: {} / 姿勢: {})\n",
                                sim.current_day,
                                sim.total_days,
                                sim.plan.destination.name(),
                                sim.plan.transport.label(),
                                sim.plan.posture.label()
                            ));
                        }
                        TravelPhase::EncounterEvent => {
                            header.push_str(&format!(
                                "  [街道突発事象！ 第{}日] 行き先: {}\n",
                                sim.current_day,
                                sim.plan.destination.name()
                            ));
                        }
                        TravelPhase::Arrived => {
                            header.push_str(&format!(
                                "  [目的地到着！] 旅程踏破: {}\n",
                                sim.plan.destination.name()
                            ));
                        }
                    }
                } else {
                    header.push_str(&format!(
                        "  [街道・移動姿勢選択中] 行き先: {} | 所持金: {}{}\n",
                        travel.destination.name(),
                        inv.gold,
                        CURRENCY_UNIT
                    ));
                }
            }
        }
    }

    match mode {
        AppMode::Town => {
            header.push_str("操作: [WASD]移動 | [Z]コマンド | [L]松明切替 | ");
            if DEBUG_KEYS {
                header.push_str("[B]戦闘切替 | ");
            }
            header.push_str("[Tab]仲間切替 | [Space]スキップ");
        }
        AppMode::Interact => match command_menu.stage {
            CommandMenuStage::ChoosingCommand => {
                header.push_str("操作: [W/S]コマンド選択 | [1/Enter]決定 | [3/Esc]やめる");
            }
            CommandMenuStage::ChoosingDirection(_) => {
                header.push_str("操作: [WASD]方向を選択 | [Esc]やめる");
            }
        },
        AppMode::Dialogue => match dialogue.as_ref().map(|s| s.stage).unwrap_or_default() {
            DialogueStage::Talking => {
                header.push_str("操作: [1]たずねる | [2]おぼえる | [3/Esc]はなれる");
            }
            DialogueStage::SelectingSubject { .. } => {
                header.push_str("操作: [W/S]対象選択 | [1/Enter]決定 | [3/Esc]戻る");
            }
            DialogueStage::SelectingQuestion { .. } => {
                header.push_str("操作: [W/S]聞き方選択 | [1/Enter]決定 | [3/Esc]戻る");
            }
            DialogueStage::ChoosingLearnTarget { .. } => {
                header.push_str("操作: [W/S]候補選択 | [1/Enter]決定 | [3/Esc]やめる");
            }
        },
        AppMode::Shop => {
            header.push_str("操作: [W/S]商品選択 | [1/Enter]かう | [3/Esc]店を出る");
        }
        AppMode::Inn => {
            let session = dialogue.as_ref();
            let stage = session.map(|s| s.stage).unwrap_or_default();
            match stage {
                DialogueStage::ChoosingLearnTarget { .. } => {
                    header.push_str("操作: [W/S]候補選択 | [1/Enter]決定 | [3/Esc]やめる");
                }
                _ => {
                    let has_learnable = session
                        .map(|s| !s.learnable_spans.is_empty())
                        .unwrap_or(false);
                    if has_learnable {
                        header.push_str("操作: [1]宿泊 | [2]おぼえる | [3]宿を出る");
                    } else {
                        header.push_str("操作: [1]宿泊 | [3]宿を出る");
                    }
                }
            }
        }
        AppMode::Battle => {
            header.push_str("操作: [1-5]コマンド/指示 | [Space/Enter]ターン進行");
            if DEBUG_KEYS {
                header.push_str(" | [N]敵切替 | [B]街へ帰還");
            }
        }
        AppMode::Travel => {
            if let Some(sim) = &travel.sim {
                match sim.phase {
                    TravelPhase::ChoosingDestination => {
                        header.push_str("操作: [1/Enter]すずかけ村を選択 | [Esc]街へ戻る");
                    }
                    TravelPhase::ChoosingPosture => {
                        header.push_str("操作: [1]慎重に | [2]普通に | [3]大胆に | [Esc]戻る");
                    }
                    TravelPhase::ChoosingTransport => {
                        header
                            .push_str("操作: [1]徒歩(0G) | [2]馬(40G) | [3]馬車(80G) | [Esc]戻る");
                    }
                    TravelPhase::Traveling => {
                        header.push_str("操作: [自動進行中] (または [Space/Enter]で早送り)");
                    }
                    TravelPhase::EncounterEvent => {
                        header.push_str("操作: [Space/Enter]突発事象に対応する");
                    }
                    TravelPhase::Arrived => {
                        header.push_str("操作: [Space/Enter]目的地へ入る");
                    }
                }
            } else {
                header.push_str("操作: [1]慎重に | [2]普通に | [3]大胆に | [Esc]やめる");
            }
        }
    }

    header
}

#[allow(clippy::too_many_arguments)]
pub fn format_left_window(
    mode: AppMode,
    inv: &PlayerInventory,
    dialogue: &Option<DialogueSession>,
    command_menu: &CommandMenuState,
    facing_target: TargetKind,
    travel: &TravelState,
    battle: &BattleState,
    party: &PartyState,
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
                    out.push_str(&format!(
                        "{} {}\n",
                        cursor,
                        cmd.dynamic_label(facing_target)
                    ));
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
            let stage = session.map(|s| s.stage).unwrap_or_default();
            match stage {
                DialogueStage::Talking => {
                    let mut out = "【覚えた話題】\n".to_string();
                    if inv.topics.is_empty() {
                        out.push_str(" (話題なし)\n");
                    } else {
                        for topic in inv.topics.iter().take(4) {
                            out.push_str(&format!("・{}\n", topic));
                        }
                    }
                    out
                }
                DialogueStage::SelectingSubject { cursor } => {
                    let mut out = "【たずねる対象】\n".to_string();
                    let subjects = DialogueSession::selectable_subjects(&inv.topics);
                    let max_visible = 4;
                    let start = if cursor >= max_visible {
                        cursor + 1 - max_visible
                    } else {
                        0
                    };
                    for (idx, (name, _)) in
                        subjects.iter().enumerate().skip(start).take(max_visible)
                    {
                        let c = if idx == cursor { "▶" } else { " " };
                        if idx < 3 {
                            out.push_str(&format!("{} [{}]\n", c, name));
                        } else {
                            out.push_str(&format!("{} {}\n", c, name));
                        }
                    }
                    out
                }
                DialogueStage::SelectingQuestion {
                    subject_cursor,
                    cursor,
                } => {
                    let subjects = DialogueSession::selectable_subjects(&inv.topics);
                    let subj_name = subjects
                        .get(subject_cursor)
                        .map(|(n, _)| n.as_str())
                        .unwrap_or("？");
                    // ウィンドウは5行分しかないため、見出しは「対象」の1行に留める
                    let mut out = format!(
                        "対象:{}\n",
                        if subject_cursor < 3 {
                            format!("[{}]", subj_name)
                        } else {
                            subj_name.to_string()
                        }
                    );
                    for (idx, (_, label)) in QUESTION_TYPES.iter().enumerate() {
                        let c = if idx == cursor { "▶" } else { " " };
                        out.push_str(&format!("{} {}\n", c, label));
                    }
                    out
                }
                DialogueStage::ChoosingLearnTarget { cursor } => {
                    let mut out = "【おぼえる語句】\n".to_string();
                    if let Some(s) = session {
                        for (idx, span) in s.learnable_spans.iter().enumerate().take(4) {
                            let c = if idx == cursor { "▶" } else { " " };
                            let word = span.slice(&s.current_text);
                            out.push_str(&format!("{} {}\n", c, word));
                        }
                    }
                    out
                }
            }
        }
        AppMode::Shop => {
            let session = dialogue.as_ref();
            let selected = session.map(|s| s.selected_shop_index).unwrap_or(0);
            let mut out = "【品物】\n".to_string();
            for (idx, item) in SHOP_ITEMS.iter().enumerate() {
                let cursor = if idx == selected { "▶" } else { " " };
                out.push_str(&format!(
                    "{}{} {:2}{}\n",
                    cursor, item.name, item.price, CURRENCY_UNIT
                ));
            }
            out
        }
        AppMode::Inn => {
            let session = dialogue.as_ref();
            let stage = session.map(|s| s.stage).unwrap_or_default();
            if let DialogueStage::ChoosingLearnTarget { cursor } = stage {
                let mut out = "【おぼえる語句】\n".to_string();
                if let Some(s) = session {
                    for (idx, span) in s.learnable_spans.iter().enumerate().take(4) {
                        let c = if idx == cursor { "▶" } else { " " };
                        let word = span.slice(&s.current_text);
                        out.push_str(&format!("{} {}\n", c, word));
                    }
                }
                out
            } else {
                let mut out = "【手帳】\n[覚えた話題]\n".to_string();
                if inv.topics.is_empty() {
                    out.push_str(" (話題なし)\n");
                } else {
                    for topic in inv.topics.iter().take(3) {
                        out.push_str(&format!("・{}\n", topic));
                    }
                }
                out
            }
        }
        AppMode::Town => {
            let mut out = format!("【手帳】金:{}{}\n[覚えた話題]\n", inv.gold, CURRENCY_UNIT);
            for topic in inv.topics.iter().take(3) {
                out.push_str(&format!("・{}\n", topic));
            }
            out
        }
        AppMode::Battle => match &battle.phase {
            BattlePhase::CommandInput { member_cursor } => {
                let cursor = *member_cursor;
                if cursor == 0 {
                    "【あなたの行動】\n[1]たたかう\n[2]みをまもる\n[3]どうぐ(草)\n[4]観察 5:逃走"
                        .into()
                } else if cursor < party.members.len() {
                    let member = &party.members[cursor];
                    format!(
                        "【{}へ指示】\n[1]たたかう\n[2]みをまもる\n[3]すてみ\n[4]じゅもん",
                        member.name
                    )
                } else {
                    "【指示決定】\nターン開始準備完了".into()
                }
            }
            BattlePhase::TurnResolving { step_cursor } => {
                format!(
                    "【ターン進行中】\n進捗: {}/{}\n\n[Space]で次へ",
                    step_cursor + 1,
                    battle.turn_steps.len().max(1)
                )
            }
            BattlePhase::Defeat => {
                "【全滅……】\nあなたは力尽きた\n\n[B]街へ逃走\n(宿屋で治療)".into()
            }
        },
        AppMode::Travel => {
            if let Some(sim) = &travel.sim {
                match sim.phase {
                    TravelPhase::ChoosingDestination => {
                        format!(
                            "【旅の計画 1/3】\n行き先を選択\n\n[1] すずかけ村\n\n所持金:\n {}{}",
                            inv.gold, CURRENCY_UNIT
                        )
                    }
                    TravelPhase::ChoosingPosture => {
                        format!(
                            "【旅の計画 2/3】\n姿勢を選択\n\n行先: {}\n\n所持金:\n {}{}",
                            sim.plan.destination.name(),
                            inv.gold,
                            CURRENCY_UNIT
                        )
                    }
                    TravelPhase::ChoosingTransport => {
                        format!(
                            "【旅の計画 3/3】\n手段を選択\n\n行先: {}\n姿勢: {}\n所持金:\n {}{}",
                            sim.plan.destination.name(),
                            sim.plan.posture.label(),
                            inv.gold,
                            CURRENCY_UNIT
                        )
                    }
                    TravelPhase::Traveling => {
                        format!(
                            "【街道道行】\n{}日目/全{}日\n\n行先: {}\n手段: {}\n姿勢: {}",
                            sim.current_day,
                            sim.total_days,
                            sim.plan.destination.name(),
                            sim.plan.transport.label(),
                            sim.plan.posture.label()
                        )
                    }
                    TravelPhase::EncounterEvent => {
                        format!(
                            "【突発事象！】\n第{}日目\n\n行先: {}\n注意せよ！",
                            sim.current_day,
                            sim.plan.destination.name()
                        )
                    }
                    TravelPhase::Arrived => {
                        format!(
                            "【到着】\n目的地:\n{}\n\n旅程完了！",
                            sim.plan.destination.name()
                        )
                    }
                }
            } else {
                format!(
                    "【街道】\n行き先:\n{}\n\nどのように\n進みますか？",
                    travel.destination.name()
                )
            }
        }
    }
}

pub fn format_right_window(
    mode: AppMode,
    dialogue: &Option<DialogueSession>,
    command_menu: &CommandMenuState,
    battle: &BattleState,
) -> String {
    match mode {
        AppMode::Interact => match command_menu.stage {
            CommandMenuStage::ChoosingCommand => "[W/S]選択\n[1/Enter]決定\n[3/Esc]やめる".into(),
            CommandMenuStage::ChoosingDirection(_) => "[WASD]方向選択\n[Esc]やめる".into(),
        },
        AppMode::Dialogue => match dialogue.as_ref().map(|s| s.stage).unwrap_or_default() {
            DialogueStage::Talking => {
                let has_learnable = dialogue
                    .as_ref()
                    .map(|s| !s.learnable_spans.is_empty())
                    .unwrap_or(false);
                let learn_str = if has_learnable {
                    "[2]おぼえる★"
                } else {
                    "[2]おぼえる"
                };
                format!("[1]たずねる\n{}\n[3]はなれる", learn_str)
            }
            DialogueStage::SelectingSubject { .. } => {
                "[1/Enter]決定\n[3/Esc]戻る\n\n(W/S:対象選択)".into()
            }
            DialogueStage::SelectingQuestion { .. } => {
                "[1/Enter]決定\n[3/Esc]戻る\n\n(W/S:聞き方)".into()
            }
            DialogueStage::ChoosingLearnTarget { .. } => {
                "[1/Enter]決定\n[3/Esc]やめる\n\n(W/S:語句選択)".into()
            }
        },
        AppMode::Shop => "[1]かう\n[3]みせをでる\n(W/S:商品選)\n(所持金消費)".into(),
        AppMode::Inn => {
            let session = dialogue.as_ref();
            let stage = session.map(|s| s.stage).unwrap_or_default();
            match stage {
                DialogueStage::ChoosingLearnTarget { .. } => {
                    "[1/Enter]決定\n[3/Esc]やめる\n\n(W/S:語句選択)".into()
                }
                _ => {
                    let has_learnable = session
                        .map(|s| !s.learnable_spans.is_empty())
                        .unwrap_or(false);
                    let learn_str = if has_learnable {
                        "\n[2]おぼえる★"
                    } else {
                        ""
                    };
                    format!(
                        "【宿屋】\n[1]宿泊({}{})\n   (HP/MP全快){}\n[3]宿を出る",
                        INN_COST, CURRENCY_UNIT, learn_str
                    )
                }
            }
        }
        AppMode::Town if DEBUG_KEYS => {
            "[探索操作]\nWASD:移動\nZ   :コマンド\nTab :仲間\nB   :戦闘".into()
        }
        AppMode::Town => "[探索操作]\nWASD:移動\nZ   :コマンド\nTab :仲間".into(),
        AppMode::Battle => match &battle.phase {
            BattlePhase::CommandInput { member_cursor } => {
                match (*member_cursor == 0, DEBUG_KEYS) {
                    (true, true) => "[1-5]行動\n[B]街へ帰還\n\n(あなた手番)".into(),
                    (true, false) => "[1-5]行動\n\n(あなた手番)".into(),
                    (false, true) => "[1-4]指示\n[Esc]戻る\n[B]街へ帰還\n\n(仲間手番)".into(),
                    (false, false) => "[1-4]指示\n[Esc]戻る\n\n(仲間手番)".into(),
                }
            }
            BattlePhase::TurnResolving { .. } => "[Space]次へ\n[Enter]次へ\n\n(ターン中)".into(),
            BattlePhase::Defeat => "[B]街へ帰還\n\n(敗北)".into(),
        },
        AppMode::Travel => "[1-3]選択\n[Space]早送\n[Esc]戻る".into(),
    }
}

pub fn format_travel_center_display(travel: &TravelState, inv: &PlayerInventory) -> String {
    let sim = match &travel.sim {
        Some(s) => s,
        None => return String::new(),
    };

    match sim.phase {
        TravelPhase::ChoosingDestination => "【 街道口：旅の計画 (1/3: 行き先の選択) 】\n\n\
             街の外へと続く街道の分岐点に立った。\n\
             案内標識が風に揺れている。どの地を目指しますか？\n\n\
             [1] すずかけ村 （街道を北東へ抜けた先にある素朴な農村）\n\n\
             [Esc] 街へ引き返す"
            .to_string(),
        TravelPhase::ChoosingPosture => {
            format!(
                "【 街道口：旅の計画 (2/3: 移動姿勢の選択) 】\n\n\
                 目的地: {} ｜ 所持金: {}{}\n\
                 街道をどのような姿勢で進みますか？\n\n\
                 [1] 慎重に歩む   (遭遇率: 15% / 所要: 基準)\n\
                 [2] 普通に進む   (遭遇率: 35% / 所要: 基準)\n\
                 [3] 大胆に急ぐ   (遭遇率: 65% / 所要: 1日短縮)\n\n\
                 [Esc] 前の選択に戻る",
                sim.plan.destination.name(),
                inv.gold,
                CURRENCY_UNIT
            )
        }
        TravelPhase::ChoosingTransport => {
            let foot_label = "[1] 徒歩でゆく     (費用:  0 G / 基準: 3日)";
            let horse_label = if inv.gold >= 40 {
                "[2] 馬を駆る       (費用: 40 G / 基準: 1日)"
            } else {
                "[2] 馬を駆る       (費用: 40 G - 資金不足)"
            };
            let carriage_label = if inv.gold >= 80 {
                "[3] 乗合馬車に乗る (費用: 80 G / 基準: 2日)"
            } else {
                "[3] 乗合馬車に乗る (費用: 80 G - 資金不足)"
            };

            format!(
                "【 街道口：旅の計画 (3/3: 移動手段の選択) 】\n\n\
                 目的地: {} ｜ 移動姿勢: {}\n\
                 旅の移動手段を選択してください。(所持金: {}{})\n\n\
                 {}\n\
                 {}\n\
                 {}\n\n\
                 [Esc] 前の選択に戻る",
                sim.plan.destination.name(),
                sim.plan.posture.label(),
                inv.gold,
                CURRENCY_UNIT,
                foot_label,
                horse_label,
                carriage_label
            )
        }
        TravelPhase::Traveling => {
            let max_logs = 4;
            let start = sim.log_history.len().saturating_sub(max_logs);
            let logs_str = sim.log_history[start..].join("\n");

            format!(
                "【 街道の道行 - 第 {}/{} 日 】 目的地: {} ｜ 手段: {} ｜ 姿勢: {}\n\
                 ──────────────────────────────────────────\n\
                 {}\n\
                 ──────────────────────────────────────────\n\
                 [自動進行中...] （または [Space / Enter] で早送り）",
                sim.current_day,
                sim.total_days,
                sim.plan.destination.name(),
                sim.plan.transport.label(),
                sim.plan.posture.label(),
                logs_str
            )
        }
        TravelPhase::EncounterEvent => {
            let event_msg = sim
                .pending_event
                .as_ref()
                .map(|e| e.message)
                .unwrap_or(&sim.current_message);
            let action_hint = match sim.pending_event.as_ref().map(|e| e.category) {
                Some(SuddenEventCategory::Bandit) | Some(SuddenEventCategory::WildAnimal) => {
                    "[Space / Enter] 武器を構えて戦闘に突入する！"
                }
                _ => "[Space / Enter] 事態を切り抜けて旅を続ける",
            };

            let prior_logs: Vec<&String> = sim
                .log_history
                .iter()
                .filter(|s| !s.contains("突発事態が発生"))
                .collect();
            let max_logs = 2;
            let start = prior_logs.len().saturating_sub(max_logs);
            let logs_str = if prior_logs.is_empty() {
                String::new()
            } else {
                format!(
                    "{}\n──────────────────────────────────────────\n",
                    prior_logs[start..]
                        .iter()
                        .map(|s| (*s).as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            };

            format!(
                "【 街道の突発事象！ - 第 {} 日目 】 目的地: {} ｜ 手段: {}\n\
                 ──────────────────────────────────────────\n\
                 {}\
                 ┌────────────────────────────────────────┐\n\
                 │ [!] 突発事態が発生！                    │\n\
                 │                                        │\n\
                 │ {}\n\
                 │                                        │\n\
                 │ （※街道の情景スチール枠）              │\n\
                 └────────────────────────────────────────┘\n\
                 {}",
                sim.current_day,
                sim.plan.destination.name(),
                sim.plan.transport.label(),
                logs_str,
                event_msg,
                action_hint
            )
        }
        TravelPhase::Arrived => {
            let scenery_logs: Vec<&String> = sim
                .log_history
                .iter()
                .filter(|s| !s.starts_with("【到着】"))
                .collect();
            let max_logs = 4;
            let start = scenery_logs.len().saturating_sub(max_logs);
            let logs_str = scenery_logs[start..]
                .iter()
                .map(|s| (*s).as_str())
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                "【 街道の道行 - 旅程踏破 】 目的地: {} ｜ 手段: {} ｜ 姿勢: {}\n\
                 ──────────────────────────────────────────\n\
                 {}\n\
                 ==========================================\n\
                 【到着】{}日間の旅路を終え、{}に無事到着した！\n\
                 [Space / Enter] エリアに入る",
                sim.plan.destination.name(),
                sim.plan.transport.label(),
                sim.plan.posture.label(),
                logs_str,
                sim.total_days,
                sim.plan.destination.name()
            )
        }
    }
}

pub fn format_hp_bar(hp: i32, max_hp: i32, length: usize) -> String {
    let ratio = (hp as f32 / max_hp as f32).clamp(0.0, 1.0);
    let filled = (ratio * length as f32).round() as usize;
    let empty = length.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), " ".repeat(empty))
}

pub fn format_monster_display(monster: &Monster) -> String {
    let hp_bar = format_hp_bar(monster.hp, monster.max_hp, 14);
    let clean_art = monster.glyph_art.trim_matches('\n');
    format!(
        "{}\n           【 {} 】  HP: {:2}/{}  {}\n",
        clean_art, monster.name, monster.hp, monster.max_hp, hp_bar
    )
}

pub fn update_town_texture_system(
    mut town: ResMut<TownStateRes>,
    party: Res<PartyStateRes>,
    mut images: ResMut<Assets<Image>>,
) {
    if party.is_changed() {
        town.state.dirty = true;
    }
    town.update_texture(&party.members, &mut images);
}

#[allow(clippy::type_complexity)]
pub fn update_center_window_visibility_system(
    mode: Res<AppMode>,
    mut town_image_query: Query<
        &mut Node,
        (
            With<TownMapImageNode>,
            Without<BattleMonsterTextNode>,
            Without<TravelSimulationTextNode>,
        ),
    >,
    mut battle_monster_query: Query<
        &mut Node,
        (
            With<BattleMonsterTextNode>,
            Without<TownMapImageNode>,
            Without<TravelSimulationTextNode>,
        ),
    >,
    mut travel_sim_query: Query<
        &mut Node,
        (
            With<TravelSimulationTextNode>,
            Without<TownMapImageNode>,
            Without<BattleMonsterTextNode>,
        ),
    >,
) {
    if !mode.is_changed() {
        return;
    }
    if let Ok(mut town_node) = town_image_query.get_single_mut() {
        town_node.display = match *mode {
            AppMode::Battle | AppMode::Travel => Display::None,
            _ => Display::Flex,
        };
    }
    if let Ok(mut battle_node) = battle_monster_query.get_single_mut() {
        battle_node.display = match *mode {
            AppMode::Battle => Display::Flex,
            _ => Display::None,
        };
    }
    if let Ok(mut travel_node) = travel_sim_query.get_single_mut() {
        travel_node.display = match *mode {
            AppMode::Travel => Display::Flex,
            _ => Display::None,
        };
    }
}

pub fn update_travel_simulation_display_system(
    mode: Res<AppMode>,
    travel: Res<TravelState>,
    inv: Res<PlayerInventoryRes>,
    mut query: Query<&mut Text, With<TravelSimulationTextNode>>,
) {
    if *mode != AppMode::Travel {
        return;
    }
    if mode.is_changed() || travel.is_changed() || inv.is_changed() {
        if let Ok(mut text) = query.get_single_mut() {
            *text = Text::new(format_travel_center_display(&travel, &inv));
        }
    }
}

pub fn update_battle_monster_display_system(
    mode: Res<AppMode>,
    battle: Res<BattleStateRes>,
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

#[allow(clippy::too_many_arguments)]
pub fn update_status_header_system(
    party: Res<PartyStateRes>,
    mode: Res<AppMode>,
    town: Res<TownStateRes>,
    inv: Res<PlayerInventoryRes>,
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

#[allow(clippy::too_many_arguments)]
pub fn update_tri_split_windows_system(
    mode: Res<AppMode>,
    inv: Res<PlayerInventoryRes>,
    dialogue: Res<ActiveDialogue>,
    command_menu: Res<CommandMenuState>,
    town: Res<TownStateRes>,
    travel: Res<TravelState>,
    battle: Res<BattleStateRes>,
    party: Res<PartyStateRes>,
    mut left_query: Query<&mut Text, (With<LeftWindowTextNode>, Without<RightWindowTextNode>)>,
    mut right_query: Query<&mut Text, (With<RightWindowTextNode>, Without<LeftWindowTextNode>)>,
) {
    if mode.is_changed()
        || inv.is_changed()
        || dialogue.is_changed()
        || command_menu.is_changed()
        || town.is_changed()
        || travel.is_changed()
        || battle.is_changed()
        || party.is_changed()
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
                &battle,
                &party,
            ));
        }
        for mut text in &mut right_query {
            *text = Text::new(format_right_window(
                *mode,
                &dialogue.0,
                &command_menu,
                &battle,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::PartyMember;
    use crate::town::DialoguePartner;

    fn create_test_party() -> PartyState {
        PartyState {
            selected_index: 0,
            debug_mode: false,
            members: vec![PartyMember::new_player("あなた").with_stats(20, 10)],
        }
    }

    #[test]
    fn test_is_dual_split_mode() {
        let inn_session = DialogueSession::start(DialoguePartner::Inn);
        assert!(is_dual_split_mode(AppMode::Inn, &Some(inn_session)));

        let mut choosing_session = DialogueSession::start(DialoguePartner::Inn);
        choosing_session.stage = DialogueStage::ChoosingLearnTarget { cursor: 0 };
        assert!(!is_dual_split_mode(AppMode::Inn, &Some(choosing_session)));

        assert!(!is_dual_split_mode(AppMode::Town, &None));
        assert!(!is_dual_split_mode(AppMode::Dialogue, &None));
        assert!(!is_dual_split_mode(AppMode::Shop, &None));
    }

    #[test]
    fn test_inn_header_non_redundant() {
        let party = create_test_party();
        let town = TownState::new();
        let mut inv = PlayerInventory::default();
        inv.gold = 50;
        let session = DialogueSession::start(DialoguePartner::Inn);
        let header = format_status_header(
            &party,
            AppMode::Inn,
            &town,
            &inv,
            &Some(session),
            &CommandMenuState::default(),
            &TravelState::default(),
        );

        // ヘッダー2行目に所持金が表示され、選択肢/料金(50G)は重複して含まれないこと
        assert!(header.contains("[宿屋・受付: 宿屋の主人] 所持金: 50G"));
        assert!(!header.contains("[1]宿泊(50G)で全快"));
        assert!(!header.contains("| [3]宿を出る\n"));

        // 操作行に料金(50G)の重複がなく、簡潔な操作表記であること
        assert!(header.contains("操作: [1]宿泊 | [3]宿を出る"));
        assert!(!header.contains("とまる(50G)"));
    }

    #[test]
    fn test_inn_panels_non_redundant() {
        let mut inv = PlayerInventory::default();
        inv.gold = 50;
        let session = DialogueSession::start(DialoguePartner::Inn);
        let battle = BattleState::new(vec![]);
        let party = create_test_party();

        // 左パネルは一泊料金や所持金の重複がなく手帳の話題一覧であること
        let left = format_left_window(
            AppMode::Inn,
            &inv,
            &Some(session.clone()),
            &CommandMenuState::default(),
            TargetKind::Nothing,
            &TravelState::default(),
            &battle,
            &party,
        );
        assert!(!left.contains("一泊料金: 50G"));
        assert!(!left.contains("所持金  : 50G"));
        assert!(left.contains("【手帳】"));

        // 右パネルに宿泊料金と全快効果が集約されていること
        let right = format_right_window(
            AppMode::Inn,
            &Some(session),
            &CommandMenuState::default(),
            &battle,
        );
        assert!(right.contains("【宿屋】"));
        assert!(right.contains("[1]宿泊(50G)"));
        assert!(right.contains("(HP/MP全快)"));
        assert!(right.contains("[3]宿を出る"));
    }

    #[test]
    fn test_shop_non_redundant() {
        let party = create_test_party();
        let town = TownState::new();
        let mut inv = PlayerInventory::default();
        inv.gold = 100;
        let session = DialogueSession::start(DialoguePartner::Shop);
        let header = format_status_header(
            &party,
            AppMode::Shop,
            &town,
            &inv,
            &Some(session.clone()),
            &CommandMenuState::default(),
            &TravelState::default(),
        );

        // ヘッダー2行目に操作が重複していないこと
        assert!(header.contains("[道具屋・取引中: 道具屋の店主] 所持金: 100G\n"));
        assert!(!header.contains("| [W/S]で商品選択"));

        // 左パネルに所持金が重複していないこと
        let battle = BattleState::new(vec![]);
        let left = format_left_window(
            AppMode::Shop,
            &inv,
            &Some(session),
            &CommandMenuState::default(),
            TargetKind::Nothing,
            &TravelState::default(),
            &battle,
            &party,
        );
        assert!(left.starts_with("【品物】\n"));
        assert!(!left.contains("(金:100G)"));
    }

    #[test]
    fn test_ui_scale_ratio_calculation() {
        let base_w = 960.0_f32;
        let base_h = 640.0_f32;

        // 960x640: 等倍
        let scale_960 = (960.0 / base_w).min(640.0 / base_h);
        assert!((scale_960 - 1.0).abs() < 0.001);

        // 1920x1080 (16:9 Full HD): 縦幅基準で 1.6875倍に拡大
        let scale_1080p = (1920.0 / base_w).min(1080.0 / base_h);
        assert!((scale_1080p - 1.6875).abs() < 0.001);

        // 2560x1440 (16:9 QHD): 縦幅基準で 2.25倍に拡大
        let scale_1440p = (2560.0 / base_w).min(1440.0 / base_h);
        assert!((scale_1440p - 2.25).abs() < 0.001);

        // 1280x720 (16:9 720p): 縦幅基準で 1.125倍に拡大
        let scale_720p = (1280.0 / base_w).min(720.0 / base_h);
        assert!((scale_720p - 1.125).abs() < 0.001);
    }
}
