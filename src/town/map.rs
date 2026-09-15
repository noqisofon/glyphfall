#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaId {
    Town,
    DungeonB1F,
    Village,
}

impl AreaId {
    pub fn name(&self) -> &'static str {
        match self {
            AreaId::Town => "王都アルカン・商業区",
            AreaId::DungeonB1F => "封魔の地下迷宮 B1F",
            AreaId::Village => "近郊の村・すずかけ村",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileType {
    // 街用タイル
    Wall,
    Floor,
    DoorClosed,
    DoorOpen,
    Water,
    Tree,
    Sign,
    Inn,
    Tavern,
    Shop,
    StairsDown,
    NpcGuard,
    NpcVillager,
    NpcSuspicious,
    /// 旅シミュレーション(ADR-0004)への入り口。接触すると移動姿勢選択（ADR-0013の
    /// 発生エンジンを介した道中ロール）を経て別の町・村へ移動する。単純な移動では
    /// 通過できない（`is_walkable`はfalse）。
    RoadExit,

    // ダンジョン用タイル
    DungeonWall,
    DungeonFloor,
    IronGateClosed,
    IronGateOpen,
    ChestClosed,
    ChestOpen,
    StairsUp,
    MonsterSymbol,
}

impl TileType {
    #[allow(dead_code)]
    pub fn glyph(&self) -> char {
        match self {
            TileType::Wall => '#',
            TileType::Floor => '.',
            TileType::DoorClosed => '+',
            TileType::DoorOpen => '\'',
            TileType::Water => '~',
            TileType::Tree => '♠',
            TileType::Sign => '§',
            TileType::Inn => 'H',
            TileType::Tavern => 'T',
            TileType::Shop => 'S',
            TileType::StairsDown => '>',
            TileType::NpcGuard => 'G',
            TileType::NpcVillager => 'P',
            TileType::NpcSuspicious => '?',
            TileType::RoadExit => '=',
            TileType::DungeonWall => '▓',
            TileType::DungeonFloor => '·',
            TileType::IronGateClosed => '#',
            TileType::IronGateOpen => '/',
            TileType::ChestClosed => 'C',
            TileType::ChestOpen => 'c',
            TileType::StairsUp => '<',
            TileType::MonsterSymbol => 'M',
        }
    }

    #[allow(dead_code)]
    pub fn is_walkable(&self) -> bool {
        matches!(
            self,
            TileType::Floor
                | TileType::DoorOpen
                | TileType::StairsDown
                | TileType::DungeonFloor
                | TileType::IronGateOpen
                | TileType::ChestOpen
                | TileType::StairsUp
        )
    }

    pub fn blocks_sight(&self) -> bool {
        matches!(
            self,
            TileType::Wall
                | TileType::DoorClosed
                | TileType::DungeonWall
                | TileType::IronGateClosed
        )
    }
}

pub struct TownMap {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<TileType>,
}

impl TownMap {
    pub fn new(width: usize, height: usize, default_tile: TileType) -> Self {
        Self {
            width,
            height,
            tiles: vec![default_tile; width * height],
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<TileType> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            None
        } else {
            Some(self.tiles[y as usize * self.width + x as usize])
        }
    }

    pub fn set(&mut self, x: i32, y: i32, tile: TileType) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.tiles[y as usize * self.width + x as usize] = tile;
        }
    }

    /// 王都アルカン 商業区の地上マップ生成
    ///
    /// 中央広場を南北に拡張し、以前より広い王都を表示できるようにしている。
    /// 北側の建物・南側の各エリアの内部構造は旧レイアウトのまま、拡張した
    /// 広場帯（y=5..=8）を挟んで4行分南へずらして配置している。
    pub fn create_arkan_capital() -> Self {
        let width = 46;
        let height = 13;
        let mut map = Self::new(width, height, TileType::Floor);

        // 外壁
        for x in 0..width as i32 {
            map.set(x, 0, TileType::Wall);
            map.set(x, height as i32 - 1, TileType::Wall);
        }
        for y in 0..height as i32 {
            map.set(0, y, TileType::Wall);
            map.set(width as i32 - 1, y, TileType::Wall);
        }

        // 北西建物：宿屋 [H] & 酒場 [T]
        for x in 1..=12 {
            map.set(x, 3, TileType::Wall);
        }
        for y in 1..=3 {
            map.set(6, y, TileType::Wall);
            map.set(12, y, TileType::Wall);
        }
        map.set(3, 3, TileType::DoorClosed); // 宿屋入口
        map.set(9, 3, TileType::DoorClosed); // 酒場入口
        map.set(3, 1, TileType::Inn);
        map.set(9, 1, TileType::Tavern);

        // 北東建物：商店 [S]
        for x in 32..=44 {
            map.set(x, 3, TileType::Wall);
        }
        for y in 1..=3 {
            map.set(32, y, TileType::Wall);
        }
        map.set(37, 3, TileType::DoorClosed); // 商店入口
        map.set(37, 1, TileType::Shop);

        // 中央広場（北側）：噴水・水路・街路樹
        map.set(22, 3, TileType::Water);
        map.set(23, 3, TileType::Water);
        map.set(22, 4, TileType::Water);
        map.set(23, 4, TileType::Water);

        map.set(17, 3, TileType::Tree);
        map.set(28, 3, TileType::Tree);

        // 中央広場（拡張帯）：水路を延長し並木を増やして広場自体を拡大
        map.set(22, 5, TileType::Water);
        map.set(23, 5, TileType::Water);
        map.set(22, 6, TileType::Water);
        map.set(23, 6, TileType::Water);

        map.set(15, 5, TileType::Tree);
        map.set(30, 5, TileType::Tree);
        map.set(15, 8, TileType::Tree);
        map.set(30, 8, TileType::Tree);

        map.set(18, 7, TileType::NpcVillager); // 広場を散策する住人

        // 案内板・街路樹（旧y=5から拡張帯の分だけ南へ）
        map.set(17, 9, TileType::Tree);
        map.set(28, 9, TileType::Tree);
        map.set(21, 9, TileType::Sign);

        // 南東エリア：封鎖された地下迷宮階段 [>] と見張りの衛兵 [G]
        for x in 35..=44 {
            map.set(x, 9, TileType::Wall);
        }
        for y in 9..=12 {
            map.set(35, y, TileType::Wall);
        }
        map.set(35, 11, TileType::DoorClosed); // 地下区画への門扉
        map.set(41, 11, TileType::StairsDown); // 地下迷宮への階段
        map.set(37, 11, TileType::NpcGuard);   // 衛兵

        // 南西エリア：貧民街裏路地
        for x in 1..=14 {
            map.set(x, 10, TileType::Wall);
        }
        map.set(8, 10, TileType::Floor); // 抜け道
        map.set(3, 11, TileType::NpcSuspicious); // 怪しい男

        // 町人配置
        map.set(15, 2, TileType::NpcVillager);
        map.set(25, 10, TileType::NpcVillager);

        // 西側の街道口：ここへ向かって移動すると旅シミュレーション（ADR-0004/0013）を開始する
        map.set(0, 6, TileType::RoadExit);

        map
    }

