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

#[derive(Debug)]
pub enum MoveOutcome {
    Moved { message: Option<String> },
    Blocked { message: Option<String> },
    TriggerBattle {
        message: String,
        monster_pos: Option<Position>,
    },
    ChangeArea {
        new_area: AreaId,
        spawn_pos: Position,
        message: String,
    },
    /// 街道口（`RoadExit`）に接触した。即座に移動はせず、呼び出し側が移動姿勢選択
    /// （ADR-0004）→ 突発イベント発生ロール（ADR-0013）を経てから`destination`へ
    /// 実際に移動するかどうかを決める。
    RequestTravel {
        destination: AreaId,
        spawn_pos: Position,
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
                // 地下迷宮内（ADR-0021）: MAX_DUNGEON_DEPTH（B6F）より深くはまだ
                // 実装がないため、最深階の下り階段は代わりに強制戦闘を発生させる。
                AreaId::DungeonB1F
                | AreaId::DungeonB2F
                | AreaId::DungeonB3F
                | AreaId::DungeonB4F
                | AreaId::DungeonB5F
                | AreaId::DungeonB6F => {
                    let depth = current_area
                        .dungeon_depth()
                        .expect("current_area is one of the dungeon floor variants");
                    match AreaId::from_dungeon_depth(depth + 1) {
                        Some(next_area) => MoveOutcome::ChangeArea {
                            new_area: next_area,
                            spawn_pos: Position { x: 3, y: 4 },
                            message: format!(
                                "さらに深層への下り階段を降りた……\n【{}】に足を踏み入れた！",
                                next_area.name()
                            ),
                        },
                        None => {
                            let prev_pos = *player_pos;
                            player_pos.x = target_x;
                            player_pos.y = target_y;
                            followers.push(prev_pos);
                            MoveOutcome::TriggerBattle {
                                message: "さらに深層への下り階段に足を踏み入れた！\n地下奥深くから強大な魔物の咆哮が響く……！".into(),
                                monster_pos: None,
                            }
                        }
                    }
                }
                // 村には地下迷宮への階段は存在しない（到達不能なタイルのため実質デッドコード）。
                AreaId::Village => MoveOutcome::Blocked { message: None },
            }
        }
        TileType::StairsUp => {
            match current_area {
                AreaId::DungeonB1F => {
                    MoveOutcome::ChangeArea {
                        new_area: AreaId::Town,
                        spawn_pos: Position { x: 40, y: 11 },
                        message: "階段を駆け上がり、夜風が吹き抜ける王都アルカンへ無事に生還した！".into(),
                    }
                }
                AreaId::DungeonB2F
                | AreaId::DungeonB3F
                | AreaId::DungeonB4F
                | AreaId::DungeonB5F
                | AreaId::DungeonB6F => {
                    let depth = current_area
                        .dungeon_depth()
                        .expect("current_area is one of the dungeon floor variants");
                    let prev_area = AreaId::from_dungeon_depth(depth - 1)
                        .expect("depth - 1 stays within 1..=MAX_DUNGEON_DEPTH for B2F..=B6F");
                    MoveOutcome::ChangeArea {
                        new_area: prev_area,
                        spawn_pos: Position { x: 3, y: 4 },
                        message: format!("階段を上り、一つ浅い【{}】へ戻った。", prev_area.name()),
                    }
                }
                AreaId::Town | AreaId::Village => MoveOutcome::Blocked {
                    message: Some("見上げるような高い塔の扉は固く閉ざされている。".into()),
                },
            }
        }
        TileType::RoadExit => match current_area {
            AreaId::Town => MoveOutcome::RequestTravel {
                destination: AreaId::Village,
                spawn_pos: Position { x: 44, y: 6 },
            },
            AreaId::Village => MoveOutcome::RequestTravel {
                destination: AreaId::Town,
                spawn_pos: Position { x: 1, y: 6 },
            },
            AreaId::DungeonB1F
            | AreaId::DungeonB2F
            | AreaId::DungeonB3F
            | AreaId::DungeonB4F
            | AreaId::DungeonB5F
            | AreaId::DungeonB6F => MoveOutcome::Blocked { message: None },
        },
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
        // 宝箱・案内看板・NPC・お店は、接触しただけでは何も起きない（ADR-0011）。
        // 移動はブロックされるのみで、開封・会話は [Z]コマンド駆動インタラクトからのみ行える。
        TileType::ChestClosed => MoveOutcome::Blocked { message: None },
        TileType::MonsterSymbol => {
            MoveOutcome::TriggerBattle {
                message: "暗闇から魔物の影が飛びかかってきた！\n不意打ちの戦闘だ！".into(),
                monster_pos: Some(Position {
                    x: target_x,
                    y: target_y,
                }),
            }
        }
        TileType::Sign => MoveOutcome::Blocked { message: None },
        TileType::Inn => MoveOutcome::Blocked { message: None },
        TileType::Tavern => MoveOutcome::Blocked { message: None },
        TileType::Shop => MoveOutcome::Blocked { message: None },
        TileType::NpcGuard => MoveOutcome::Blocked { message: None },
        TileType::NpcVillager => MoveOutcome::Blocked { message: None },
        TileType::NpcSuspicious => MoveOutcome::Blocked { message: None },
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
    fn test_stairs_down_descends_one_floor_at_a_time() {
        // ADR-0021: B1F〜B5Fの下り階段は、それぞれ一つ深い階層へ`ChangeArea`する。
        let floors = [
            (AreaId::DungeonB1F, AreaId::DungeonB2F),
            (AreaId::DungeonB2F, AreaId::DungeonB3F),
            (AreaId::DungeonB3F, AreaId::DungeonB4F),
            (AreaId::DungeonB4F, AreaId::DungeonB5F),
            (AreaId::DungeonB5F, AreaId::DungeonB6F),
        ];

        for (current, expected_next) in floors {
            let mut map = TownMap::new(10, 10, TileType::DungeonFloor);
            map.set(5, 4, TileType::StairsDown);
            let mut player_pos = Position { x: 5, y: 5 };
            let mut player_facing = Facing::Down;
            let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

            let outcome = try_move_player(
                &mut map,
                current,
                &mut player_pos,
                &mut player_facing,
                &mut followers,
                0,
                -1,
            );

            assert!(
                matches!(
                    outcome,
                    MoveOutcome::ChangeArea { new_area, .. } if new_area == expected_next
                ),
                "descending from {current:?} should lead to {expected_next:?}, got {outcome:?}"
            );
        }
    }

    #[test]
    fn test_stairs_down_from_deepest_floor_triggers_battle_instead_of_new_area() {
        // ADR-0021: MAX_DUNGEON_DEPTH（B6F）より深い階層はまだ実装されていないため、
        // 最深階の下り階段はこれ以上潜らせず強制戦闘にする。
        let mut map = TownMap::new(10, 10, TileType::DungeonFloor);
        map.set(5, 4, TileType::StairsDown);
        let mut player_pos = Position { x: 5, y: 5 };
        let mut player_facing = Facing::Down;
        let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

        let outcome = try_move_player(
            &mut map,
            AreaId::DungeonB6F,
            &mut player_pos,
            &mut player_facing,
            &mut followers,
            0,
            -1,
        );

        assert!(matches!(outcome, MoveOutcome::TriggerBattle { .. }));
        assert_eq!(player_pos, Position { x: 5, y: 4 });
    }

    #[test]
    fn test_stairs_up_ascends_one_floor_at_a_time() {
        // ADR-0021: B2F〜B6Fの上り階段は、それぞれ一つ浅い階層へ`ChangeArea`する。
        let floors = [
            (AreaId::DungeonB2F, AreaId::DungeonB1F),
            (AreaId::DungeonB3F, AreaId::DungeonB2F),
            (AreaId::DungeonB4F, AreaId::DungeonB3F),
            (AreaId::DungeonB5F, AreaId::DungeonB4F),
            (AreaId::DungeonB6F, AreaId::DungeonB5F),
        ];

        for (current, expected_prev) in floors {
            let mut map = TownMap::new(10, 10, TileType::DungeonFloor);
            map.set(5, 4, TileType::StairsUp);
            let mut player_pos = Position { x: 5, y: 5 };
            let mut player_facing = Facing::Down;
            let mut followers = FollowerHistory::new(Position { x: 5, y: 5 }, 2);

            let outcome = try_move_player(
                &mut map,
                current,
                &mut player_pos,
                &mut player_facing,
                &mut followers,
                0,
                -1,
            );

            assert!(
                matches!(
                    outcome,
                    MoveOutcome::ChangeArea { new_area, .. } if new_area == expected_prev
                ),
                "ascending from {current:?} should lead to {expected_prev:?}, got {outcome:?}"
            );
        }
    }

    #[test]
    fn test_chest_does_not_open_on_bump() {
        // ADR-0011: 接触しただけでは何も起きない。宝箱の開封は [Z]コマンド経由のみ。
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

        assert!(matches!(outcome, MoveOutcome::Blocked { .. }));
        assert_eq!(player_pos, Position { x: 5, y: 5 });
        assert_eq!(map.get(5, 4), Some(TileType::ChestClosed));
    }

    #[test]
    fn test_monster_symbol_triggers_battle_with_position() {
        // ADR-0024: 固定魔物シンボルに接触すると戦闘に突入し、シンボルの座標が返される。
        let mut map = TownMap::new(10, 10, TileType::DungeonFloor);
        map.set(5, 4, TileType::MonsterSymbol);

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

        match outcome {
            MoveOutcome::TriggerBattle { monster_pos, .. } => {
                assert_eq!(monster_pos, Some(Position { x: 5, y: 4 }));
            }
            other => panic!("expected TriggerBattle, got {other:?}"),
        }
        // プレイヤーはその場に留まり、不意打ち戦闘を受ける
        assert_eq!(player_pos, Position { x: 5, y: 5 });
        assert_eq!(player_facing, Facing::Up);
    }
}
