use glyphfall_core::town::{FollowerHistory, Position, TileType, TownMap};

pub const TILE_SIZE: usize = 16;
pub const MAP_WIDTH_TILES: usize = 46;
pub const MAP_HEIGHT_TILES: usize = 13;

pub const TEXTURE_WIDTH: usize = MAP_WIDTH_TILES * TILE_SIZE; // 736
pub const TEXTURE_HEIGHT: usize = MAP_HEIGHT_TILES * TILE_SIZE; // 208

type Pixel = [u8; 4];

// カラーパレット定数 (RGBA)
#[allow(dead_code)]
mod colors {
    pub const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];
    pub const BLACK: [u8; 4] = [15, 15, 20, 255];
    pub const DARK_GRAY: [u8; 4] = [55, 60, 70, 255];
    pub const GRAY: [u8; 4] = [115, 120, 130, 255];
    pub const LIGHT_GRAY: [u8; 4] = [185, 190, 200, 255];
    pub const WHITE: [u8; 4] = [245, 245, 250, 255];

    pub const BROWN: [u8; 4] = [115, 65, 35, 255];
    pub const LIGHT_BROWN: [u8; 4] = [175, 115, 65, 255];
    pub const DARK_BROWN: [u8; 4] = [70, 35, 15, 255];

    pub const BRICK_RED: [u8; 4] = [165, 65, 55, 255];
    pub const DARK_RED: [u8; 4] = [110, 30, 25, 255];
    pub const RED: [u8; 4] = [225, 55, 55, 255];

    pub const GREEN: [u8; 4] = [45, 145, 55, 255];
    pub const DARK_GREEN: [u8; 4] = [20, 85, 35, 255];
    pub const LIGHT_GREEN: [u8; 4] = [95, 205, 85, 255];

    pub const BLUE: [u8; 4] = [45, 95, 205, 255];
    pub const DARK_BLUE: [u8; 4] = [25, 50, 125, 255];
    pub const WATER_BLUE: [u8; 4] = [40, 120, 200, 255];
    pub const WATER_LIGHT: [u8; 4] = [95, 185, 245, 255];

    pub const YELLOW: [u8; 4] = [245, 215, 55, 255];
    pub const GOLD: [u8; 4] = [215, 170, 35, 255];

    pub const SKIN: [u8; 4] = [255, 210, 175, 255];
    pub const SHADOW_SKIN: [u8; 4] = [215, 165, 130, 255];

    pub const PURPLE: [u8; 4] = [145, 65, 185, 255];
    pub const DARK_PURPLE: [u8; 4] = [85, 35, 115, 255];
}

fn parse_pattern(lines: [&str; 16], palette: impl Fn(char) -> Pixel) -> [Pixel; 256] {
    let mut pixels = [colors::TRANSPARENT; 256];
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            if x < 16 && y < 16 {
                pixels[y * 16 + x] = palette(ch);
            }
        }
    }
    pixels
}

// ─────────────────────────────────────────────
// タイルパターンの定義
// ─────────────────────────────────────────────

fn get_floor_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "................",
            ".GGGGGG..GGGGGG.",
            ".GGGGGG..GGGGGG.",
            ".GggGgg..GggGgg.",
            "................",
            "...GGGGGG..GGGG.",
            "...GGGGGG..GGGG.",
            "...GggGgg..GggG.",
            "................",
            ".GGGG..GGGGGG...",
            ".GGGG..GGGGGG...",
            ".GggG..GggGgg...",
            "................",
            "....GGGGGG..GGGG",
            "....GGGGGG..GGGG",
            "................",
        ],
        |ch| match ch {
            'G' => colors::GRAY,
            'g' => colors::LIGHT_GRAY,
            '.' => colors::DARK_GRAY,
            _ => colors::BLACK,
        },
    )
}

