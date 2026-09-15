//! `glyphfall-core::event`をBevyの`Resource`として使うための薄いラップ層。

use bevy::prelude::*;
use std::ops::{Deref, DerefMut};

pub use glyphfall_core::event::{try_trigger_sudden_event, SuddenEventCategory};

#[derive(Resource)]
pub struct SuddenEventRegistryRes(pub glyphfall_core::event::SuddenEventRegistry);

impl Deref for SuddenEventRegistryRes {
    type Target = glyphfall_core::event::SuddenEventRegistry;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SuddenEventRegistryRes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Resource, Default)]
pub struct SuddenEventHistoryRes(pub glyphfall_core::event::SuddenEventHistory);

impl Deref for SuddenEventHistoryRes {
    type Target = glyphfall_core::event::SuddenEventHistory;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SuddenEventHistoryRes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
