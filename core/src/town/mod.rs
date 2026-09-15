pub mod dialogue;
pub mod dungeon_gen;
pub mod fov;
pub mod interact;
pub mod map;
pub mod movement;

pub use dialogue::{DialogueLearnStage, DialoguePartner, DialogueSession, LearnableSpan, SHOP_ITEMS};
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

    pub fn switch_area<R: rand::Rng>(&mut self, new_area: AreaId, spawn_pos: Position, rng: &mut R) {
        self.current_area = new_area;
        self.map = match new_area {
            AreaId::Town => TownMap::create_arkan_capital(),
            AreaId::DungeonB1F => TownMap::create_dungeon_b1f(rng),
            AreaId::Village => TownMap::create_suzukake_village(),
        };
        self.player_pos = spawn_pos;
        self.followers.reset(spawn_pos);
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

        self.recompute_fov();
        outcome
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
        let outcome = interact::resolve(&mut self.map, self.player_pos, self.player_facing, command);
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