fn get_wall_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "bbbbbbbbbbbbbbbb",
            "rRRRRrr.rRRRRrr.",
            "rRRRRrr.rRRRRrr.",
            "........rRRRRrr.",
            "rRRRRrr.........",
            "rRRRRrr.rRRRRrr.",
            "rRRRRrr.rRRRRrr.",
            "........rRRRRrr.",
            "rRRRRrr.........",
            "rRRRRrr.rRRRRrr.",
            "rRRRRrr.rRRRRrr.",
            "........rRRRRrr.",
            "rRRRRrr.........",
            "rRRRRrr.rRRRRrr.",
            "rRRRRrr.rRRRRrr.",
            "bbbbbbbbbbbbbbbb",
        ],
        |ch| match ch {
            'R' => colors::BRICK_RED,
            'r' => colors::DARK_RED,
            '.' => colors::DARK_GRAY,
            'b' => colors::BLACK,
            _ => colors::BLACK,
        },
    )
}

fn get_water_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "bbbbbbbbbbbbbbbb",
            "bBBBBBBBBBBBBBBb",
            "bBBBBWWBBBBBBBBb",
            "bBBBBWBBBBBBWWbb",
            "bBBBBBBBBBBBWBBb",
            "bBBBBBBWWBBBBBBb",
            "bBBBBBBWBBBBBBBb",
            "bBBWWBBBBBBBBBBb",
            "bBWBBBBBBWWBBBBb",
            "bBBBBBBBBWBBBBBb",
            "bBBBBWWBBBBBBBBb",
            "bBBBBWBBBBBBWWbb",
            "bBBBBBBBBBBBWBBb",
            "bBBBBBBBBBBBBBBb",
            "bBBBBBBBBBBBBBBb",
            "bbbbbbbbbbbbbbbb",
        ],
        |ch| match ch {
            'B' => colors::WATER_BLUE,
            'W' => colors::WATER_LIGHT,
            'b' => colors::DARK_BLUE,
            _ => colors::BLACK,
        },
    )
}

fn get_tree_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....gggggg.....",
            "...ggGGGGGGgg...",
            "..gGGGGGGGGGGg..",
            ".gGGGGGGGGGGGGg.",
            ".gGGGGGGGGGGGGg.",
            "gGGGGGGGGGGGGGGg",
            "gGGGGGGGGGGGGGGg",
            ".gGGGGGGGGGGGGg.",
            "..gGGgGGGGgGGg..",
            "....g..BB..g....",
            ".......BB.......",
            ".......BB.......",
            ".......BB.......",
            "......BBBB......",
            ".....BBBBBB.....",
            "................",
        ],
        |ch| match ch {
            'G' => colors::GREEN,
            'g' => colors::LIGHT_GREEN,
            'B' => colors::BROWN,
            '.' => colors::DARK_GRAY,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_door_closed_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "################",
            "#WWWWWWWWWWWWWW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBYBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WBBBBBBBBBBBBW#",
            "#WWWWWWWWWWWWWW#",
            "################",
        ],
        |ch| match ch {
            '#' => colors::BLACK,
            'W' => colors::DARK_BROWN,
            'B' => colors::BROWN,
            'Y' => colors::GOLD,
            _ => colors::BLACK,
        },
    )
}

fn get_door_open_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "################",
            "#W............W#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WB..........BW#",
            "#WW..........WW#",
            "################",
        ],
        |ch| match ch {
            '#' => colors::BLACK,
            'W' => colors::DARK_BROWN,
            'B' => colors::BROWN,
            '.' => colors::DARK_GRAY,
            _ => colors::BLACK,
        },
    )
}

fn get_sign_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "................",
            "...BBBBBBBBBB...",
            "..BYYYYYYYYYYB..",
            "..BY.b.bb.b.YB..",
            "..BY..b.b.b.YB..",
            "..BY.bb.b.b.YB..",
            "..BYYYYYYYYYYB..",
            "...BBBBBBBBBB...",
            "........BB......",
            "........BB......",
            "........BB......",
            "........BB......",
            ".......BBBB.....",
            "......BBBBBB....",
            "................",
            "................",
        ],
        |ch| match ch {
            'B' => colors::DARK_BROWN,
            'Y' => colors::SKIN,
            'b' => colors::DARK_BROWN,
            '.' => colors::DARK_GRAY,
            _ => colors::DARK_GRAY,
        },
    )
}

