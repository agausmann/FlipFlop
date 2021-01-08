mod board;
mod colored;
mod uv_sprite;

use self::board::{Board, BoardBundle, BoardPlugin};
use self::colored::{Colored, ColoredPlugin};
use self::uv_sprite::UvSpritePlugin;
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::texture::AddressMode;
use indoc::formatdoc;
use std::collections::HashSet;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(BoardPlugin)
        .add_plugin(ColoredPlugin)
        .add_plugin(UvSpritePlugin)
        .add_resource(TickTimer(Timer::from_seconds(0.01, true)))
        .add_resource(Config::default())
        .add_resource(GameAssets::default())
        .add_resource(State::new(AppState::Loading))
        .add_resource(Cursor::default())
        .add_resource(CameraState::default())
        .add_stage_after(stage::UPDATE, STAGE, StateStage::<AppState>::default())
        .on_state_enter(STAGE, AppState::Loading, load_assets.system())
        .on_state_update(STAGE, AppState::Loading, configure_textures.system())
        .on_state_enter(STAGE, AppState::InGame, setup_game.system())
        .on_state_update(STAGE, AppState::InGame, camera_movement.system())
        .on_state_update(STAGE, AppState::InGame, cursor_position.system())
        .on_state_update(STAGE, AppState::InGame, tick.system())
        .on_state_update(STAGE, AppState::InGame, debug_text.system())
        .run();
}

const TILE_PIXELS: f32 = 16.0;
const STAGE: &str = "app_state";

#[derive(Clone)]
enum AppState {
    Loading,
    InGame,
}

fn load_assets(mut game_assets: ResMut<GameAssets>, asset_server: Res<AssetServer>) {
    game_assets.board_texture = asset_server.load("board.png");
}

fn configure_textures(
    mut textures: ResMut<Assets<Texture>>,
    game_assets: Res<GameAssets>,
    events: Res<Events<AssetEvent<Texture>>>,
    mut reader: Local<EventReader<AssetEvent<Texture>>>,
    mut state: ResMut<State<AppState>>,
) {
    for ev in reader.iter(&events) {
        match ev {
            AssetEvent::Created { handle } => {
                if *handle == game_assets.board_texture {
                    let texture = textures.get_mut(handle).unwrap();
                    texture.sampler.address_mode_u = AddressMode::Repeat;
                    texture.sampler.address_mode_v = AddressMode::Repeat;
                    state.set_next(AppState::InGame).unwrap(); //TODO more sophisticated loading progress
                }
            }
            _ => {}
        }
    }
}

#[derive(Default)]
struct GameAssets {
    board_texture: Handle<Texture>,
}

struct CameraControlled;
struct DebugText;

#[derive(Default)]
struct Config {
    camera: CameraConfig,
}

struct CameraConfig {
    pan_speed: f32,
    zoom_step: f32,
}

struct CameraState {
    pan: Vec2,
    zoom: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            pan_speed: 30.0,
            zoom_step: 0.05,
        }
    }
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pan: Vec2::zero(),
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Tile {
    x: i32,
    y: i32,
}

impl Tile {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl From<Vec2> for Tile {
    fn from(v: Vec2) -> Self {
        Self {
            x: v.x.floor() as i32,
            y: v.y.floor() as i32,
        }
    }
}

impl From<Tile> for Vec2 {
    fn from(tile: Tile) -> Self {
        Self::new(tile.x as f32, tile.y as f32)
    }
}

#[derive(Default)]
struct Cursor {
    screen_position: Vec2,
    position: Vec2,
    tile: Tile,
}

fn setup_game(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(Camera2dBundle::default())
        .with(CameraControlled);
    commands.spawn(CameraUiBundle::default());

    commands.spawn(BoardBundle {
        board: Board {
            start: Tile::new(-1000, -1000),
            end: Tile::new(1000, 1000),
            ..Default::default()
        },
        colored: Colored {
            color: Color::rgb(0.5, 0.5, 0.5),
        },
    });

    commands
        .spawn(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                font: asset_server.load("fonts/FiraSans-Regular.ttf"),
                ..Default::default()
            },
            ..Default::default()
        })
        .with(DebugText);
}

fn debug_text(
    diagnostics: Res<Diagnostics>,
    cursor: Res<Cursor>,
    camera: Res<CameraState>,
    mut query: Query<&mut Text, With<DebugText>>,
) {
    let fps = diagnostics
        .get(FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.average())
        .unwrap_or(f64::NAN);
    let frame_time = diagnostics
        .get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|diag| diag.average())
        .map(|seconds| seconds * 1000.0)
        .unwrap_or(f64::NAN);
    let debug_text = formatdoc!(
        "
            FPS: {:.0}
            Frame time: {:.3}ms

            Pan x: {:.2} y: {:.2}
            Zoom: {:.2}

            Cursor x: {:.2} y: {:.2}
            Tile x: {} y: {}
        ",
        fps,
        frame_time,
        camera.pan.x,
        camera.pan.y,
        camera.zoom,
        cursor.position.x,
        cursor.position.y,
        cursor.tile.x,
        cursor.tile.y,
    );

    for mut text in query.iter_mut() {
        text.value = debug_text.clone();
    }
}

fn camera_movement(
    config: Res<Config>,
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_wheel_events: Res<Events<MouseWheel>>,
    mut camera: ResMut<CameraState>,
    mut mouse_wheel_reader: Local<EventReader<MouseWheel>>,
    mut query: Query<&mut Transform, With<CameraControlled>>,
) {
    let mut pan_direction = Vec2::zero();
    if keyboard_input.pressed(KeyCode::W) {
        pan_direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) {
        pan_direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) {
        pan_direction.x += 1.0;
    }
    if keyboard_input.pressed(KeyCode::A) {
        pan_direction.x -= 1.0;
    }
    let pan_amount = pan_direction * config.camera.pan_speed * time.delta_seconds() / camera.zoom;
    camera.pan += pan_amount;

    let mut zoom_amount = 0;
    for ev in mouse_wheel_reader.iter(&mouse_wheel_events) {
        let MouseWheel { y, .. } = *ev;
        if y > 0.0 {
            zoom_amount += 1;
        } else if y < 0.0 {
            zoom_amount -= 1;
        }
    }
    camera.zoom *= (1.0 + config.camera.zoom_step).powi(zoom_amount);

    let new_transform = Transform {
        translation: camera.pan.extend(0.0),
        scale: Vec2::splat(1.0 / (camera.zoom * TILE_PIXELS)).extend(1.0),
        ..Default::default()
    };

    for mut transform in query.iter_mut() {
        *transform = new_transform;
    }
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

struct TickTimer(Timer);

struct Wire {
    inputs: HashSet<Entity>,
    state: bool,
}

struct Gate {
    input: Entity,
    flip: bool,
    output: bool,
}

fn tick(
    time: Res<Time>,
    mut timer: ResMut<TickTimer>,
    mut gates: Query<&mut Gate>,
    mut wires: Query<&mut Wire>,
) {
    if !timer.0.tick(time.delta_seconds()).just_finished() {
        return;
    }

    for mut gate in gates.iter_mut() {
        gate.output = gate.flip ^ wires.get_mut(gate.input).unwrap().state
    }

    for mut wire in wires.iter_mut() {
        wire.state = wire
            .inputs
            .iter()
            .map(|&entity| gates.get_mut(entity).unwrap().output)
            .fold(false, |acc, state| acc | state);
    }
}
