//! ADR-0013: 突発イベント発生システム（トリガー／抽選エンジン）
//!
//! 「発生するかどうか」（呼び出し元が渡す基礎確率）と「発生するなら何が選ばれるか」
//! （本モジュールが握る重み付き抽選）を分離した2段階抽選として実装する。
//! 発生後の効果（会話・戦闘突入など）はここでは扱わず、`SuddenEventTriggered` を
//! 受け取った呼び出し側（現状は `main.rs` の旅シミュレーション処理）に委ねる。

use rand::Rng;
use std::collections::{HashMap, HashSet};

/// ADR-0004に記載された突発イベントの初期カテゴリ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuddenEventCategory {
    Bandit,
    WildAnimal,
    PotentialCompanion,
    DyingTraveler,
    Checkpoint,
    WeatherShift,
}

/// イベントごとの再発生ルール。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReoccurrenceRule {
    /// 何度でも再発生しうる。
    Unlimited,
    /// 直近の発生から最低`_0`トリップ空けないと再抽選対象にならない。
    Cooldown(u32),
    /// 生涯で一度しか発生しない。
    OneShot,
}

#[derive(Clone, Copy, Debug)]
pub struct SuddenEventDef {
    pub id: &'static str,
    pub category: SuddenEventCategory,
    pub weight: f32,
    pub rule: ReoccurrenceRule,
    pub message: &'static str,
}

/// 起動時にロードするイベント定義の一覧（ADR-0013時点ではRustのリテラル定義。
/// 将来Luaなど外部スクリプトからのロードに差し替える際も、この構造体の形は
/// 変えずにロード元だけを差し替えられるようにしてある）。
pub struct SuddenEventRegistry {
    pub events: Vec<SuddenEventDef>,
}

impl SuddenEventRegistry {
    /// 旅シミュレーション（ADR-0004）向けの初期イベント一覧。
    pub fn travel_default() -> Self {
        Self {
            events: vec![
                SuddenEventDef {
                    id: "bandit_ambush",
                    category: SuddenEventCategory::Bandit,
                    weight: 3.0,
                    rule: ReoccurrenceRule::Unlimited,
                    message: "街道の物陰から山賊が飛び出してきた！",
                },
                SuddenEventDef {
                    id: "wild_animal",
                    category: SuddenEventCategory::WildAnimal,
                    weight: 2.0,
                    rule: ReoccurrenceRule::Unlimited,
                    message: "唸り声とともに、大きなクマが襲いかかってきた！",
                },
                SuddenEventDef {
                    id: "potential_companion",
                    category: SuddenEventCategory::PotentialCompanion,
                    weight: 0.5,
                    rule: ReoccurrenceRule::OneShot,
                    message: "旅装の若者が、物言いたげにこちらをじっと見ている……（仲間化はまだ実装されていない）",
                },
                SuddenEventDef {
                    id: "dying_traveler",
                    category: SuddenEventCategory::DyingTraveler,
                    weight: 1.0,
                    rule: ReoccurrenceRule::Cooldown(3),
                    message: "道端に旅人が力尽きて倒れている……（救助はまだ実装されていない）",
                },
                SuddenEventDef {
                    id: "checkpoint",
                    category: SuddenEventCategory::Checkpoint,
                    weight: 1.5,
                    rule: ReoccurrenceRule::Unlimited,
                    message: "検問中の衛兵に呼び止められ、荷物を確認された。",
                },
                SuddenEventDef {
                    id: "weather_shift",
                    category: SuddenEventCategory::WeatherShift,
                    weight: 1.5,
                    rule: ReoccurrenceRule::Unlimited,
                    message: "急に空が搔き曇り、冷たい雨が降り出した。",
                },
            ],
        }
    }
}

/// イベントごとのクールダウン・一発限り消化状況（ADR-0013）。
#[derive(Default)]
pub struct SuddenEventHistory {
    /// 街道移動（旅）を1回試みるごとに1つ進む単調カウンタ。実時間や日数ではなく
    /// 「旅を何回試みたか」を単位にすることで、時間経過システム未実装のうちも
    /// クールダウンを表現できるようにしている。
    trip_count: u32,
    last_triggered_at: HashMap<&'static str, u32>,
    consumed_one_shots: HashSet<&'static str>,
}

impl SuddenEventHistory {
    fn is_available(&self, def: &SuddenEventDef, trip_count: u32) -> bool {
        match def.rule {
            ReoccurrenceRule::Unlimited => true,
            ReoccurrenceRule::OneShot => !self.consumed_one_shots.contains(def.id),
            ReoccurrenceRule::Cooldown(min_interval) => self
                .last_triggered_at
                .get(def.id)
                .map(|&last| trip_count.saturating_sub(last) >= min_interval)
                .unwrap_or(true),
        }
    }

