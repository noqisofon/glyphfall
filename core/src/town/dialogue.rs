use crate::party::{PartyMember, PlayerInventory, CURRENCY_NAME, CURRENCY_UNIT};

/// 宿屋の一泊あたりの宿泊費用（フォリン）
pub const INN_COST: i32 = 50;

/// 話題ノードの種別（ADR-0028）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopicKind {
    Entity,    // 魔物・事物（マンティコア）
    Attribute, // 属性（弱点、生態）
    Person,    // 人物・組織（★★騎士団、団長）
    Location,  // 場所・地名（トリポリ、◇◇砦）
}

/// たずねる対象（話題ノード or 文脈代名詞。ADR-0028）
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuerySubject {
    /// 手帳にストックされた話題
    Topic { name: String, kind: TopicKind },
    /// 文脈トークン: 現在地（「ここ」）
    Here,
    /// 文脈トークン: 話し相手（「あなた」）
    You,
    /// 文脈トークン: プレイヤー自身（「私」）
    Me,
}

impl QuerySubject {
    pub fn from_topic_name(name: &str) -> Self {
        let kind = match name {
            "マンティコア" => TopicKind::Entity,
            "弱点" | "生態" => TopicKind::Attribute,
            "★★騎士団" | "★★騎士団団長" => TopicKind::Person,
            "迂回路"
            | "トリポリ"
            | "マンティコア関所"
            | "◇◇砦"
            | "冒険者ギルド"
            | "王都アルカン"
            | "洞穴"
            | "宿屋" => TopicKind::Location,
            _ => TopicKind::Entity,
        };
        QuerySubject::Topic {
            name: name.to_string(),
            kind,
        }
    }
}

/// 聞き方（純粋な疑問詞のみ。ADR-0028）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestionType {
    What,      // 「何？」：定義・生態・客観的事実
    Where,     // 「どこ？」：所在・方角・所要日数
    DoYouKnow, // 「知っている？」：関係性・世間の評判・警戒感
    Who,       // 「誰？」：素性（あなた）または評判（私）
}

pub const QUESTION_TYPES: [(QuestionType, &str); 4] = [
    (QuestionType::What, "何？"),
    (QuestionType::Where, "どこ？"),
    (QuestionType::DoYouKnow, "知っている？"),
    (QuestionType::Who, "誰？"),
];

/// 謁見・面会の社会的障壁（静的データ。ADR-0028）
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccessBarrier {
    /// 第三者が発行した書類・アイテムが必要（例: 紹介状、通行証、免許）
    RequiresItem(String),
    /// 地域・組織の評判が必要（影響度とは別軸の顔見知り度）
    RequiresReputation {
        faction: String,
        min_reputation: i32,
    },
}

impl AccessBarrier {
    /// 所持品と評判値に基づいて障壁を突破可能かを判定（静的条件とプレイヤー状態の分離）
    pub fn is_satisfied(&self, inv: Option<&PlayerInventory>, reputation: i32) -> bool {
        match self {
            AccessBarrier::RequiresItem(item_name) => inv.is_some_and(|i| i.has_item(item_name)),
            AccessBarrier::RequiresReputation { min_reputation, .. } => {
                reputation >= *min_reputation
            }
        }
    }
}

/// 知識に対するNPCの返答（MVP第1段階。ADR-0028）
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KnowledgeResponse {
    /// 1. 完全に知らない（「知らんな」）
    Unknown,
    /// 2. 知らないが、誰に聞けばいいか知っている（ポインタ＋社会的障壁）
    ///    人物や地名は自動開示されず、本文中の下線対象として提示される
    Pointer {
        text: String,
        barrier: Option<AccessBarrier>,
    },
    /// 3. 知識そのものを答える
    Answer { text: String },
}

/// 新人判定のタイミング（ADR-0028）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NewbieJudgment {
    #[default]
    None,
    Before, // 警戒先行型（関所番人）
    After,  // 世話焼き先行型（呑兵衛等）
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialoguePartner {
    Inn,
    Shop,
    Tavern,
    Guard,
    Villager,
    Suspicious,
    CheckpointGuard, // マンティコア関所の番人
    KnightCommander, // ★★騎士団団長
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
            DialoguePartner::CheckpointGuard => "関所の番人",
            DialoguePartner::KnightCommander => "騎士団団長",
        }
    }

    pub fn newbie_judgment(&self) -> NewbieJudgment {
        match self {
            DialoguePartner::CheckpointGuard => NewbieJudgment::Before,
            DialoguePartner::Tavern => NewbieJudgment::After,
            _ => NewbieJudgment::None,
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
        text.chars()
            .skip(self.start)
            .take(self.end - self.start)
            .collect()
    }
}

/// テキスト中から`words`の各語句を検索し、見つかったすべての範囲を本文出現順にソートして返す。
/// 見つからない語句は黙って無視する（下線が付かないだけで、致命的な不整合にはしない）。
fn spans_for(text: &str, words: &[&str]) -> Vec<LearnableSpan> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();

    for &word in words {
        let word_chars: Vec<char> = word.chars().collect();
        if word_chars.is_empty() || word_chars.len() > chars.len() {
            continue;
        }
        let w_len = word_chars.len();
        for i in 0..=chars.len().saturating_sub(w_len) {
            if chars[i..i + w_len] == word_chars[..] {
                let span = LearnableSpan {
                    start: i,
                    end: i + w_len,
                };
                if !spans.contains(&span) {
                    spans.push(span);
                }
            }
        }
    }

    // 出現位置順（start昇順、同じ位置なら長さ降順）にソート
    spans.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| (b.end - b.start).cmp(&(a.end - a.start)))
    });

    // より長いスパンに内包される短いスパンを除外（重なり防止）
    let mut filtered: Vec<LearnableSpan> = Vec::new();
    for span in spans {
        if !filtered
            .iter()
            .any(|f| span.start >= f.start && span.end <= f.end)
        {
            filtered.push(span);
        }
    }
    filtered
}

