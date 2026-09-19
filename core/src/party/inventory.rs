/// ゲーム内通貨の正式名称（ADR-0019: 銀貨相当）
pub const CURRENCY_NAME: &str = "フォリン";

/// ゲーム内通貨の略称・単位記号（当面「G」を維持。ADR-0019）
pub const CURRENCY_UNIT: &str = "G";

#[derive(Clone, Debug)]
pub struct PlayerInventory {
    pub gold: i32,
    pub items: Vec<String>,
    pub topics: Vec<String>,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            gold: 100,
            items: vec!["やくそう".into(), "松明".into()],
            topics: vec!["王都アルカン".into(), "封魔の迷宮".into()],
        }
    }
}

impl PlayerInventory {
    pub fn add_gold(&mut self, amount: i32) {
        self.gold += amount;
    }

    pub fn spend_gold(&mut self, amount: i32) -> bool {
        if self.gold >= amount {
            self.gold -= amount;
            true
        } else {
            false
        }
    }

    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    pub fn has_item(&self, item: &str) -> bool {
        self.items.iter().any(|i| i == item)
    }

    pub fn remove_item(&mut self, item: &str) -> bool {
        if let Some(pos) = self.items.iter().position(|i| i == item) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn learn_topic(&mut self, topic: impl Into<String>) -> bool {
        let topic_str = topic.into();
        if self.has_topic(&topic_str) {
            false // 既に記憶済み
        } else {
            self.topics.push(topic_str);
            true
        }
    }

    pub fn has_topic(&self, topic: &str) -> bool {
        self.topics.iter().any(|t| t == topic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gold_spending() {
        let mut inv = PlayerInventory::default();
        assert_eq!(inv.gold, 100);

        assert!(inv.spend_gold(50));
        assert_eq!(inv.gold, 50);

        assert!(!inv.spend_gold(60)); // 所持金不足
        assert_eq!(inv.gold, 50);

        inv.add_gold(100);
        assert_eq!(inv.gold, 150);
    }

    #[test]
    fn test_topic_learning() {
        let mut inv = PlayerInventory::default();
        assert!(inv.has_topic("王都アルカン"));
        assert!(!inv.has_topic("封印の祭壇"));

        // 新しい話題を覚える
        assert!(inv.learn_topic("封印の祭壇"));
        assert!(inv.has_topic("封印の祭壇"));

        // 二重記憶は false
        assert!(!inv.learn_topic("封印の祭壇"));
    }

    #[test]
    fn test_item_management() {
        let mut inv = PlayerInventory::default();
        assert!(inv.has_item("やくそう"));
        assert!(!inv.has_item("どくけしそう"));

        assert!(inv.remove_item("やくそう"));
        assert!(!inv.has_item("やくそう"));
        assert!(!inv.remove_item("やくそう")); // 二度目は存在しない

        inv.add_item("どくけしそう");
        assert!(inv.has_item("どくけしそう"));
    }
}
