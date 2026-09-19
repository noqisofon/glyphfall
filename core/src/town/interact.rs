//! コマンド駆動インタラクト（ADR-0011）
//!
//! Zキーでコマンドウィンドウを開き、コマンド→方向の順に選択して初めて判定が行われる。
//! 隣接・接触しただけでは何も起きない（移動側の自動判定は movement.rs から排除済み）。

use super::dialogue::DialoguePartner;
use super::map::{AreaId, TileType, TownMap};
use super::movement::{Facing, Position};
use crate::party::CURRENCY_NAME;

/// コマンドウィンドウに並ぶコマンド。将来「ぬすむ」以外の追加も見据え、
/// 対象になれるものの型（TargetKind）との組み合わせで判定を分岐させる。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    Examine,
    Talk,
    Item,
    Steal,
}

impl CommandKind {
    pub const ALL: [CommandKind; 4] = [
        CommandKind::Examine,
        CommandKind::Talk,
        CommandKind::Item,
        CommandKind::Steal,
    ];

    pub fn base_label(&self) -> &'static str {
        match self {
            CommandKind::Examine => "しらべる",
            CommandKind::Talk => "はなす",
            CommandKind::Item => "どうぐ",
            CommandKind::Steal => "ぬすむ",
        }
    }

    /// 「しらべる」は対象が人物の場合「みる」に呼称が変わる。
    pub fn dynamic_label(&self, target: TargetKind) -> &'static str {
        match (self, target) {
            (CommandKind::Examine, TargetKind::Npc) => "みる",
            _ => self.base_label(),
        }
    }

    /// 判定に対象方向の選択を伴うかどうか
    pub fn needs_direction(&self) -> bool {
        !matches!(self, CommandKind::Item)
    }
}

/// 方向の先にいる対象の種別。コマンドごとに対象になれる型が異なるため
/// （しらべる→地形/NPC/宝箱、はなす→NPCのみ、ぬすむ→NPC/モンスターのみ）、
/// 判定本体（resolve）側で種別ごとに空振りとの分岐を行う。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    Npc,
    Object,
    Nothing,
}

pub fn classify_tile(tile: Option<TileType>) -> TargetKind {
    match tile {
        None => TargetKind::Nothing,
        Some(
            TileType::Inn
            | TileType::Tavern
            | TileType::Shop
            | TileType::NpcGuard
            | TileType::NpcVillager
            | TileType::NpcSuspicious,
        ) => TargetKind::Npc,
        Some(_) => TargetKind::Object,
    }
}

pub fn facing_delta(facing: Facing) -> (i32, i32) {
    match facing {
        Facing::Up => (0, -1),
        Facing::Down => (0, 1),
        Facing::Left => (-1, 0),
        Facing::Right => (1, 0),
    }
}

pub enum InteractOutcome {
    Message(String),
    StartDialogue(DialoguePartner),
    ChestOpened {
        gold: i32,
        item: String,
        message: String,
    },
}

const SIGN_TEXT_TOWN: &str = "【王都アルカン・案内看板】\n北西: 宿屋[H]・酒場[T]  北東: 道具屋[S]\n南西: 貧民街裏路地     南東: 地下迷宮封鎖地区";
const SIGN_TEXT_VILLAGE: &str =
    "【すずかけ村・案内看板】\n西: のどかな農家・畑    東: 街道への出入口\n中央: 村の古井戸と広場";
const SIGN_TEXT_DUNGEON: &str = "風化した古い警告板だ。\n『深層へ進む者よ、引き返す勇気を持て』";

/// Zメニューでコマンドと方向が確定した後の判定本体。
pub fn resolve(
    map: &mut TownMap,
    player_pos: Position,
    facing: Facing,
    command: CommandKind,
    area: AreaId,
) -> InteractOutcome {
    let (dx, dy) = facing_delta(facing);
    let target_x = player_pos.x + dx;
    let target_y = player_pos.y + dy;
    let tile = map.get(target_x, target_y);

    match command {
        CommandKind::Examine => resolve_examine(map, target_x, target_y, tile, area),
        CommandKind::Talk => resolve_talk(tile),
        CommandKind::Steal => resolve_steal(tile),
        CommandKind::Item => {
            InteractOutcome::Message("どうぐの確認は「どうぐ」コマンドから行える。".into())
        }
    }
}

