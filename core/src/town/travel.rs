//! ADR-0004 / ADR-0013 / ADR-0026: 街道旅シミュレーション
//!
//! 旅計画（移動先×姿勢×手段）と暗転ファストトラベル進行ステートマシン。

use super::map::AreaId;
use super::movement::Position;
use crate::event::{
    try_trigger_sudden_event, SuddenEventHistory, SuddenEventRegistry, SuddenEventTriggered,
};

/// 移動姿勢（ADR-0004）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TravelPosture {
    Cautious,
    #[default]
    Normal,
    Bold,
}

impl TravelPosture {
    pub const ALL: [TravelPosture; 3] = [
        TravelPosture::Cautious,
        TravelPosture::Normal,
        TravelPosture::Bold,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            TravelPosture::Cautious => "慎重に",
            TravelPosture::Normal => "普通に",
            TravelPosture::Bold => "大胆に",
        }
    }

    /// 基礎エンカウント確率
    pub fn base_probability(&self) -> f32 {
        match self {
            TravelPosture::Cautious => 0.15,
            TravelPosture::Normal => 0.35,
            TravelPosture::Bold => 0.65,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TravelPosture::Cautious => "警戒を怠らず進む。遭遇率: 15% / 所要日数: 標準",
            TravelPosture::Normal => "無理のないペースで進む。遭遇率: 35% / 所要日数: 基準",
            TravelPosture::Bold => "危険を承知で急行する。遭遇率: 65% / 所要日数: 1日短縮",
        }
    }
}

/// 移動手段（ADR-0026）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TravelTransport {
    #[default]
    Foot,
    Horse,
    Carriage,
}

impl TravelTransport {
    pub const ALL: [TravelTransport; 3] = [
        TravelTransport::Foot,
        TravelTransport::Horse,
        TravelTransport::Carriage,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            TravelTransport::Foot => "徒歩",
            TravelTransport::Horse => "馬",
            TravelTransport::Carriage => "乗合馬車",
        }
    }

    /// 必要費用（フォリン / G）
    pub fn cost(&self) -> i32 {
        match self {
            TravelTransport::Foot => 0,
            TravelTransport::Horse => 40,
            TravelTransport::Carriage => 80,
        }
    }

    /// 基準所要日数
    pub fn base_days(&self) -> u32 {
        match self {
            TravelTransport::Foot => 3,
            TravelTransport::Horse => 1,
            TravelTransport::Carriage => 2,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TravelTransport::Foot => "自らの足で歩く。費用: 0G / 所要: 3日 / 安全度: 基準",
            TravelTransport::Horse => {
                "貸し馬で疾走する。費用: 40G / 所要: 1日 / 安全度: 高め (少人数)"
            }
            TravelTransport::Carriage => {
                "乗合馬車に乗る。費用: 80G / 所要: 2日 / 安全度: 山賊注意 (目立つ)"
            }
        }
    }
}

/// 旅の計画
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TravelPlan {
    pub from_area: AreaId,
    pub destination: AreaId,
    pub posture: TravelPosture,
    pub transport: TravelTransport,
}

impl Default for TravelPlan {
    fn default() -> Self {
        Self {
            from_area: AreaId::Town,
            destination: AreaId::Village,
            posture: TravelPosture::Normal,
            transport: TravelTransport::Foot,
        }
    }
}

impl TravelPlan {
    pub fn new(
        from_area: AreaId,
        destination: AreaId,
        posture: TravelPosture,
        transport: TravelTransport,
    ) -> Self {
        Self {
            from_area,
            destination,
            posture,
            transport,
        }
    }

    /// 姿勢と手段から総所要日数を算出
    pub fn calculate_total_days(&self) -> u32 {
        let base = self.transport.base_days();
        match self.posture {
            TravelPosture::Cautious => base,
            TravelPosture::Normal => base,
            TravelPosture::Bold => base.saturating_sub(1).max(1),
        }
    }
}

/// 旅シミュレーションの進行フェーズ
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TravelPhase {
    /// 計画フェーズ Step 1: 移動先選択
    #[default]
    ChoosingDestination,
    /// 計画フェーズ Step 2: 移動姿勢選択
    ChoosingPosture,
    /// 計画フェーズ Step 3: 移動手段選択
    ChoosingTransport,
    /// 暗転ファストトラベル進行中（メッセージ送り待機）
    Traveling,
    /// 突発イベント発生中（選択肢・戦闘突入待機）
    EncounterEvent,
    /// 目的地到着完了
    Arrived,
}

