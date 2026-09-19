//! ADR-0022 / ADR-0023: 素性（出自）× 旅立ちの動機 キャラクリシステム
//!
//! 属性軸（どういう人物か）と動機軸（なぜ旅に出たか）の2軸分離による
//! キャラクター作成・初期化ロジック。

use super::{
    Influence, MentalState, PartyMember, Personality, PlayerInventory, PlayerSkills, ReserveRoster,
};

/// 属性軸（出自 / 社会的出自・職業的背景）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OriginKind {
    /// 経済系: 豪商の御曹司。路銀と物資は潤沢だが武芸は素人。
    MerchantSon,
    /// 武力系: 歴戦の傭兵。高いHPと観察眼を持つが懐は寂しい。
    FormerMercenary,
    /// 知識系: 神殿出身。高い魔術知識と目星、豊富なMPを持つ。
    FormerAcolyte,
    /// 没落/底辺系: パンピー代表。粘り強い肉体以外は何もない。
    #[default]
    FarmerThirdSon,
}

impl OriginKind {
    pub const ALL: [OriginKind; 4] = [
        OriginKind::MerchantSon,
        OriginKind::FormerMercenary,
        OriginKind::FormerAcolyte,
        OriginKind::FarmerThirdSon,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            OriginKind::MerchantSon => "商人の息子",
            OriginKind::FormerMercenary => "傭兵崩れ",
            OriginKind::FormerAcolyte => "元神官見習い",
            OriginKind::FarmerThirdSon => "農家の三男坊",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            OriginKind::MerchantSon => "経済系",
            OriginKind::FormerMercenary => "武力系",
            OriginKind::FormerAcolyte => "知識系",
            OriginKind::FarmerThirdSon => "没落／底辺系",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            OriginKind::MerchantSon => {
                "豪商の家に生まれ、計算と交渉に長けた青年。\n路銀と備蓄は潤沢だが、腕っぷしは素人並み。"
            }
            OriginKind::FormerMercenary => {
                "戦場を渡り歩いてきた歴戦の傭兵。\nタフな肉体と観察眼を持つが、財布は常に寂しい。"
            }
            OriginKind::FormerAcolyte => {
                "神殿で祈りと学問に捧げた元修道士。\n魔術や超常現象の知識に明るく、豊富なMPを持つ。"
            }
            OriginKind::FarmerThirdSon => {
                "家督を継げず村を出たパンピー（非英雄）。\n頑丈で粘り強いが、特別な知識も金もない。"
            }
        }
    }

    pub fn bonus_summary(&self) -> &'static str {
        match self {
            OriginKind::MerchantSon => "初期資金: 350G / HP: 20 / MP: 5 / 特やくそう・松明多数",
            OriginKind::FormerMercenary => "初期資金: 80G / HP: 32 / MP: 0 / やくそう・松明",
            OriginKind::FormerAcolyte => "初期資金: 120G / HP: 18 / MP: 20 / 魔術知識・目星高 / どくけし草",
            OriginKind::FarmerThirdSon => "初期資金: 40G / HP: 24 / MP: 0 / 頑丈なパンピースタート",
        }
    }

    pub fn base_stats(&self) -> (i32, i32) {
        match self {
            OriginKind::MerchantSon => (20, 5),
            OriginKind::FormerMercenary => (32, 0),
            OriginKind::FormerAcolyte => (18, 20),
            OriginKind::FarmerThirdSon => (24, 0),
        }
    }

    pub fn base_skills(&self) -> PlayerSkills {
        match self {
            OriginKind::MerchantSon => PlayerSkills {
                magic_knowledge: 20,
                keen_eye: 40,
            },
            OriginKind::FormerMercenary => PlayerSkills {
                magic_knowledge: 10,
                keen_eye: 50,
            },
            OriginKind::FormerAcolyte => PlayerSkills {
                magic_knowledge: 65,
                keen_eye: 60,
            },
            OriginKind::FarmerThirdSon => PlayerSkills {
                magic_knowledge: 15,
                keen_eye: 30,
            },
        }
    }

    pub fn base_gold(&self) -> i32 {
        match self {
            OriginKind::MerchantSon => 350,
            OriginKind::FormerMercenary => 80,
            OriginKind::FormerAcolyte => 120,
            OriginKind::FarmerThirdSon => 40,
        }
    }

    pub fn base_items(&self) -> Vec<String> {
        match self {
            OriginKind::MerchantSon => vec![
                "やくそう".into(),
                "やくそう".into(),
                "やくそう".into(),
                "特やくそう".into(),
                "松明".into(),
                "松明".into(),
            ],
            OriginKind::FormerMercenary => {
                vec!["やくそう".into(), "やくそう".into(), "松明".into()]
            }
            OriginKind::FormerAcolyte => vec![
                "やくそう".into(),
                "どくけし草".into(),
                "どくけし草".into(),
                "松明".into(),
            ],
            OriginKind::FarmerThirdSon => {
                vec!["やくそう".into(), "やくそう".into(), "松明".into()]
            }
        }
    }
}

