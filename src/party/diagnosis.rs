use rand::Rng;
use super::influence::{PartyMember, Personality, MentalState};

/// 主人公（パンピー）の観察・判定スキル
#[derive(Debug, Clone, Copy)]
pub struct PlayerSkills {
    /// 魔術の知識 (0〜100): 呪文の残滓や魔力波長を感知する力
    pub magic_knowledge: u8,
    /// 目星 (0〜100): 表情、脈拍、視線の違和感などの観察力
    pub keen_eye: u8,
}

impl Default for PlayerSkills {
    fn default() -> Self {
        Self {
            magic_knowledge: 45,
            keen_eye: 55,
        }
    }
}

/// 観察・診察の判定レポート
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisReport {
    /// 判定に成功したか（異常か正常かを正しく見抜けたか）
    pub success: bool,
    /// メインの描写メッセージ
    pub observation_msg: String,
    /// 詳細・結論メッセージ
    pub conclusion_msg: String,
}

/// 仲間の精神状態・執着を「魔術の知識」＋「目星」で観察する
pub fn diagnose_member<R: Rng>(
    skills: &PlayerSkills,
    member: &PartyMember,
    rng: &mut R,
) -> DiagnosisReport {
    // 目星ロール (1〜100)
    let eye_roll: u8 = rng.gen_range(1..=100);
    let eye_success = eye_roll <= skills.keen_eye;

    // 魔術の知識ロール (1〜100)
    let magic_roll: u8 = rng.gen_range(1..=100);
    let magic_success = magic_roll <= skills.magic_knowledge;

    match (member.mental_state, member.personality) {
        // 1. 魅了（Charmed）されている場合
        (MentalState::Charmed, _) => {
            let obs = format!(
                "{}の　ようすを　注意深く観察した。\n瞳孔が開き、うっとりとあなたを見つめている……",
                member.name
            );

            if magic_success && eye_success {
                DiagnosisReport {
                    success: true,
                    observation_msg: obs,
                    conclusion_msg: "【看破】こめかみに微かな紫の魔力光を感知した！\nこれは素の愛情ではない……魅了呪文に操られている！".to_string(),
                }
            } else if eye_success && !magic_success {
                // 目星だけで魔術知識がない場合：素の狂信と誤認
                DiagnosisReport {
                    success: false,
                    observation_msg: obs,
                    conclusion_msg: "【誤認】全身全霊であなたに心酔しているようだ。\n……しかし、どこか表情筋が不自然に固定されている気もする。".to_string(),
                }
            } else {
                DiagnosisReport {
                    success: false,
                    observation_msg: obs,
                    conclusion_msg: "あなたに熱烈な信頼を寄せているように見える。特に不審な点はない。".to_string(),
                }
            }
        }

        // 2. 呪縛（Cursed）されている場合
        (MentalState::Cursed, _) => {
            let obs = format!(
                "{}の　ようすを　注意深く観察した。\n時折、何かに怯えるように首筋を掻きむしっている……",
                member.name
            );

            if magic_success {
                DiagnosisReport {
                    success: true,
                    observation_msg: obs,
                    conclusion_msg: "【看破】肌の奥にどす黒い呪術の残滓が渦巻いている！\n強い呪詛によって精神を蝕まれているようだ。".to_string(),
                }
            } else {
                DiagnosisReport {
                    success: false,
                    observation_msg: obs,
                    conclusion_msg: "単に旅の疲労か、虫刺されを気にしているだけのようだ。".to_string(),
                }
            }
        }

        // 3. 精神は正常だが、性格が「ヤンデレ（Yandere）」の場合
        (MentalState::Normal, Personality::Yandere) => {
            let obs = format!(
                "{}の　ようすを　注意深く観察した。\nこちらが一歩動くたび、瞬きもせずその視線が追ってくる……",
                member.name
            );

            if magic_success && eye_success {
                DiagnosisReport {
                    success: true,
                    observation_msg: obs,
                    conclusion_msg: "【看破】魔術の痕跡は一切ない。完全にシラフだ。\nこれは外的な呪いではなく……純粋な「素の執着」だ……！".to_string(),
                }
            } else if !magic_success && eye_success {
                // 魔術知識が足りず、ヤンデレの異常行動を魔術か何かと疑ってしまう
                DiagnosisReport {
                    success: false,
                    observation_msg: obs,
                    conclusion_msg: "【混乱】あまりに執拗な視線だ。何か精神呪文でも受けているのだろうか？\n（魔術の知識が足りず真偽が判別できない）".to_string(),
                }
            } else {
                DiagnosisReport {
                    success: false,
                    observation_msg: obs,
                    conclusion_msg: "あなたをとても大切に想ってくれているようだ。頼もしい仲間だ。".to_string(),
                }
            }
        }

        // 4. その他の正常な仲間
        (MentalState::Normal, personality) => {
            let obs = format!("{}の　ようすを　注意深く観察した。", member.name);
            let detail = match personality {
                Personality::Loyal => {
                    "背筋を伸ばし、あなたの次の言葉を真摯に待っている。\n精神状態は極めて健全だ。"
                }
                Personality::Slacker => {
                    "鼻歌を歌いながら、道端の小石を蹴っている。\n緊張感はゼロだが、邪気もない。"
                }
                Personality::Coward => {
                    "キョロキョロと周囲を警戒し、いつでも逃げられる体勢をとっている。\nビビっているだけで呪われてはいない。"
                }
                Personality::Yandere => unreachable!(),
            };

            DiagnosisReport {
                success: true,
                observation_msg: obs,
                conclusion_msg: detail.to_string(),
            }
        }
    }
}
