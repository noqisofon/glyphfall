//! ダンジョンのプロシージャル生成（ADR-0017 / ADR-0018）
//!
//! 生成アルゴリズムは`DungeonGenKind`で切り替える構造にしている。現時点では
//! セルオートマトン法による洞窟風レイアウト（`Cave`）のみを実装しているが、
//! 将来的に部屋＋通路型など別アルゴリズムを追加する余地を残すため、
//! `generate`を単一のエントリポイントとし、具体的な生成処理は
//! アルゴリズムごとの関数に分離している。
//!
//! `Cave`生成には、ADR-0017で完全ランダム化した際に失った手作りゾーニング
//! （宝物庫・礼拝堂・牢獄区画）を、確率的な「物語テンプレート部屋」として
//! 部分的に復活させるハイブリッド処理（ADR-0018）が入っている。

use super::map::{TileType, TownMap};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::{HashSet, VecDeque};

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

/// 物語テンプレート部屋（宝物庫・礼拝堂・牢獄区画、ADR-0018）の一辺の長さ
const TEMPLATE_ROOM_SIZE: i32 = 5;
/// テンプレート矩形の中心から左上までのオフセット（`TEMPLATE_ROOM_SIZE`が奇数である前提）
const TEMPLATE_ROOM_HALF: i32 = TEMPLATE_ROOM_SIZE / 2;
/// テンプレート部屋を2個出す場合に要求する、入口からのBFS距離差の下限
const TEMPLATE_MIN_DISTANCE_GAP: usize = 5;

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
    let region_set: HashSet<(i32, i32)> = region.iter().copied().collect();

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
    let region_set: HashSet<(i32, i32)> = region.iter().copied().collect();
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

    let mut reachable_before = walkable_region(&map, entrance).len();

    // ADR-0018: ADR-0017で捨てた手作りゾーニング（宝物庫・礼拝堂・牢獄区画）を、
    // 完全ランダムな洞窟生成とのハイブリッドで部分的に復活させる。入口・上り階段・
    // 下り階段の3マスと重ならない範囲で、入口から遠い側の座標にテンプレート部屋を
    // 上書きする。詳細はADR-0018を参照。
    let reserved: HashSet<(i32, i32)> = [entrance, stairs_up, exit].into_iter().collect();
    let mut sorted_by_dist = distances.clone();
    // `sort_by_key`は安定ソートであり、`distances`自体が`bfs_distances`のコメントの通り
    // 行優先の決定的な順序で並んでいるため、距離が同点の場合のタイブレークも
    // 乱数シードだけに依存する決定的な結果になる。
    sorted_by_dist.sort_by_key(|a| std::cmp::Reverse(a.1));

    let template_count = choose_template_room_count(rng);
    let template_slots =
        find_template_slots(&sorted_by_dist, &region_set, &reserved, template_count);

    let mut template_used: HashSet<(i32, i32)> = HashSet::new();
    for top_left in template_slots {
        template_used.extend(apply_template_room(
            &mut map,
            entrance,
            &mut reachable_before,
            top_left,
            rng,
        ));
    }

    // 宝箱・魔物を残りの床マスからランダムに配置する。
    // ChestClosed/MonsterSymbolは`TileType::is_walkable`がfalseを返す（Zコマンドで
    // 調べる／ぶつかって戦闘に入るタイルであり、踏み越えては進めない）ため、
    // 隘路に置くと入口からその先の区画へ実際には歩いて到達できなくなる恐れがある。
    // 1個置くごとに「入口から歩いて到達できるマスの総数」を比較し、置いたタイル
    // 自身の1マス分を超えて減っていたら（＝他のマスを巻き添えで塞いでいたら）
    // 取り消す。これにより連結領域内のどのマスも、宝箱・魔物の配置によって
    // 到達不能になることはない。テンプレート部屋が使ったマスは座標の重複を
    // 避けるためここでは除外する。
    let mut candidates: Vec<(i32, i32)> = region
        .iter()
        .copied()
        .filter(|&p| p != entrance && p != stairs_up && p != exit && !template_used.contains(&p))
        .collect();
    candidates.shuffle(rng);

    let mut placed_chests = 0;
    let mut placed_monster = false;
    for pos in candidates {
        if placed_chests >= 2 && placed_monster {
            break;
        }
        let tile = if placed_chests < 2 {
            TileType::ChestClosed
        } else {
            TileType::MonsterSymbol
        };
        if try_place_tile(&mut map, entrance, &mut reachable_before, pos, tile) {
            if placed_chests < 2 {
                placed_chests += 1;
            } else {
                placed_monster = true;
            }
        }
    }

    map
}