/// 1日の旅程進行結果
#[derive(Debug, Clone)]
pub enum TravelStepOutcome {
    Continued {
        day: u32,
        total_days: u32,
        message: String,
    },
    EventOccurred {
        day: u32,
        total_days: u32,
        event: SuddenEventTriggered,
        message: String,
    },
    Arrived {
        total_days: u32,
        message: String,
    },
}

/// 旅シミュレーション管理ステート
#[derive(Clone, Debug)]
pub struct TravelSimulation {
    pub plan: TravelPlan,
    pub phase: TravelPhase,
    pub current_day: u32,
    pub total_days: u32,
    pub spawn_pos: Position,
    pub current_message: String,
    pub log_history: Vec<String>,
    pub pending_event: Option<SuddenEventTriggered>,
    /// 旅計画UIの選択中カーソル
    pub selected_index: usize,
}

impl Default for TravelSimulation {
    fn default() -> Self {
        Self::new(AreaId::Town, AreaId::Village, Position { x: 20, y: 5 })
    }
}

impl TravelSimulation {
    pub fn new(from_area: AreaId, destination: AreaId, spawn_pos: Position) -> Self {
        let plan = TravelPlan {
            from_area,
            destination,
            posture: TravelPosture::Normal,
            transport: TravelTransport::Foot,
        };
        let total_days = plan.calculate_total_days();
        Self {
            plan,
            phase: TravelPhase::ChoosingDestination,
            current_day: 0,
            total_days,
            spawn_pos,
            current_message: String::new(),
            log_history: Vec::new(),
            pending_event: None,
            selected_index: 0,
        }
    }

    /// 移動先の選択を行い、移動姿勢選択フェーズへ移行
    pub fn select_destination(&mut self, destination: AreaId, spawn_pos: Position) {
        self.plan.destination = destination;
        self.spawn_pos = spawn_pos;
        self.phase = TravelPhase::ChoosingPosture;
        self.selected_index = 0;
    }

    /// 移動姿勢の選択を行い、移動手段選択フェーズへ移行
    pub fn select_posture(&mut self, posture: TravelPosture) {
        self.plan.posture = posture;
        self.phase = TravelPhase::ChoosingTransport;
        self.selected_index = 0;
    }

    /// 移動手段の選択
    pub fn select_transport(&mut self, transport: TravelTransport) {
        self.plan.transport = transport;
    }

    /// 計画フェーズで1段階前に戻る。ChoosingDestination であれば false を返す
    pub fn back_phase(&mut self) -> bool {
        match self.phase {
            TravelPhase::ChoosingPosture => {
                self.phase = TravelPhase::ChoosingDestination;
                self.selected_index = 0;
                true
            }
            TravelPhase::ChoosingTransport => {
                self.phase = TravelPhase::ChoosingPosture;
                self.selected_index = 0;
                true
            }
            _ => false,
        }
    }

    /// 計画完了後、暗転旅程を開始する
    pub fn start_travel(&mut self) -> String {
        self.phase = TravelPhase::Traveling;
        self.current_day = 1;
        self.total_days = self.plan.calculate_total_days();
        self.log_history.clear();
        self.pending_event = None;

        let msg = self.generate_day_scenery(1);
        self.current_message = msg.clone();
        self.log_history.push(msg.clone());
        msg
    }

