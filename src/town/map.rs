#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileType {
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
        }
    }

    #[allow(dead_code)]
    pub fn is_walkable(&self) -> bool {
        matches!(
            self,
            TileType::Floor | TileType::DoorOpen | TileType::StairsDown
        )
    }

    pub fn blocks_sight(&self) -> bool {
        matches!(self, TileType::Wall | TileType::DoorClosed)
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
}
