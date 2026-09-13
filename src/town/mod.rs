pub mod fov;
pub mod map;
pub mod movement;

pub use movement::{try_move_player, MoveOutcome};

use bevy::prelude::*;
use fov::FovMap;
use map::TownMap;
use movement::{Facing, FollowerHistory, Position};

#[derive(Resource)]
pub struct TownState {
    pub map: TownMap,
    pub fov: FovMap,
    pub player_pos: Position,
    pub player_facing: Facing,
    pub followers: FollowerHistory,
    pub torch_active: bool,
    pub debug_see_all: bool,
}

impl TownState {
    pub fn new() -> Self {
        let map = TownMap::create_arkan_capital();
        let fov = FovMap::new(map.width, map.height);
        let player_pos = Position { x: 20, y: 5 }; // 中央広場下
        let followers = FollowerHistory::new(Position { x: 20, y: 6 }, 4);

        let mut state = Self {
            map,
            fov,
            player_pos,
            player_facing: Facing::Up,
            followers,
            torch_active: true,
            debug_see_all: false,
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
    }

    pub fn move_player(&mut self, dx: i32, dy: i32) -> MoveOutcome {
        let outcome = try_move_player(
            &mut self.map,
            &mut self.player_pos,
            &mut self.player_facing,
            &mut self.followers,
            dx,
            dy,
        );
        self.recompute_fov();
        outcome
    }
}

pub fn format_town_display(town: &TownState) -> String {
    let mut output = String::new();
    let width = town.map.width;
    let height = town.map.height;

    let follower_glyphs = ['f', 's', 'm', 'k'];

    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let is_visible = town.fov.is_visible(x, y);

            if !is_visible {
                // 視界外は暗闇の影文字
                output.push('░');
                continue;
            }

            // 視界内の描画優先度: 主人公 > 仲間 > マップタイル
            if x == town.player_pos.x && y == town.player_pos.y {
                output.push('@');
            } else {
                let mut drawn_follower = false;
                for (idx, &f_char) in follower_glyphs.iter().enumerate() {
                    if let Some(f_pos) = town.followers.get_follower_position(idx) {
                        if x == f_pos.x && y == f_pos.y {
                            output.push(f_char);
                            drawn_follower = true;
                            break;
                        }
                    }
                }

                if !drawn_follower {
                    if let Some(tile) = town.map.get(x, y) {
                        output.push(tile.glyph());
                    } else {
                        output.push(' ');
                    }
                }
            }
        }
        if y < height as i32 - 1 {
            output.push('\n');
        }
    }

    output
}
