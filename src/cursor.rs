use crate::camera::CameraState;
use crate::{AppState, Tile, APP_STATE, TILE_PIXELS};
use bevy::prelude::*;

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(Cursor::default()).on_state_update(
            APP_STATE,
            AppState::InGame,
            cursor_position.system(),
        );
    }
}

#[derive(Default)]
pub struct Cursor {
    pub screen_position: Vec2,
    pub position: Vec2,
    pub tile: Tile,
}

fn cursor_position(
    events: Res<Events<CursorMoved>>,
    windows: Res<Windows>,
    camera: Res<CameraState>,
    mut reader: Local<EventReader<CursorMoved>>,
    mut cursor: ResMut<Cursor>,
) {
    if let Some(ev) = reader.latest(&events) {
        let window = windows.get_primary().unwrap();
        let window_size = Vec2::new(window.width(), window.height());
        cursor.screen_position = ev.position - window_size / 2.0;
    }
    cursor.position = cursor.screen_position / TILE_PIXELS / camera.zoom + camera.pan;
    cursor.tile = cursor.position.into();
}