    /// 1日進める
    pub fn advance_day<R: rand::Rng>(
        &mut self,
        registry: &SuddenEventRegistry,
        history: &mut SuddenEventHistory,
        rng: &mut R,
    ) -> TravelStepOutcome {
        // 突発イベントの抽選
        let triggered =
            try_trigger_sudden_event(registry, history, self.plan.posture.base_probability(), rng);

        if let Some(event) = triggered {
            let msg = format!(
                "【第{}日目】街道を進む一行に突発事態が発生！\n{}",
                self.current_day, event.message
            );
            self.phase = TravelPhase::EncounterEvent;
            self.pending_event = Some(event);
            self.current_message = msg.clone();
            self.log_history.push(msg.clone());
            return TravelStepOutcome::EventOccurred {
                day: self.current_day,
                total_days: self.total_days,
                event,
                message: msg,
            };
        }

        // イベントなしの場合
        if self.current_day >= self.total_days {
            self.phase = TravelPhase::Arrived;
            let msg = format!(
                "【到着】{}日間の旅路の末、{}に無事到着した！",
                self.total_days,
                self.plan.destination.name()
            );
            self.current_message = msg.clone();
            self.log_history.push(msg.clone());
            TravelStepOutcome::Arrived {
                total_days: self.total_days,
                message: msg,
            }
        } else {
            self.current_day += 1;
            let msg = self.generate_day_scenery(self.current_day);
            self.current_message = msg.clone();
            self.log_history.push(msg.clone());
            TravelStepOutcome::Continued {
                day: self.current_day,
                total_days: self.total_days,
                message: msg,
            }
        }
    }

    /// イベント解決後に旅程を再開する
    pub fn resolve_event_and_continue(&mut self) -> TravelStepOutcome {
        self.pending_event = None;
        if self.current_day >= self.total_days {
            self.phase = TravelPhase::Arrived;
            let msg = format!(
                "【到着】困難を乗り越え、{}に無事到着した！",
                self.plan.destination.name()
            );
            self.current_message = msg.clone();
            self.log_history.push(msg.clone());
            TravelStepOutcome::Arrived {
                total_days: self.total_days,
                message: msg,
            }
        } else {
            self.phase = TravelPhase::Traveling;
            self.current_day += 1;
            let msg = self.generate_day_scenery(self.current_day);
            self.current_message = msg.clone();
            self.log_history.push(msg.clone());
            TravelStepOutcome::Continued {
                day: self.current_day,
                total_days: self.total_days,
                message: msg,
            }
        }
    }

