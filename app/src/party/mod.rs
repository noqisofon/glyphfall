//! `glyphfall-core::party`をBevyの`Resource`として使うための薄いラップ層。

use bevy::prelude::*;
use std::ops::{Deref, DerefMut};

pub use glyphfall_core::party::{
    Influence, MentalState, PartyCommand, PartyMember, PartyState, Personality,
    PlayerBattleAction, PlayerInventory, PlayerSkills,
};

#[derive(Resource, Default)]
pub struct ReserveRosterRes(pub glyphfall_core::party::ReserveRoster);

impl Deref for ReserveRosterRes {
    type Target = glyphfall_core::party::ReserveRoster;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ReserveRosterRes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Resource)]
pub struct PartyStateRes(pub glyphfall_core::party::PartyState);

impl Deref for PartyStateRes {
    type Target = glyphfall_core::party::PartyState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PartyStateRes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Resource)]
pub struct PlayerResourceRes(pub glyphfall_core::party::PlayerResource);

impl Deref for PlayerResourceRes {
    type Target = glyphfall_core::party::PlayerResource;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PlayerResourceRes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Resource, Default)]
pub struct PlayerInventoryRes(pub glyphfall_core::party::PlayerInventory);

impl Deref for PlayerInventoryRes {
    type Target = glyphfall_core::party::PlayerInventory;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PlayerInventoryRes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
