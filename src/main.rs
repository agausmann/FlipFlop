use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use std::collections::HashSet;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_resource(TickTimer(Timer::from_seconds(0.01, true)))
        .add_resource(Config::default())
        .add_startup_system(setup.system())
        .add_system(camera_movement.system())
        .add_system(tick.system())
        .run();
}

const TILE_PIXELS: f32 = 16.0;

fn setup(commands: &mut Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands
        .spawn(Camera2dBundle::default())
        .with(Camera::default())
        .spawn(SpriteBundle {
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            sprite: Sprite::new(Vec2::new(1.0, 1.0)),
            ..Default::default()
        });
}

#[derive(Default)]
struct Config {
    camera: CameraConfig,
}

struct CameraConfig {
    pan_speed: f32,
    zoom_step: f32,
}

struct Camera {
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

impl Default for Camera {
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
    mut mouse_wheel_reader: Local<EventReader<MouseWheel>>,
    mut query: Query<(&mut Camera, &mut Transform)>,
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
    let pan_amount = pan_direction * config.camera.pan_speed * time.delta_seconds();

    let mut zoom_amount = 0;
    for ev in mouse_wheel_reader.iter(&mouse_wheel_events) {
        let MouseWheel { y, .. } = *ev;
        if y > 0.0 {
            zoom_amount += 1;
        } else if y < 0.0 {
            zoom_amount -= 1;
        }
    }
    let zoom_factor = (1.0 + config.camera.zoom_step).powi(zoom_amount);

    for (mut camera, mut transform) in query.iter_mut() {
        camera.zoom *= zoom_factor;
        let local_pan_amount = pan_amount / camera.zoom;
        camera.pan += local_pan_amount;

        *transform = Transform {
            translation: camera.pan.extend(0.0),
            scale: Vec2::splat(1.0 / (camera.zoom * TILE_PIXELS)).extend(1.0),
            ..Default::default()
        }
    }
}

struct TickTimer(Timer);

struct Wire {
    inputs: HashSet<Entity>,
    state: bool,
}

struct FlipFlop {
    input: Entity,
    flip: bool,
    output: bool,
}

fn tick(
    time: Res<Time>,
    mut timer: ResMut<TickTimer>,
    mut flipflops: Query<&mut FlipFlop>,
    mut wires: Query<&mut Wire>,
) {
    if !timer.0.tick(time.delta_seconds()).just_finished() {
        return;
    }

    for mut ff in flipflops.iter_mut() {
        ff.output = ff.flip ^ wires.get_mut(ff.input).unwrap().state
    }

    for mut wire in wires.iter_mut() {
        wire.state = wire
            .inputs
            .iter()
            .map(|&entity| flipflops.get_mut(entity).unwrap().output)
            .fold(false, |acc, state| acc | state);
    }
}