/// 動機軸（旅立ちの動機 / なぜ今旅をしているのか）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MotiveKind {
    /// 見果てぬ大地と栄誉を夢見て旅立った標準スタイル。
    #[default]
    AspiringAdventurer,
    /// 恋人と手に手を取って故郷を出奔した。高信頼の相棒と二人旅。
    Eloper,
    /// 姿を消した大切な誰かを探している。聞き込みで正体を追う。
    Seeker,
    /// 過去の悲劇に対し報いを受けさせるため刃を研ぐ復讐者。
    Avenger,
    /// 濡れ衣（あるいは罪）により追手から逃げ延びてきた逃亡者。
    Fugitive,
}

impl MotiveKind {
    pub const ALL: [MotiveKind; 5] = [
        MotiveKind::AspiringAdventurer,
        MotiveKind::Eloper,
        MotiveKind::Seeker,
        MotiveKind::Avenger,
        MotiveKind::Fugitive,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            MotiveKind::AspiringAdventurer => "冒険者志望",
            MotiveKind::Eloper => "駆け落ち者",
            MotiveKind::Seeker => "探し人",
            MotiveKind::Avenger => "復讐者",
            MotiveKind::Fugitive => "お尋ね者",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MotiveKind::AspiringAdventurer => {
                "見果てぬ大地と栄誉を夢見て旅立った若者。\n誰からも警戒されず、自由に行動できる。"
            }
            MotiveKind::Eloper => {
                "身分違いの恋人と手に手を取って故郷を出奔した。\n最初から信頼度の高い相棒（2人旅）で始まる。"
            }
            MotiveKind::Seeker => {
                "忽然と姿を消した大切な誰かを探して旅を続ける。\n街の住人への聞き込みで手がかりを手繰り寄せる。"
            }
            MotiveKind::Avenger => {
                "過去の悲劇に報いを受けさせるため刃を研ぎ続ける。\n復讐の執念により生命力が底上げされている。"
            }
            MotiveKind::Fugitive => {
                "とある事件で追われる身となり、逃げ延びてきた。\n隠し金を持つが、衛兵や役人の視線に晒される。"
            }
        }
    }

    pub fn bonus_summary(&self) -> &'static str {
        match self {
            MotiveKind::AspiringAdventurer => "同行者: なし（単独） / 標準的な旅立ち",
            MotiveKind::Eloper => "同行者: 相棒1名（初期影響度88） / 話題「駆け落ちの道行」",
            MotiveKind::Seeker => "同行者: なし / 固定話題「人を探している」を獲得",
            MotiveKind::Avenger => "同行者: なし / HP+4 ボーナス / 固定話題「復讐の誓い」",
            MotiveKind::Fugitive => "同行者: なし / 逃亡資金+100G / 固定話題「追手からの逃亡」",
        }
    }

    pub fn fixed_topic(&self) -> Option<&'static str> {
        match self {
            MotiveKind::AspiringAdventurer => None,
            MotiveKind::Eloper => Some("駆け落ちの道行"),
            MotiveKind::Seeker => Some("人を探している"),
            MotiveKind::Avenger => Some("復讐の誓い"),
            MotiveKind::Fugitive => Some("追手からの逃亡"),
        }
    }
}

/// 駆け落ち相手の候補（ADR-0022 プール制）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EloperPartnerCandidate {
    pub name: String,
    pub job: String,
    pub personality: Personality,
    pub hp: i32,
    pub mp: i32,
    pub backstory: String,
}

impl EloperPartnerCandidate {
    pub fn default_candidates() -> Vec<Self> {
        vec![
            Self {
                name: "エレナ".into(),
                job: "村娘".into(),
                personality: Personality::Loyal,
                hp: 20,
                mp: 10,
                backstory: "共に育った幼馴染。身分違いの恋のため、家を捨ててあなたと共に旅立った心優しい少女。".into(),
            },
            Self {
                name: "クラリス".into(),
                job: "元修道女".into(),
                personality: Personality::Yandere,
                hp: 18,
                mp: 16,
                backstory: "没落貴族の令嬢。修道院に入れられそうになったところをあなたと駆け落ちした。あなたに強い執着を持つ。".into(),
            },
            Self {
                name: "コレット".into(),
                job: "町娘".into(),
                personality: Personality::Loyal,
                hp: 22,
                mp: 8,
                backstory: "豪商の末娘。親が決めた政略結婚から逃れ、自由と愛を信じてあなたの手を取った気丈な女性。".into(),
            },
        ]
    }
}

/// キャラクター作成の結果設定
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterBuild {
    pub origin: OriginKind,
    pub motive: MotiveKind,
    pub partner: Option<EloperPartnerCandidate>,
}

impl Default for CharacterBuild {
    fn default() -> Self {
        Self {
            origin: OriginKind::FarmerThirdSon,
            motive: MotiveKind::AspiringAdventurer,
            partner: None,
        }
    }
}

