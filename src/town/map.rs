#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaId {
    Town,
    DungeonB1F,
}

impl AreaId {
    pub fn name(&self) -> &'static str {
        match self {
            AreaId::Town => "王都アルカン・商業区",
            AreaId::DungeonB1F => "封魔の地下迷宮 B1F",
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
    pub fn create_arkan_capital() -> Self {
        let width = 46;
        let height = 9;
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

        // 中央広場：噴水・水路・街路樹・案内板
        map.set(22, 3, TileType::Water);
        map.set(23, 3, TileType::Water);
        map.set(22, 4, TileType::Water);
        map.set(23, 4, TileType::Water);

        map.set(17, 3, TileType::Tree);
        map.set(28, 3, TileType::Tree);
        map.set(17, 5, TileType::Tree);
        map.set(28, 5, TileType::Tree);

        map.set(21, 5, TileType::Sign);

        // 南東エリア：封鎖された地下迷宮階段 [>] と見張りの衛兵 [G]
        for x in 35..=44 {
            map.set(x, 5, TileType::Wall);
        }
        for y in 5..=8 {
            map.set(35, y, TileType::Wall);
        }
        map.set(35, 7, TileType::DoorClosed); // 地下区画への門扉
        map.set(41, 7, TileType::StairsDown); // 地下迷宮への階段
        map.set(37, 7, TileType::NpcGuard);   // 衛兵

        // 南西エリア：貧民街裏路地
        for x in 1..=14 {
            map.set(x, 6, TileType::Wall);
        }
        map.set(8, 6, TileType::Floor); // 抜け道
        map.set(3, 7, TileType::NpcSuspicious); // 怪しい男

        // 町人配置
        map.set(15, 2, TileType::NpcVillager);
        map.set(25, 6, TileType::NpcVillager);

        map
    }

    /// 封魔の地下迷宮 B1F マップ生成
    /// （将来的なプロシージャル生成を見据えた初期固定迷宮）
    pub fn create_dungeon_b1f() -> Self {
        let width = 46;
        let height = 9;
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
