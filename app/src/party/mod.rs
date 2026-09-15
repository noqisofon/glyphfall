//! `glyphfall-core::party`をBevyの`Resource`として使うための薄いラップ層。

use crate::macros::newtype_resource;

pub use glyphfall_core::party::{
    Influence, MentalState, PartyCommand, PartyMember, PartyState, Personality,
    PlayerBattleAction, PlayerInventory, PlayerSkills,
};

newtype_resource!(ReserveRosterRes, glyphfall_core::party::ReserveRoster, default);
newtype_resource!(PartyStateRes, glyphfall_core::party::PartyState);
newtype_resource!(PlayerResourceRes, glyphfall_core::party::PlayerResource);
newtype_resource!(PlayerInventoryRes, glyphfall_core::party::PlayerInventory, default);
