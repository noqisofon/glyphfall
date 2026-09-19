//! `glyphfall-core::event`をBevyの`Resource`として使うための薄いラップ層。

use crate::macros::newtype_resource;

pub use glyphfall_core::event::SuddenEventCategory;

newtype_resource!(
    SuddenEventRegistryRes,
    glyphfall_core::event::SuddenEventRegistry
);
newtype_resource!(
    SuddenEventHistoryRes,
    glyphfall_core::event::SuddenEventHistory,
    default
);
