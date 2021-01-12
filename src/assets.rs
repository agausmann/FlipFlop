use bevy::prelude::*;

pub struct GameAssets {
    pub regular_font: Handle<Font>,
    pub board_texture: Handle<Texture>,
    pub pin_texture: Handle<Texture>,
}

impl FromResources for GameAssets {
    fn from_resources(resources: &Resources) -> Self {
        let asset_server = resources.get_mut::<AssetServer>().expect("AssetServer not loaded");
        Self {
            regular_font: asset_server.load("fonts/FiraSans-Regular.ttf"),
            board_texture: asset_server.load("textures/board.png"),
            pin_texture: asset_server.load("textures/pin.png"),
        }
    }
}