fn resolve_examine(
    map: &mut TownMap,
    x: i32,
    y: i32,
    tile: Option<TileType>,
    area: AreaId,
) -> InteractOutcome {
    let Some(tile) = tile else {
        return InteractOutcome::Message("そこには何もない。".into());
    };

    match tile {
        TileType::Inn => {
            InteractOutcome::Message("宿屋の主人がのんびりと帳簿をつけている。".into())
        }
        TileType::Tavern => {
            InteractOutcome::Message("酒場の呑兵衛が上機嫌に酒をあおっている。".into())
        }
        TileType::Shop => InteractOutcome::Message("道具屋の店主が商品を並べ直している。".into()),
        TileType::NpcGuard => {
            let msg = match area {
                AreaId::Village => "村の自警団員が周囲を警戒している。",
                _ => "王都衛兵が鋭い眼光でこちらを見張っている。",
            };
            InteractOutcome::Message(msg.into())
        }
        TileType::NpcVillager => {
            let msg = match area {
                AreaId::Village => "村の娘がのんびりと畑や井戸を見守っている。",
                _ => "街の女性が穏やかに微笑んでいる。",
            };
            InteractOutcome::Message(msg.into())
        }
        TileType::NpcSuspicious => {
            InteractOutcome::Message("裏通りの怪しい男が油断なく辺りをうかがっている。".into())
        }
        TileType::Sign => {
            let msg = match area {
                AreaId::Town => SIGN_TEXT_TOWN,
                AreaId::Village => SIGN_TEXT_VILLAGE,
                _ => SIGN_TEXT_DUNGEON,
            };
            InteractOutcome::Message(msg.into())
        }
        TileType::ChestClosed => {
            map.set(x, y, TileType::ChestOpen);
            InteractOutcome::ChestOpened {
                gold: 120,
                item: "特やくそう".into(),
                message: format!(
                    "宝箱を調べた！\n120{}と「特やくそう」を手に入れた！",
                    CURRENCY_NAME
                ),
            }
        }
        TileType::ChestOpen => InteractOutcome::Message("空になった宝箱だ。".into()),
        TileType::Wall => InteractOutcome::Message("頑丈な石壁だ。".into()),
        TileType::DungeonWall => InteractOutcome::Message("苔むした冷たい岩壁だ。".into()),
        TileType::Water => InteractOutcome::Message("澄んだ水路の水だ。".into()),
        TileType::Tree => InteractOutcome::Message("青々と茂る街路樹だ。".into()),
        TileType::DoorClosed | TileType::DoorOpen => {
            InteractOutcome::Message("木製の扉だ。".into())
        }
        TileType::IronGateClosed | TileType::IronGateOpen => {
            InteractOutcome::Message("錆びついた鉄格子の扉だ。".into())
        }
        TileType::StairsDown => InteractOutcome::Message("地下へと続く石段だ。".into()),
        TileType::StairsUp => InteractOutcome::Message("地上へと続く石段だ。".into()),
        TileType::MonsterSymbol => {
            InteractOutcome::Message("魔物の気配がする……油断できない。".into())
        }
        TileType::Floor | TileType::DungeonFloor => InteractOutcome::Message("なにもない。".into()),
        TileType::RoadExit => {
            InteractOutcome::Message("他の町・村へ続く街道だ。歩いて踏み出せば旅立てる。".into())
        }
    }
}

fn resolve_talk(tile: Option<TileType>) -> InteractOutcome {
    match tile {
        Some(TileType::Inn) => InteractOutcome::StartDialogue(DialoguePartner::Inn),
        Some(TileType::Tavern) => InteractOutcome::StartDialogue(DialoguePartner::Tavern),
        Some(TileType::Shop) => InteractOutcome::StartDialogue(DialoguePartner::Shop),
        Some(TileType::NpcGuard) => InteractOutcome::StartDialogue(DialoguePartner::Guard),
        Some(TileType::NpcVillager) => InteractOutcome::StartDialogue(DialoguePartner::Villager),
        Some(TileType::NpcSuspicious) => {
            InteractOutcome::StartDialogue(DialoguePartner::Suspicious)
        }
        _ => InteractOutcome::Message("そこには話せる相手がいない。".into()),
    }
}

