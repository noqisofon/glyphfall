use bevy::prelude::*;
use rand::Rng;
use crate::party::{
    diagnose_member, evaluate_command, ActionOutcome, PartyCommand, PartyMember, Personality,
    PlayerBattleAction, PlayerSkills,
};

pub mod input;
pub use input::handle_battle_input;

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

/// 戦闘の進行フェーズ
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattlePhase {
    /// コマンド・指示入力中（member_cursor: 0=あなた, 1=ガルツ, 2=ロロ, 3=ミレイ）
    CommandInput { member_cursor: usize },
    /// ターン解決・ログ送り中
    TurnResolving { step_cursor: usize },
    /// 敗北（主人公が倒れた）
    Defeat,
}

/// ターン解決時の1アクションステップ
#[derive(Debug, Clone)]
pub struct BattleStep {
    pub message: String,
    pub monster_damage: Option<i32>,
    #[allow(dead_code)]
    pub member_damage: Option<(usize, i32)>,
    #[allow(dead_code)]
    pub member_heal: Option<(usize, i32)>,
    pub monster_defeated: bool,
    pub player_defeated: bool,
    pub flee_success: bool,
}

/// 戦闘状態の管理リソース
#[derive(Resource)]
pub struct BattleState {
    pub current_index: usize,
    pub monsters: Vec<Monster>,
    pub flash_timer: Timer,
    pub is_flashing: bool,
    pub phase: BattlePhase,
    pub player_action: Option<PlayerBattleAction>,
    pub party_commands: Vec<Option<PartyCommand>>,
    pub turn_steps: Vec<BattleStep>,
}

impl BattleState {
    pub fn new(monsters: Vec<Monster>) -> Self {
        Self {
            current_index: 0,
            monsters,
            flash_timer: Timer::from_seconds(0.18, TimerMode::Once),
            is_flashing: false,
            phase: BattlePhase::CommandInput { member_cursor: 0 },
            player_action: None,
            party_commands: vec![None, None, None, None],
            turn_steps: Vec::new(),
        }
    }

    pub fn current_monster(&self) -> &Monster {
        &self.monsters[self.current_index]
    }

    pub fn current_monster_mut(&mut self) -> &mut Monster {
        &mut self.monsters[self.current_index]
    }

    pub fn reset_turn_commands(&mut self) {
        self.phase = BattlePhase::CommandInput { member_cursor: 0 };
        self.player_action = None;
        self.party_commands = vec![None, None, None, None];
        self.turn_steps.clear();
    }

    pub fn next_monster(&mut self) {
        self.current_index = (self.current_index + 1) % self.monsters.len();
        self.monsters[self.current_index].hp = self.monsters[self.current_index].max_hp;
        self.reset_turn_commands();
    }

    /// 現在のモンスターにダメージを与え、(モンスター名, 撃破されたか) を返す
    #[allow(dead_code)]
    pub fn apply_damage(&mut self, damage: i32) -> (String, bool) {
        let mon = &mut self.monsters[self.current_index];
        mon.take_damage(damage);
        let name = mon.name.clone();
        let is_dead = mon.is_dead();
        self.is_flashing = true;
        self.flash_timer.reset();
        (name, is_dead)
    }

