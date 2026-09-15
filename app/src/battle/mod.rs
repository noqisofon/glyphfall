//! `glyphfall-core::battle`のBevy側の薄い皮。
//!
//! ロジック本体（`BattleState`とその実装）はコアクレートに移した。ここでは
//! Bevyの`Resource`としてECSに載せるためのニュータイプラップと、Bevy入力系に
//! 依存する`input`ハンドラのみを持つ。

pub mod input;
pub use input::handle_battle_input;

use crate::macros::newtype_resource;

pub use glyphfall_core::battle::{create_default_monsters, BattlePhase, BattleState, Monster};

newtype_resource!(BattleStateRes, BattleState);
