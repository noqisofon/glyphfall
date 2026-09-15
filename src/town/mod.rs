pub mod dialogue;
pub mod fov;
pub mod interact;
pub mod map;
pub mod movement;
pub mod pixel_art;

pub use dialogue::{DialogueLearnStage, DialoguePartner, DialogueSession, LearnableSpan, SHOP_ITEMS};
pub use interact::{CommandKind, InteractOutcome, TargetKind};
pub use map::AreaId;
pub use movement::{try_move_player, MoveOutcome, Position};

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use fov::FovMap;
use map::{TileType, TownMap};
use movement::{Facing, FollowerHistory};
use pixel_art::{render_town_to_texture, TEXTURE_HEIGHT, TEXTURE_WIDTH};

#[derive(Resource)]
pub struct TownState {
    pub current_area: AreaId,
    pub map: TownMap,
    pub fov: FovMap,
    pub player_pos: Position,
    pub player_facing: Facing,
    pub followers: FollowerHistory,
    pub torch_active: bool,
    pub debug_see_all: bool,
    pub texture_handle: Handle<Image>,
    pub dirty: bool,
}

impl TownState {
    pub fn new(images: &mut Assets<Image>) -> Self {
        let current_area = AreaId::Town;
        let map = TownMap::create_arkan_capital();
        let fov = FovMap::new(map.width, map.height);
        let player_pos = Position { x: 20, y: 5 }; // 中央広場下
        let followers = FollowerHistory::new(Position { x: 20, y: 6 }, 4);

        let mut buffer = vec![0u8; TEXTURE_WIDTH * TEXTURE_HEIGHT * 4];

        let mut state = Self {
            current_area,
            map,
            fov,
            player_pos,
            player_facing: Facing::Up,
            followers,
            torch_active: true,
            debug_see_all: false,
            texture_handle: Handle::default(),
            dirty: true,
        };
        state.recompute_fov();

        render_town_to_texture(
            &state.map,
            &state.fov,
            state.player_pos,
            &state.followers,
            &mut buffer,
        );

        let mut image = Image::new(
            Extent3d {
                width: TEXTURE_WIDTH as u32,
                height: TEXTURE_HEIGHT as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            buffer,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = bevy::image::ImageSampler::nearest();

        state.texture_handle = images.add(image);
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

    pub fn switch_area(&mut self, new_area: AreaId, spawn_pos: Position) {
        self.current_area = new_area;
        self.map = match new_area {
            AreaId::Town => TownMap::create_arkan_capital(),
            AreaId::DungeonB1F => TownMap::create_dungeon_b1f(),
            AreaId::Village => TownMap::create_suzukake_village(),
        };
        self.player_pos = spawn_pos;
        self.followers.reset(spawn_pos);
        self.recompute_fov();
        self.dirty = true;
    }

    pub fn move_player(&mut self, dx: i32, dy: i32) -> MoveOutcome {
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
            self.switch_area(new_area, spawn_pos);
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

    pub fn update_texture(&mut self, images: &mut Assets<Image>) {
        if !self.dirty {
            return;
        }
        if let Some(image) = images.get_mut(&self.texture_handle) {
            render_town_to_texture(
                &self.map,
                &self.fov,
                self.player_pos,
                &self.followers,
                &mut image.data,
            );
        }
        self.dirty = false;
    }
}