    /// 日数・移動手段に応じた情景メッセージを生成
    fn generate_day_scenery(&self, day: u32) -> String {
        let transport_prefix = match self.plan.transport {
            TravelTransport::Foot => "自らの足で歩みを進める。",
            TravelTransport::Horse => "軽快に馬を駆り、風を切って疾走する。",
            TravelTransport::Carriage => "荷車を引く馬車に揺られ、車輪がきしむ音が響く。",
        };

        if day == 1 {
            format!(
                "【第1日目】{}を出発し、{}へ向けて街道へ踏み出した。{}",
                self.plan.from_area.name(),
                self.plan.destination.name(),
                transport_prefix
            )
        } else if day >= self.total_days {
            format!(
                "【第{}日目】前方に目的地の集落がかすかに見えてきた。{}",
                day, transport_prefix
            )
        } else {
            let scenery = match day % 3 {
                0 => "遠くの山並みに黒い雲が立ち込め、冷たい小雨が降り始めた。",
                1 => "街道沿いの木立から鳥の群れが飛び立つ。穏やかな日差しが差し込む。",
                _ => "道端の苔むした古い道標を通り過ぎる。旅人の祈りの言葉が刻まれている。",
            };
            format!("【第{}日目】{} {}", day, scenery, transport_prefix)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_travel_simulation_phase_transitions() {
        let mut sim = TravelSimulation::new(AreaId::Town, AreaId::Village, Position { x: 5, y: 5 });
        assert_eq!(sim.phase, TravelPhase::ChoosingDestination);

        // Step 1: Destination
        sim.select_destination(AreaId::Village, Position { x: 20, y: 5 });
        assert_eq!(sim.phase, TravelPhase::ChoosingPosture);

        // Back to Destination
        assert!(sim.back_phase());
        assert_eq!(sim.phase, TravelPhase::ChoosingDestination);
        assert!(!sim.back_phase()); // Cannot go back further

        // Forward to Posture
        sim.select_destination(AreaId::Village, Position { x: 20, y: 5 });
        assert_eq!(sim.phase, TravelPhase::ChoosingPosture);

        // Step 2: Posture -> Transport
        sim.select_posture(TravelPosture::Bold);
        assert_eq!(sim.phase, TravelPhase::ChoosingTransport);
        assert_eq!(sim.plan.posture, TravelPosture::Bold);

        // Back to Posture
        assert!(sim.back_phase());
        assert_eq!(sim.phase, TravelPhase::ChoosingPosture);

        // Forward to Transport again
        sim.select_posture(TravelPosture::Normal);
        assert_eq!(sim.phase, TravelPhase::ChoosingTransport);

        // Step 3: Transport & Start
        sim.select_transport(TravelTransport::Carriage);
        assert_eq!(sim.plan.transport, TravelTransport::Carriage);
        let msg = sim.start_travel();
        assert_eq!(sim.phase, TravelPhase::Traveling);
        assert_eq!(sim.current_day, 1);
        assert_eq!(sim.total_days, 2);
        assert!(msg.contains("馬車"));
    }

    #[test]
    fn test_travel_plan_calculate_total_days() {
        // 徒歩 (base 3)
        let plan_foot_normal = TravelPlan::new(
            AreaId::Town,
            AreaId::Village,
            TravelPosture::Normal,
            TravelTransport::Foot,
        );
        assert_eq!(plan_foot_normal.calculate_total_days(), 3);

        let plan_foot_bold = TravelPlan::new(
            AreaId::Town,
            AreaId::Village,
            TravelPosture::Bold,
            TravelTransport::Foot,
        );
        assert_eq!(plan_foot_bold.calculate_total_days(), 2); // 1日短縮

        // 馬 (base 1)
        let plan_horse = TravelPlan::new(
            AreaId::Town,
            AreaId::Village,
            TravelPosture::Normal,
            TravelTransport::Horse,
        );
        assert_eq!(plan_horse.calculate_total_days(), 1);

        // 馬車 (base 2)
        let plan_carriage = TravelPlan::new(
            AreaId::Town,
            AreaId::Village,
            TravelPosture::Cautious,
            TravelTransport::Carriage,
        );
        assert_eq!(plan_carriage.calculate_total_days(), 2);
    }

    #[test]
    fn test_travel_simulation_event_occurrence_and_resolution() {
        let mut sim =
            TravelSimulation::new(AreaId::Town, AreaId::Village, Position { x: 10, y: 10 });
        sim.plan.transport = TravelTransport::Horse; // 1日で到着
        sim.plan.posture = TravelPosture::Cautious;

        sim.start_travel();
        assert_eq!(sim.current_day, 1);

        let registry = SuddenEventRegistry::travel_default();
        let mut history = SuddenEventHistory::default();
        let mut rng = StdRng::seed_from_u64(999); // seed 999 は検問イベントを引く

        let outcome = sim.advance_day(&registry, &mut history, &mut rng);
        match &outcome {
            TravelStepOutcome::EventOccurred { day, event, .. } => {
                assert_eq!(*day, 1);
                assert_eq!(event.id, "checkpoint");
                assert_eq!(sim.phase, TravelPhase::EncounterEvent);
            }
            other => panic!("Expected event, but got {:?}", other),
        }

        // イベント解決して旅程完了
        let resolve_outcome = sim.resolve_event_and_continue();
        match resolve_outcome {
            TravelStepOutcome::Arrived {
                total_days,
                message,
            } => {
                assert_eq!(total_days, 1);
                assert!(message.contains("到着"));
                assert_eq!(sim.phase, TravelPhase::Arrived);
            }
            other => panic!("Expected arrival after resolve, but got {:?}", other),
        }
    }

    #[test]
    fn test_travel_simulation_without_event_arrives() {
        let mut sim =
            TravelSimulation::new(AreaId::Town, AreaId::Village, Position { x: 10, y: 10 });
        sim.plan.transport = TravelTransport::Horse; // 1日で到着
        sim.plan.posture = TravelPosture::Cautious;

        sim.start_travel();

        // 空のレジストリ（イベントなし）
        let empty_registry = SuddenEventRegistry { events: Vec::new() };
        let mut history = SuddenEventHistory::default();
        let mut rng = StdRng::seed_from_u64(1);

        let outcome = sim.advance_day(&empty_registry, &mut history, &mut rng);
        match outcome {
            TravelStepOutcome::Arrived {
                total_days,
                message,
            } => {
                assert_eq!(total_days, 1);
                assert!(message.contains("到着"));
                assert_eq!(sim.phase, TravelPhase::Arrived);
            }
            other => panic!("Expected arrival, but got {:?}", other),
        }
    }
}