    /// ターン解決ステップを生成する
    pub fn build_turn_resolution<R: Rng>(
        &mut self,
        members: &mut [PartyMember],
        skills: &PlayerSkills,
        rng: &mut R,
    ) {
        let mut steps = Vec::new();
        let mon_name = self.current_monster().name.clone();

        // 1. あなた（主人公）の行動
        let player_action = self.player_action.unwrap_or(PlayerBattleAction::Attack);
        let mut player_defending = false;

        match player_action {
            PlayerBattleAction::Attack => {
                let dmg = rng.gen_range(2..=5);
                let is_dead = {
                    let mon = self.current_monster_mut();
                    mon.take_damage(dmg);
                    mon.is_dead()
                };
                steps.push(BattleStep {
                    message: format!("あなた（一般人）は　必死に石を投げつけた！\n{}に {}の ダメージ！", mon_name, dmg),
                    monster_damage: Some(dmg),
                    member_damage: None,
                    member_heal: None,
                    monster_defeated: is_dead,
                    player_defeated: false,
                    flee_success: false,
                });
                if is_dead {
                    self.turn_steps = steps;
                    self.phase = BattlePhase::TurnResolving { step_cursor: 0 };
                    return;
                }
            }
            PlayerBattleAction::Defend => {
                player_defending = true;
                steps.push(BattleStep {
                    message: "あなたは　身をかがめて　警戒を強めた！（被ダメージ軽減）".into(),
                    monster_damage: None,
                    member_damage: None,
                    member_heal: None,
                    monster_defeated: false,
                    player_defeated: false,
                    flee_success: false,
                });
            }
            PlayerBattleAction::UseItem => {
                // 最もHPの割合が低い味方を回復
                let target_idx = members
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, m)| m.hp * 100 / m.max_hp.max(1))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let target = &mut members[target_idx];
                let heal_amount = 20;
                let prev_hp = target.hp;
                target.hp = (target.hp + heal_amount).min(target.max_hp);
                let actual_heal = target.hp - prev_hp;
                steps.push(BattleStep {
                    message: format!(
                        "あなたは　やくそうを取り出し　{}にあてた！\n{}のHPが {} 回復した！",
                        target.name, target.name, actual_heal
                    ),
                    monster_damage: None,
                    member_damage: None,
                    member_heal: Some((target_idx, actual_heal)),
                    monster_defeated: false,
                    player_defeated: false,
                    flee_success: false,
                });
            }
            PlayerBattleAction::Diagnose => {
                // 仲間（インデックス1..members.len()）からランダムで観察
                let message = if members.len() > 1 {
                    let target_idx = rng.gen_range(1..members.len());
                    let report = diagnose_member(skills, &members[target_idx], rng);
                    format!("あなた「{}の様子を見よう」\n{}\n{}", members[target_idx].name, report.observation_msg, report.conclusion_msg)
                } else {
                    "あなた「様子を見ようにも、仲間がいない……」".to_string()
                };
                steps.push(BattleStep {
                    message,
                    monster_damage: None,
                    member_damage: None,
                    member_heal: None,
                    monster_defeated: false,
                    player_defeated: false,
                    flee_success: false,
                });
            }
            PlayerBattleAction::Flee => {
                let success = rng.gen_bool(0.5);
                if success {
                    steps.push(BattleStep {
                        message: "あなたは仲間たちに合図を出し、脱出路へ走り込んだ！\nうまく逃げ切れた！".into(),
                        monster_damage: None,
                        member_damage: None,
                        member_heal: None,
                        monster_defeated: false,
                        player_defeated: false,
                        flee_success: true,
                    });
                    self.turn_steps = steps;
                    self.phase = BattlePhase::TurnResolving { step_cursor: 0 };
                    return;
                } else {
                    steps.push(BattleStep {
                        message: "あなたは逃げ出そうとしたが、魔物に退路を塞がれた！".into(),
                        monster_damage: None,
                        member_damage: None,
                        member_heal: None,
                        monster_defeated: false,
                        player_defeated: false,
                        flee_success: false,
                    });
                }
            }
        }

        // 2. 仲間たちの行動
        for i in 1..members.len() {
            if self.current_monster().is_dead() {
                break;
            }
            if members[i].hp <= 0 {
                // 戦闘不能なメンバーは行動できない
                continue;
            }
            let cmd = self.party_commands.get(i).and_then(|c| *c).unwrap_or(PartyCommand::Attack);
            let outcome = evaluate_command(&members[i], cmd, rng);

            match outcome {
                ActionOutcome::Obeyed { action_msg } => {
                    match cmd {
                        PartyCommand::Attack => {
                            let dmg = rng.gen_range(8..=14);
                            let is_dead = {
                                let mon = self.current_monster_mut();
                                mon.take_damage(dmg);
                                mon.is_dead()
                            };
                            steps.push(BattleStep {
                                message: format!("{}に「たたかう」よう指示した！\n{}\n{}に {}の ダメージ！", members[i].name, action_msg, mon_name, dmg),
                                monster_damage: Some(dmg),
                                member_damage: None,
                                member_heal: None,
                                monster_defeated: is_dead,
                                player_defeated: false,
                                flee_success: false,
                            });
                            if is_dead {
                                break;
                            }
                        }
                        PartyCommand::Defend => {
                            steps.push(BattleStep {
                                message: format!("{}に「みをまもる」よう指示した。\n{}", members[i].name, action_msg),
                                monster_damage: None,
                                member_damage: None,
                                member_heal: None,
                                monster_defeated: false,
                                player_defeated: false,
                                flee_success: false,
                            });
                        }
                        PartyCommand::DesperateAttack => {
                            let dmg = rng.gen_range(20..=30);
                            let is_dead = {
                                let mon = self.current_monster_mut();
                                mon.take_damage(dmg);
                                mon.is_dead()
                            };
                            steps.push(BattleStep {
                                message: format!("{}に「すてみ」を命じた！\n{}\n痛恨の一撃！ {}に {}の ダメージ！", members[i].name, action_msg, mon_name, dmg),
                                monster_damage: Some(dmg),
                                member_damage: None,
                                member_heal: None,
                                monster_defeated: is_dead,
                                player_defeated: false,
                                flee_success: false,
                            });
                            if is_dead {
                                break;
                            }
                        }
                        PartyCommand::CastSpell => {
                            let dmg = rng.gen_range(14..=22);
                            let is_dead = {
                                let mon = self.current_monster_mut();
                                mon.take_damage(dmg);
                                mon.is_dead()
                            };
                            steps.push(BattleStep {
                                message: format!("{}に「じゅもん」を命じた！\n{}\n火炎弾が炸裂！ {}に {}の ダメージ！", members[i].name, action_msg, mon_name, dmg),
                                monster_damage: Some(dmg),
                                member_damage: None,
                                member_heal: None,
                                monster_defeated: is_dead,
                                player_defeated: false,
                                flee_success: false,
                            });
                            if is_dead {
                                break;
                            }
                        }
                    }
                }
                ActionOutcome::Disobeyed { reason_msg, action_msg } => {
                    // ヤンデレの暴走攻撃
                    if members[i].personality == Personality::Yandere {
                        let dmg = rng.gen_range(16..=24);
                        let is_dead = {
                            let mon = self.current_monster_mut();
                            mon.take_damage(dmg);
                            mon.is_dead()
                        };
                        steps.push(BattleStep {
                            message: format!("{}に指示した！\n{}\n{}\nなんと {}に {}の 暴走ダメージ！", members[i].name, reason_msg, action_msg, mon_name, dmg),
                            monster_damage: Some(dmg),
                            member_damage: None,
                            member_heal: None,
                            monster_defeated: is_dead,
                            player_defeated: false,
                            flee_success: false,
                        });
                        if is_dead {
                            break;
                        }
                    } else {
                        steps.push(BattleStep {
                            message: format!("{}に指示した！\n{}\n{}", members[i].name, reason_msg, action_msg),
                            monster_damage: None,
                            member_damage: None,
                            member_heal: None,
                            monster_defeated: false,
                            player_defeated: false,
                            flee_success: false,
                        });
                    }
                }
            }
        }

        // 3. 敵モンスターの反撃（生きていれば）
        if !self.current_monster().is_dead() {
            // 生存メンバーから攻撃対象を選定
            let living_indices: Vec<usize> = (0..members.len()).filter(|&idx| members[idx].hp > 0).collect();
            if !living_indices.is_empty() {
                let target_pick = living_indices[rng.gen_range(0..living_indices.len())];

                // あなたが狙われた場合、ミレイのヤンデレ庇いが発生することがある！
                let yandere_idx = members.iter().position(|m| m.personality == Personality::Yandere && m.hp > 0);
                if target_pick == 0 && yandere_idx.is_some() && rng.gen_bool(0.4) {
                    let y_idx = yandere_idx.unwrap();
                    let raw_dmg = rng.gen_range(6..=12);
                    members[y_idx].hp = (members[y_idx].hp - raw_dmg).max(0);
                    steps.push(BattleStep {
                        message: format!(
                            "{}の攻撃があなたに迫る！\n魔法使いミレイ「あなたに触るなッ！」\nミレイが前に飛び出して身代わりになった！ ミレイに {}の ダメージ！",
                            mon_name, raw_dmg
                        ),
                        monster_damage: None,
                        member_damage: Some((y_idx, raw_dmg)),
                        member_heal: None,
                        monster_defeated: false,
                        player_defeated: false,
                        flee_success: false,
                    });
                } else {
                    let mut raw_dmg = rng.gen_range(5..=10);
                    if target_pick == 0 && player_defending {
                        raw_dmg = (raw_dmg / 2).max(1);
                    }
                    members[target_pick].hp = (members[target_pick].hp - raw_dmg).max(0);
                    let is_player_dead = target_pick == 0 && members[0].hp <= 0;

                    let target_name = &members[target_pick].name;
                    steps.push(BattleStep {
                        message: format!("{}の反撃！\n{}は {}の ダメージを受けた！", mon_name, target_name, raw_dmg),
                        monster_damage: None,
                        member_damage: Some((target_pick, raw_dmg)),
                        member_heal: None,
                        monster_defeated: false,
                        player_defeated: is_player_dead,
                        flee_success: false,
                    });
                }
            }
        }

        self.turn_steps = steps;
        self.phase = BattlePhase::TurnResolving { step_cursor: 0 };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::{Influence, MentalState, Personality};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn create_test_party() -> Vec<PartyMember> {
        vec![
            PartyMember::new_player("あなた").with_stats(20, 0),
            PartyMember::new(
                "戦士ガルツ",
                "せんし",
                Influence::new_natural(85),
                MentalState::Normal,
                Personality::Loyal,
            )
            .with_stats(35, 10),
            PartyMember::new(
                "遊び人ロロ",
                "あそびにん",
                Influence::new_natural(40),
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
        ]
    }

    #[test]
    fn test_battle_turn_resolution_and_counterattack() {
        let mut battle = BattleState::new(vec![Monster::new("テストスライム", 100, "")]);
        let mut party = create_test_party();
        let skills = PlayerSkills {
            magic_knowledge: 50,
            keen_eye: 50,
        };
        let mut rng = StdRng::seed_from_u64(12345);

        battle.player_action = Some(PlayerBattleAction::Attack);
        battle.party_commands = vec![
            None,
            Some(PartyCommand::Attack),
            Some(PartyCommand::Defend),
            Some(PartyCommand::Attack),
        ];

        battle.build_turn_resolution(&mut party, &skills, &mut rng);

        // ステップが生成され、TurnResolvingフェーズになっていること
        assert!(!battle.turn_steps.is_empty());
        assert_eq!(battle.phase, BattlePhase::TurnResolving { step_cursor: 0 });

        // 敵にダメージが入っていること
        assert!(battle.current_monster().hp < 100);

        // 敵が生きていれば敵の反撃ステップが含まれること
        let has_counter = battle
            .turn_steps
            .iter()
            .any(|s| s.message.contains("反撃") || s.message.contains("迫る"));
        assert!(has_counter);
    }

    #[test]
    fn test_battle_player_defeated() {
        let mut battle = BattleState::new(vec![Monster::new("強敵ドラゴン", 100, "")]);
        let mut party = create_test_party();
        // あなたのHPを残り1にする
        party[0].hp = 1;
        // 他のメンバーを戦闘不能にしてあなただけを狙わせる
        party[1].hp = 0;
        party[2].hp = 0;
        party[3].hp = 0;

        let skills = PlayerSkills {
            magic_knowledge: 50,
            keen_eye: 50,
        };
        let mut rng = StdRng::seed_from_u64(999);

        battle.player_action = Some(PlayerBattleAction::Attack);
        battle.party_commands = vec![None, None, None, None];

        battle.build_turn_resolution(&mut party, &skills, &mut rng);

        // あなたが倒されたステップが存在すること
        let has_player_defeated = battle.turn_steps.iter().any(|s| s.player_defeated);
        assert!(has_player_defeated);
        assert_eq!(party[0].hp, 0);
    }

    #[test]
    fn test_battle_monster_defeated_stops_action() {
        // HPがわずか3の敵
        let mut battle = BattleState::new(vec![Monster::new("ひん死スライム", 3, "")]);
        let mut party = create_test_party();
        let skills = PlayerSkills {
            magic_knowledge: 50,
            keen_eye: 50,
        };
        let mut rng = StdRng::seed_from_u64(42);

        // あなたの攻撃（石投げ: 2〜5ダメ）で確実に倒せるか判定
        battle.player_action = Some(PlayerBattleAction::Attack);
        battle.party_commands = vec![
            None,
            Some(PartyCommand::Attack),
            Some(PartyCommand::Attack),
            Some(PartyCommand::Attack),
        ];

        battle.build_turn_resolution(&mut party, &skills, &mut rng);

        // 倒されたらそこでステップが終了し、撃破フラグが立つこと
        assert!(battle.current_monster().is_dead());
        let last_step = battle.turn_steps.last().unwrap();
        assert!(last_step.monster_defeated);
    }
}
