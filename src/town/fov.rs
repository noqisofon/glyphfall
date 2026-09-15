use super::map::TownMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Visible,
    Hidden,
}

pub struct FovMap {
    pub width: usize,
    pub height: usize,
    pub visibility: Vec<Visibility>,
}

impl FovMap {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            visibility: vec![Visibility::Hidden; width * height],
        }
    }

    pub fn is_visible(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            false
        } else {
            self.visibility[y as usize * self.width + x as usize] == Visibility::Visible
        }
    }

    pub fn compute(
        &mut self,
        map: &TownMap,
        player_x: i32,
        player_y: i32,
        radius: i32,
        see_all: bool,
    ) {
        if see_all {
            for v in self.visibility.iter_mut() {
                *v = Visibility::Visible;
            }
            return;
        }

        // 初期化：すべて非表示
        for v in self.visibility.iter_mut() {
            *v = Visibility::Hidden;
        }

        // プレイヤー自身とその足元は常に可視
        self.set_visible(player_x, player_y);

        let min_x = (player_x - radius).max(0);
        let max_x = (player_x + radius).min(self.width as i32 - 1);
        let min_y = (player_y - radius).max(0);
        let max_y = (player_y + radius).min(self.height as i32 - 1);

        let r_squared = radius * radius;

        for ty in min_y..=max_y {
            for tx in min_x..=max_x {
                let dx = tx - player_x;
                let dy = ty - player_y;
                // 横縦のアスペクト比を補正（等幅文字は縦長なので縦の重みをやや大きめに）
                if dx * dx + (dy * 2) * (dy * 2) <= r_squared * 2
                    && has_line_of_sight(map, player_x, player_y, tx, ty)
                {
                    self.set_visible(tx, ty);
                }
            }
        }
    }

    fn set_visible(&mut self, x: i32, y: i32) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.visibility[y as usize * self.width + x as usize] = Visibility::Visible;
        }
    }
}

/// Bresenham's line algorithm で視線が通っているか判定
fn has_line_of_sight(map: &TownMap, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    loop {
        // 目標地点に到達したなら視線は通っている（壁そのものは見えるようにする）
        if x == x1 && y == y1 {
            return true;
        }

        // 始点以外の通過セルで視界を遮る障害物があれば遮断
        if x != x0 || y != y0 {
            if let Some(tile) = map.get(x, y) {
                if tile.blocks_sight() {
                    return false;
                }
            }
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}
