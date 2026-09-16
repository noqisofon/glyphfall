use crate::party::{PartyMember, PlayerInventory};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialoguePartner {
    Inn,
    Shop,
    Tavern,
    Guard,
    Villager,
    Suspicious,
}

impl DialoguePartner {
    pub fn name(&self) -> &'static str {
        match self {
            DialoguePartner::Inn => "宿屋の主人",
            DialoguePartner::Shop => "道具屋の店主",
            DialoguePartner::Tavern => "呑兵衛の冒険者",
            DialoguePartner::Guard => "王都衛兵",
            DialoguePartner::Villager => "街の女性",
            DialoguePartner::Suspicious => "裏通りの怪しい男",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShopItem {
    pub name: &'static str,
    pub price: i32,
    pub description: &'static str,
}

pub const SHOP_ITEMS: [ShopItem; 4] = [
    ShopItem {
        name: "やくそう",
        price: 8,
        description: "傷を癒やす薬草。HPを約30回復。",
    },
    ShopItem {
        name: "特やくそう",
        price: 25,
        description: "上質な薬草。HPを約70回復。",
    },
    ShopItem {
        name: "松明",
        price: 15,
        description: "迷宮を照らす松明の油。視界確保用。",
    },
    ShopItem {
        name: "どくけし草",
        price: 12,
        description: "体内の毒素を中和する薬草。",
    },
];

/// メッセージ本文（`current_text`）中で、覚えられる対象語が占める文字範囲。
/// 文字インデックスは`current_text.chars()`基準（改行も1文字に数える）。
/// 覚える文字列自体はここでは保持せず、常に`current_text`からスライスして得る
/// （下線位置と覚えた単語が構造的にズレないようにするため。ADR-0012）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LearnableSpan {
    pub start: usize,
    pub end: usize,
}

impl LearnableSpan {
    pub fn slice(&self, text: &str) -> String {
        text.chars().skip(self.start).take(self.end - self.start).collect()
    }
}

/// テキスト中から`words`の各語句を検索し、見つかった範囲を`LearnableSpan`として返す。
/// 見つからない語句は黙って無視する（下線が付かないだけで、致命的な不整合にはしない）。
fn spans_for(text: &str, words: &[&str]) -> Vec<LearnableSpan> {
    let chars: Vec<char> = text.chars().collect();
    words
        .iter()
        .filter_map(|word| {
            let word_chars: Vec<char> = word.chars().collect();
            if word_chars.is_empty() || word_chars.len() > chars.len() {
                return None;
            }
            chars
                .windows(word_chars.len())
                .position(|w| w == word_chars.as_slice())
                .map(|start| LearnableSpan {
                    start,
                    end: start + word_chars.len(),
                })
        })
        .collect()
}

/// 「おぼえる」コマンドの進行段階（ADR-0011の`CommandMenuStage`と同様の2段階パターン）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DialogueLearnStage {
    #[default]
    Talking,
    ChoosingLearnTarget {
        cursor: usize,
    },
}

#[derive(Clone, Debug)]
pub struct DialogueSession {
    pub partner: DialoguePartner,
    pub selected_topic_index: usize,
    pub selected_shop_index: usize,
    pub current_text: String,
    pub learnable_spans: Vec<LearnableSpan>,
    pub learn_stage: DialogueLearnStage,
}

impl DialogueSession {
    pub fn start(partner: DialoguePartner) -> Self {
        let (initial_text, learnable_spans) = match partner {
            DialoguePartner::Inn => (
                "宿屋の主人「旅の方かい？\n一泊50フォリンで仲間全員の体力を全快できるよ。[1]で宿泊するかい？」".into(),
                Vec::new(),
            ),
            DialoguePartner::Shop => (
                "道具屋「へいらっしゃい！何にするかい？\n[W/S]で選んで[1]で購入、[3]で店を出られるぜ」".into(),
                Vec::new(),
            ),
            DialoguePartner::Tavern => (
                "呑兵衛「ヒック…王都の酒は最高だな！\n何か気になることでもあんのかい？話題を振ってくれな」".into(),
                Vec::new(),
            ),
            DialoguePartner::Guard => (
                "王都衛兵「止まれ！この先は立ち入り禁止の封鎖迷宮だ！\n何か事件の手がかりでも掴んだのか？」".into(),
                Vec::new(),
            ),
            DialoguePartner::Villager => (
                "街の女性「こんにちは、旅の冒険者さん。\n王都アルカンについて何かお知りになりたいですか？」".into(),
                Vec::new(),
            ),
            DialoguePartner::Suspicious => (
                "怪しい男「ヒヒッ…あんた、裏の事情に首を突っ込みたいのかい？\nいい情報を持ってるぜ…」".into(),
                Vec::new(),
            ),
        };

        Self {
            partner,
            selected_topic_index: 0,
            selected_shop_index: 0,
            current_text: initial_text,
            learnable_spans,
            learn_stage: DialogueLearnStage::default(),
        }
    }