    /// 近郊の村「すずかけ村」の地上マップ生成（ADR-0013 突発イベント発生システムの
    /// 動作検証用に追加した最小の目的地）。
    ///
    /// テクスチャサイズを王都・ダンジョンと揃えるため 46×13 の同一サイズで生成する。
    pub fn create_suzukake_village() -> Self {
        let width = 46;
        let height = 13;
        let mut map = Self::new(width, height, TileType::Floor);

        // 外壁
        for x in 0..width as i32 {
            map.set(x, 0, TileType::Wall);
            map.set(x, height as i32 - 1, TileType::Wall);
        }
        for y in 0..height as i32 {
            map.set(0, y, TileType::Wall);
            map.set(width as i32 - 1, y, TileType::Wall);
        }

        // 東側の街道口：王都アルカンへ戻る旅シミュレーションの入り口
        map.set(width as i32 - 1, 6, TileType::RoadExit);

        // 中央の農村家屋
        for x in 18..=22 {
            map.set(x, 3, TileType::Wall);
        }
        for y in 1..=3 {
            map.set(18, y, TileType::Wall);
            map.set(22, y, TileType::Wall);
        }
        map.set(20, 3, TileType::DoorClosed);
        map.set(20, 1, TileType::NpcVillager);

        // 井戸まわりの街路樹
        map.set(10, 6, TileType::Tree);
        map.set(35, 6, TileType::Tree);
        map.set(10, 9, TileType::Tree);
        map.set(35, 9, TileType::Tree);

        map.set(23, 8, TileType::Sign);
        map.set(15, 8, TileType::NpcVillager);
        map.set(30, 4, TileType::NpcVillager);

        map
    }

    /// 封魔の地下迷宮 B1F マップ生成
    /// （将来的なプロシージャル生成を見据えた初期固定迷宮）
    ///
    /// 高さは王都マップ（拡張後13マス）とテクスチャサイズを揃えるために合わせて
    /// あるが、迷宮自体の内部構造（y=1..=7）は変更していない。増えた分の下端は
    /// 既定の DungeonWall で塞がれるだけで、到達不能な余白になる。
    pub fn create_dungeon_b1f() -> Self {
        let width = 46;
        let height = 13;
        let mut map = Self::new(width, height, TileType::DungeonWall);

        // 基本通路を掘る (Floor)
        // 西のエントランスホール
        for x in 1..=5 {
            for y in 3..=5 {
                map.set(x, y, TileType::DungeonFloor);
            }
        }
        map.set(2, 4, TileType::StairsUp); // 地上への上り階段

        // 中央大通路
        for x in 5..=36 {
            map.set(x, 4, TileType::DungeonFloor);
        }

        // 北側：宝物庫 (12..18, 1..3)
        for x in 12..=18 {
            for y in 1..=3 {
                map.set(x, y, TileType::DungeonFloor);
            }
        }
        map.set(15, 3, TileType::IronGateClosed); // 鉄格子扉
        map.set(15, 1, TileType::ChestClosed);    // 宝箱A

        // 南側：水没した礼拝堂 (18..26, 5..7)
        for x in 18..=26 {
            for y in 5..=7 {
                map.set(x, y, TileType::DungeonFloor);
            }
        }
        map.set(22, 4, TileType::DungeonFloor);
        map.set(22, 6, TileType::Water);
        map.set(23, 6, TileType::Water);
        map.set(25, 7, TileType::ChestClosed); // 宝箱B

        // 北東：牢獄区画 (28..34, 1..3)
        for x in 28..=34 {
            for y in 1..=3 {
                map.set(x, y, TileType::DungeonFloor);
            }
        }
        map.set(31, 3, TileType::IronGateClosed); // 牢獄の鉄格子

        // 東端の大広間：魔物の巣窟 (36..44, 2..7)
        for x in 36..=44 {
            for y in 2..=7 {
                map.set(x, y, TileType::DungeonFloor);
            }
        }
        map.set(40, 4, TileType::MonsterSymbol); // 魔物の気配
        map.set(43, 4, TileType::StairsDown);    // B2Fへの階段

        map
    }
}
