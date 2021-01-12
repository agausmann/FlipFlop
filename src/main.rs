mod assets;
mod board;
mod camera;
mod colored;
mod config;
mod cursor;
mod debug_text;
mod direction;
mod editor;
mod pin;
mod simulation;
mod tile;
mod uv_sprite;
mod wire;
mod wire_colored;

use self::assets::GameAssets;
use self::board::{Board, BoardBundle, BoardPlugin};
use self::camera::{CameraControlled, CameraPlugin};
use self::colored::{Colored, ColoredPlugin};
use self::config::Config;
use self::cursor::CursorPlugin;
use self::debug_text::{DebugText, DebugTextPlugin};
use self::direction::Direction;
use self::editor::EditorPlugin;
use self::pin::{Pin, PinPlugin};
use self::simulation::SimulationPlugin;
use self::tile::Tile;
use self::uv_sprite::UvSpritePlugin;
use self::wire::{Wire, WirePlugin};
use self::wire_colored::WireColoredPlugin;
use bevy::prelude::*;
use bevy::render::texture::AddressMode;

const TILE_PIXELS: f32 = 16.0;

//TODO come up with better names / more explicit sequencing for systems
const RENDER_SETUP: &str = "render_setup";
const PRE_RENDER_SETUP: &str = "pre_render_setup";
const APP_STATE: &str = "app_state";

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_stage_before(stage::POST_UPDATE, RENDER_SETUP, SystemStage::parallel())
        .add_stage_before(RENDER_SETUP, PRE_RENDER_SETUP, SystemStage::parallel())
        .add_resource(State::new(AppState::Loading))
        .add_stage_after(stage::UPDATE, APP_STATE, StateStage::<AppState>::default())
        .add_plugin(BoardPlugin)
        .add_plugin(CameraPlugin)
        .add_plugin(ColoredPlugin)
        .add_plugin(CursorPlugin)
        .add_plugin(DebugTextPlugin)
        .add_plugin(EditorPlugin)
        .add_plugin(PinPlugin)
        .add_plugin(SimulationPlugin)
        .add_plugin(UvSpritePlugin)
        .add_plugin(WirePlugin)
        .add_plugin(WireColoredPlugin)
        .add_resource(Config::default())
        .init_resource::<GameAssets>()
        .on_state_update(APP_STATE, AppState::Loading, configure_textures.system())
        .on_state_enter(APP_STATE, AppState::InGame, setup_game.system())
        .add_system(foo.system())
        .run();
}

use self::wire_colored::WireColored;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ElementState;
fn foo(
    events: Res<Events<KeyboardInput>>,
    mut reader: Local<EventReader<KeyboardInput>>,
    mut query: Query<&mut WireColored>,
) {
    for ev in reader.iter(&events) {
        match (ev.key_code, ev.state) {
            (Some(KeyCode::J), ElementState::Pressed) => {
                for mut wire_colored in query.iter_mut() {
                    wire_colored.is_on = !wire_colored.is_on;
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
enum AppState {
    Loading,
    InGame,
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

fn setup_game(commands: &mut Commands, assets: Res<GameAssets>) {
    commands
        .spawn(Camera2dBundle::default())
        .with(CameraControlled);
    commands.spawn(CameraUiBundle::default());

    commands.spawn(BoardBundle {
        board: Board {
            start: Tile::new(-1000, -1000),
            end: Tile::new(1000, 1000),
            z: -0.5,
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
                font: assets.regular_font.clone(),
                ..Default::default()
            },
            ..Default::default()
        })
        .with(DebugText);

    commands.spawn((Wire {
        start: Tile::new(1, 1),
        direction: Direction::Down,
        length: 3,
        z: 0.0,
    },));
    commands.spawn((Pin {
        position: Tile::new(1, 1),
        z: 0.0,
    },));
    commands.spawn((Pin {
        position: Tile::new(1, -2),
        z: 0.0,
    },));
}
