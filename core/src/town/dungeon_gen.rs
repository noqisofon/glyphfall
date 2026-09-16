//! ダンジョンのプロシージャル生成（ADR-0017）
//!
//! 生成アルゴリズムは`DungeonGenKind`で切り替える構造にしている。現時点では
//! セルオートマトン法による洞窟風レイアウト（`Cave`）のみを実装しているが、
//! 将来的に部屋＋通路型など別アルゴリズムを追加する余地を残すため、
//! `generate`を単一のエントリポイントとし、具体的な生成処理は
//! アルゴリズムごとの関数に分離している。

use super::map::{TileType, TownMap};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::VecDeque;

/// ダンジョン自動生成のアルゴリズム種別
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonGenKind {
    /// セルオートマトン法による洞窟風レイアウト
    Cave,
}

/// 生成試行1回あたりの初期充填率（この割合で床マスの種をまく）。
/// RogueBasin等でよく紹介される45%は正方形に近い盤面向けの値で、本作の
/// マップは46×13という横に極端に細長い形状のため、そのままでは入口を含む
/// 連結領域が育ちにくく大半の試行が失敗することが実測で分かった
/// （45%・4回平滑化では`CAVE_MIN_FLOOR_TILES`到達率が実測3%程度しかなく、
/// 50回の再試行でも失敗しきる確率が無視できなかった）。62%まで引き上げると
/// 実測で8割以上の試行が単発で成功する。
const CAVE_FILL_PROB: f64 = 0.62;
/// セルオートマトンの平滑化を適用する回数
const CAVE_SMOOTH_ITERATIONS: usize = 4;
/// 入口を含む連結領域がこの床マス数を下回った場合は生成をやり直す
const CAVE_MIN_FLOOR_TILES: usize = 150;
/// 上記条件を満たすまで生成をやり直す最大回数
const CAVE_MAX_ATTEMPTS: usize = 50;

/// 指定領域にダンジョンを生成する。
///
/// `entrance`は生成後も必ず床マスとして残ることが保証される（呼び出し側は
/// 固定座標へのプレイヤー出現など、外部と契約している座標を安全に渡せる）。
pub fn generate<R: Rng>(
    kind: DungeonGenKind,
    width: usize,
    height: usize,
    entrance: (i32, i32),
    rng: &mut R,
) -> TownMap {
    match kind {
        DungeonGenKind::Cave => generate_cave(width, height, entrance, rng),
    }
}

fn generate_cave<R: Rng>(width: usize, height: usize, entrance: (i32, i32), rng: &mut R) -> TownMap {
    for _ in 0..CAVE_MAX_ATTEMPTS {
        let mut floor = random_fill(width, height, rng);

        for _ in 0..CAVE_SMOOTH_ITERATIONS {
            floor = smooth_step(&floor, width, height);
        }

        // 入口は生成結果によらず必ず床として扱う
        let entrance_idx = idx(width, entrance.0, entrance.1);
        floor[entrance_idx] = true;

        let region = flood_fill(&floor, width, height, entrance);
        if region.len() < CAVE_MIN_FLOOR_TILES {
            continue;
        }

        return build_map(width, height, &region, entrance, rng);
    }

    // 実用上まず到達しない保険。万一乱数が悪条件を引き続けた場合でも
    // 到達可能なダンジョンを返せるよう、最小限の一本道にフォールバックする。
    fallback_corridor(width, height, entrance)
}

fn idx(width: usize, x: i32, y: i32) -> usize {
    y as usize * width + x as usize
}

fn random_fill<R: Rng>(width: usize, height: usize, rng: &mut R) -> Vec<bool> {
    let mut floor = vec![false; width * height];
    for y in 1..height as i32 - 1 {
        for x in 1..width as i32 - 1 {
            floor[idx(width, x, y)] = rng.gen_bool(CAVE_FILL_PROB);
        }
    }
    floor
}

/// セルオートマトンの平滑化を1ステップ適用する（RogueBasin方式の4-5ルール）。
/// 周囲8マス中の壁マス数が5以上なら壁化、3以下なら床化、4ならそのまま。
fn smooth_step(floor: &[bool], width: usize, height: usize) -> Vec<bool> {
    let mut next = floor.to_vec();
    for y in 1..height as i32 - 1 {
        for x in 1..width as i32 - 1 {
            let wall_neighbors = neighbors8(x, y)
                .iter()
                .filter(|&&(nx, ny)| !is_floor(floor, width, height, nx, ny))
                .count();

            next[idx(width, x, y)] = if wall_neighbors >= 5 {
                false
            } else if wall_neighbors <= 3 {
                true
            } else {
                floor[idx(width, x, y)]
            };
        }
    }
    next
}

fn neighbors8(x: i32, y: i32) -> [(i32, i32); 8] {
    [
        (x - 1, y - 1), (x, y - 1), (x + 1, y - 1),
        (x - 1, y),                 (x + 1, y),
        (x - 1, y + 1), (x, y + 1), (x + 1, y + 1),
    ]
}

fn neighbors4(x: i32, y: i32) -> [(i32, i32); 4] {
    [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]
}

