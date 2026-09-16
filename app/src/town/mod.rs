//! `glyphfall-core::town`のBevy側の薄い皮。
//!
//! 探索状態そのもの（マップ・視界・プレイヤー位置など）はコアクレートの
//! `TownState`が持つ。ここではBevyの`Image`アセットとして管理する街マップ
//! テクスチャの生成・更新、そしてBevy入力系に依存する`input`ハンドラのみを
//! 追加する。

pub mod input;
pub mod pixel_art;

pub use input::{
    handle_dialogue_input, handle_inn_input, handle_interact_input, handle_shop_input,
    handle_town_input, handle_travel_input,
};

pub use glyphfall_core::town::{
    AreaId, CommandKind, DialogueLearnStage, DialoguePartner, DialogueSession, InteractOutcome,
    LearnableSpan, MoveOutcome, Position, TargetKind, TownState, SHOP_ITEMS,
};

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use pixel_art::{render_town_to_texture, TEXTURE_HEIGHT, TEXTURE_WIDTH};
use std::ops::{Deref, DerefMut};

#[derive(Resource)]
pub struct TownStateRes {
    pub state: TownState,
    pub texture_handle: Handle<Image>,
}

impl TownStateRes {
    pub fn new(images: &mut Assets<Image>, party_members: &[glyphfall_core::party::PartyMember]) -> Self {
        let state = TownState::new();

        let mut buffer = vec![0u8; TEXTURE_WIDTH * TEXTURE_HEIGHT * 4];
        render_town_to_texture(
            &state.map,
            &state.fov,
            state.player_pos,
            &state.followers,
            party_members,
            &mut buffer,
        );

        let mut image = Image::new(
            Extent3d {
                width: TEXTURE_WIDTH as u32,
                height: TEXTURE_HEIGHT as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            buffer,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = bevy::image::ImageSampler::nearest();

        let texture_handle = images.add(image);

        Self {
            state,
            texture_handle,
        }
    }

    pub fn update_texture(
        &mut self,
        party_members: &[glyphfall_core::party::PartyMember],
        images: &mut Assets<Image>,
    ) {
        if !self.state.dirty {
            return;
        }
        if let Some(image) = images.get_mut(&self.texture_handle) {
            render_town_to_texture(
                &self.state.map,
                &self.state.fov,
                self.state.player_pos,
                &self.state.followers,
                party_members,
                &mut image.data,
            );
        }
        self.state.dirty = false;
    }
}

impl Deref for TownStateRes {
    type Target = TownState;
    fn deref(&self) -> &TownState {
        &self.state
    }
}

impl DerefMut for TownStateRes {
    fn deref_mut(&mut self) -> &mut TownState {
        &mut self.state
    }
}
