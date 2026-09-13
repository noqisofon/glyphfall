use bevy::prelude::*;

/// 敵モンスターの定義
#[derive(Component, Debug, Clone)]
pub struct Monster {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub glyph_art: &'static str,
}

impl Monster {
    pub fn new(name: impl Into<String>, max_hp: i32, glyph_art: &'static str) -> Self {
        Self {
            name: name.into(),
            hp: max_hp,
            max_hp,
            glyph_art,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.hp = (self.hp - damage).max(0);
    }
}

/// 戦闘状態の管理リソース
#[derive(Resource)]
pub struct BattleState {
    pub current_index: usize,
    pub monsters: Vec<Monster>,
    pub flash_timer: Timer,
    pub is_flashing: bool,
}

impl BattleState {
    pub fn current_monster(&self) -> &Monster {
        &self.monsters[self.current_index]
    }

    #[allow(dead_code)]
    pub fn current_monster_mut(&mut self) -> &mut Monster {
        &mut self.monsters[self.current_index]
    }


    pub fn next_monster(&mut self) {
        self.current_index = (self.current_index + 1) % self.monsters.len();
        self.monsters[self.current_index].hp = self.monsters[self.current_index].max_hp;
    }

    /// 現在のモンスターにダメージを与え、(モンスター名, 撃破されたか) を返す
    pub fn apply_damage(&mut self, damage: i32) -> (String, bool) {
        let mon = &mut self.monsters[self.current_index];
        mon.take_damage(damage);
        let name = mon.name.clone();
        let is_dead = mon.is_dead();
        self.is_flashing = true;
        self.flash_timer.reset();
        (name, is_dead)
    }
}


// ─────────────────────────────────────────────
// モンスターのグリフアート（ASCII / Unicode）
// ─────────────────────────────────────────────

pub const SLIME_ART: &str = r#"
        /\
       /  \
      /    \
    .( o  o ).
   (   .__.   )
    `--------'
"#;

pub const SKELETON_ART: &str = r#"
     .-""""-.
    /        \
   /_O      O_\
     \  ||  /
      `===='
    .-'|  |'-.
   /   |  |   \
"#;

pub const BEAR_ART: &str = r#"
   ((.-.))  ((.-.))
    /   `    '   \
   |  (o)    (o)  |
   |      V       |
    \   `----'   /
     \  |    |  /
"#;

pub const BANDIT_ART: &str = r#"
      .-----.
     /_______\
    ( | o o | )
     \   =   /
    .-'--+--'-.
   /  |  |  |  \
"#;

/// 初期出現モンスターのリストを生成
pub fn create_default_monsters() -> Vec<Monster> {
    vec![
        Monster::new("スライムグリフ", 24, SLIME_ART),
        Monster::new("がいこつせんし", 36, SKELETON_ART),
        Monster::new("キラーベア", 50, BEAR_ART),
        Monster::new("山賊のとうぞく", 42, BANDIT_ART),
    ]
}
