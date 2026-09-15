//! `glyphfall-core::battle`のBevy側の薄い皮。
//!
//! ロジック本体（`BattleState`とその実装）はコアクレートに移した。ここでは
//! Bevyの`Resource`としてECSに載せるためのニュータイプラップと、Bevy入力系に
//! 依存する`input`ハンドラのみを持つ。

pub mod input;
pub use input::handle_battle_input;

use bevy::prelude::*;
use std::ops::{Deref, DerefMut};

pub use glyphfall_core::battle::{create_default_monsters, BattlePhase, BattleState, Monster};

#[derive(Resource)]
pub struct BattleStateRes(pub BattleState);

impl Deref for BattleStateRes {
    type Target = BattleState;
    fn deref(&self) -> &BattleState {
        &self.0
    }
}

impl DerefMut for BattleStateRes {
    fn deref_mut(&mut self) -> &mut BattleState {
        &mut self.0
    }
}