impl CharacterBuild {
    pub fn new(origin: OriginKind, motive: MotiveKind) -> Self {
        let partner = if motive == MotiveKind::Eloper {
            EloperPartnerCandidate::default_candidates().into_iter().next()
        } else {
            None
        };
        Self {
            origin,
            motive,
            partner,
        }
    }

    /// 主人公の初期ステータス（HP, MP）を算出
    pub fn calculate_player_stats(&self) -> (i32, i32) {
        let (mut hp, mp) = self.origin.base_stats();
        if self.motive == MotiveKind::Avenger {
            hp += 4; // 復讐者の執念ボーナス
        }
        (hp, mp)
    }

    /// 主人公の初期スキルを算出
    pub fn calculate_player_skills(&self) -> PlayerSkills {
        self.origin.base_skills()
    }

    /// 初期インベントリ（所持金、アイテム、話題）を生成
    pub fn create_inventory(&self) -> PlayerInventory {
        let mut gold = self.origin.base_gold();
        if self.motive == MotiveKind::Fugitive {
            gold += 100; // お尋ね者の逃亡資金
        }

        let items = self.origin.base_items();

        let mut topics = vec!["王都アルカン".into(), "封魔の迷宮".into()];
        if let Some(fixed) = self.motive.fixed_topic() {
            topics.push(fixed.into());
        }

        PlayerInventory {
            gold,
            items,
            topics,
        }
    }

    /// 初期パーティメンバーを生成
    pub fn create_party_members(&self, player_name: &str) -> Vec<PartyMember> {
        let (hp, mp) = self.calculate_player_stats();
        let player = PartyMember::new_player(player_name).with_stats(hp, mp);

        let mut members = vec![player];

        if self.motive == MotiveKind::Eloper {
            if let Some(partner) = &self.partner {
                // ADR-0022: 初期影響度は自然上限89に近い高値（88）
                let partner_member = PartyMember::new(
                    &partner.name,
                    &partner.job,
                    Influence::new_natural(88),
                    MentalState::Normal,
                    partner.personality,
                )
                .with_stats(partner.hp, partner.mp);
                members.push(partner_member);
            }
        }

        members
    }

    /// 酒場・控えメンバー名簿を生成
    pub fn create_reserve_roster(&self) -> ReserveRoster {
        ReserveRoster::new(vec![
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
                Influence::new_natural(88),
                MentalState::Normal,
                Personality::Yandere,
            )
            .with_stats(22, 25),
            PartyMember::new(
                "騎士アルヴィン",
                "きし",
                Influence::new_supernatural(115),
                MentalState::Charmed,
                Personality::Loyal,
            )
            .with_stats(40, 15),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_merchant_son() {
        let build = CharacterBuild::new(OriginKind::MerchantSon, MotiveKind::AspiringAdventurer);
        let (hp, mp) = build.calculate_player_stats();
        assert_eq!(hp, 20);
        assert_eq!(mp, 5);

        let inv = build.create_inventory();
        assert_eq!(inv.gold, 350);
        assert!(inv.has_item("特やくそう"));
        assert!(inv.has_item("松明"));

        let skills = build.calculate_player_skills();
        assert_eq!(skills.magic_knowledge, 20);
        assert_eq!(skills.keen_eye, 40);

        let members = build.create_party_members("アルト");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "アルト");
    }

    #[test]
    fn test_motive_eloper_party_and_influence() {
        let mut build = CharacterBuild::new(OriginKind::FarmerThirdSon, MotiveKind::Eloper);
        let candidate = EloperPartnerCandidate::default_candidates().remove(0);
        build.partner = Some(candidate);

        let members = build.create_party_members("主人公");
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].name, "主人公");
        assert_eq!(members[1].name, "エレナ");
        assert_eq!(members[1].job, "村娘");
        assert_eq!(members[1].influence.raw_value, 88);
        assert_eq!(members[1].personality, Personality::Loyal);

        let inv = build.create_inventory();
        assert!(inv.has_topic("駆け落ちの道行"));
    }

    #[test]
    fn test_motive_seeker_topic() {
        let build = CharacterBuild::new(OriginKind::FormerAcolyte, MotiveKind::Seeker);
        let inv = build.create_inventory();
        assert!(inv.has_topic("人を探している"));
        assert_eq!(build.create_party_members("探索者").len(), 1);
    }

    #[test]
    fn test_motive_avenger_hp_bonus() {
        let build = CharacterBuild::new(OriginKind::FormerMercenary, MotiveKind::Avenger);
        let (hp, _) = build.calculate_player_stats();
        // FormerMercenary(32) + Avenger(+4) = 36
        assert_eq!(hp, 36);

        let inv = build.create_inventory();
        assert!(inv.has_topic("復讐の誓い"));
    }

    #[test]
    fn test_motive_fugitive_gold_bonus() {
        let build = CharacterBuild::new(OriginKind::FarmerThirdSon, MotiveKind::Fugitive);
        let inv = build.create_inventory();
        // FarmerThirdSon(40) + Fugitive(+100) = 140
        assert_eq!(inv.gold, 140);
        assert!(inv.has_topic("追手からの逃亡"));
    }
}
