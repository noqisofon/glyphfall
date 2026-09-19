use crate::party::{
    diagnose_member, evaluate_command, ActionOutcome, PartyCommand, PartyMember, Personality,
    PlayerBattleAction, PlayerSkills,
};
use crate::timer::SimpleTimer;
use rand::Rng;

/// 敵モンスターの定義
#[derive(Debug, Clone)]
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
pub struct BattleState {
    pub current_index: usize,
    pub monsters: Vec<Monster>,
    pub flash_timer: SimpleTimer,
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
            flash_timer: SimpleTimer::from_seconds(0.18),
            is_flashing: false,
            phase: BattlePhase::CommandInput { member_cursor: 0 },
            player_action: None,
            party_commands: Vec::new(),
            turn_steps: Vec::new(),
        }
    }

    pub fn current_monster(&self) -> &Monster {
        &self.monsters[self.current_index]
    }

    pub fn current_monster_mut(&mut self) -> &mut Monster {
        &mut self.monsters[self.current_index]
    }

    pub fn reset_turn_commands(&mut self, member_count: usize) {
        self.phase = BattlePhase::CommandInput { member_cursor: 0 };
        self.player_action = None;
        self.party_commands = vec![None; member_count];
        self.turn_steps.clear();
    }

    pub fn set_party_command(&mut self, index: usize, command: PartyCommand) {
        if index >= self.party_commands.len() {
            self.party_commands.resize(index + 1, None);
        }
        self.party_commands[index] = Some(command);
    }

    pub fn next_monster(&mut self, member_count: usize) {
        self.current_index = (self.current_index + 1) % self.monsters.len();
        self.monsters[self.current_index].hp = self.monsters[self.current_index].max_hp;
        self.reset_turn_commands(member_count);
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
        members: &[PartyMember],
        skills: &PlayerSkills,
        rng: &mut R,
    ) {
        let mut steps = Vec::new();
        let mon_name = self.current_monster().name.clone();
        let mut sim_mon_hp = self.current_monster().hp;
        let mut sim_member_hps: Vec<i32> = members.iter().map(|m| m.hp).collect();

        // 1. あなた（主人公）の行動
        let player_action = self.player_action.unwrap_or(PlayerBattleAction::Attack);
        let mut player_defending = false;

        match player_action {
            PlayerBattleAction::Attack => {
                let dmg = rng.gen_range(2..=5);
                sim_mon_hp = (sim_mon_hp - dmg).max(0);
                let is_dead = sim_mon_hp <= 0;
                steps.push(BattleStep {
                    message: format!(
                        "あなた（一般人）は　必死に石を投げつけた！\n{}に {}の ダメージ！",
                        mon_name, dmg
                    ),
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
                // 生存している味方の中から最もHPの割合が低い味方を回復
                let living_targets: Vec<usize> = (0..members.len())
                    .filter(|&i| sim_member_hps[i] > 0)
                    .collect();

                if let Some(&target_idx) = living_targets
                    .iter()
                    .min_by_key(|&&i| sim_member_hps[i] * 100 / members[i].max_hp.max(1))
                {
                    let heal_amount = 20;
                    let prev_hp = sim_member_hps[target_idx];
                    let new_hp = (prev_hp + heal_amount).min(members[target_idx].max_hp);
                    let actual_heal = new_hp - prev_hp;
                    sim_member_hps[target_idx] = new_hp;

                    let target_name = &members[target_idx].name;
                    steps.push(BattleStep {
                        message: format!(
                            "あなたは　やくそうを取り出し　{}にあてた！\n{}のHPが {} 回復した！",
                            target_name, target_name, actual_heal
                        ),
                        monster_damage: None,
                        member_damage: None,
                        member_heal: Some((target_idx, actual_heal)),
                        monster_defeated: false,
                        player_defeated: false,
                        flee_success: false,
                    });
                } else {
                    steps.push(BattleStep {
                        message: "あなたは　やくそうを取り出したが、使う相手がいなかった！".into(),
                        monster_damage: None,
                        member_damage: None,
                        member_heal: None,
                        monster_defeated: false,
                        player_defeated: false,
                        flee_success: false,
                    });
                }
            }
            PlayerBattleAction::Diagnose => {
                // 生存している仲間（インデックス1..members.len()）からランダムで観察
                let living_companions: Vec<usize> = (1..members.len())
                    .filter(|&i| sim_member_hps[i] > 0)
                    .collect();
                let message = if !living_companions.is_empty() {
                    let pick = rng.gen_range(0..living_companions.len());
                    let target_idx = living_companions[pick];
                    let report = diagnose_member(skills, &members[target_idx], rng);
                    format!(
                        "あなた「{}の様子を見よう」\n{}\n{}",
                        members[target_idx].name, report.observation_msg, report.conclusion_msg
                    )
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
                        message:
                            "あなたは仲間たちに合図を出し、脱出路へ走り込んだ！\nうまく逃げ切れた！"
                                .into(),
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
        for (i, member) in members.iter().enumerate().skip(1) {
            if sim_mon_hp <= 0 {
                break;
            }
            if sim_member_hps[i] <= 0 {
                // 戦闘不能なメンバーは行動できない
                continue;
            }
            let cmd = self
                .party_commands
                .get(i)
                .and_then(|c| *c)
                .unwrap_or(PartyCommand::Attack);
            let outcome = evaluate_command(member, cmd, rng);

            match outcome {
                ActionOutcome::Obeyed { action_msg } => {
                    match cmd {
                        PartyCommand::Attack => {
                            let dmg = rng.gen_range(8..=14);
                            sim_mon_hp = (sim_mon_hp - dmg).max(0);
                            let is_dead = sim_mon_hp <= 0;
                            steps.push(BattleStep {
                                message: format!(
                                    "{}に「たたかう」よう指示した！\n{}\n{}に {}の ダメージ！",
                                    member.name, action_msg, mon_name, dmg
                                ),
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
                                message: format!(
                                    "{}に「みをまもる」よう指示した。\n{}",
                                    member.name, action_msg
                                ),
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
                            sim_mon_hp = (sim_mon_hp - dmg).max(0);
                            let is_dead = sim_mon_hp <= 0;
                            steps.push(BattleStep {
                                message: format!("{}に「すてみ」を命じた！\n{}\n痛恨の一撃！ {}に {}の ダメージ！", member.name, action_msg, mon_name, dmg),
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
                            sim_mon_hp = (sim_mon_hp - dmg).max(0);
                            let is_dead = sim_mon_hp <= 0;
                            steps.push(BattleStep {
                                message: format!("{}に「じゅもん」を命じた！\n{}\n火炎弾が炸裂！ {}に {}の ダメージ！", member.name, action_msg, mon_name, dmg),
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
                ActionOutcome::Disobeyed {
                    reason_msg,
                    action_msg,
                } => {
                    // ヤンデレの暴走攻撃
                    if member.personality == Personality::Yandere {
                        let dmg = rng.gen_range(16..=24);
                        sim_mon_hp = (sim_mon_hp - dmg).max(0);
                        let is_dead = sim_mon_hp <= 0;
                        steps.push(BattleStep {
                            message: format!(
                                "{}に指示した！\n{}\n{}\nなんと {}に {}の 暴走ダメージ！",
                                member.name, reason_msg, action_msg, mon_name, dmg
                            ),
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
                            message: format!(
                                "{}に指示した！\n{}\n{}",
                                member.name, reason_msg, action_msg
                            ),
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
        if sim_mon_hp > 0 {
            // 生存メンバーから攻撃対象を選定
            let living_indices: Vec<usize> = (0..members.len())
                .filter(|&idx| sim_member_hps[idx] > 0)
                .collect();
            if !living_indices.is_empty() {
                let target_pick = living_indices[rng.gen_range(0..living_indices.len())];

                // あなたが狙われた場合、ミレイ等のヤンデレ庇いが発生することがある！
                let yandere_cover = if target_pick == 0 && rng.gen_bool(0.4) {
                    (1..members.len()).find(|&i| {
                        members[i].personality == Personality::Yandere && sim_member_hps[i] > 0
                    })
                } else {
                    None
                };

                if let Some(y_idx) = yandere_cover {
                    let raw_dmg = rng.gen_range(6..=12);
                    sim_member_hps[y_idx] = (sim_member_hps[y_idx] - raw_dmg).max(0);
                    let y_name = &members[y_idx].name;
                    steps.push(BattleStep {
                        message: format!(
                            "{}の攻撃があなたに迫る！\n{}「あなたに触るなッ！」\n{}が前に飛び出して身代わりになった！ {}に {}の ダメージ！",
                            mon_name, y_name, y_name, y_name, raw_dmg
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
                    sim_member_hps[target_pick] = (sim_member_hps[target_pick] - raw_dmg).max(0);
                    let is_player_dead = target_pick == 0 && sim_member_hps[0] <= 0;

                    let target_name = &members[target_pick].name;
                    steps.push(BattleStep {
                        message: format!(
                            "{}の反撃！\n{}は {}の ダメージを受けた！",
                            mon_name, target_name, raw_dmg
                        ),
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

    /// 指定ステップの効果（モンスター被ダメージ、味方被ダメージ、味方回復）を適用する
    pub fn apply_step_effects(&mut self, step_idx: usize, members: &mut [PartyMember]) {
        let (monster_dmg, member_dmg, member_heal) = match self.turn_steps.get(step_idx) {
            Some(step) => (step.monster_damage, step.member_damage, step.member_heal),
            None => return,
        };

        if let Some(dmg) = monster_dmg {
            self.current_monster_mut().take_damage(dmg);
            self.is_flashing = true;
            self.flash_timer.reset();
        }
        if let Some((idx, dmg)) = member_dmg {
            if idx < members.len() {
                members[idx].hp = (members[idx].hp - dmg).max(0);
            }
        }
        if let Some((idx, heal)) = member_heal {
            if idx < members.len() {
                members[idx].hp = (members[idx].hp + heal).min(members[idx].max_hp);
            }
        }
    }

    /// 全ステップの効果を一括適用する（主にテスト用ヘルパー）
    pub fn execute_all_steps(&mut self, members: &mut [PartyMember]) {
        for i in 0..self.turn_steps.len() {
            self.apply_step_effects(i, members);
        }
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

        battle.build_turn_resolution(&party, &skills, &mut rng);

        // ステップが生成され、TurnResolvingフェーズになっていること
        assert!(!battle.turn_steps.is_empty());
        assert_eq!(battle.phase, BattlePhase::TurnResolving { step_cursor: 0 });

        // ステップ適用前は敵のHPはそのまま
        assert_eq!(battle.current_monster().hp, 100);

        // 全ステップ適用後に敵にダメージが入っていること
        battle.execute_all_steps(&mut party);
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

        battle.build_turn_resolution(&party, &skills, &mut rng);

        // あなたが倒されたステップが存在すること
        let has_player_defeated = battle.turn_steps.iter().any(|s| s.player_defeated);
        assert!(has_player_defeated);

        // ステップ適用後にあなたのHPが0になること
        battle.execute_all_steps(&mut party);
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

        battle.build_turn_resolution(&party, &skills, &mut rng);

        // ステップの最後で撃破フラグが立つこと
        let last_step = battle.turn_steps.last().unwrap();
        assert!(last_step.monster_defeated);

        // ステップ適用でHPが0になること
        battle.execute_all_steps(&mut party);
        assert!(battle.current_monster().is_dead());
    }

    #[test]
    fn test_battle_use_item_does_not_revive_dead_member() {
        let mut battle = BattleState::new(vec![Monster::new("スライム", 100, "")]);
        let mut party = create_test_party();
        // ガルツは死亡、ロロは負傷（HP 5 / 25）
        party[1].hp = 0;
        party[2].hp = 5;

        let skills = PlayerSkills::default();
        let mut rng = StdRng::seed_from_u64(1);

        battle.player_action = Some(PlayerBattleAction::UseItem);
        battle.party_commands = vec![
            None,
            None,
            Some(PartyCommand::Defend),
            Some(PartyCommand::Defend),
        ];

        battle.build_turn_resolution(&party, &skills, &mut rng);

        // ステップ適用
        battle.execute_all_steps(&mut party);

        // ガルツ（死亡）は 0 のままで蘇生しない
        assert_eq!(party[1].hp, 0);
        // ロロ（負傷）が回復していること
        assert!(party[2].hp > 5);
    }

    #[test]
    fn test_battle_solo_player_resolution() {
        // 主人公1人（仲間なし）のパーティでの戦闘解決テスト
        let mut battle = BattleState::new(vec![Monster::new("テストスライム", 30, "")]);
        let mut solo_party = vec![PartyMember::new_player("あなた").with_stats(20, 0)];
        let skills = PlayerSkills::default();
        let mut rng = StdRng::seed_from_u64(100);

        battle.reset_turn_commands(solo_party.len());
        assert_eq!(battle.party_commands.len(), 1);

        battle.player_action = Some(PlayerBattleAction::Attack);

        battle.build_turn_resolution(&solo_party, &skills, &mut rng);

        // 主人公の攻撃ステップと敵の反撃ステップが存在すること
        assert_eq!(battle.turn_steps.len(), 2);
        assert!(battle.turn_steps[0].message.contains("石を投げつけた"));
        assert!(battle.turn_steps[1].message.contains("反撃"));
        assert_eq!(battle.phase, BattlePhase::TurnResolving { step_cursor: 0 });

        // ステップ0適用でモンスターにダメージ
        battle.apply_step_effects(0, &mut solo_party);
        assert!(battle.current_monster().hp < 30);
    }
}