fn get_inn_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....RRRRRR.....",
            "...RRRRRRRRRR...",
            "..RRRRRRRRRRRR..",
            ".RRRRRRRRRRRRRR.",
            "################",
            "#WWWWWWWWWWWWWW#",
            "#W..HH....HH..W#",
            "#W..HH....HH..W#",
            "#W..HHHHHHHH..W#",
            "#W..HHHHHHHH..W#",
            "#W..HH....HH..W#",
            "#W..HH....HH..W#",
            "#W............W#",
            "#W...WWWWWW...W#",
            "#WWWWWWWWWWWWWW#",
            "################",
        ],
        |ch| match ch {
            'R' => colors::RED,
            '#' => colors::BLACK,
            'W' => colors::DARK_BROWN,
            'H' => colors::GOLD,
            '.' => colors::WHITE,
            _ => colors::BLACK,
        },
    )
}

fn get_tavern_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....BBBBBB.....",
            "...BBBBBBBBBB...",
            "..BBBBBBBBBBBB..",
            ".BBBBBBBBBBBBBB.",
            "################",
            "#WWWWWWWWWWWWWW#",
            "#W..TTTTTTTT..W#",
            "#W..TTTTTTTT..W#",
            "#W.....TT.....W#",
            "#W.....TT.....W#",
            "#W.....TT.....W#",
            "#W.....TT.....W#",
            "#W............W#",
            "#W...WWWWWW...W#",
            "#WWWWWWWWWWWWWW#",
            "################",
        ],
        |ch| match ch {
            'B' => colors::BROWN,
            '#' => colors::BLACK,
            'W' => colors::DARK_BROWN,
            'T' => colors::YELLOW,
            '.' => colors::DARK_GRAY,
            _ => colors::BLACK,
        },
    )
}

fn get_shop_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....GGGGGG.....",
            "...GGGGGGGGGG...",
            "..GGGGGGGGGGGG..",
            ".GGGGGGGGGGGGGG.",
            "################",
            "#WWWWWWWWWWWWWW#",
            "#W...SSSSSS...W#",
            "#W..SS....SS..W#",
            "#W...SS.......W#",
            "#W....SSSS....W#",
            "#W.......SS...W#",
            "#W..SS....SS..W#",
            "#W...SSSSSS...W#",
            "#W............W#",
            "#WWWWWWWWWWWWWW#",
            "################",
        ],
        |ch| match ch {
            'G' => colors::GREEN,
            '#' => colors::BLACK,
            'W' => colors::DARK_BROWN,
            'S' => colors::GOLD,
            '.' => colors::WHITE,
            _ => colors::BLACK,
        },
    )
}

fn get_stairs_down_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "################",
            "#bbbbbbbbbbbbbb#",
            "#bbbbbbbbbbbbbb#",
            "#bbbbbbbbbbbbbb#",
            "#bDDDDDDDDDDDDb#",
            "#bDDDDDDDDDDDDb#",
            "#bGGGGGGGGGGGGb#",
            "#bGGGGGGGGGGGGb#",
            "#bggggggggggggb#",
            "#bggggggggggggb#",
            "#bWWWWWWWWWWWWb#",
            "#bWWWWWWWWWWWWb#",
            "#b............b#",
            "#b............b#",
            "#bbbbbbbbbbbbbb#",
            "################",
        ],
        |ch| match ch {
            '#' => colors::BLACK,
            'b' => colors::BLACK,
            'D' => colors::DARK_GRAY,
            'G' => colors::GRAY,
            'g' => colors::LIGHT_GRAY,
            'W' => colors::WHITE,
            '.' => colors::DARK_GRAY,
            _ => colors::BLACK,
        },
    )
}

fn get_road_exit_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "................",
            ".BBBB......BBBB.",
            ".BbBB......BBbB.",
            ".BBBB......BBBB.",
            "................",
            "..DDDDDDDDDDDD..",
            "..DdDDdDDdDDdD..",
            "..DDDDDDDDDDDD..",
            "..DDdDDDDdDDdD..",
            "..DDDDDDDDDDDD..",
            "................",
            ".GGGG......GGGG.",
            ".GgGG......GGgG.",
            ".GGGG......GGGG.",
            "................",
            "................",
        ],
        |ch| match ch {
            'B' => colors::DARK_GREEN,
            'b' => colors::GREEN,
            'D' => colors::LIGHT_BROWN,
            'd' => colors::BROWN,
            'G' => colors::DARK_GREEN,
            'g' => colors::GREEN,
            '.' => colors::DARK_GRAY,
            _ => colors::DARK_GRAY,
        },
    )
}

