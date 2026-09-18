//! 街・ダンジョンマップのタイルセット描画およびピクセルアートレンダリング
//!
//! ツクール2000規格のチップセット画像（480×256px、16×16タイル）および
//! キャラクタースプライトシート（32×32グリッド）をコンパイル時埋め込みで
//! ロードし、マップ全体のテクスチャ（736×208px）へ描画する。

use glyphfall_core::town::{AreaId, FollowerHistory, Position, TileType, TownMap};
use std::sync::OnceLock;

pub const TILE_SIZE: usize = 16;
pub const MAP_WIDTH_TILES: usize = 46;
pub const MAP_HEIGHT_TILES: usize = 13;

pub const TEXTURE_WIDTH: usize = MAP_WIDTH_TILES * TILE_SIZE; // 736
pub const TEXTURE_HEIGHT: usize = MAP_HEIGHT_TILES * TILE_SIZE; // 208

type Pixel = [u8; 4];

pub struct Tileset {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
}

impl Tileset {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let img = image::load_from_memory(bytes)
            .expect("Failed to decode embedded tileset PNG")
            .to_rgba8();
        let width = img.width() as usize;
        let height = img.height() as usize;
        let mut rgba = img.into_raw();
        // ツクール2000規格の透過指定色 (RGB: 255, 103, 139) を完全透明に変換
        for chunk in rgba.as_chunks_mut::<4>().0 {
            if chunk[0] == 255 && chunk[1] == 103 && chunk[2] == 139 {
                chunk[0] = 0;
                chunk[1] = 0;
                chunk[2] = 0;
                chunk[3] = 0;
            }
        }
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn get_tile(&self, tx: usize, ty: usize) -> [Pixel; 256] {
        let mut pixels = [[0, 0, 0, 0]; 256];
        let base_x = tx * TILE_SIZE;
        let base_y = ty * TILE_SIZE;
        for py in 0..TILE_SIZE {
            for px in 0..TILE_SIZE {
                let x = base_x + px;
                let y = base_y + py;
                if x < self.width && y < self.height {
                    let idx = (y * self.width + x) * 4;
                    pixels[py * TILE_SIZE + px] = [
                        self.rgba[idx],
                        self.rgba[idx + 1],
                        self.rgba[idx + 2],
                        self.rgba[idx + 3],
                    ];
                }
            }
        }
        pixels
    }
}

pub struct TilesetAtlas {
    pub exterior: Tileset,
    pub dungeon: Tileset,
    #[allow(dead_code)]
    pub interior: Tileset,
    #[allow(dead_code)]
    pub ship: Tileset,
    #[allow(dead_code)]
    pub world: Tileset,
}

impl TilesetAtlas {
    pub fn global() -> &'static TilesetAtlas {
        static ATLAS: OnceLock<TilesetAtlas> = OnceLock::new();
        ATLAS.get_or_init(|| TilesetAtlas {
            exterior: Tileset::from_bytes(include_bytes!("../../assets/exterior.png")),
            dungeon: Tileset::from_bytes(include_bytes!("../../assets/dungeon.png")),
            interior: Tileset::from_bytes(include_bytes!("../../assets/interior.png")),
            ship: Tileset::from_bytes(include_bytes!("../../assets/ship.png")),
            world: Tileset::from_bytes(include_bytes!("../../assets/world.png")),
        })
    }
}

/// 32×32グリッドのキャラクタースプライトシート
pub struct SpriteSheet {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
}

impl SpriteSheet {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let img = image::load_from_memory(bytes)
            .expect("Failed to decode embedded sprite PNG")
            .to_rgba8();
        Self {
            width: img.width() as usize,
            height: img.height() as usize,
            rgba: img.into_raw(),
        }
    }

    /// 32×32グリッドの (col, row) から中央の 16×16 ピクセル（オフセット +8, +8）を切り出す
    pub fn get_sprite(&self, col: usize, row: usize) -> [Pixel; 256] {
        let mut pixels = [[0, 0, 0, 0]; 256];
        let base_x = col * 32 + 8;
        let base_y = row * 32 + 8;
        for py in 0..16 {
            for px in 0..16 {
                let x = base_x + px;
                let y = base_y + py;
                if x < self.width && y < self.height {
                    let idx = (y * self.width + x) * 4;
                    pixels[py * TILE_SIZE + px] = [
                        self.rgba[idx],
                        self.rgba[idx + 1],
                        self.rgba[idx + 2],
                        self.rgba[idx + 3],
                    ];
                }
            }
        }
        pixels
    }
}