    /// 話題を振る
    pub fn ask_topic(&mut self, topic: &str) {
        // 新しい話題を振ったら、進行中の「おぼえる」候補選択は必ずキャンセルする。
        self.learn_stage = DialogueLearnStage::Talking;

        match self.partner {
            DialoguePartner::Guard => match topic {
                "王都アルカン" => {
                    self.current_text = "王都衛兵「王都アルカンは平和な街だ。だが南東の地下迷宮だけは絶対近寄るなよ」".into();
                    self.learnable_spans = Vec::new();
                }
                "封魔の迷宮" => {
                    self.current_text = "王都衛兵「かつて大魔王軍を封じた迷宮だ。奥深くには封印の祭壇があると言われている…」".into();
                    self.learnable_spans = spans_for(&self.current_text, &["封印の祭壇"]);
                }
                "封印の祭壇" => {
                    self.current_text = "王都衛兵「祭壇の封印を解くには、古の光のオーブが必要だと古文書に記されているらしい」".into();
                    self.learnable_spans = spans_for(&self.current_text, &["光のオーブ"]);
                }
                _ => {
                    self.current_text = format!("王都衛兵「『{}』だと？…すまんが俺の知るところではないな」", topic);
                    self.learnable_spans = Vec::new();
                }
            },
            DialoguePartner::Tavern => match topic {
                "封魔の迷宮" => {
                    self.current_text = "呑兵衛「ヒック…夜になると地下から『魔物の咆哮』が聞こえてくるんだよ…不気味だぜ」".into();
                    self.learnable_spans = spans_for(&self.current_text, &["魔物の咆哮"]);
                }
                "封印の祭壇" => {
                    self.current_text = "呑兵衛「祭壇か！そういや爺さんが光のオーブを古物商に売っちまったとか言ってたな…」".into();
                    self.learnable_spans = spans_for(&self.current_text, &["光のオーブ"]);
                }
                "光のオーブ" => {
                    self.current_text = "呑兵衛「オーブなら、裏路地にいる怪しい男がヤバいルートを握ってるらしいぜ…」".into();
                    self.learnable_spans = spans_for(&self.current_text, &["裏の抜け道"]);
                }
                _ => {
                    self.current_text = format!("呑兵衛「『{}』かぁ？知らねえな！酒がうめえ！」", topic);
                    self.learnable_spans = Vec::new();
                }
            },
            DialoguePartner::Suspicious => match topic {
                "光のオーブ" => {
                    self.current_text = "怪しい男「ヒヒッ…オーブの話かい？地下迷宮の宝箱に隠された銀の鍵があれば手に入るぜ…」".into();
                    self.learnable_spans = spans_for(&self.current_text, &["地下迷宮", "銀の鍵"]);
                }
                "銀の鍵" => {
                    self.current_text = "怪しい男「迷宮の北の宝物庫だ。鉄格子の奥の宝箱に入ってるはずだぜ…ヒヒッ」".into();
                    self.learnable_spans = Vec::new();
                }
                "裏の抜け道" => {
                    self.current_text = "怪しい男「裏壁の崩れかけのレンガを押せば、見張りを通らずに裏口へ行けるのさ」".into();
                    self.learnable_spans = Vec::new();
                }
                _ => {
                    self.current_text = format!("怪しい男「ヒヒッ…『{}』かい？あっしには関係ねえ話だな」", topic);
                    self.learnable_spans = Vec::new();
                }
            },
            DialoguePartner::Villager => match topic {
                "王都アルカン" => {
                    self.current_text = "街の女性「中央の噴水広場は憩いの場なんです。夜は少し冷えますから気をつけて」".into();
                    self.learnable_spans = Vec::new();
                }
                "封魔の迷宮" => {
                    self.current_text = "街の女性「きゃあっ！そんな恐ろしい迷宮、お願いですから近づかないでください！」".into();
                    self.learnable_spans = Vec::new();
                }
                _ => {
                    self.current_text = format!("街の女性「『{}』ですか？私にはよくわかりませんね…」", topic);
                    self.learnable_spans = Vec::new();
                }
            },
            _ => {
                self.current_text = format!("「『{}』についてですね。よく覚えておきましょう」", topic);
                self.learnable_spans = Vec::new();
            }
        }
    }

    /// 宿屋に泊まる処理
    pub fn rest_at_inn(&mut self, inv: &mut PlayerInventory, members: &mut [PartyMember]) -> bool {
        if inv.spend_gold(50) {
            for m in members.iter_mut() {
                m.hp = m.max_hp;
                m.mp = m.max_mp;
            }
            self.current_text = "宿屋の主人「まいど！ぐっすり休んでいきなよ」\n宿をとった。朝の光が差し込み、仲間全員のHPとMPが全快した！".into();
            true
        } else {
            self.current_text = "宿屋の主人「おや、フォリンが足りないようだね。一泊50フォリンだよ」".into();
            false
        }
    }