fn get_npc_guard_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....SSSSSS.....",
            "....SSSSSSSS....",
            "....SSSSSSSS....",
            "....SSkkkkSS....",
            "....SSSSSSSS....",
            "...bbBBBBBBbb...",
            "..bbbBBBBBBbbb..",
            "..bW.BBBBBB.Wb..",
            "..bW.BBBBBB.Wb..",
            "..b..BBBBBB..b..",
            ".....BBBBBB.....",
            ".....SSSSSS.....",
            ".....SS..SS.....",
            ".....SS..SS.....",
            "....SSS..SSS....",
            "....SSS..SSS....",
        ],
        |ch| match ch {
            'S' => colors::LIGHT_GRAY,
            'k' => colors::BLACK,
            'B' => colors::BLUE,
            'b' => colors::DARK_BLUE,
            'W' => colors::WHITE,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_npc_villager_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....YYYYYY.....",
            "....YYYYYYYY....",
            "....hssssssh....",
            "....hskksskh....",
            "....hssssssh....",
            "....hsswwssh....",
            ".....GGGGGG.....",
            "....gWWWWWWg....",
            "...ggWWWWWWgg...",
            "...ggWWWWWWgg...",
            "....gWWWWWWg....",
            ".....GGGGGG.....",
            ".....GG..GG.....",
            ".....GG..GG.....",
            "....bbb..bbb....",
            "....bbb..bbb....",
        ],
        |ch| match ch {
            'Y' => colors::YELLOW,
            'h' => colors::BROWN,
            's' => colors::SKIN,
            'k' => colors::BLACK,
            'w' => colors::RED,
            'G' => colors::GREEN,
            'g' => colors::LIGHT_GREEN,
            'W' => colors::WHITE,
            'b' => colors::DARK_BROWN,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_npc_suspicious_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....bbbbbb.....",
            "....bbbbbbbb....",
            "...bbbbbbbbbb...",
            "...bb..bb..bb...",
            "...bb.R..R.bb...",
            "...bbbbbbbbbb...",
            "...bbbbbbbbbb...",
            "....bbbbbbbb....",
            "...bbbbbbbbbb...",
            "..bbbbbbbbbbbb..",
            "..bbbbbbbbbbbb..",
            "..bbbbbbbbbbbb..",
            "....bbbbbbbb....",
            "....bb....bb....",
            "...bbb....bbb...",
            "...bbb....bbb...",
        ],
        |ch| match ch {
            'b' => colors::BLACK,
            'R' => colors::RED,
            '.' => colors::DARK_GRAY,
            _ => colors::TRANSPARENT,
        },
    )
}

// ─────────────────────────────────────────────
// キャラクターパターンの定義 (16×16)
// ─────────────────────────────────────────────