pub struct CharacterAtlas {
    pub player: SpriteSheet,
    pub warrior: SpriteSheet,
    pub slacker: SpriteSheet,
    pub mage: SpriteSheet,
    pub knight: SpriteSheet,
    pub guard: SpriteSheet,
    pub villager: SpriteSheet,
    pub suspicious: SpriteSheet,
    pub monster: SpriteSheet,
}

impl CharacterAtlas {
    pub fn global() -> &'static CharacterAtlas {
        static ATLAS: OnceLock<CharacterAtlas> = OnceLock::new();
        ATLAS.get_or_init(|| CharacterAtlas {
            player: SpriteSheet::from_bytes(include_bytes!("../../assets/Character-Base.png")),
            warrior: SpriteSheet::from_bytes(include_bytes!("../../assets/Warrior-Red.png")),
            slacker: SpriteSheet::from_bytes(include_bytes!("../../assets/Human-Worker-Cyan.png")),
            mage: SpriteSheet::from_bytes(include_bytes!("../../assets/Mage-Cyan.png")),
            knight: SpriteSheet::from_bytes(include_bytes!("../../assets/Human-Soldier-Cyan.png")),
            guard: SpriteSheet::from_bytes(include_bytes!("../../assets/Soldier-Blue.png")),
            villager: SpriteSheet::from_bytes(include_bytes!("../../assets/Human-Worker-Red.png")),
            suspicious: SpriteSheet::from_bytes(include_bytes!("../../assets/Orc-Peon-Cyan.png")),
            monster: SpriteSheet::from_bytes(include_bytes!("../../assets/Slime.png")),
        })
    }
}

// ─────────────────────────────────────────────
// タイル＆スプライトの合成レンダリング
// ─────────────────────────────────────────────

fn get_tile_pixels(tile: TileType, area: AreaId) -> [Pixel; 256] {
    let atlas = TilesetAtlas::global();
    let chars = CharacterAtlas::global();
    let is_dungeon = area.dungeon_depth().is_some();

    // ベースとなる床タイル（上層オブジェクトを配置する下地）
    let floor_tile = if is_dungeon {
        atlas.dungeon.get_tile(14, 14)
    } else if area == AreaId::Village {
        atlas.exterior.get_tile(7, 0)
    } else {
        atlas.exterior.get_tile(7, 10) // 王都の石畳
    };

    match tile {
        TileType::Floor => floor_tile,
        TileType::Wall => {
            if is_dungeon {
                atlas.dungeon.get_tile(12, 0)
            } else {
                atlas.exterior.get_tile(21, 2) // サンプル画像準拠の青紫城壁石レンガ
            }
        }
        TileType::Water => atlas.exterior.get_tile(1, 0),
        TileType::Tree => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(20, 8)); // 針葉樹
            base
        }
        TileType::DoorClosed => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(19, 5));
            base
        }
        TileType::DoorOpen => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(19, 6));
            base
        }
        TileType::Sign => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(20, 9));
            base
        }
        TileType::Inn => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(22, 13));
            base
        }
        TileType::Tavern => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(28, 4));
            base
        }
        TileType::Shop => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(23, 13));
            base
        }
        TileType::StairsDown => {
            let mut base = floor_tile;
            let spr = if is_dungeon {
                atlas.dungeon.get_tile(19, 2)
            } else {
                atlas.exterior.get_tile(18, 1)
            };
            overlay_sprite(&mut base, &spr);
            base
        }
        TileType::StairsUp => {
            let mut base = floor_tile;
            let spr = if is_dungeon {
                atlas.dungeon.get_tile(18, 2)
            } else {
                atlas.exterior.get_tile(18, 0)
            };
            overlay_sprite(&mut base, &spr);
            base
        }
        TileType::RoadExit => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.exterior.get_tile(0, 12));
            base
        }
        TileType::DungeonWall => atlas.dungeon.get_tile(12, 0),
        TileType::DungeonFloor => atlas.dungeon.get_tile(14, 14),
        TileType::IronGateClosed => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.dungeon.get_tile(26, 1));
            base
        }
        TileType::IronGateOpen => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.dungeon.get_tile(26, 2));
            base
        }
        TileType::ChestClosed => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.dungeon.get_tile(27, 4));
            base
        }
        TileType::ChestOpen => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &atlas.dungeon.get_tile(28, 4));
            base
        }

        TileType::NpcGuard => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &chars.guard.get_sprite(0, 0));
            base
        }
        TileType::NpcVillager => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &chars.villager.get_sprite(0, 0));
            base
        }
        TileType::NpcSuspicious => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &chars.suspicious.get_sprite(0, 0));
            base
        }
        TileType::MonsterSymbol => {
            let mut base = floor_tile;
            overlay_sprite(&mut base, &chars.monster.get_sprite(0, 0));
            base
        }
    }
}

