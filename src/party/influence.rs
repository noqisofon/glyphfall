use bevy::prelude::*;

/// 素の人間（世界最強の人たらしクラス）の自然な影響度上限
pub const MAX_NATURAL_INFLUENCE: u8 = 89;

/// 精神異常とみなされる影響度の閾値
pub const ABNORMAL_INFLUENCE_THRESHOLD: u8 = 100;

/// パーティメンバーの服従度を司る隠しパラメータ
#[derive(Component, Debug, Clone, Copy)]
pub struct Influence {
    /// 内部的な影響度の実数値 (0〜255)
    pub raw_value: u8,
}

impl Influence {
    /// 自然な範囲での影響度を生成 (上限89でクランプ)
    pub fn new_natural(val: u8) -> Self {
        Self {
            raw_value: val.min(MAX_NATURAL_INFLUENCE),
        }
    }

    /// 魔術や呪術による異常な影響度を生成 (100以上も許容)
    pub fn new_supernatural(val: u8) -> Self {
        Self { raw_value: val }
    }

    /// 素の人間として影響度を上昇させる（自然上限89で頭打ち）
    #[allow(dead_code)]
    pub fn add_natural(&mut self, amount: u8) {
        self.raw_value = self.raw_value.saturating_add(amount).min(MAX_NATURAL_INFLUENCE);
    }


    /// 異常な領域（外的魔術介入レベル）に達しているか
    pub fn is_abnormal(&self) -> bool {
        self.raw_value >= ABNORMAL_INFLUENCE_THRESHOLD
    }
}

/// 精神状態（外的介入の有無）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MentalState {
    /// 正常（素の人間）
    Normal,
    /// 魅了・チャーム（魔術による強制的な執着・服従）
    Charmed,
    /// 呪縛・精神汚染
    Cursed,
}

/// キャラクターの性格・特性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Personality {

    /// 忠実：誠実で命令に従いやすいが、上限89の制約は受ける
    Loyal,
    /// 遊び人：気まぐれでサボり率が高く、影響度が高くてもふざける
    Slacker,
    /// 臆病：危険な指示や強敵に対して逃亡・拒否しやすい
    Coward,
    /// ヤンデレ：主人公への過剰な執着。指示には従うが独断専行しやすく、魅了と酷似する
    Yandere,
}

/// パーティメンバーのコンポーネント
#[derive(Component, Debug, Clone)]
pub struct PartyMember {
    pub name: String,
    pub job: String,
    pub influence: Influence,
    pub mental_state: MentalState,
    pub personality: Personality,
    pub hp: i32,
    pub max_hp: i32,
    pub mp: i32,
    pub max_mp: i32,
}

impl PartyMember {
    pub fn new(
        name: impl Into<String>,
        job: impl Into<String>,
        influence: Influence,
        mental_state: MentalState,
        personality: Personality,
    ) -> Self {
        Self {
            name: name.into(),
            job: job.into(),
            influence,
            mental_state,
            personality,
            hp: 30,
            max_hp: 30,
            mp: 10,
            max_mp: 10,
        }
    }
}