/// 到達可能性を壊さない範囲でタイルを1枚配置する。
///
/// `TileType::is_walkable`がfalseのタイル（宝箱・魔物シンボル等）は踏み越えて
/// 通過できないため、隘路に置くと他のマスへの経路を塞ぎかねない。配置前後で
/// 「入口から歩いて到達できるマスの総数」を比較し、置いたタイル自身の1マス分を
/// 超えて減っていれば（＝他のマスを巻き添えにしていれば）配置を取り消して床に戻す。
/// 戻り値は配置できたかどうか。
fn try_place_tile(
    map: &mut TownMap,
    entrance: (i32, i32),
    reachable_before: &mut usize,
    pos: (i32, i32),
    tile: TileType,
) -> bool {
    map.set(pos.0, pos.1, tile);
    let reachable_after = walkable_region(map, entrance).len();
    if *reachable_before <= reachable_after + 1 {
        *reachable_before = reachable_after;
        true
    } else {
        map.set(pos.0, pos.1, TileType::DungeonFloor);
        false
    }
}

/// 物語テンプレート部屋の種類（ADR-0018）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TemplateRoomKind {
    /// 宝物庫：中央付近を宝箱で固める
    Treasury,
    /// 礼拝堂：装飾を置かない静かな小部屋
    Chapel,
    /// 牢獄区画：見張り役の魔物シンボルをまとめて配置する
    Prison,
}

/// 0個(25%)/1個(60%)/2個(15%)の重み付き乱数で、今回出すテンプレート部屋の個数を決める
fn choose_template_room_count<R: Rng>(rng: &mut R) -> usize {
    let roll = rng.gen_range(0..100u32);
    if roll < 25 {
        0
    } else if roll < 85 {
        1
    } else {
        2
    }
}

fn random_template_kind<R: Rng>(rng: &mut R) -> TemplateRoomKind {
    match rng.gen_range(0..3u32) {
        0 => TemplateRoomKind::Treasury,
        1 => TemplateRoomKind::Chapel,
        _ => TemplateRoomKind::Prison,
    }
}

/// テンプレート部屋の矩形（`TEMPLATE_ROOM_SIZE`四方）内の局所座標(0..size, 0..size)を列挙する
fn template_rect_cells(top_left: (i32, i32)) -> impl Iterator<Item = (i32, i32)> {
    (0..TEMPLATE_ROOM_SIZE).flat_map(move |dy| {
        (0..TEMPLATE_ROOM_SIZE).map(move |dx| (top_left.0 + dx, top_left.1 + dy))
    })
}

/// テンプレート部屋の矩形が、`region_set`に全て収まり、かつ`forbidden`（入口・上り
/// 階段・下り階段や既存のテンプレート）と重ならないかどうかを判定する
fn template_rect_fits(
    region_set: &HashSet<(i32, i32)>,
    forbidden: &HashSet<(i32, i32)>,
    top_left: (i32, i32),
) -> bool {
    template_rect_cells(top_left).all(|p| region_set.contains(&p) && !forbidden.contains(&p))
}

fn template_rects_overlap(a: (i32, i32), b: (i32, i32)) -> bool {
    let a_max = (a.0 + TEMPLATE_ROOM_SIZE - 1, a.1 + TEMPLATE_ROOM_SIZE - 1);
    let b_max = (b.0 + TEMPLATE_ROOM_SIZE - 1, b.1 + TEMPLATE_ROOM_SIZE - 1);
    a.0 <= b_max.0 && b.0 <= a_max.0 && a.1 <= b_max.1 && b.1 <= a_max.1
}