fn get_player_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....hhhhhh.....",
            "....hhhhhhhh....",
            "....hsskkssh....",
            "....hssssssh....",
            "....hssssssh....",
            "....hhsbbshh....",
            ".....bbccbb.....",
            "....bccccccb....",
            "....bccccccb....",
            "....bccccccb....",
            "....bccccccb....",
            "....bbccccbb....",
            ".....bb..bb.....",
            ".....DD..DD.....",
            ".....DD..DD.....",
            "....DDD..DDD....",
        ],
        |ch| match ch {
            'h' => colors::BROWN,
            's' => colors::SKIN,
            'k' => colors::BLACK,
            'c' => colors::BLUE,
            'b' => colors::DARK_BLUE,
            'D' => colors::DARK_BROWN,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_warrior_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "......RRRR......",
            ".....RRRRRR.....",
            "....SSSSSSSS....",
            "....SSssssSS....",
            "....SSkkkkSS....",
            "....SSSSSSSS....",
            "....AAAcccAA....",
            "...AAAAAAAcA....",
            "...AAAAAAAAA....",
            "...AAAAAAAAA....",
            "...AAAAAAAcA....",
            "....AAAAAAAA....",
            "....AAAA.AAA....",
            "....AAAA.AAA....",
            "....SSSS.SSSS...",
            "...SSSS..SSSS...",
        ],
        |ch| match ch {
            'R' => colors::RED,
            'S' => colors::LIGHT_GRAY,
            's' => colors::SKIN,
            'k' => colors::BLACK,
            'A' => colors::GRAY,
            'c' => colors::GOLD,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_slacker_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "..YY......GG....",
            ".YYYY....GGGG...",
            ".YYYY....GGGG...",
            "..YYYY..GGGG....",
            "...YYYYGGGG.....",
            "....YssssG......",
            "....sskkss......",
            "....ssssss......",
            "....sswwss......",
            "....YYMMGG......",
            "...YYYYMMGG.....",
            "...YYYYMMGG.....",
            "....YYYYGG......",
            ".....YY..GG.....",
            "....bbb..bbb....",
            "...bbbb..bbbb...",
        ],
        |ch| match ch {
            'Y' => colors::YELLOW,
            'G' => colors::GREEN,
            's' => colors::SKIN,
            'k' => colors::BLACK,
            'w' => colors::RED,
            'M' => colors::PURPLE,
            'b' => colors::DARK_BROWN,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_mage_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".......PP.......",
            "......PPPP......",
            ".....PPPPPP.....",
            "....PPPPPPPP....",
            "...PPPPPPPPPP...",
            "..PPPPPPPPPPPP..",
            ".PPPPPPPPPPPPPP.",
            "....PPssssPP....",
            "....hskksh......",
            "....hsssssh.....",
            "....hsssssh.....",
            "....PPPPPPPP....",
            "...PPPPPPPPPP...",
            "..PPPPPPPPPPPP..",
            "..PPPPPPPPPPPP..",
            ".PPPPPPPPPPPPPP.",
        ],
        |ch| match ch {
            'P' => colors::PURPLE,
            's' => colors::SKIN,
            'k' => colors::BLACK,
            'h' => colors::DARK_BROWN,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_knight_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            ".....WWWWWW.....",
            "....WWWWWWWW....",
            "....WWbbbbWW....",
            "....WWbbbbWW....",
            "....WWWWWWWW....",
            "...bWWWWWWWWb...",
            "..bbWWWWWWWWbb..",
            "..bbWWSSWWWWbb..",
            "..bbWWSSWWWWbb..",
            "..bbWWWWWWWWbb..",
            "...bWWWWWWWWb...",
            "....WWWWWWWW....",
            "....WWWW.WWWW...",
            "....WWWW.WWWW...",
            "...WWWW...WWWW..",
            "..WWWW.....WWWW.",
        ],
        |ch| match ch {
            'W' => colors::WHITE,
            'b' => colors::BLUE,
            'S' => colors::GOLD,
            _ => colors::TRANSPARENT,
        },
    )
}

fn get_dungeon_wall_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "bbbbbbbbbbbbbbbb",
            "bDDDDDDDDDDDDDDb",
            "bDDMMDDDDDDDDDDb",
            "bDDMMDDDDMDDDDDb",
            "b.......DDMDDDDb",
            "bDDDDDD...DDDDDb",
            "bDDDDDDDD.DDDDDb",
            "bDDDDDDDD.DDDDDb",
            "b.......DDDDDDDb",
            "bDDDDDD...DDDDDb",
            "bDDDDDDDD.DDMDDb",
            "bDDMMDDDD.DDMDDb",
            "bDDMMDDDD......b",
            "bDDDDDDDDDDDDDDb",
            "bDDDDDDDDDDDDDDb",
            "bbbbbbbbbbbbbbbb",
        ],
        |ch| match ch {
            'D' => [50, 55, 68, 255],
            'M' => [35, 90, 45, 255],
            '.' => [20, 22, 28, 255],
            'b' => colors::BLACK,
            _ => colors::BLACK,
        },
    )
}

fn get_dungeon_floor_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "................",
            ".DDDDDD..DDDDDD.",
            ".DDDDDD..DDDDDD.",
            ".DD..DD..DD..DD.",
            "................",
            "...DDDDDD..DDDD.",
            "...DDDDDD..DDDD.",
            "...DD..DD..DD.D.",
            "................",
            ".DDDD..DDDDDD...",
            ".DDDD..DDDDDD...",
            ".D..D..DD..DD...",
            "................",
            "....DDDDDD..DDDD",
            "....DDDDDD..DDDD",
            "................",
        ],
        |ch| match ch {
            'D' => [40, 45, 55, 255],
            '.' => [22, 25, 32, 255],
            _ => colors::BLACK,
        },
    )
}