/// 対話・会話の進行段階（ADR-0012「おぼえる」多段階選択＋ADR-0028「たずねる」名詞先行多段階入力）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DialogueStage {
    #[default]
    Talking,
    SelectingSubject {
        cursor: usize,
    },
    SelectingQuestion {
        subject_cursor: usize,
        cursor: usize,
    },
    ChoosingLearnTarget {
        cursor: usize,
    },
}

pub type DialogueLearnStage = DialogueStage;

#[derive(Clone, Debug)]
pub struct DialogueSession {
    pub partner: DialoguePartner,
    pub selected_topic_index: usize,
    pub selected_shop_index: usize,
    pub current_text: String,
    pub learnable_spans: Vec<LearnableSpan>,
    pub stage: DialogueStage,
}

impl DialogueSession {
    pub fn selectable_subjects(topics: &[String]) -> Vec<(String, QuerySubject)> {
        let mut list = vec![
            ("ここ".to_string(), QuerySubject::Here),
            ("あなた".to_string(), QuerySubject::You),
            ("私".to_string(), QuerySubject::Me),
        ];
        for t in topics {
            list.push((t.clone(), QuerySubject::from_topic_name(t)));
        }
        list
    }

    pub fn start(partner: DialoguePartner) -> Self {
        let (initial_text, learnable_spans) = match partner {
            DialoguePartner::Inn => (
                format!(
                    "宿屋の主人「旅の方かい？\n一泊{}{}で仲間全員の体力を全快できるよ。[1]で宿泊するかい？」",
                    INN_COST, CURRENCY_NAME
                ),
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
            DialoguePartner::CheckpointGuard => (
                "番人「知っての通り、この先はマンティコアの生息地域だ。この先は通れんよ」".into(),
                spans_for(
                    "番人「知っての通り、この先はマンティコアの生息地域だ。この先は通れんよ」",
                    &["マンティコア", "この先"],
                ),
            ),
            DialoguePartner::KnightCommander => (
                "団長「私が★★騎士団の団長だ。貴公に何用か？」".into(),
                spans_for(
                    "団長「私が★★騎士団の団長だ。貴公に何用か？」",
                    &["★★騎士団"],
                ),
            ),
        };

        Self {
            partner,
            selected_topic_index: 0,
            selected_shop_index: 0,
            current_text: initial_text,
            learnable_spans,
            stage: DialogueStage::default(),
        }
    }

    /// 話題を振る
    pub fn ask_topic(&mut self, topic: &str) {
        // 新しい話題を振ったら、進行中の「おぼえる」候補選択は必ずキャンセルする。
        self.stage = DialogueStage::Talking;

        if let Some((text, words)) = self.legacy_topic_reply(topic) {
            self.learnable_spans = spans_for(&text, words);
            self.current_text = text;
            return;
        }

        self.current_text = match self.partner {
            DialoguePartner::Guard => format!(
                "王都衛兵「『{}』だと？…すまんが俺の知るところではないな」",
                topic
            ),
            DialoguePartner::Tavern => {
                format!("呑兵衛「『{}』かぁ？知らねえな！酒がうめえ！」", topic)
            }
            DialoguePartner::Suspicious => format!(
                "怪しい男「ヒヒッ…『{}』かい？あっしには関係ねえ話だな」",
                topic
            ),
            DialoguePartner::Villager => {
                format!("街の女性「『{}』ですか？私にはよくわかりませんね…」", topic)
            }
            _ => format!("「『{}』についてですね。よく覚えておきましょう」", topic),
        };
        self.learnable_spans = Vec::new();
    }

    /// 封魔の迷宮クエスト（旧来の話題振り）の固定返答。
    /// 返答テキストと下線候補の語句を返す。該当しなければ None。
    fn legacy_topic_reply(&self, topic: &str) -> Option<(String, &'static [&'static str])> {
        let (text, words): (&str, &'static [&'static str]) = match (self.partner, topic) {
            (DialoguePartner::Guard, "王都アルカン") => (
                "王都衛兵「王都アルカンは平和な街だ。だが南東の地下迷宮だけは絶対近寄るなよ」",
                &[],
            ),
            (DialoguePartner::Guard, "封魔の迷宮") => (
                "王都衛兵「かつて大魔王軍を封じた迷宮だ。奥深くには封印の祭壇があると言われている…」",
                &["封印の祭壇"],
            ),
            (DialoguePartner::Guard, "封印の祭壇") => (
                "王都衛兵「祭壇の封印を解くには、古の光のオーブが必要だと古文書に記されているらしい」",
                &["光のオーブ"],
            ),
            (DialoguePartner::Tavern, "封魔の迷宮") => (
                "呑兵衛「ヒック…夜になると地下から『魔物の咆哮』が聞こえてくるんだよ…不気味だぜ」",
                &["魔物の咆哮"],
            ),
            (DialoguePartner::Tavern, "封印の祭壇") => (
                "呑兵衛「祭壇か！そういや爺さんが光のオーブを古物商に売っちまったとか言ってたな…」",
                &["光のオーブ"],
            ),
            (DialoguePartner::Tavern, "光のオーブ") => (
                "呑兵衛「オーブなら、裏路地にいる怪しい男がヤバいルートを握ってるらしいぜ…」",
                &["裏の抜け道"],
            ),
            (DialoguePartner::Suspicious, "光のオーブ") => (
                "怪しい男「ヒヒッ…オーブの話かい？地下迷宮の宝箱に隠された銀の鍵があれば手に入るぜ…」",
                &["地下迷宮", "銀の鍵"],
            ),
            (DialoguePartner::Suspicious, "銀の鍵") => (
                "怪しい男「迷宮の北の宝物庫だ。鉄格子の奥の宝箱に入ってるはずだぜ…ヒヒッ」",
                &[],
            ),
            (DialoguePartner::Suspicious, "裏の抜け道") => (
                "怪しい男「裏壁の崩れかけのレンガを押せば、見張りを通らずに裏口へ行けるのさ」",
                &[],
            ),
            (DialoguePartner::Villager, "王都アルカン") => (
                "街の女性「中央の噴水広場は憩いの場なんです。夜は少し冷えますから気をつけて」",
                &[],
            ),
            (DialoguePartner::Villager, "封魔の迷宮") => (
                "街の女性「きゃあっ！そんな恐ろしい迷宮、お願いですから近づかないでください！」",
                &[],
            ),
            _ => return None,
        };
        Some((text.to_string(), words))
    }

    /// 名詞先行型「たずねる」コマンド（ADR-0028）
    /// subject: 対象（話題ノードまたは文脈トークン）
    /// attribute: 属性（"弱点" 等の話題語句。省略可）
    /// question: 疑問詞（What/Where/DoYouKnow/Who）
    /// current_location: 現在地名（例: "マンティコア関所", "王都アルカン", "トリポリ"）
    /// inventory: プレイヤー所持品（社会的障壁判定用）
    /// reputation: 地域・組織の評判値（素性・顔見知り度。ADR-0028）
    pub fn ask_query(
        &mut self,
        subject: &QuerySubject,
        attribute: Option<&str>,
        question: QuestionType,
        current_location: &str,
        inventory: Option<&PlayerInventory>,
        reputation: i32,
    ) -> KnowledgeResponse {
        self.stage = DialogueStage::Talking;

        let response = match (self.partner, subject, attribute, question) {
            // --- 1. 文脈トークン: 「ここ」 ---
            (_, QuerySubject::Here, None, QuestionType::Where) => {
                match self.partner {
                    DialoguePartner::CheckpointGuard => KnowledgeResponse::Answer {
                        text: "番人「ここはマンティコア関所だ。東へ進めばトリポリだが、この先は通れんよ」".into(),
                    },
                    DialoguePartner::Villager => KnowledgeResponse::Answer {
                        text: format!("街の女性「ようこそ、旅の人。ここは{}です。冒険者ギルドをお探しですか？」", current_location),
                    },
                    DialoguePartner::Guard => KnowledgeResponse::Answer {
                        text: format!("王都衛兵「ここは{}だ。怪しい行動は慎むようにな」", current_location),
                    },
                    _ => KnowledgeResponse::Answer {
                        text: format!("{}「ここは{}だな」", self.partner.name(), current_location),
                    },
                }
            }
            (_, QuerySubject::Here, None, QuestionType::What) => {
                match self.partner {
                    DialoguePartner::Villager => KnowledgeResponse::Answer {
                        text: format!("街の女性「ここは{}です。活気のある素晴らしい街ですよ」", current_location),
                    },
                    DialoguePartner::Guard => KnowledgeResponse::Answer {
                        text: format!("王都衛兵「ここは{}だ。見ればわかるだろう」", current_location),
                    },
                    _ => KnowledgeResponse::Answer {
                        text: format!("{}「ここは{}だな」", self.partner.name(), current_location),
                    },
                }
            }

            // --- 2. 文脈トークン: 「あなた」 ---
            (_, QuerySubject::You, None, QuestionType::Who) => {
                match self.partner {
                    DialoguePartner::CheckpointGuard => KnowledgeResponse::Answer {
                        text: "番人「俺はこの関所を守る番人だ。通行証のねえ奴は通さん」".into(),
                    },
                    DialoguePartner::KnightCommander => KnowledgeResponse::Answer {
                        text: "団長「私は★★騎士団の団長だ。規律と武勇を重んじる」".into(),
                    },
                    _ => KnowledgeResponse::Answer {
                        text: format!("{}「私かい？ただの{}だよ」", self.partner.name(), self.partner.name()),
                    },
                }
            }
            (_, QuerySubject::You, None, _) => {
                match self.partner {
                    DialoguePartner::CheckpointGuard => KnowledgeResponse::Answer {
                        text: "番人「俺はこの関所を守る番人だ。通行証のねえ奴は通さん」".into(),
                    },
                    DialoguePartner::KnightCommander => KnowledgeResponse::Answer {
                        text: "団長「私は★★騎士団の団長だ。規律と武勇を重んじる」".into(),
                    },
                    _ => KnowledgeResponse::Answer {
                        text: format!("{}「私かい？ただの{}だよ」", self.partner.name(), self.partner.name()),
                    },
                }
            }

            // --- 3. 文脈トークン: 「私」 ---
            (_, QuerySubject::Me, None, QuestionType::Who) => {
                match self.partner {
                    DialoguePartner::CheckpointGuard => KnowledgeResponse::Answer {
                        text: "番人「お前？見たところ新米の旅人だな。おかわいそうに、記憶喪失なのですか？」".into(),
                    },
                    DialoguePartner::Villager => KnowledgeResponse::Answer {
                        text: "街の女性「あなたは…立派な旅の冒険者さんにお見受けしますよ」".into(),
                    },
                    _ => KnowledgeResponse::Answer {
                        text: format!("{}「あんたのことかい？元気な旅人に見えるぜ」", self.partner.name()),
                    },
                }
            }
            (_, QuerySubject::Me, None, _) => {
                match self.partner {
                    DialoguePartner::CheckpointGuard => KnowledgeResponse::Answer {
                        text: "番人「お前？見たところ新米の旅人だな。おかわいそうに、記憶喪失なのですか？」".into(),
                    },
                    DialoguePartner::Villager => KnowledgeResponse::Answer {
                        text: "街の女性「あなたは…立派な旅の冒険者さんにお見受けしますよ」".into(),
                    },
                    _ => KnowledgeResponse::Answer {
                        text: format!("{}「あんたのことかい？元気な旅人に見えるぜ」", self.partner.name()),
                    },
                }
            }

            // --- 4. マンティコア関所：マンティコアの質問チェーン ---
            (DialoguePartner::CheckpointGuard, QuerySubject::Topic { name, kind: TopicKind::Entity }, None, QuestionType::What)
                if name == "マンティコア" =>
            {
                KnowledgeResponse::Answer {
                    text: "番人「マンティコアについて聞くなんてお前新人か？ マンティコアは人食いの凶暴な獅子だ。悪いことは言わない、迂回路を通れ」".into(),
                }
            }

            (DialoguePartner::CheckpointGuard, QuerySubject::Topic { name, kind: TopicKind::Location }, None, QuestionType::Where | QuestionType::What)
                if name == "迂回路" =>
            {
                KnowledgeResponse::Answer {
                    text: "番人「比較的マンティコアがいない道だ。10日もあれば通過できるだろ」".into(),
                }
            }

            (DialoguePartner::CheckpointGuard, QuerySubject::Topic { name, .. }, Some("弱点"), QuestionType::What)
                if name == "マンティコア" =>
            {
                KnowledgeResponse::Pointer {
                    text: "番人「マンティコアの弱点については全くわかっていない。だが、★★騎士団団長なら何か知っているかもしれん」".into(),
                    barrier: None,
                }
            }

            (DialoguePartner::CheckpointGuard, QuerySubject::Topic { name, .. }, None, QuestionType::Where)
                if name == "★★騎士団団長" || name == "★★騎士団" =>
            {
                KnowledgeResponse::Pointer {
                    text: "番人「団長なら◇◇砦に赴任しているが、お前のようなやつにはお会いにならないだろうな」".into(),
                    barrier: Some(AccessBarrier::RequiresItem("衛兵の紹介状".into())),
                }
            }

            // --- 5. 騎士団長との対面（社会的障壁判定） ---
            (DialoguePartner::KnightCommander, QuerySubject::Topic { name, .. }, Some("弱点"), QuestionType::What)
                if name == "マンティコア" =>
            {
                let barrier = AccessBarrier::RequiresItem("衛兵の紹介状".into());
                if barrier.is_satisfied(inventory, reputation) {
                    KnowledgeResponse::Answer {
                        text: "団長「ほう、衛兵の紹介状か。…マンティコアの弱点だな？あいつの分厚い鬣には氷の魔術が有効だ」".into(),
                    }
                } else {
                    KnowledgeResponse::Pointer {
                        text: "団長「ふん、紹介状も持たぬ者に教える機密はない。立ち去れ」".into(),
                        barrier: Some(barrier),
                    }
                }
            }

            // --- 6. Scenario000: 漂着＋冒険者ギルド免許チェーン ---
            // 村人に「冒険者ギルド」の場所や概要を尋ねる
            (DialoguePartner::Villager, QuerySubject::Topic { name, .. }, None, QuestionType::Where)
                if name == "冒険者ギルド" =>
            {
                KnowledgeResponse::Pointer {
                    text: "街の女性「冒険者ギルドなら、中央広場の宿屋の隣ですよ。身元の保証もしてくれるはずです」".into(),
                    barrier: None,
                }
            }
            (DialoguePartner::Villager, QuerySubject::Topic { name, .. }, None, QuestionType::What | QuestionType::DoYouKnow)
                if name == "冒険者ギルド" =>
            {
                KnowledgeResponse::Answer {
                    text: "街の女性「冒険者ギルドは仕事の斡旋や身元の証明をしてくれる場所です。免許があれば宿屋にも泊まれますよ」".into(),
                }
            }

            // 衛兵に「冒険者ギルド免許」について尋ねる（取得条件を知る）
            (DialoguePartner::Guard, QuerySubject::Topic { name, .. }, None, QuestionType::What | QuestionType::DoYouKnow | QuestionType::Where)
                if name == "冒険者ギルド免許" =>
            {
                KnowledgeResponse::Pointer {
                    text: "王都衛兵「冒険者ギルド免許か。身元のない流れ者でも、衛兵の依頼をこなせばギルドから免許が発行されるぜ」".into(),
                    barrier: Some(AccessBarrier::RequiresItem("冒険者ギルド免許".into())),
                }
            }

            // 衛兵に「衛兵の依頼」について尋ねる（依頼達成・免許発行）
            (DialoguePartner::Guard, QuerySubject::Topic { name, .. }, None, QuestionType::What | QuestionType::DoYouKnow | QuestionType::Where)
                if name == "衛兵の依頼" =>
            {
                KnowledgeResponse::Answer {
                    text: "王都衛兵「迷宮周辺の巡回任務だな。よく引き受けてくれた！…よし、任務完了だ。これが【冒険者ギルド免許】だ！」".into(),
                }
            }

            // 「洞穴」について尋ねる（野宿スポット）
            (DialoguePartner::Villager, QuerySubject::Topic { name, .. }, None, QuestionType::Where | QuestionType::What)
                if name == "洞穴" =>
            {
                KnowledgeResponse::Answer {
                    text: "街の女性「街の外にある洞穴ですね。昔の旅人が野宿に使っていたそうですが、魔物には気をつけてくださいね」".into(),
                }
            }
            (DialoguePartner::Guard, QuerySubject::Topic { name, .. }, None, QuestionType::Where | QuestionType::What)
                if name == "洞穴" =>
            {
                KnowledgeResponse::Answer {
                    text: "王都衛兵「街の外の洞穴か。あそこなら誰でも野宿できるが、安全の保証はできんぞ」".into(),
                }
            }

            // 宿屋の主人に「冒険者ギルド免許」について尋ねる
            (DialoguePartner::Inn, QuerySubject::Topic { name, .. }, None, QuestionType::DoYouKnow | QuestionType::What)
                if name == "冒険者ギルド免許" =>
            {
                KnowledgeResponse::Answer {
                    text: "宿屋の主人「免許を持っていれば、うちの宿を君の正式な住所として登録して宿泊できるようになるよ」".into(),
                }
            }
            (DialoguePartner::Inn, QuerySubject::Topic { name, .. }, None, QuestionType::Where | QuestionType::What)
                if name == "洞穴" =>
            {
                KnowledgeResponse::Answer {
                    text: "宿屋の主人「外の洞穴かい？あそこは野宿スポットだが、風邪を引かないようにな」".into(),
                }
            }

            // --- 7. 封魔の迷宮クエスト（旧来の話題振り）へのフォールバック ---
            // 暫定仕様: 旧返答は聞き方を区別しないため、「誰？」でも同じ答えを返す。
            // 聞き方ごとの返答表（知識グラフ化）に置き換える際に見直すこと。
            (_, QuerySubject::Topic { name, .. }, None, _) => {
                if let Some((text, words)) = self.legacy_topic_reply(name) {
                    self.learnable_spans = spans_for(&text, words);
                    self.current_text = text.clone();
                    return KnowledgeResponse::Answer { text };
                }
                KnowledgeResponse::Unknown
            }

            // --- 8. 該当なし（完全な未知） ---
            _ => KnowledgeResponse::Unknown,
        };

        // 返答テキストの設定
        let raw_text = match &response {
            KnowledgeResponse::Answer { text } => text.clone(),
            KnowledgeResponse::Pointer { text, .. } => text.clone(),
            KnowledgeResponse::Unknown => {
                format!("{}「すまんが、それは知らんな」", self.partner.name())
            }
        };

        // 下線語句の自動抽出（キーワードリストから一致するものを抽出）
        // プレイヤーが手動で「おぼえる」を実行することで手帳に登録される（手動記憶ルールの遵守）
        let known_keywords = [
            "マンティコア",
            "迂回路",
            "トリポリ",
            "マンティコア関所",
            "★★騎士団",
            "★★騎士団団長",
            "◇◇砦",
            "氷の魔術",
            "封魔の迷宮",
            "封印の祭壇",
            "光のオーブ",
            "銀の鍵",
            "地下迷宮",
            "裏の抜け道",
            "冒険者ギルド",
            "冒険者ギルド免許",
            "衛兵の依頼",
            "宿屋",
            "洞穴",
        ];
        self.learnable_spans = spans_for(&raw_text, &known_keywords);
        self.current_text = raw_text;

        response
    }

    /// 宿屋に泊まる処理（ADR-0028: 身元確認障壁の判定付き）
    /// 免許アイテム所持、または地域評判10以上で身元確認成立
    pub fn rest_at_inn(
        &mut self,
        inv: &mut PlayerInventory,
        members: &mut [PartyMember],
        reputation: i32,
    ) -> Result<bool, AccessBarrier> {
        let has_license = inv.has_item("冒険者ギルド免許");
        let has_reputation = reputation >= 10;

        if !has_license && !has_reputation {
            self.current_text = "宿屋の主人「すまないが、うちでは素性の知れないお方（評判不足、冒険者ギルド免許なし）を泊めるわけにはいかないんだ。…泊まる場所がないなら、外の洞穴で夜露をしのぐといい」".into();
            self.learnable_spans = spans_for(&self.current_text, &["洞穴", "冒険者ギルド免許"]);
            return Err(AccessBarrier::RequiresReputation {
                faction: "王都アルカン".into(),
                min_reputation: 10,
            });
        }

        if inv.spend_gold(INN_COST) {
            for m in members.iter_mut() {
                m.hp = m.max_hp;
                m.mp = m.max_mp;
            }
            if has_license {
                self.current_text = "宿屋の主人「まいど！冒険者ギルド免許を確認したよ。ここを君の正式な住所として登録しておこう。ぐっすり休んでいきな！」\n仲間全員のHPとMPが全快した！".into();
            } else {
                self.current_text = "宿屋の主人「おお、街での評判は聞いているよ！あんたなら大歓迎だ。ぐっすり休んでいきな！」\n仲間全員のHPとMPが全快した！".into();
            }
            self.learnable_spans = Vec::new();
            Ok(true)
        } else {
            self.current_text = format!(
                "宿屋の主人「おや、{}が足りないようだね。一泊{}{}だよ」",
                CURRENCY_NAME, INN_COST, CURRENCY_NAME
            );
            self.learnable_spans = Vec::new();
            Ok(false)
        }
    }

    /// 道具屋で買い物
    pub fn buy_item(&mut self, inv: &mut PlayerInventory) -> bool {
        let item = &SHOP_ITEMS[self.selected_shop_index];
        if inv.spend_gold(item.price) {
            inv.add_item(item.name);
            self.current_text = format!(
                "道具屋「まいど！【{}】をお買い上げだ。\n大切に使いなよ！」(所持金: {}{})",
                item.name, inv.gold, CURRENCY_UNIT
            );
            true
        } else {
            self.current_text = format!(
                "道具屋「おいおい、{}が足りねえぜ！【{}】は {}{} だ」(所持金: {}{})",
                CURRENCY_NAME, item.name, item.price, CURRENCY_UNIT, inv.gold, CURRENCY_UNIT
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::{Influence, MentalState, Personality};

    fn create_dummy_member(name: &str, hp: i32, max_hp: i32, mp: i32, max_mp: i32) -> PartyMember {
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
        let mut inv = PlayerInventory {
            gold: 60,
            ..Default::default()
        };
        let mut members = vec![
            create_dummy_member("ガルツ", 10, 50, 0, 10),
            create_dummy_member("ミレイ", 5, 25, 2, 40),
        ];

        // 宿泊成功（評判10で身元確認OK）
        let success = session.rest_at_inn(&mut inv, &mut members, 10);
        assert_eq!(success, Ok(true));
        assert_eq!(inv.gold, 60 - INN_COST);
        assert_eq!(members[0].hp, 50);
        assert_eq!(members[0].mp, 10);
        assert_eq!(members[1].hp, 25);
        assert_eq!(members[1].mp, 40);

        // フォリン不足で宿泊失敗
        let success2 = session.rest_at_inn(&mut inv, &mut members, 10);
        assert_eq!(success2, Ok(false));
        assert_eq!(inv.gold, 60 - INN_COST);
    }

    #[test]
    fn test_shop_purchase() {
        let mut session = DialogueSession::start(DialoguePartner::Shop);
        let mut inv = PlayerInventory {
            gold: 20,
            ..Default::default()
        };
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
        session.stage = DialogueStage::ChoosingLearnTarget { cursor: 1 };

        session.ask_topic("銀の鍵");
        assert_eq!(session.stage, DialogueStage::Talking);
    }

    #[test]
    fn test_spans_for_multiple_occurrences_and_ordering() {
        let text = "王都の銀の鍵と、地下迷宮の銀の鍵。どちらも鍵だ。";
        // 探索語の順序はあえて本文の出現順と逆にする
        let spans = spans_for(text, &["地下迷宮", "銀の鍵"]);

        // 「銀の鍵」(0..3)、次に「地下迷宮」(7..11)、次に2個目の「銀の鍵」(12..15)
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].slice(text), "銀の鍵");
        assert_eq!(spans[0].start, 3); // "王都の" = 3文字目から
        assert_eq!(spans[1].slice(text), "地下迷宮");
        assert_eq!(spans[2].slice(text), "銀の鍵");
        assert!(spans[0].start < spans[1].start);
        assert!(spans[1].start < spans[2].start);
    }

    #[test]
    fn test_query_here_where() {
        let mut session = DialogueSession::start(DialoguePartner::CheckpointGuard);
        let resp = session.ask_query(
            &QuerySubject::Here,
            None,
            QuestionType::Where,
            "マンティコア関所",
            None,
            0,
        );
        match resp {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("ここはマンティコア関所だ"));
            }
            _ => panic!("Expected Answer response"),
        }
        assert_eq!(
            learned_words(&session),
            vec!["マンティコア関所".to_string(), "トリポリ".to_string()]
        );
    }

    #[test]
    fn test_query_who_me_and_you() {
        let mut session = DialogueSession::start(DialoguePartner::CheckpointGuard);

        // 「あなた」→「誰？」: NPC自身の素性
        let resp_you =
            session.ask_query(&QuerySubject::You, None, QuestionType::Who, "関所", None, 0);
        match resp_you {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("俺はこの関所を守る番人だ"));
            }
            _ => panic!("Expected Answer response"),
        }

        // 「私」→「誰？」: 評判照会・記憶喪失ジョーク
        let resp_me =
            session.ask_query(&QuerySubject::Me, None, QuestionType::Who, "関所", None, 0);
        match resp_me {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("おかわいそうに、記憶喪失なのですか？"));
            }
            _ => panic!("Expected Answer response"),
        }
    }

    #[test]
    fn test_query_manticore_checkpoint_chain() {
        let mut session = DialogueSession::start(DialoguePartner::CheckpointGuard);

        // 1. マンティコアとは何？
        let manticore = QuerySubject::Topic {
            name: "マンティコア".into(),
            kind: TopicKind::Entity,
        };
        let resp1 = session.ask_query(&manticore, None, QuestionType::What, "関所", None, 0);
        match resp1 {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("お前新人か？"));
                assert!(text.contains("人食いの凶暴な獅子"));
            }
            _ => panic!("Expected Answer response"),
        }
        // 「マンティコア」(2箇所)「迂回路」(1箇所) が下線対象になる（自動開示ではなく手動記憶対象）
        assert_eq!(
            learned_words(&session),
            vec![
                "マンティコア".to_string(),
                "マンティコア".to_string(),
                "迂回路".to_string()
            ]
        );

        // 2. 迂回路とはどこ？
        let detour = QuerySubject::Topic {
            name: "迂回路".into(),
            kind: TopicKind::Location,
        };
        let resp2 = session.ask_query(&detour, None, QuestionType::Where, "関所", None, 0);
        match resp2 {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("比較的マンティコアがいない道だ"));
            }
            _ => panic!("Expected Answer response"),
        }

        // 3. マンティコアの弱点は何？（ポインタ発生）
        let resp3 = session.ask_query(
            &manticore,
            Some("弱点"),
            QuestionType::What,
            "関所",
            None,
            0,
        );
        match resp3 {
            KnowledgeResponse::Pointer { text, barrier } => {
                assert!(text.contains("★★騎士団団長なら何か知っているかもしれん"));
                assert!(barrier.is_none());
            }
            _ => panic!("Expected Pointer response"),
        }
        // ポインタ先（★★騎士団団長）と、本文中の（マンティコア）が下線語句として提示される
        assert_eq!(
            learned_words(&session),
            vec!["マンティコア".to_string(), "★★騎士団団長".to_string()]
        );

        // 4. ★★騎士団団長はどこ？（社会的障壁付きポインタ）
        let commander = QuerySubject::Topic {
            name: "★★騎士団団長".into(),
            kind: TopicKind::Person,
        };
        let resp4 = session.ask_query(&commander, None, QuestionType::Where, "関所", None, 0);
        match resp4 {
            KnowledgeResponse::Pointer { text, barrier } => {
                assert!(text.contains(
                    "◇◇砦に赴任しているが、お前のようなやつにはお会いにならないだろうな"
                ));
                assert_eq!(
                    barrier,
                    Some(AccessBarrier::RequiresItem("衛兵の紹介状".into()))
                );
            }
            _ => panic!("Expected Pointer with barrier"),
        }
        assert_eq!(learned_words(&session), vec!["◇◇砦".to_string()]);
    }

    #[test]
    fn test_knight_commander_barrier_check() {
        let mut session = DialogueSession::start(DialoguePartner::KnightCommander);
        let manticore = QuerySubject::Topic {
            name: "マンティコア".into(),
            kind: TopicKind::Entity,
        };

        // 紹介状なしで団長に弱点を訊く -> 門前払い（社会的障壁）
        let empty_inv = PlayerInventory::default();
        let resp_no_intro = session.ask_query(
            &manticore,
            Some("弱点"),
            QuestionType::What,
            "◇◇砦",
            Some(&empty_inv),
            0,
        );
        match resp_no_intro {
            KnowledgeResponse::Pointer { text, barrier } => {
                assert!(text.contains("紹介状も持たぬ者に教える機密はない"));
                assert!(barrier.is_some());
            }
            _ => panic!("Expected Barrier rejection"),
        }

        // 衛兵から紹介状をもらった状態で訊く -> 弱点開示！
        let mut ready_inv = PlayerInventory::default();
        ready_inv.add_item("衛兵の紹介状");
        let resp_with_intro = session.ask_query(
            &manticore,
            Some("弱点"),
            QuestionType::What,
            "◇◇砦",
            Some(&ready_inv),
            0,
        );
        match resp_with_intro {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("ほう、衛兵の紹介状か"));
                assert!(text.contains("氷の魔術が有効だ"));
            }
            _ => panic!("Expected Answer with weakness revealed"),
        }
        // 「マンティコア」と「氷の魔術」が下線語句になり、手動でおぼえることができる
        assert_eq!(
            learned_words(&session),
            vec!["マンティコア".to_string(), "氷の魔術".to_string()]
        );
    }

    #[test]
    fn test_scenario000_shipwrecked_to_licensed_inn_stay() {
        // 1. 漂着：海岸/洞穴で目覚める。所持金わずか、評判ゼロ、免許なし
        let mut inv = PlayerInventory {
            gold: 50,
            items: Vec::new(),
            topics: Vec::new(),
            reputation: 0,
        };
        let reputation = 0;
        let mut members = vec![create_dummy_member("主人公", 5, 20, 0, 5)];

        // 2. 身元不明のまま宿屋に行き、宿泊を試みる -> 評判不足・免許なしで門前払い！
        let mut inn_session = DialogueSession::start(DialoguePartner::Inn);
        let rest_attempt1 = inn_session.rest_at_inn(&mut inv, &mut members, reputation);
        assert!(rest_attempt1.is_err());
        assert_eq!(
            rest_attempt1.unwrap_err(),
            AccessBarrier::RequiresReputation {
                faction: "王都アルカン".into(),
                min_reputation: 10,
            }
        );
        assert!(inn_session
            .current_text
            .contains("外の洞穴で夜露をしのぐといい"));
        // 「冒険者ギルド免許」「洞穴」が下線語句になり、手動でおぼえられる
        assert_eq!(
            learned_words(&inn_session),
            vec!["冒険者ギルド免許".to_string(), "洞穴".to_string()]
        );
        inv.learn_topic("冒険者ギルド免許");

        // 3. 評判軸（RequiresReputation）の単体検証:
        // もし街で善行を重ねて評判が10以上になれば、免許がなくても顔見知りとして泊まれる！
        let high_reputation = 10;
        let rest_with_rep = inn_session.rest_at_inn(&mut inv, &mut members, high_reputation);
        assert_eq!(rest_with_rep, Ok(true));
        assert!(inn_session
            .current_text
            .contains("街での評判は聞いているよ"));

        // 4. 町の住人（Villager）に「ここ→どこ？」「冒険者ギルド→どこ？」をたずねる
        let mut villager_session = DialogueSession::start(DialoguePartner::Villager);
        let resp_here = villager_session.ask_query(
            &QuerySubject::Here,
            None,
            QuestionType::Where,
            "港町",
            Some(&inv),
            reputation,
        );
        match resp_here {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("ここは港町です"));
                assert!(text.contains("冒険者ギルドをお探しですか？"));
            }
            _ => panic!("Expected Answer"),
        }
        assert_eq!(
            learned_words(&villager_session),
            vec!["冒険者ギルド".to_string()]
        );
        inv.learn_topic("冒険者ギルド");

        let guild_subject = QuerySubject::Topic {
            name: "冒険者ギルド".into(),
            kind: TopicKind::Location,
        };
        let resp_guild = villager_session.ask_query(
            &guild_subject,
            None,
            QuestionType::Where,
            "港町",
            Some(&inv),
            reputation,
        );
        match resp_guild {
            KnowledgeResponse::Pointer { text, barrier } => {
                assert!(text.contains("中央広場の宿屋の隣ですよ"));
                assert!(barrier.is_none());
            }
            _ => panic!("Expected Pointer"),
        }
        assert_eq!(
            learned_words(&villager_session),
            vec!["冒険者ギルド".to_string(), "宿屋".to_string()]
        );

        // 5. 衛兵（Guard）に「冒険者ギルド免許」について尋ね、取得条件（衛兵の依頼）を知る
        let mut guard_session = DialogueSession::start(DialoguePartner::Guard);
        let license_subject = QuerySubject::Topic {
            name: "冒険者ギルド免許".into(),
            kind: TopicKind::Entity,
        };
        let resp_license = guard_session.ask_query(
            &license_subject,
            None,
            QuestionType::What,
            "港町",
            Some(&inv),
            reputation,
        );
        match resp_license {
            KnowledgeResponse::Pointer { text, barrier } => {
                assert!(text.contains("衛兵の依頼をこなせばギルドから免許が発行されるぜ"));
                assert_eq!(
                    barrier,
                    Some(AccessBarrier::RequiresItem("冒険者ギルド免許".into()))
                );
            }
            _ => panic!("Expected Pointer"),
        }
        assert_eq!(
            learned_words(&guard_session),
            vec!["冒険者ギルド免許".to_string(), "衛兵の依頼".to_string()]
        );

        // 6. 衛兵の依頼を数件こなした結果、第三者から免許（アイテム）が発行される
        inv.add_item("冒険者ギルド免許");

        // 7. 宿屋（Inn）に「冒険者ギルド免許」について尋ねる
        let mut inn_session2 = DialogueSession::start(DialoguePartner::Inn);
        let resp_inn_q = inn_session2.ask_query(
            &license_subject,
            None,
            QuestionType::DoYouKnow,
            "港町",
            Some(&inv),
            reputation,
        );
        match resp_inn_q {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("君の正式な住所として登録して宿泊できるようになるよ"));
            }
            _ => panic!("Expected Answer"),
        }

        // 8. 再び宿泊アクション：評判ゼロのままでも、免許があるため住所登録が成立して宿泊成功！
        inv.gold = 50; // 宿泊費用を補充
        members[0].hp = 5;
        let rest_success = inn_session2.rest_at_inn(&mut inv, &mut members, reputation);
        assert_eq!(rest_success, Ok(true));
        assert!(inn_session2
            .current_text
            .contains("冒険者ギルド免許を確認したよ"));
        assert!(inn_session2
            .current_text
            .contains("君の正式な住所として登録"));
        assert_eq!(members[0].hp, 20); // HP全回復！
    }

    /// 暫定仕様のテスト: 旧返答へのフォールバックで迷宮クエストのヒントが聞けることだけを固定する。
    /// 聞き方ごとの返答表に置き換える際は、このテストごと見直すこと。
    #[test]
    fn test_ask_query_falls_back_to_legacy_dungeon_topics() {
        let mut guard = DialogueSession::start(DialoguePartner::Guard);
        let resp = guard.ask_query(
            &QuerySubject::from_topic_name("封魔の迷宮"),
            None,
            QuestionType::What,
            "王都アルカン",
            None,
            0,
        );
        assert!(matches!(resp, KnowledgeResponse::Answer { .. }));
        assert!(guard.current_text.contains("封印の祭壇"));
        assert_eq!(learned_words(&guard), vec!["封印の祭壇".to_string()]);

        let mut suspicious = DialogueSession::start(DialoguePartner::Suspicious);
        let resp = suspicious.ask_query(
            &QuerySubject::from_topic_name("光のオーブ"),
            None,
            QuestionType::Where,
            "王都アルカン",
            None,
            0,
        );
        assert!(matches!(resp, KnowledgeResponse::Answer { .. }));
        assert_eq!(
            learned_words(&suspicious),
            vec!["地下迷宮".to_string(), "銀の鍵".to_string()]
        );
    }

    #[test]
    fn test_guard_request_who_is_unknown() {
        let mut guard = DialogueSession::start(DialoguePartner::Guard);
        let resp = guard.ask_query(
            &QuerySubject::from_topic_name("衛兵の依頼"),
            None,
            QuestionType::Who,
            "王都アルカン",
            None,
            0,
        );
        assert_eq!(resp, KnowledgeResponse::Unknown);
    }

    #[test]
    fn test_selectable_subjects_and_query_stages() {
        let topics = vec!["王都アルカン".to_string(), "冒険者ギルド".to_string()];
        let subjects = DialogueSession::selectable_subjects(&topics);
        assert_eq!(subjects.len(), 5);
        assert_eq!(subjects[0].0, "ここ");
        assert_eq!(subjects[0].1, QuerySubject::Here);
        assert_eq!(subjects[1].0, "あなた");
        assert_eq!(subjects[1].1, QuerySubject::You);
        assert_eq!(subjects[2].0, "私");
        assert_eq!(subjects[2].1, QuerySubject::Me);
        assert_eq!(subjects[3].0, "王都アルカン");
        assert_eq!(subjects[4].0, "冒険者ギルド");

        // Guard query for 衛兵の依頼 gives Answer and has license in text
        let mut guard_session = DialogueSession::start(DialoguePartner::Guard);
        let resp = guard_session.ask_query(
            &QuerySubject::from_topic_name("衛兵の依頼"),
            None,
            QuestionType::What,
            "王都アルカン",
            None,
            0,
        );
        match resp {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("冒険者ギルド免許"));
            }
            _ => panic!("Expected Answer"),
        }
        assert!(learned_words(&guard_session).contains(&"冒険者ギルド免許".to_string()));
        assert_eq!(guard_session.stage, DialogueStage::Talking);

        // Villager query for 洞穴
        let mut villager_session = DialogueSession::start(DialoguePartner::Villager);
        let resp_cave = villager_session.ask_query(
            &QuerySubject::from_topic_name("洞穴"),
            None,
            QuestionType::Where,
            "王都アルカン",
            None,
            0,
        );
        match resp_cave {
            KnowledgeResponse::Answer { text } => {
                assert!(text.contains("洞穴"));
            }
            _ => panic!("Expected Answer"),
        }
    }

    fn learned_words(session: &DialogueSession) -> Vec<String> {
        session
            .learnable_spans
            .iter()
            .map(|span| span.slice(&session.current_text))
            .collect()
    }
}
