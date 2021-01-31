use crate::camera::CameraState;
use crate::circuit::Circuit;
use crate::cursor::Cursor;
use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use indoc::formatdoc;

pub struct DebugTextPlugin;

impl Plugin for DebugTextPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(FrameTimeDiagnosticsPlugin)
            .add_system(debug_text.system());
    }
}

pub struct DebugText;

fn debug_text(
    diagnostics: Res<Diagnostics>,
    cursor: Res<Cursor>,
    camera: Res<CameraState>,
    circuit: Res<Circuit>,
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
    let wire_connectivity = circuit
        .tile_wires
        .get(&cursor.tile)
        .cloned()
        .unwrap_or(Default::default());
    let debug_text = formatdoc!(
        "
            FPS: {:.0}
            Frame time: {:.3}ms

            Pan x: {:.2} y: {:.2}
            Zoom: {:.2}

            Cursor x: {:.2} y: {:.2}
            Tile x: {} y: {}

            Pin: {:?}
            Wire up: {:?}
            Wire down: {:?}
            Wire right: {:?}
            Wire left: {:?}
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
        circuit.tile_pins.get(&cursor.tile),
        wire_connectivity.up,
        wire_connectivity.down,
        wire_connectivity.right,
        wire_connectivity.left,
    );

    for mut text in query.iter_mut() {
        text.value = debug_text.clone();
    }
}