fn get_iron_gate_closed_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "################",
            "#b.b.b.b.b.b.b.#",
            "#S.S.S.S.S.S.S.#",
            "#S.S.S.S.S.S.S.#",
            "#SSSSSSSSSSSSSS#",
            "#S.S.S.S.S.S.S.#",
            "#S.S.S.S.S.S.S.#",
            "#S.S.S.S.S.S.S.#",
            "#SSSSSSSSSSSSSS#",
            "#S.S.S.S.S.S.S.#",
            "#S.S.S.S.S.S.S.#",
            "#S.S.S.S.S.S.S.#",
            "#SSSSSSSSSSSSSS#",
            "#S.S.S.S.S.S.S.#",
            "#b.b.b.b.b.b.b.#",
            "################",
        ],
        |ch| match ch {
            '#' => colors::BLACK,
            'S' => [120, 125, 135, 255],
            'b' => [60, 65, 75, 255],
            '.' => [22, 25, 32, 255],
            _ => colors::BLACK,
        },
    )
}

fn get_iron_gate_open_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "################",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "#S............S#",
            "################",
        ],
        |ch| match ch {
            '#' => colors::BLACK,
            'S' => [120, 125, 135, 255],
            '.' => [22, 25, 32, 255],
            _ => colors::BLACK,
        },
    )
}

fn get_chest_closed_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "................",
            "....bbbbbbbb....",
            "...bYYYYYYYYb...",
            "..bYBBBBBBBBYb..",
            "..bYBBBBBBBBYb..",
            "..bYYYYYYYYYYb..",
            "..bYBBBYYBBBYb..",
            "..bYBBBYYBBBYb..",
            "..bYBBBYkBBBYb..",
            "..bYBBBYYBBBYb..",
            "..bYYYYYYYYYYb..",
            "..bYBBBBBBBBYb..",
            "..bYYYYYYYYYYb..",
            "...bbbbbbbbbb...",
            "................",
            "................",
        ],
        |ch| match ch {
            'b' => colors::BLACK,
            'Y' => colors::GOLD,
            'B' => colors::BROWN,
            'k' => colors::BLACK,
            '.' => [22, 25, 32, 255],
            _ => colors::BLACK,
        },
    )
}

fn get_chest_open_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "...bYYYYYYYYb...",
            "..bYBBBBBBBBYb..",
            "..bYYYYYYYYYYb..",
            "....bbbbbbbb....",
            "..b..........b..",
            "..b.bbbbbbbb.b..",
            "..b.b......b.b..",
            "..b.b......b.b..",
            "..b.bbbbbbbb.b..",
            "..bYBBBYYBBBYb..",
            "..bYYYYYYYYYYb..",
            "..bYBBBBBBBBYb..",
            "..bYYYYYYYYYYb..",
            "...bbbbbbbbbb...",
            "................",
            "................",
        ],
        |ch| match ch {
            'b' => colors::BLACK,
            'Y' => colors::GOLD,
            'B' => colors::DARK_BROWN,
            '.' => [22, 25, 32, 255],
            _ => colors::BLACK,
        },
    )
}

fn get_stairs_up_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "################",
            "#bWWWWWWWWWWWWb#",
            "#bWWWWWWWWWWWWb#",
            "#bWWWWWWWWWWWWb#",
            "#bggggggggggggb#",
            "#bggggggggggggb#",
            "#bGGGGGGGGGGGGb#",
            "#bGGGGGGGGGGGGb#",
            "#bDDDDDDDDDDDDb#",
            "#bDDDDDDDDDDDDb#",
            "#b............b#",
            "#b............b#",
            "#bbbbbbbbbbbbbb#",
            "#bbbbbbbbbbbbbb#",
            "#bbbbbbbbbbbbbb#",
            "################",
        ],
        |ch| match ch {
            '#' => colors::BLACK,
            'W' => [255, 255, 230, 255],
            'g' => colors::WHITE,
            'G' => colors::LIGHT_GRAY,
            'D' => colors::GRAY,
            '.' => colors::DARK_GRAY,
            'b' => colors::BLACK,
            _ => colors::BLACK,
        },
    )
}

