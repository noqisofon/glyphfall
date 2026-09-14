use super::dialogue::DialoguePartner;
use super::map::{AreaId, TileType, TownMap};
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

    pub fn reset(&mut self, new_pos: Position) {
        self.history.clear();
        for _ in 0..self.max_len {
            self.history.push_back(new_pos);
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
    ChangeArea {
        new_area: AreaId,
        spawn_pos: Position,
        message: String,
    },
    StartDialogue(DialoguePartner),
    ChestOpened {
        gold: i32,
        item: String,
        message: String,
    },
}

pub fn try_move_player(
    map: &mut TownMap,
    current_area: AreaId,
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
        TileType::Floor
        | TileType::DoorOpen
        | TileType::DungeonFloor
        | TileType::IronGateOpen
        | TileType::ChestOpen => {
            let prev_pos = *player_pos;
            player_pos.x = target_x;
            player_pos.y = target_y;
            followers.push(prev_pos);
            MoveOutcome::Moved { message: None }
        }
        TileType::StairsDown => {
            match current_area {
                AreaId::Town => {
                    MoveOutcome::ChangeArea {
                        new_area: AreaId::DungeonB1F,
                        spawn_pos: Position { x: 3, y: 4 },
                        message: "地下迷宮への石段を降りた……\n湿った冷気と獣の臭いが漂う【封魔の地下迷宮 B1F】に足を踏み入れた！".into(),
                    }
                }
                AreaId::DungeonB1F => {
                    let prev_pos = *player_pos;
                    player_pos.x = target_x;
                    player_pos.y = target_y;
                    followers.push(prev_pos);
                    MoveOutcome::TriggerBattle {
                        message: "さらに深層への下り階段に足を踏み入れた！\n地下奥深くから強大な魔物の咆哮が響く……！([B]キーで戦闘突入)".into(),
                    }
                }
            }
        }
        TileType::StairsUp => {
            match current_area {
                AreaId::DungeonB1F => {
                    MoveOutcome::ChangeArea {
                        new_area: AreaId::Town,
                        spawn_pos: Position { x: 40, y: 7 },
                        message: "階段を駆け上がり、夜風が吹き抜ける王都アルカンへ無事に生還した！".into(),
                    }
                }
                AreaId::Town => {
                    MoveOutcome::Blocked {
                        message: Some("見上げるような高い塔の扉は固く閉ざされている。".into()),
                    }
                }
            }
        }
        TileType::DoorClosed => {
            map.set(target_x, target_y, TileType::DoorOpen);
            MoveOutcome::Blocked {
                message: Some("扉を開けた。".into()),
            }
        }
        TileType::IronGateClosed => {
            map.set(target_x, target_y, TileType::IronGateOpen);
            MoveOutcome::Blocked {
                message: Some("ギギギ…と錆びた音を立てて鉄格子扉を開けた。".into()),
            }
        }
        TileType::ChestClosed => {
            map.set(target_x, target_y, TileType::ChestOpen);
            MoveOutcome::ChestOpened {
                gold: 120,
                item: "特やくそう".into(),
                message: "宝箱を開けた！\n120ゴールドと「特やくそう」を手に入れた！".into(),
            }
        }
        TileType::MonsterSymbol => {
            MoveOutcome::TriggerBattle {
                message: "暗闇から魔物の影が飛びかかってきた！\n不意打ちの戦闘だ！".into(),
            }
        }
        TileType::Sign => MoveOutcome::StartDialogue(DialoguePartner::Sign),
        TileType::Inn => MoveOutcome::StartDialogue(DialoguePartner::Inn),
        TileType::Tavern => MoveOutcome::StartDialogue(DialoguePartner::Tavern),
        TileType::Shop => MoveOutcome::StartDialogue(DialoguePartner::Shop),
        TileType::NpcGuard => MoveOutcome::StartDialogue(DialoguePartner::Guard),
        TileType::NpcVillager => MoveOutcome::StartDialogue(DialoguePartner::Villager),
        TileType::NpcSuspicious => MoveOutcome::StartDialogue(DialoguePartner::Suspicious),
        TileType::Wall => MoveOutcome::Blocked {
            message: Some("頑丈な石壁だ。進むことはできない。".into()),
        },
        TileType::DungeonWall => MoveOutcome::Blocked {
            message: Some("苔むした冷たい岩壁だ。進むことはできない。".into()),
        },
        TileType::Water => MoveOutcome::Blocked {
            message: Some("澄んだ水路の水だ。".into()),
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

        let outcome = try_move_player(
            &mut map,
            AreaId::Town,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            1,
            0,
        );

        assert!(matches!(outcome, MoveOutcome::Moved { .. }));
        assert_eq!(player_pos, Position { x: 6, y: 5 });
        assert_eq!(player_facing, Facing::Right);
        assert_eq!(followers.get_follower_position(0), Some(Position { x: 5, y: 5 }));

        let outcome2 = try_move_player(
            &mut map,
            AreaId::Town,
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
            AreaId::Town,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(outcome, MoveOutcome::Blocked { .. }));
        assert_eq!(player_pos, Position { x: 5, y: 5 });
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
            AreaId::Town,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(outcome, MoveOutcome::Blocked { .. }));
        assert_eq!(map.get(5, 4), Some(TileType::DoorOpen));
    }

    #[test]
    fn test_stairs_down_changes_area_to_dungeon() {
        let mut map = TownMap::new(10, 10, TileType::Floor);
        map.set(5, 4, TileType::StairsDown);

        let mut player_pos = Position { x: 5, y: 5 };
        let mut player_facing = Facing::Down;
        let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

        let outcome = try_move_player(
            &mut map,
            AreaId::Town,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(
            outcome,
            MoveOutcome::ChangeArea {
                new_area: AreaId::DungeonB1F,
                ..
            }
        ));
    }

    #[test]
    fn test_chest_opens_on_bump() {
        let mut map = TownMap::new(10, 10, TileType::DungeonFloor);
        map.set(5, 4, TileType::ChestClosed);

        let mut player_pos = Position { x: 5, y: 5 };
        let mut player_facing = Facing::Up;
        let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

        let outcome = try_move_player(
            &mut map,
            AreaId::DungeonB1F,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(outcome, MoveOutcome::ChestOpened { .. }));
        assert_eq!(map.get(5, 4), Some(TileType::ChestOpen));
    }
}
