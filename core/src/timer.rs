//! `bevy::time::Timer`に依存しない、演出用の最小限のカウントダウンタイマー。
//! SSH越しの将来のサーバー実装でも同じ経過判定ロジックを使い回せるようにする。

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SimpleTimer {
    duration: Duration,
    elapsed: Duration,
    finished: bool,
}

impl SimpleTimer {
    pub fn from_seconds(seconds: f32) -> Self {
        Self {
            duration: Duration::from_secs_f32(seconds),
            elapsed: Duration::ZERO,
            finished: false,
        }
    }

    pub fn tick(&mut self, delta: Duration) -> &mut Self {
        if !self.finished {
            self.elapsed += delta;
            if self.elapsed >= self.duration {
                self.finished = true;
            }
        }
        self
    }

    pub fn finished(&self) -> bool {
        self.finished
    }

    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.finished = false;
    }
}
