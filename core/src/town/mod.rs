pub mod dialogue;
pub mod dungeon_gen;
pub mod fov;
pub mod interact;
pub mod map;
pub mod movement;

pub use dialogue::{
    DialogueLearnStage, DialoguePartner, DialogueSession, LearnableSpan, INN_COST, SHOP_ITEMS,
};
pub use fov::FovMap;
pub use interact::{CommandKind, InteractOutcome, TargetKind};
pub use map::{AreaId, TileType, TownMap};
pub use movement::{try_move_player, Facing, FollowerHistory, MoveOutcome, Position};

/// 街・迷宮の探索状態（ADR-0006/0010ほか）。
///
/// テクスチャ描画（`pixel_art`によるRGBAバッファ生成とBevyの`Image`管理）は
/// UI側の関心事のため、ここでは持たない。描画が必要なホスト側（`app`クレート）
/// は`map` / `fov` / `player_pos` / `player_facing` / `followers` / `dirty`を
/// 読み取ってテクスチャを再生成する。
pub struct TownState {
    pub current_area: AreaId,
    pub map: TownMap,
    pub fov: FovMap,
    pub player_pos: Position,
    pub player_facing: Facing,
    pub followers: FollowerHistory,
    pub torch_active: bool,
    pub debug_see_all: bool,
    /// 最後にエンカウントした魔物シンボルの座標（戦闘勝利時に床へ置換するため。ADR-0024）
    pub last_encounter_pos: Option<Position>,
    /// 描画側（テクスチャ）の再生成が必要かどうかを示すフラグ。
    pub dirty: bool,
}

impl TownState {
    pub fn new() -> Self {
        let current_area = AreaId::Town;
        let map = TownMap::create_arkan_capital();
        let fov = FovMap::new(map.width, map.height);
        let player_pos = Position { x: 20, y: 5 }; // 中央広場下
        let followers = FollowerHistory::new(Position { x: 20, y: 6 }, 3);

        let mut state = Self {
            current_area,
            map,
            fov,
            player_pos,
            player_facing: Facing::Up,
            followers,
            torch_active: true,
            debug_see_all: false,
            last_encounter_pos: None,
            dirty: true,
        };
        state.recompute_fov();
        state
    }

    pub fn recompute_fov(&mut self) {
        let radius = if self.torch_active { 6 } else { 2 };
        self.fov.compute(
            &self.map,
            self.player_pos.x,
            self.player_pos.y,
            radius,
            self.debug_see_all,
        );
        self.dirty = true;
    }

    pub fn switch_area<R: rand::Rng>(
        &mut self,
        new_area: AreaId,
        spawn_pos: Position,
        rng: &mut R,
    ) {
        self.current_area = new_area;
        self.map = match new_area {
            AreaId::Town => TownMap::create_arkan_capital(),
            AreaId::DungeonB1F
            | AreaId::DungeonB2F
            | AreaId::DungeonB3F
            | AreaId::DungeonB4F
            | AreaId::DungeonB5F
            | AreaId::DungeonB6F => TownMap::create_dungeon_floor(rng),
            AreaId::Village => TownMap::create_suzukake_village(),
        };
        self.player_pos = spawn_pos;
        self.followers.reset(spawn_pos);
        self.last_encounter_pos = None;
        self.recompute_fov();
        self.dirty = true;
    }

    pub fn move_player<R: rand::Rng>(&mut self, dx: i32, dy: i32, rng: &mut R) -> MoveOutcome {
        let outcome = try_move_player(
            &mut self.map,
            self.current_area,
            &mut self.player_pos,
            &mut self.player_facing,
            &mut self.followers,
            dx,
            dy,
        );

        if let MoveOutcome::ChangeArea {
            new_area,
            spawn_pos,
            ref message,
        } = outcome
        {
            self.switch_area(new_area, spawn_pos, rng);
            return MoveOutcome::ChangeArea {
                new_area,
                spawn_pos,
                message: message.clone(),
            };
        }

        if let MoveOutcome::TriggerBattle { monster_pos, .. } = &outcome {
            self.last_encounter_pos = *monster_pos;
        }

        self.recompute_fov();
        outcome
    }

    /// 戦闘勝利時に、直前に接触した魔物シンボルをダンジョン床に置き換えて消去する（ADR-0024）。
    /// シンボルが存在して消去された場合は true を返す。
    pub fn clear_defeated_monster(&mut self) -> bool {
        if let Some(pos) = self.last_encounter_pos.take() {
            if self.map.get(pos.x, pos.y) == Some(TileType::MonsterSymbol) {
                self.map.set(pos.x, pos.y, TileType::DungeonFloor);
                self.recompute_fov();
                self.dirty = true;
                return true;
            }
        }
        false
    }

    /// プレイヤーが現在向いている先のタイル（コマンド駆動インタラクトの対象候補）
    pub fn tile_ahead(&self) -> Option<TileType> {
        let (dx, dy) = interact::facing_delta(self.player_facing);
        self.map.get(self.player_pos.x + dx, self.player_pos.y + dy)
    }

    /// コマンドウィンドウのラベル動的切り替え（しらべる→みる）に使う対象種別
    pub fn facing_target_kind(&self) -> TargetKind {
        interact::classify_tile(self.tile_ahead())
    }

    /// 移動を伴わずに向きだけを変える（方向選択ステップ用）
    pub fn face(&mut self, dx: i32, dy: i32) {
        if dx > 0 {
            self.player_facing = Facing::Right;
        } else if dx < 0 {
            self.player_facing = Facing::Left;
        } else if dy > 0 {
            self.player_facing = Facing::Down;
        } else if dy < 0 {
            self.player_facing = Facing::Up;
        }
        self.dirty = true;
    }

    /// Zメニューで確定したコマンドを、向いている方向に対して判定する
    pub fn resolve_interact(&mut self, command: CommandKind) -> InteractOutcome {
        let outcome = interact::resolve(
            &mut self.map,
            self.player_pos,
            self.player_facing,
            command,
            self.current_area,
        );
        if matches!(outcome, InteractOutcome::ChestOpened { .. }) {
            self.dirty = true;
        }
        outcome
    }
}

impl Default for TownState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_town_state_monster_encounter_and_clear() {
        let mut state = TownState::new();
        state.switch_area(
            AreaId::DungeonB1F,
            Position { x: 5, y: 5 },
            &mut StdRng::seed_from_u64(1),
        );
        // (5, 4) に魔物シンボルを配置
        state.map.set(5, 4, TileType::MonsterSymbol);

        let mut rng = StdRng::seed_from_u64(42);
        // 上へ移動して魔物に接触
        let outcome = state.move_player(0, -1, &mut rng);
        assert!(matches!(outcome, MoveOutcome::TriggerBattle { .. }));
        assert_eq!(state.last_encounter_pos, Some(Position { x: 5, y: 4 }));
        // プレイヤー自身は (5, 5) のまま
        assert_eq!(state.player_pos, Position { x: 5, y: 5 });

        // 戦闘勝利によりシンボル消去
        let cleared = state.clear_defeated_monster();
        assert!(cleared);
        assert_eq!(state.last_encounter_pos, None);
        // (5, 4) が床になっていることを確認
        assert_eq!(state.map.get(5, 4), Some(TileType::DungeonFloor));

        // 2回目呼んでも false
        assert!(!state.clear_defeated_monster());
    }
}