fn is_floor(floor: &[bool], width: usize, height: usize, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
        false // 盤面外は壁として扱う
    } else {
        floor[idx(width, x, y)]
    }
}

/// `start`から4方向連結でたどれる床マスの座標一覧を返す
fn flood_fill(floor: &[bool], width: usize, height: usize, start: (i32, i32)) -> Vec<(i32, i32)> {
    let mut visited = vec![false; width * height];
    let mut queue = VecDeque::new();
    let mut region = Vec::new();

    visited[idx(width, start.0, start.1)] = true;
    queue.push_back(start);

    while let Some((x, y)) = queue.pop_front() {
        region.push((x, y));
        for (nx, ny) in neighbors4(x, y) {
            if !is_floor(floor, width, height, nx, ny) {
                continue;
            }
            let ni = idx(width, nx, ny);
            if !visited[ni] {
                visited[ni] = true;
                queue.push_back((nx, ny));
            }
        }
    }

    region
}

/// `start`から連結領域内の各マスへの最短距離（マス数）をBFSで求める。
/// `HashMap`の反復順序は実行ごとに変わり得るため、同じシード値からの生成が
/// 常に同じ結果になるよう、距離は座標を添字にした`Vec`（走査順は行優先で固定）
/// に格納する。
fn bfs_distances(
    region: &[(i32, i32)],
    width: usize,
    height: usize,
    start: (i32, i32),
) -> Vec<((i32, i32), usize)> {
    let region_set: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();

    let mut dist = vec![usize::MAX; width * height];
    let mut queue = VecDeque::new();
    dist[idx(width, start.0, start.1)] = 0;
    queue.push_back(start);

    while let Some((x, y)) = queue.pop_front() {
        let d = dist[idx(width, x, y)];
        for (nx, ny) in neighbors4(x, y) {
            if !region_set.contains(&(nx, ny)) {
                continue;
            }
            let ni = idx(width, nx, ny);
            if dist[ni] == usize::MAX {
                dist[ni] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }

    // 行優先の固定順で返すことで、最遠点選択時のタイブレークも決定的になる。
    region.iter().map(|&p| (p, dist[idx(width, p.0, p.1)])).collect()
}

fn build_map<R: Rng>(
    width: usize,
    height: usize,
    region: &[(i32, i32)],
    entrance: (i32, i32),
    rng: &mut R,
) -> TownMap {
    let mut map = TownMap::new(width, height, TileType::DungeonWall);
    for &(x, y) in region {
        map.set(x, y, TileType::DungeonFloor);
    }

    // 入口の隣に地上へ戻る階段を置く（隣接マスが領域になければ入口自体を階段にする）
    let region_set: std::collections::HashSet<(i32, i32)> = region.iter().copied().collect();
    let stairs_up = neighbors4(entrance.0, entrance.1)
        .into_iter()
        .find(|p| region_set.contains(p))
        .unwrap_or(entrance);
    map.set(stairs_up.0, stairs_up.1, TileType::StairsUp);

    // 入口から最も離れたマスに下り階段（次の深層への入り口）を置く
    let distances = bfs_distances(region, width, height, entrance);
    let exit = distances
        .iter()
        .max_by_key(|(_, d)| *d)
        .map(|(p, _)| *p)
        .unwrap_or(entrance);
    map.set(exit.0, exit.1, TileType::StairsDown);

    // 宝箱・魔物を残りの床マスからランダムに配置する。
    // ChestClosed/MonsterSymbolは`TileType::is_walkable`がfalseを返す（Zコマンドで
    // 調べる／ぶつかって戦闘に入るタイルであり、踏み越えては進めない）ため、
    // 隘路に置くと入口からその先の区画へ実際には歩いて到達できなくなる恐れがある。
    // 1個置くごとに「入口から歩いて到達できるマスの総数」を比較し、置いたタイル
    // 自身の1マス分を超えて減っていたら（＝他のマスを巻き添えで塞いでいたら）
    // 取り消す。これにより連結領域内のどのマスも、宝箱・魔物の配置によって
    // 到達不能になることはない。
    let mut candidates: Vec<(i32, i32)> = region
        .iter()
        .copied()
        .filter(|&p| p != entrance && p != stairs_up && p != exit)
        .collect();
    candidates.shuffle(rng);

    let mut reachable_before = walkable_region(&map, entrance).len();
    let mut placed_chests = 0;
    let mut placed_monster = false;
    for (x, y) in candidates {
        if placed_chests >= 2 && placed_monster {
            break;
        }
        let tile = if placed_chests < 2 {
            TileType::ChestClosed
        } else {
            TileType::MonsterSymbol
        };
        map.set(x, y, tile);

        let reachable_after = walkable_region(&map, entrance).len();
        if reachable_before <= reachable_after + 1 {
            reachable_before = reachable_after;
            if placed_chests < 2 {
                placed_chests += 1;
            } else {
                placed_monster = true;
            }
        } else {
            map.set(x, y, TileType::DungeonFloor); // 他のマスを巻き添えで塞ぐ配置だったので取り消す
        }
    }

    map
}

/// `start`から実際に歩行可能なタイル（`TileType::is_walkable`）のみをたどって
/// 到達できるマスの集合を求める。宝箱・魔物シンボルは歩行不可のため、
/// 連結領域（床マスの集合）に含まれていてもこの意味では通れないことがある。
fn walkable_region(map: &TownMap, start: (i32, i32)) -> std::collections::HashSet<(i32, i32)> {
    let mut visited = std::collections::HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start);
    queue.push_back(start);

    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in neighbors4(x, y) {
            if visited.contains(&(nx, ny)) {
                continue;
            }
            if map.get(nx, ny).map(|t| t.is_walkable()).unwrap_or(false) {
                visited.insert((nx, ny));
                queue.push_back((nx, ny));
            }
        }
    }

    visited
}

