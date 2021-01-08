use crate::config::Config;
use crate::TILE_PIXELS;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(camera_movement.system())
            .add_resource(CameraState::default());
    }
}

pub struct CameraControlled;

pub struct CameraState {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            pan: Vec2::zero(),
            zoom: 1.0,
        }
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
    camera.zoom = camera
        .zoom
        .min(config.camera.max_zoom)
        .max(config.camera.min_zoom);

    let new_transform = Transform {
        translation: camera.pan.extend(0.0),
        scale: Vec2::splat(1.0 / (camera.zoom * TILE_PIXELS)).extend(1.0),
        ..Default::default()
    };

    for mut transform in query.iter_mut() {
        *transform = new_transform;
    }
}