    fn record(&mut self, def: &SuddenEventDef, trip_count: u32) {
        self.last_triggered_at.insert(def.id, trip_count);
        if def.rule == ReoccurrenceRule::OneShot {
            self.consumed_one_shots.insert(def.id);
        }
    }
}

/// 抽選エンジンが発火する「どのイベントが選ばれたか」の通知。実際の演出・分岐
/// （会話・戦闘突入・仲間加入判定など）はこれを受け取った呼び出し側の責務とする。
#[derive(Clone, Copy, Debug)]
pub struct SuddenEventTriggered {
    /// 呼び出し側での将来のログ出力・条件分岐（例: 特定イベントIDだけ個別演出を出す）
    /// のために残してあるが、現状の呼び出し側は`category`しか見ていない。
    #[allow(dead_code)]
    pub id: &'static str,
    pub category: SuddenEventCategory,
    pub message: &'static str,
}

/// 2段階抽選（ADR-0013）。
///
/// 1. `base_probability`（呼び出し元が握る文脈依存の確率。旅シミュレーションでは
///    ADR-0004の移動姿勢別エンカウント率）で「何か起きるか」をロールする。
/// 2. 起きる場合、クールダウン／OneShotの制約を満たす候補群から`weight`に基づく
///    重み付き抽選で1件選ぶ。候補が0件なら何も起こさない。
pub fn try_trigger_sudden_event<R: Rng>(
    registry: &SuddenEventRegistry,
    history: &mut SuddenEventHistory,
    base_probability: f32,
    rng: &mut R,
) -> Option<SuddenEventTriggered> {
    history.trip_count += 1;
    let trip_count = history.trip_count;

    if !rng.gen_bool(base_probability.clamp(0.0, 1.0) as f64) {
        return None;
    }

    let candidates: Vec<&SuddenEventDef> = registry
        .events
        .iter()
        .filter(|def| history.is_available(def, trip_count))
        .collect();

    let total_weight: f32 = candidates.iter().map(|def| def.weight).sum();
    if candidates.is_empty() || total_weight <= 0.0 {
        return None;
    }

    let mut roll = rng.gen_range(0.0..total_weight);
    let chosen = *candidates.iter().find(|def| {
        if roll < def.weight {
            true
        } else {
            roll -= def.weight;
            false
        }
    })?;

    history.record(chosen, trip_count);

    Some(SuddenEventTriggered {
        id: chosen.id,
        category: chosen.category,
        message: chosen.message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn zero_probability_never_triggers() {
        let registry = SuddenEventRegistry::travel_default();
        let mut history = SuddenEventHistory::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);

        for _ in 0..20 {
            assert!(try_trigger_sudden_event(&registry, &mut history, 0.0, &mut rng).is_none());
        }
    }

    #[test]
    fn certain_probability_always_triggers_when_candidates_remain() {
        let registry = SuddenEventRegistry::travel_default();
        let mut history = SuddenEventHistory::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);

        for _ in 0..20 {
            assert!(try_trigger_sudden_event(&registry, &mut history, 1.0, &mut rng).is_some());
        }
    }

    #[test]
    fn one_shot_event_fires_at_most_once() {
        // 一発限りイベントだけのレジストリで確認する。
        let registry = SuddenEventRegistry {
            events: vec![SuddenEventDef {
                id: "only_one_shot",
                category: SuddenEventCategory::PotentialCompanion,
                weight: 1.0,
                rule: ReoccurrenceRule::OneShot,
                message: "一期一会の出会い。",
            }],
        };
        let mut history = SuddenEventHistory::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);

        let first = try_trigger_sudden_event(&registry, &mut history, 1.0, &mut rng);
        assert!(first.is_some());

        for _ in 0..10 {
            assert!(try_trigger_sudden_event(&registry, &mut history, 1.0, &mut rng).is_none());
        }
    }

    #[test]
    fn cooldown_event_waits_minimum_trips_before_reoccurring() {
        let registry = SuddenEventRegistry {
            events: vec![SuddenEventDef {
                id: "only_cooldown",
                category: SuddenEventCategory::DyingTraveler,
                weight: 1.0,
                rule: ReoccurrenceRule::Cooldown(3),
                message: "行き倒れの旅人。",
            }],
        };
        let mut history = SuddenEventHistory::default();
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);

        assert!(try_trigger_sudden_event(&registry, &mut history, 1.0, &mut rng).is_some());
        // クールダウン中（trip_count 2, 3）は候補が無くなり発生しない。
        assert!(try_trigger_sudden_event(&registry, &mut history, 1.0, &mut rng).is_none());
        assert!(try_trigger_sudden_event(&registry, &mut history, 1.0, &mut rng).is_none());
        // trip_count 4 目でクールダウン(3)を満たし再度発生する。
        assert!(try_trigger_sudden_event(&registry, &mut history, 1.0, &mut rng).is_some());
    }
}