/// セルオートマトンが規定回数内に十分な広さの洞窟を作れなかった場合の保険。
/// 入口から東へ一直線に掘っただけの最小限の通路を返す（到達可能性のみ保証する）。
fn fallback_corridor(width: usize, height: usize, entrance: (i32, i32)) -> TownMap {
    let mut map = TownMap::new(width, height, TileType::DungeonWall);
    let y = entrance.1;
    let end_x = width as i32 - 2;
    for x in entrance.0..=end_x {
        map.set(x, y, TileType::DungeonFloor);
    }
    // 入口の1マス西に上り階段を置く。入口が盤面の縁に近く西側が取れない場合は
    // 東側に置く（外周の壁マスを上書きしてしまわないようにするため）。
    let stairs_up_x = if entrance.0 > 1 {
        entrance.0 - 1
    } else {
        entrance.0 + 1
    };
    map.set(stairs_up_x, y, TileType::StairsUp);
    map.set(end_x, y, TileType::StairsDown);
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_cave_generation_is_connected_across_many_seeds() {
        for seed in 0..30u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let entrance = (3, 4);
            let map = generate(DungeonGenKind::Cave, 46, 13, entrance, &mut rng);

            assert_eq!(
                map.get(entrance.0, entrance.1),
                Some(TileType::DungeonFloor),
                "seed {seed}: entrance must be floor"
            );

            // `reachable`は`TileType::is_walkable`基準の到達数であり、通行不可の
            // 宝箱2個・魔物シンボル1個ぶんだけ`CAVE_MIN_FLOOR_TILES`（連結した
            // 床マス数の下限）より少なくなり得る。この3マス分だけ許容する
            // （それ以上減っていないことは`build_map`側の配置ガードで保証している）。
            let reachable = walkable_region(&map, entrance);
            assert!(
                reachable.len() + 3 >= CAVE_MIN_FLOOR_TILES,
                "seed {seed}: reachable area too small ({}, expected at least {})",
                reachable.len(),
                CAVE_MIN_FLOOR_TILES - 3
            );

            let mut has_stairs_up = false;
            let mut stairs_down_pos = None;
            for y in 0..map.height as i32 {
                for x in 0..map.width as i32 {
                    match map.get(x, y) {
                        Some(TileType::StairsUp) => has_stairs_up = true,
                        Some(TileType::StairsDown) => stairs_down_pos = Some((x, y)),
                        _ => {}
                    }
                }
            }
            assert!(has_stairs_up, "seed {seed}: missing stairs up");
            let stairs_down_pos =
                stairs_down_pos.unwrap_or_else(|| panic!("seed {seed}: missing stairs down"));

            // 実際にゲーム中の移動判定が使う`is_walkable`基準で、入口から下り階段まで
            // 歩いてたどり着けることを検証する（宝箱・魔物シンボルの配置が通路を
            // 塞いでいないことの直接的な保証）。
            assert!(
                reachable.contains(&stairs_down_pos),
                "seed {seed}: stairs down is not walkably reachable from entrance"
            );
        }
    }

    #[test]
    fn test_cave_generation_is_deterministic_for_same_seed() {
        let mut rng_a = StdRng::seed_from_u64(42);
        let mut rng_b = StdRng::seed_from_u64(42);
        let entrance = (3, 4);

        let map_a = generate(DungeonGenKind::Cave, 46, 13, entrance, &mut rng_a);
        let map_b = generate(DungeonGenKind::Cave, 46, 13, entrance, &mut rng_b);

        assert_eq!(map_a.tiles.len(), map_b.tiles.len());
        for (a, b) in map_a.tiles.iter().zip(map_b.tiles.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_cave_generation_varies_between_seeds() {
        let mut rng_a = StdRng::seed_from_u64(1);
        let mut rng_b = StdRng::seed_from_u64(2);
        let entrance = (3, 4);

        let map_a = generate(DungeonGenKind::Cave, 46, 13, entrance, &mut rng_a);
        let map_b = generate(DungeonGenKind::Cave, 46, 13, entrance, &mut rng_b);

        let differs = map_a
            .tiles
            .iter()
            .zip(map_b.tiles.iter())
            .any(|(a, b)| a != b);
        assert!(differs, "different seeds should not produce an identical cave");
    }
}