fn get_monster_symbol_pixels() -> [Pixel; 256] {
    parse_pattern(
        [
            "................",
            "....bb....bb....",
            "...bbbb..bbbb...",
            "...bbbbbbbbbb...",
            "..bbbbbbbbbbbb..",
            "..bb.RR..RR.bb..",
            "..bb.RR..RR.bb..",
            "..bbbbbbbbbbbb..",
            "...bbbbWWbbbb...",
            "....bbWWWWbb....",
            "...bbbbbbbbbb...",
            "..bbbbbbbbbbbb..",
            "..bb..bbbb..bb..",
            "......bbbb......",
            "................",
            "................",
        ],
        |ch| match ch {
            'b' => [45, 18, 55, 255],
            'R' => colors::RED,
            'W' => colors::WHITE,
            '.' => [22, 25, 32, 255],
            _ => colors::BLACK,
        },
    )
}

// ─────────────────────────────────────────────
// タイル＆スプライトの合成レンダリング
// ─────────────────────────────────────────────

fn get_tile_pixels(tile: TileType) -> [Pixel; 256] {
    match tile {
        TileType::Floor => get_floor_pixels(),
        TileType::Wall => get_wall_pixels(),
        TileType::Water => get_water_pixels(),
        TileType::Tree => get_tree_pixels(),
        TileType::DoorClosed => get_door_closed_pixels(),
        TileType::DoorOpen => get_door_open_pixels(),
        TileType::Sign => get_sign_pixels(),
        TileType::Inn => get_inn_pixels(),
        TileType::Tavern => get_tavern_pixels(),
        TileType::Shop => get_shop_pixels(),
        TileType::StairsDown => get_stairs_down_pixels(),
        TileType::NpcGuard => get_npc_guard_pixels(),
        TileType::NpcVillager => get_npc_villager_pixels(),
        TileType::NpcSuspicious => get_npc_suspicious_pixels(),
        TileType::RoadExit => get_road_exit_pixels(),
        TileType::DungeonWall => get_dungeon_wall_pixels(),
        TileType::DungeonFloor => get_dungeon_floor_pixels(),
        TileType::IronGateClosed => get_iron_gate_closed_pixels(),
        TileType::IronGateOpen => get_iron_gate_open_pixels(),
        TileType::ChestClosed => get_chest_closed_pixels(),
        TileType::ChestOpen => get_chest_open_pixels(),
        TileType::StairsUp => get_stairs_up_pixels(),
        TileType::MonsterSymbol => get_monster_symbol_pixels(),
    }
}

/// 16×16スプライトをベースタイル上に透過合成 (alpha blend)
fn overlay_sprite(base: &mut [Pixel; 256], sprite: &[Pixel; 256]) {
    for i in 0..256 {
        let over = sprite[i];
        if over[3] > 0 {
            // アルファ値に応じたブレンド（完全不透明ならそのまま上書き）
            if over[3] == 255 {
                base[i] = over;
            } else {
                let a = over[3] as f32 / 255.0;
                let inv_a = 1.0 - a;
                base[i] = [
                    (over[0] as f32 * a + base[i][0] as f32 * inv_a) as u8,
                    (over[1] as f32 * a + base[i][1] as f32 * inv_a) as u8,
                    (over[2] as f32 * a + base[i][2] as f32 * inv_a) as u8,
                    255,
                ];
            }
        }
    }
}

/// 暗闇（FOV外）のシャドウ効果を適用（明度を落とし、夜の青黒さをブレンド）
fn apply_shadow(pixels: &mut [Pixel; 256]) {
    for p in pixels.iter_mut() {
        p[0] = (p[0] as f32 * 0.12) as u8;
        p[1] = (p[1] as f32 * 0.14) as u8;
        p[2] = ((p[2] as f32 * 0.22) + 15.0).min(255.0) as u8; // 夜の青みを少し残す
    }
}