fn resolve_steal(tile: Option<TileType>) -> InteractOutcome {
    match tile {
        Some(
            TileType::NpcGuard
            | TileType::NpcVillager
            | TileType::NpcSuspicious
            | TileType::Inn
            | TileType::Tavern
            | TileType::Shop,
        ) => InteractOutcome::Message("「ぬすむ」コマンドはまだ準備中だ……".into()),
        _ => InteractOutcome::Message("そこには盗めるものがない。".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examine_npc_gives_look_text_without_dialogue() {
        let mut map = TownMap::new(5, 5, TileType::Floor);
        map.set(2, 1, TileType::NpcGuard);
        let outcome = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Examine,
            AreaId::Town,
        );
        assert!(matches!(outcome, InteractOutcome::Message(_)));
    }

    #[test]
    fn test_talk_to_npc_starts_dialogue() {
        let mut map = TownMap::new(5, 5, TileType::Floor);
        map.set(2, 1, TileType::NpcGuard);
        let outcome = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Talk,
            AreaId::Town,
        );
        assert!(matches!(
            outcome,
            InteractOutcome::StartDialogue(DialoguePartner::Guard)
        ));
    }

    #[test]
    fn test_talk_to_empty_direction_fails() {
        let mut map = TownMap::new(5, 5, TileType::Floor);
        let outcome = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Talk,
            AreaId::Town,
        );
        assert!(matches!(outcome, InteractOutcome::Message(_)));
    }

    #[test]
    fn test_examine_chest_opens_it() {
        let mut map = TownMap::new(5, 5, TileType::DungeonFloor);
        map.set(2, 1, TileType::ChestClosed);
        let outcome = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Examine,
            AreaId::DungeonB1F,
        );
        assert!(matches!(outcome, InteractOutcome::ChestOpened { .. }));
        assert_eq!(map.get(2, 1), Some(TileType::ChestOpen));
    }

    #[test]
    fn test_steal_on_npc_is_not_yet_implemented() {
        let mut map = TownMap::new(5, 5, TileType::Floor);
        map.set(2, 1, TileType::NpcSuspicious);
        let outcome = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Steal,
            AreaId::Town,
        );
        assert!(matches!(outcome, InteractOutcome::Message(_)));
    }

    #[test]
    fn test_examine_sign_varies_by_area() {
        let mut map = TownMap::new(5, 5, TileType::Floor);
        map.set(2, 1, TileType::Sign);

        // 王都
        let outcome_town = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Examine,
            AreaId::Town,
        );
        if let InteractOutcome::Message(msg) = outcome_town {
            assert!(msg.contains("王都アルカン・案内看板"));
        } else {
            panic!("Expected Message outcome");
        }

        // すずかけ村
        let outcome_village = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Examine,
            AreaId::Village,
        );
        if let InteractOutcome::Message(msg) = outcome_village {
            assert!(msg.contains("すずかけ村・案内看板"));
        } else {
            panic!("Expected Message outcome");
        }

        // 地下迷宮
        let outcome_dungeon = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Examine,
            AreaId::DungeonB1F,
        );
        if let InteractOutcome::Message(msg) = outcome_dungeon {
            assert!(msg.contains("風化した古い警告板"));
        } else {
            panic!("Expected Message outcome");
        }
    }

    #[test]
    fn test_examine_npc_varies_by_area() {
        let mut map = TownMap::new(5, 5, TileType::Floor);
        map.set(2, 1, TileType::NpcVillager);

        let outcome_town = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Examine,
            AreaId::Town,
        );
        if let InteractOutcome::Message(msg) = outcome_town {
            assert!(msg.contains("街の女性"));
        } else {
            panic!("Expected Message outcome");
        }

        let outcome_village = resolve(
            &mut map,
            Position { x: 2, y: 2 },
            Facing::Up,
            CommandKind::Examine,
            AreaId::Village,
        );
        if let InteractOutcome::Message(msg) = outcome_village {
            assert!(msg.contains("村の娘"));
        } else {
            panic!("Expected Message outcome");
        }
    }

    #[test]
    fn test_dynamic_label_switches_to_look_for_npc() {
        assert_eq!(CommandKind::Examine.dynamic_label(TargetKind::Npc), "みる");
        assert_eq!(
            CommandKind::Examine.dynamic_label(TargetKind::Object),
            "しらべる"
        );
    }

    #[test]
    fn test_only_item_skips_direction_selection() {
        assert!(!CommandKind::Item.needs_direction());
        assert!(CommandKind::Examine.needs_direction());
        assert!(CommandKind::Talk.needs_direction());
        assert!(CommandKind::Steal.needs_direction());
    }
}
