use rand::Rng;
use super::influence::{PartyMember, Personality, MentalState};

/// プレイヤー（あなた）自身の戦闘行動
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerBattleAction {
    /// 「たたかう」（パンピーの微力な攻撃）
    Attack,
    /// 「みをまもる」（被ダメージ軽減）
    Defend,
    /// 「どうぐ」（やくそう等で味方を治療）
    UseItem,
    /// 「かんさつ」（仲間の精神状態を観察）
    Diagnose,
    /// 「にげる」（戦闘から逃走を試みる）
    Flee,
}

impl PlayerBattleAction {
    pub fn name(&self) -> &'static str {
        match self {
            PlayerBattleAction::Attack => "たたかう",
            PlayerBattleAction::Defend => "みをまもる",
            PlayerBattleAction::UseItem => "どうぐ",
            PlayerBattleAction::Diagnose => "かんさつ",
            PlayerBattleAction::Flee => "にげる",
        }
    }
}

/// プレイヤーから仲間への指示
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PartyCommand {
    /// 「たたかう」（基本的な戦闘行動）
    Attack,
    /// 「ぼうぎょ」（身を守る）
    Defend,
    /// 「すてみ」（自己犠牲・危険度の極めて高い行動）
    DesperateAttack,
    /// 「じゅもん」（MPを消費する呪文行使）
    CastSpell,
}

#[allow(dead_code)]
impl PartyCommand {
    pub fn name(&self) -> &'static str {
        match self {
            PartyCommand::Attack => "たたかう",
            PartyCommand::Defend => "みをまもる",
            PartyCommand::DesperateAttack => "すてみ",
            PartyCommand::CastSpell => "じゅもん",
        }
    }

    /// 指示の危険度・抵抗ペナルティ（マイナス値ほど従いにくい）
    pub fn resistance_penalty(&self) -> i32 {
        match self {
            PartyCommand::Attack => 0,
            PartyCommand::Defend => 5, // 防御は比較的受け入れやすい
            PartyCommand::DesperateAttack => -40, // 危険な命令は激しく抵抗される
            PartyCommand::CastSpell => -10,
        }
    }
}

/// 指示に対する仲間の行動結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOutcome {
    /// 指示通りに行動した
    Obeyed {
        action_msg: String,
    },
    /// 指示を拒否し、自律行動（またはサボり・暴走）した
    Disobeyed {
        reason_msg: String,
        action_msg: String,
    },
}

/// 指示を実行させようとしたときの対抗判定
pub fn evaluate_command<R: Rng>(
    member: &PartyMember,
    command: PartyCommand,
    rng: &mut R,
) -> ActionOutcome {
    if member.is_player {
        return ActionOutcome::Obeyed {
            action_msg: format!("{}は　指示どおり行動した！", member.name),
        };
    }

    // 基礎成功率は隠し影響度
    let base_rate = member.influence.raw_value as i32;

    // コマンド自体の難易度・危険度ペナルティ
    let command_mod = command.resistance_penalty();

    // 性格による補正
    let personality_mod = match member.personality {
        Personality::Player => 100,
        Personality::Loyal => 15,
        Personality::Slacker => -30,
        Personality::Coward => {
            if command == PartyCommand::DesperateAttack {
                -50
            } else {
                -10
            }
        }
        Personality::Yandere => {
            // 主人公を守る・戦う指示には前向きだが、防御などで待たされると不満
            if command == PartyCommand::Attack || command == PartyCommand::DesperateAttack {
                15
            } else {
                -5
            }
        }
    };

    // 精神状態による補正
    let mental_mod = match member.mental_state {
        MentalState::Charmed => 50, // 魅了されていると盲目的に従いやすい
        MentalState::Cursed => -20,
        MentalState::Normal => 0,
    };

    // 総合服従確率の算出
    let final_chance = (base_rate + command_mod + personality_mod + mental_mod).clamp(5, 95);

    // 1〜100 のロール
    let roll: i32 = rng.gen_range(1..=100);

    if roll <= final_chance {
        // 服従成功
        let action_msg = match command {
            PartyCommand::Attack => format!("{}は　指示どおり　果敢に　切り込んだ！", member.name),
            PartyCommand::Defend => format!("{}は　身構えて　身を守っている！", member.name),
            PartyCommand::DesperateAttack => {

                format!("{}は　覚悟を決めて　すてみの突撃を敢行した！", member.name)
            }
            PartyCommand::CastSpell => {
                format!("{}は　呪文を唱えた！　カエンダンが炸裂する！", member.name)
            }

        };
        ActionOutcome::Obeyed { action_msg }
    } else {
        // 不服従・自律行動
        let (reason_msg, action_msg) = generate_disobedient_behavior(member, command, rng);
        ActionOutcome::Disobeyed {
            reason_msg,
            action_msg,
        }
    }
}

fn generate_disobedient_behavior<R: Rng>(
    member: &PartyMember,
    command: PartyCommand,
    rng: &mut R,
) -> (String, String) {
    match member.personality {
        Personality::Player => {
            ("".to_string(), format!("{}は　指示どおり行動した！", member.name))
        }
        Personality::Slacker => {
            let roll = rng.gen_range(0..3);
            let reason = if command == PartyCommand::Defend {
                format!("{}は「守るなんてダサいっしょ〜」とヘラヘラしている！", member.name)
            } else {
                format!("{}は　指示をきかず　そっぽを向いた！", member.name)
            };
            let action = match roll {
                0 => format!("{}は　あくびをして　爪をといでいる！", member.name),
                1 => format!("{}は　くだらないダジャレを言った！　だれも笑っていない。", member.name),
                _ => format!("{}は　勝手にポケットのパンをかじっている！", member.name),
            };
            (reason, action)
        }
        Personality::Coward => {
            let reason = if command == PartyCommand::DesperateAttack {
                format!("{}は「そんなの無理です！」と涙目で拒絶した！", member.name)
            } else if command == PartyCommand::Defend {
                format!("{}は　パニックを起こして指示が耳に入っていない！", member.name)
            } else {
                format!("{}は　恐怖で足がすくんでいる！", member.name)
            };
            let action = format!("{}は　後ろの岩陰へ逃げ込もうとした！", member.name);
            (reason, action)
        }
        Personality::Yandere => {
            if command == PartyCommand::Defend {
                let reason = format!(
                    "{}は「あなたが危険なのに、私だけ身を守るなんて……！」と叫んだ！",
                    member.name
                );
                let action = format!(
                    "{}は　あなたの前に飛び出し、両手を広げて立ちはだかった！",
                    member.name
                );
                (reason, action)
            } else {
                let reason = format!("{}は　虚ろな瞳で　あなたを凝視している……", member.name);
                let action = format!(
                    "{}は「私の邪魔をしないで……」とつぶやき、勝手に敵を滅多刺しにした！",
                    member.name
                );
                (reason, action)
            }
        }
        Personality::Loyal => {
            let reason = if command == PartyCommand::DesperateAttack {
                format!("{}は「あなたを残して死ねません！」と諫言した！", member.name)
            } else if command == PartyCommand::Defend {
                format!("{}は「ここは私が攻めるべきです！」と進言した！", member.name)
            } else {
                format!("{}は　とっさの判断で指示と違う行動をとった！", member.name)
            };
            let action = format!("{}は　周囲の警戒を固めている！", member.name);
            (reason, action)
        }

    }
}