/// パーティメンバーの職業や名前に応じたスプライトを取得する
pub fn get_member_sprite(member: &glyphfall_core::party::PartyMember) -> [Pixel; 256] {
    if member.is_player {
        return get_player_pixels();
    }
    match member.job.as_str() {
        "せんし" | "戦士" => get_warrior_pixels(),
        "あそびにん" | "遊び人" => get_slacker_pixels(),
        "まほうつかい" | "魔法使い" => get_mage_pixels(),
        "きし" | "騎士" => get_knight_pixels(),
        _ => match member.name.as_str() {
            n if n.contains("ガルツ") => get_warrior_pixels(),
            n if n.contains("ロロ") => get_slacker_pixels(),
            n if n.contains("ミレイ") => get_mage_pixels(),
            n if n.contains("アルヴィン") => get_knight_pixels(),
            _ => get_player_pixels(),
        },
    }
}

/// マップ全体（46×13タイル）を 736×208 ピクセルバッファに描画
pub fn render_town_to_texture(
    map: &TownMap,
    fov: &glyphfall_core::town::FovMap,
    player_pos: Position,
    followers: &FollowerHistory,
    party_members: &[glyphfall_core::party::PartyMember],
    buffer: &mut [u8],
) {
    let player_sprite = party_members
        .first()
        .map(get_member_sprite)
        .unwrap_or_else(get_player_pixels);

    let follower_sprites: Vec<[Pixel; 256]> = party_members
        .iter()
        .skip(1)
        .map(get_member_sprite)
        .collect();

    for ty in 0..MAP_HEIGHT_TILES as i32 {
        for tx in 0..MAP_WIDTH_TILES as i32 {
            let tile = map.get(tx, ty).unwrap_or(TileType::Wall);
            let mut tile_pixels = get_tile_pixels(tile);

            // キャラクター描画（主人公・仲間）
            if tx == player_pos.x && ty == player_pos.y {
                overlay_sprite(&mut tile_pixels, &player_sprite);
            } else {
                for (i, sprite) in follower_sprites.iter().enumerate() {
                    if let Some(p) = followers.get_follower_position(i) {
                        if tx == p.x && ty == p.y {
                            overlay_sprite(&mut tile_pixels, sprite);
                            break;
                        }
                    }
                }
            }

            // 視界制限シャドウ
            if !fov.is_visible(tx, ty) {
                apply_shadow(&mut tile_pixels);
            }

            // 736×208 バッファへのピクセル転送
            let base_px = tx as usize * TILE_SIZE;
            let base_py = ty as usize * TILE_SIZE;

            for py in 0..TILE_SIZE {
                for px in 0..TILE_SIZE {
                    let dest_x = base_px + px;
                    let dest_y = base_py + py;
                    let dest_index = (dest_y * TEXTURE_WIDTH + dest_x) * 4;
                    let src_pixel = tile_pixels[py * TILE_SIZE + px];

                    buffer[dest_index] = src_pixel[0];
                    buffer[dest_index + 1] = src_pixel[1];
                    buffer[dest_index + 2] = src_pixel[2];
                    buffer[dest_index + 3] = src_pixel[3];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_png_tilesets() {
        let exterior = image::open("assets/exterior.png").unwrap().to_rgba8();
        let dungeon = image::open("assets/dungeon.png").unwrap().to_rgba8();

        println!("=== DUNGEON TILE SAMPLES ===");
        for ty in 0..16 {
            for tx in 0..30 {
                let mut colors = Vec::new();
                for py in [4, 8, 12] {
                    for px in [4, 8, 12] {
                        colors.push(dungeon.get_pixel(tx * 16 + px, ty * 16 + py));
                    }
                }
                let avg_r: u32 = colors.iter().map(|p| p[0] as u32).sum::<u32>() / 9;
                let avg_g: u32 = colors.iter().map(|p| p[1] as u32).sum::<u32>() / 9;
                let avg_b: u32 = colors.iter().map(|p| p[2] as u32).sum::<u32>() / 9;
                let avg_a: u32 = colors.iter().map(|p| p[3] as u32).sum::<u32>() / 9;
                if avg_a > 100 {
                    print!("[{:02},{:02}:{:02x}{:02x}{:02x}] ", tx, ty, avg_r, avg_g, avg_b);
                } else {
                    print!("[{:02},{:02}:  none  ] ", tx, ty);
                }
            }
            println!();
        }
    }
}