/// `sorted_by_dist`（入口からの距離で降順ソート済み）を遠い側から順に走査し、
/// テンプレート部屋が収まる矩形の左上座標を`count`個まで探す。
/// 2個目を探す場合は、1個目の矩形と重ならず、かつ距離が`TEMPLATE_MIN_DISTANCE_GAP`
/// 以上離れている座標のみを採用する。条件を満たす座標が見つからなければ、
/// その分は無理に置かず通常のランダム配置に回す（返すVecの要素数がcountを下回る）。
fn find_template_slots(
    sorted_by_dist: &[((i32, i32), usize)],
    region_set: &HashSet<(i32, i32)>,
    forbidden: &HashSet<(i32, i32)>,
    count: usize,
) -> Vec<(i32, i32)> {
    let mut chosen: Vec<((i32, i32), usize)> = Vec::new();
    if count == 0 {
        return Vec::new();
    }

    for &(center, dist) in sorted_by_dist {
        let top_left = (center.0 - TEMPLATE_ROOM_HALF, center.1 - TEMPLATE_ROOM_HALF);
        if template_rect_fits(region_set, forbidden, top_left) {
            chosen.push((top_left, dist));
            break;
        }
    }

    if count >= 2 && !chosen.is_empty() {
        let (first_rect, first_dist) = chosen[0];
        for &(center, dist) in sorted_by_dist {
            if first_dist.abs_diff(dist) < TEMPLATE_MIN_DISTANCE_GAP {
                continue;
            }
            let top_left = (center.0 - TEMPLATE_ROOM_HALF, center.1 - TEMPLATE_ROOM_HALF);
            if template_rects_overlap(first_rect, top_left) {
                continue;
            }
            if template_rect_fits(region_set, forbidden, top_left) {
                chosen.push((top_left, dist));
                break;
            }
        }
    }

    chosen.into_iter().map(|(rect, _)| rect).collect()
}

/// テンプレート部屋の種類ごとの局所タイル配置（局所座標は0..TEMPLATE_ROOM_SIZE四方）
fn template_layout(kind: TemplateRoomKind) -> Vec<(i32, i32, TileType)> {
    match kind {
        // 宝物庫：中央3x3を宝箱で固め、外周1マスは床のまま残す
        TemplateRoomKind::Treasury => {
            let mut tiles = Vec::new();
            for dy in 1..=3 {
                for dx in 1..=3 {
                    tiles.push((dx, dy, TileType::ChestClosed));
                }
            }
            tiles
        }
        // 礼拝堂：何もない静かな小部屋（床のまま）
        TemplateRoomKind::Chapel => Vec::new(),
        // 牢獄区画：四隅寄りに見張りの魔物シンボルをまとめて配置する
        TemplateRoomKind::Prison => vec![
            (1, 1, TileType::MonsterSymbol),
            (3, 1, TileType::MonsterSymbol),
            (1, 3, TileType::MonsterSymbol),
            (3, 3, TileType::MonsterSymbol),
        ],
    }
}