    /// 道具屋で買い物
    pub fn buy_item(&mut self, inv: &mut PlayerInventory) -> bool {
        let item = &SHOP_ITEMS[self.selected_shop_index];
        if inv.spend_gold(item.price) {
            inv.add_item(item.name);
            self.current_text = format!(
                "道具屋「まいど！【{}】をお買い上げだ。\n大切に使いなよ！」(所持金: {}G)",
                item.name, inv.gold
            );
            true
        } else {
            self.current_text = format!(
                "道具屋「おいおい、フォリンが足りねえぜ！【{}】は {}G だ」(所持金: {}G)",
                item.name, item.price, inv.gold
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::{Influence, MentalState, Personality};

    fn create_dummy_member(name: &'static str, hp: i32, max_hp: i32, mp: i32, max_mp: i32) -> PartyMember {
        let mut member = PartyMember::new(
            name,
            "戦士",
            Influence::new_natural(30),
            MentalState::Normal,
            Personality::Loyal,
        );
        member.hp = hp;
        member.max_hp = max_hp;
        member.mp = mp;
        member.max_mp = max_mp;
        member
    }

    #[test]
    fn test_dialogue_partner_name() {
        assert_eq!(DialoguePartner::Inn.name(), "宿屋の主人");
        assert_eq!(DialoguePartner::Shop.name(), "道具屋の店主");
        assert_eq!(DialoguePartner::Guard.name(), "王都衛兵");
        assert_eq!(DialoguePartner::Tavern.name(), "呑兵衛の冒険者");
    }

    #[test]
    fn test_inn_rest() {
        let mut session = DialogueSession::start(DialoguePartner::Inn);
        let mut inv = PlayerInventory::default();
        inv.gold = 60;
        let mut members = vec![
            create_dummy_member("ガルツ", 10, 50, 0, 10),
            create_dummy_member("ミレイ", 5, 25, 2, 40),
        ];

        // 宿泊成功
        let success = session.rest_at_inn(&mut inv, &mut members);
        assert!(success);
        assert_eq!(inv.gold, 10);
        assert_eq!(members[0].hp, 50);
        assert_eq!(members[0].mp, 10);
        assert_eq!(members[1].hp, 25);
        assert_eq!(members[1].mp, 40);

        // フォリン不足で宿泊失敗
        let success2 = session.rest_at_inn(&mut inv, &mut members);
        assert!(!success2);
        assert_eq!(inv.gold, 10);
    }

    #[test]
    fn test_shop_purchase() {
        let mut session = DialogueSession::start(DialoguePartner::Shop);
        let mut inv = PlayerInventory::default();
        inv.gold = 20;
        session.selected_shop_index = 0; // やくそう (8G)

        let success = session.buy_item(&mut inv);
        assert!(success);
        assert_eq!(inv.gold, 12);
        assert!(inv.items.contains(&"やくそう".to_string()));

        // 高額アイテム購入テスト（特やくそう 25G）
        session.selected_shop_index = 1;
        let success2 = session.buy_item(&mut inv);
        assert!(!success2); // 12Gしかないので失敗
        assert_eq!(inv.gold, 12);
    }

    #[test]
    fn test_dialogue_topic_chain() {
        let mut session = DialogueSession::start(DialoguePartner::Guard);

        // ガードに「封魔の迷宮」を聞く
        session.ask_topic("封魔の迷宮");
        assert_eq!(learned_words(&session), vec!["封印の祭壇".to_string()]);

        // ガードに覚えた「封印の祭壇」を聞く
        session.ask_topic("封印の祭壇");
        assert_eq!(learned_words(&session), vec!["光のオーブ".to_string()]);

        // 無関係な話題
        session.ask_topic("ピザのレシピ");
        assert!(session.learnable_spans.is_empty());
    }

    #[test]
    fn test_dialogue_multiple_learnable_spans() {
        let mut session = DialogueSession::start(DialoguePartner::Suspicious);

        session.ask_topic("光のオーブ");
        assert_eq!(
            learned_words(&session),
            vec!["地下迷宮".to_string(), "銀の鍵".to_string()]
        );
    }

    #[test]
    fn test_ask_topic_cancels_pending_learn_selection() {
        let mut session = DialogueSession::start(DialoguePartner::Suspicious);
        session.ask_topic("光のオーブ");
        session.learn_stage = DialogueLearnStage::ChoosingLearnTarget { cursor: 1 };

        session.ask_topic("銀の鍵");
        assert_eq!(session.learn_stage, DialogueLearnStage::Talking);
    }

    fn learned_words(session: &DialogueSession) -> Vec<String> {
        session
            .learnable_spans
            .iter()
            .map(|span| span.slice(&session.current_text))
            .collect()
    }
}