/// 16×16スプライトをベースタイル上に透過合成 (alpha blend)
pub fn overlay_sprite(base: &mut [Pixel; 256], sprite: &[Pixel; 256]) {
    for i in 0..256 {
        let over = sprite[i];
        if over[3] > 0 {
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
    let chars = CharacterAtlas::global();
    if member.is_player {
        return chars.player.get_sprite(0, 0);
    }
    match member.job.as_str() {
        "せんし" | "戦士" => chars.warrior.get_sprite(0, 0),
        "あそびにん" | "遊び人" => chars.slacker.get_sprite(0, 0),
        "まほうつかい" | "魔法使い" => chars.mage.get_sprite(0, 0),
        "きし" | "騎士" => chars.knight.get_sprite(0, 0),
        _ => match member.name.as_str() {
            n if n.contains("ガルツ") => chars.warrior.get_sprite(0, 0),
            n if n.contains("ロロ") => chars.slacker.get_sprite(0, 0),
            n if n.contains("ミレイ") => chars.mage.get_sprite(0, 0),
            n if n.contains("アルヴィン") => chars.knight.get_sprite(0, 0),
            _ => chars.player.get_sprite(0, 0),
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
    area: AreaId,
    buffer: &mut [u8],
) {
    let player_sprite = party_members
        .first()
        .map(get_member_sprite)
        .unwrap_or_else(|| CharacterAtlas::global().player.get_sprite(0, 0));

    let follower_sprites: Vec<[Pixel; 256]> = party_members
        .iter()
        .skip(1)
        .map(get_member_sprite)
        .collect();

    for ty in 0..MAP_HEIGHT_TILES as i32 {
        for tx in 0..MAP_WIDTH_TILES as i32 {
            let tile = map.get(tx, ty).unwrap_or(TileType::Wall);
            let mut tile_pixels = get_tile_pixels(tile, area);

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
    fn test_tileset_atlas_loading() {
        let atlas = TilesetAtlas::global();
        assert_eq!(atlas.exterior.width, 480);
        assert_eq!(atlas.exterior.height, 256);
        assert_eq!(atlas.dungeon.width, 480);
        assert_eq!(atlas.dungeon.height, 256);
        assert_eq!(atlas.interior.width, 480);
        assert_eq!(atlas.interior.height, 256);
    }

    #[test]
    fn test_tile_transparency() {
        let atlas = TilesetAtlas::global();
        // 木のタイルの左上ピクセルは背景色なので透明 [0,0,0,0] になっているはず
        let tree = atlas.exterior.get_tile(20, 8);
        assert_eq!(tree[0][3], 0);

        // 石畳は不透明
        let floor = atlas.exterior.get_tile(7, 10);
        assert_eq!(floor[0][3], 255);
    }

    #[test]
    fn test_character_atlas_loading() {
        let chars = CharacterAtlas::global();
        let p = chars.player.get_sprite(0, 0);
        assert_eq!(p.len(), 256);
        // 主人公スプライトに不透明ピクセルが存在すること
        assert!(p.iter().any(|px| px[3] > 0));

        let slime = chars.monster.get_sprite(0, 0);
        assert!(slime.iter().any(|px| px[3] > 0));
    }

    #[test]
    fn test_render_town_to_texture() {
        let town = glyphfall_core::town::TownState::new();
        let mut buffer = vec![0u8; TEXTURE_WIDTH * TEXTURE_HEIGHT * 4];

        render_town_to_texture(
            &town.map,
            &town.fov,
            town.player_pos,
            &town.followers,
            &[],
            AreaId::Town,
            &mut buffer,
        );

        assert!(buffer.iter().any(|&b| b > 0));
    }

    #[test]
    fn test_render_dungeon_to_texture() {
        let mut town = glyphfall_core::town::TownState::new();
        town.switch_area(
            AreaId::DungeonB1F,
            Position { x: 5, y: 5 },
            &mut rand::thread_rng(),
        );
        let mut buffer = vec![0u8; TEXTURE_WIDTH * TEXTURE_HEIGHT * 4];

        render_town_to_texture(
            &town.map,
            &town.fov,
            town.player_pos,
            &town.followers,
            &[],
            AreaId::DungeonB1F,
            &mut buffer,
        );

        assert!(buffer.iter().any(|&b| b > 0));
    }
}

