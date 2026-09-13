use super::map::{TileType, TownMap};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct FollowerHistory {
    /// 過去の足跡（先頭が最も直近の歩行位置）
    pub history: VecDeque<Position>,
    pub max_len: usize,
}

impl FollowerHistory {
    pub fn new(initial_pos: Position, count: usize) -> Self {
        let mut history = VecDeque::new();
        for _ in 0..count {
            history.push_back(initial_pos);
        }
        Self {
            history,
            max_len: count,
        }
    }

    pub fn push(&mut self, pos: Position) {
        self.history.push_front(pos);
        while self.history.len() > self.max_len {
            self.history.pop_back();
        }
    }

    pub fn get_follower_position(&self, index: usize) -> Option<Position> {
        self.history.get(index).copied()
    }
}

pub enum MoveOutcome {
    Moved { message: Option<String> },
    Blocked { message: Option<String> },
    TriggerBattle { message: String },
}

pub fn try_move_player(
    map: &mut TownMap,
    player_pos: &mut Position,
    player_facing: &mut Facing,
    followers: &mut FollowerHistory,
    dx: i32,
    dy: i32,
) -> MoveOutcome {
    // 向きの更新
    if dx > 0 {
        *player_facing = Facing::Right;
    } else if dx < 0 {
        *player_facing = Facing::Left;
    } else if dy > 0 {
        *player_facing = Facing::Down;
    } else if dy < 0 {
        *player_facing = Facing::Up;
    }

    let target_x = player_pos.x + dx;
    let target_y = player_pos.y + dy;

    let target_tile = match map.get(target_x, target_y) {
        Some(t) => t,
        None => return MoveOutcome::Blocked { message: None },
    };

    match target_tile {
        TileType::Floor | TileType::DoorOpen => {
            let prev_pos = *player_pos;
            player_pos.x = target_x;
            player_pos.y = target_y;
            followers.push(prev_pos);
            MoveOutcome::Moved { message: None }
        }
        TileType::StairsDown => {
            let prev_pos = *player_pos;
            player_pos.x = target_x;
            player_pos.y = target_y;
            followers.push(prev_pos);
            MoveOutcome::TriggerBattle {
                message: "地下迷宮への階段に足を踏み入れた！\n冷気が立ち込め、奥から魔物の気配が漂ってくる……！([B]キーで戦闘突入)".into(),
            }
        }
        TileType::DoorClosed => {
            // 扉を開ける
            map.set(target_x, target_y, TileType::DoorOpen);
            MoveOutcome::Blocked {
                message: Some("扉を開けた。".into()),
            }
        }
        TileType::Sign => MoveOutcome::Blocked {
            message: Some("立て看板【王都アルカン・商業区】\n北西: 宿屋[H]・酒場[T]  北東: 道具屋[S]  南東: 封鎖地区".into()),
        },
        TileType::Inn => MoveOutcome::Blocked {
            message: Some("宿屋の主人「旅の方かい？\nあいにく今は旅の一座で満室なんだ。すまないね」".into()),
        },
        TileType::Tavern => MoveOutcome::Blocked {
            message: Some("呑兵衛の冒険者「南東の地下道は近づかねえ方がいいぜ。\n最近夜になると、恐ろしい唸り声が響いてくるんだ…」".into()),
        },
        TileType::Shop => MoveOutcome::Blocked {
            message: Some("道具屋の店主「へいらっしゃい！\n夜歩くなら[L]キーで松明を灯しなよ。暗闇じゃ足元も見えねえぜ」".into()),
        },
        TileType::NpcGuard => MoveOutcome::Blocked {
            message: Some("王都衛兵「止まれ！この先は立ち入り禁止の地下迷宮だ！\n生半可な覚悟で降りれば命はないぞ！」".into()),
        },
        TileType::NpcVillager => MoveOutcome::Blocked {
            message: Some("街の女性「ようこそアルカン王都へ！\n夜は辻斬りが出る噂もあるから、気をつけて歩いてね」".into()),
        },
        TileType::NpcSuspicious => MoveOutcome::Blocked {
            message: Some("裏通りの怪しい男「ヒヒッ…あんたたち、魔物の討伐かい？\n裏の壁に抜け道があるぜ…気をつけな」".into()),
        },
        TileType::Wall => MoveOutcome::Blocked {
            message: Some("頑丈な石壁だ。進むことはできない。".into()),
        },
        TileType::Water => MoveOutcome::Blocked {
            message: Some("澄んだ噴水の水だ。水しぶきが心地よい。".into()),
        },
        TileType::Tree => MoveOutcome::Blocked {
            message: Some("青々と茂る街路樹だ。".into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_movement_and_follower_trail() {
        let mut map = TownMap::new(10, 10, TileType::Floor);
        let mut player_pos = Position { x: 5, y: 5 };
        let mut player_facing = Facing::Up;
        let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 3);

        // 1歩右へ移動
        let outcome = try_move_player(
            &mut map,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            1,
            0,
        );

        assert!(matches!(outcome, MoveOutcome::Moved { .. }));
        assert_eq!(player_pos, Position { x: 6, y: 5 });
        assert_eq!(player_facing, Facing::Right);
        // 直前の位置 (5, 5) がフォロワー1人目の位置になっている
        assert_eq!(followers.get_follower_position(0), Some(Position { x: 5, y: 5 }));

        // さらに1歩上へ移動
        let outcome2 = try_move_player(
            &mut map,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );
        assert!(matches!(outcome2, MoveOutcome::Moved { .. }));
        assert_eq!(player_pos, Position { x: 6, y: 4 });
        assert_eq!(player_facing, Facing::Up);
        assert_eq!(followers.get_follower_position(0), Some(Position { x: 6, y: 5 }));
        assert_eq!(followers.get_follower_position(1), Some(Position { x: 5, y: 5 }));
    }

    #[test]
    fn test_wall_blocks_movement() {
        let mut map = TownMap::new(10, 10, TileType::Floor);
        map.set(5, 4, TileType::Wall);

        let mut player_pos = Position { x: 5, y: 5 };
        let mut player_facing = Facing::Down;
        let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

        let outcome = try_move_player(
            &mut map,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(outcome, MoveOutcome::Blocked { .. }));
        assert_eq!(player_pos, Position { x: 5, y: 5 }); // 移動していない
    }

    #[test]
    fn test_door_opens_on_bump() {
        let mut map = TownMap::new(10, 10, TileType::Floor);
        map.set(5, 4, TileType::DoorClosed);

        let mut player_pos = Position { x: 5, y: 5 };
        let mut player_facing = Facing::Down;
        let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

        let outcome = try_move_player(
            &mut map,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(outcome, MoveOutcome::Blocked { .. }));
        assert_eq!(map.get(5, 4), Some(TileType::DoorOpen)); // 扉が開いた！
    }

    #[test]
    fn test_stairs_down_triggers_battle() {
        let mut map = TownMap::new(10, 10, TileType::Floor);
        map.set(5, 4, TileType::StairsDown);

        let mut player_pos = Position { x: 5, y: 5 };
        let mut player_facing = Facing::Down;
        let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

        let outcome = try_move_player(
            &mut map,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(outcome, MoveOutcome::TriggerBattle { .. }));
        assert_eq!(player_pos, Position { x: 5, y: 4 }); // 階段マスに入っている
    }
}