/// `top_left`を左上とするテンプレート部屋を1つ、ランダムに選んだ種類で上書きする。
/// 到達不能化を防ぐため、各タイルの配置は`try_place_tile`で1枚ずつガードする
/// （置くと他のマスを巻き添えで塞ぐ場合は床のまま残る）。戻り値は矩形全体の座標
/// 集合で、実際に特殊タイルを置けたかによらず、以後のランダム配置から除外するために使う。
fn apply_template_room<R: Rng>(
    map: &mut TownMap,
    entrance: (i32, i32),
    reachable_before: &mut usize,
    top_left: (i32, i32),
    rng: &mut R,
) -> HashSet<(i32, i32)> {
    let used: HashSet<(i32, i32)> = template_rect_cells(top_left).collect();

    let kind = random_template_kind(rng);
    for (dx, dy, tile) in template_layout(kind) {
        let pos = (top_left.0 + dx, top_left.1 + dy);
        try_place_tile(map, entrance, reachable_before, pos, tile);
    }

    used
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
        for seed in 0..200u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let entrance = (3, 4);
            let map = generate(DungeonGenKind::Cave, 46, 13, entrance, &mut rng);

            assert_eq!(
                map.get(entrance.0, entrance.1),
                Some(TileType::DungeonFloor),
                "seed {seed}: entrance must be floor"
            );

            // `reachable`は`TileType::is_walkable`基準の到達数であり、通行不可の
            // 宝箱・魔物シンボル（通常のランダム配置に加え、ADR-0018のテンプレート
            // 部屋が出た場合はその分も）だけ`CAVE_MIN_FLOOR_TILES`（連結した床マス数
            // の下限）より少なくなり得る。実際に置かれた非歩行タイルの枚数を数えて
            // 差し引くことで、テンプレート部屋の有無によらず検証できるようにする
            // （それ以上減っていないことは`try_place_tile`の配置ガードで保証している）。
            let decoration_count = (0..map.height as i32)
                .flat_map(|y| (0..map.width as i32).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    matches!(
                        map.get(x, y),
                        Some(TileType::ChestClosed) | Some(TileType::MonsterSymbol)
                    )
                })
                .count();
            let reachable = walkable_region(&map, entrance);
            assert!(
                reachable.len() + decoration_count >= CAVE_MIN_FLOOR_TILES,
                "seed {seed}: reachable area too small ({}, {} decorations, expected at least {})",
                reachable.len(),
                decoration_count,
                CAVE_MIN_FLOOR_TILES,
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

    /// ADR-0018: テンプレート部屋の個数抽選が想定の重み
    /// （0個25%/1個60%/2個15%）にほぼ従っていることを、多数のシードで確認する。
    #[test]
    fn test_template_room_count_matches_expected_distribution() {
        const TRIALS: u64 = 200_000;
        let mut counts = [0u64; 3];
        for seed in 0..TRIALS {
            let mut rng = StdRng::seed_from_u64(seed);
            let count = choose_template_room_count(&mut rng);
            counts[count] += 1;
        }

        let ratios: Vec<f64> = counts.iter().map(|&c| c as f64 / TRIALS as f64).collect();
        // サンプル数20万に対する許容誤差2ポイント（多少の乱数バイアスは許容しつつ、
        // 想定分布からの実装ミスは検出できる幅）。
        const TOLERANCE: f64 = 0.02;
        let expected = [0.25, 0.60, 0.15];
        for (i, (&actual, &exp)) in ratios.iter().zip(expected.iter()).enumerate() {
            assert!(
                (actual - exp).abs() <= TOLERANCE,
                "template count {i}: expected ~{exp}, got {actual} (counts={counts:?})"
            );
        }
    }

    /// ADR-0018: テンプレート部屋を2個選ぶ場合、多数の実際の洞窟形状に対して
    /// 選ばれた2つの矩形が座標的に重ならないこと、かつ`build_map`が実際に使う
    /// `reserved`（入口・上り階段・下り階段）とも重ならないことを確認する。
    #[test]
    fn test_template_slots_never_overlap_across_many_cave_shapes() {
        let width = 46;
        let height = 13;
        let entrance = (3, 4);
        let mut checked_two_slots = 0;

        for seed in 0..500u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut floor = random_fill(width, height, &mut rng);
            for _ in 0..CAVE_SMOOTH_ITERATIONS {
                floor = smooth_step(&floor, width, height);
            }
            floor[idx(width, entrance.0, entrance.1)] = true;

            let region = flood_fill(&floor, width, height, entrance);
            if region.len() < CAVE_MIN_FLOOR_TILES {
                continue; // このシードは通常のリトライ対象。テンプレート判定の対象外。
            }

            let region_set: HashSet<(i32, i32)> = region.iter().copied().collect();

            // `build_map`と同じ手順で上り階段・下り階段を求め、同じ`reserved`
            // 集合（入口・上り階段・下り階段）をforbiddenとして使う。テンプレート
            // 探索ロジック単体ではなく、本番の`build_map`が実際に組む条件と
            // 一致させることで、階段との座標衝突が起きないことも合わせて検証する。
            let stairs_up = neighbors4(entrance.0, entrance.1)
                .into_iter()
                .find(|p| region_set.contains(p))
                .unwrap_or(entrance);
            let distances = bfs_distances(&region, width, height, entrance);
            let exit = distances
                .iter()
                .max_by_key(|(_, d)| *d)
                .map(|(p, _)| *p)
                .unwrap_or(entrance);
            let reserved: HashSet<(i32, i32)> = [entrance, stairs_up, exit].into_iter().collect();

            let mut sorted_by_dist = distances.clone();
            sorted_by_dist.sort_by_key(|a| std::cmp::Reverse(a.1));

            let slots = find_template_slots(&sorted_by_dist, &region_set, &reserved, 2);
            if slots.len() == 2 {
                checked_two_slots += 1;
                assert!(
                    !template_rects_overlap(slots[0], slots[1]),
                    "seed {seed}: template rects overlap: {:?} vs {:?}",
                    slots[0],
                    slots[1]
                );
                for &top_left in &slots {
                    assert!(
                        template_rect_fits(&region_set, &HashSet::new(), top_left),
                        "seed {seed}: template rect {top_left:?} does not fully fit region"
                    );
                    for reserved_pos in &reserved {
                        assert!(
                            !template_rect_cells(top_left).any(|p| p == *reserved_pos),
                            "seed {seed}: template rect {top_left:?} collides with reserved tile {reserved_pos:?}"
                        );
                    }
                }
            }
        }

        assert!(
            checked_two_slots > 0,
            "no seed in the sampled range produced two template slots; test is vacuous"
        );
    }
}
