use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use std::collections::HashSet;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_resource(TickTimer(Timer::from_seconds(0.01, true)))
        .add_system(camera_movement.system())
        .add_system(tick.system())
        .run();
}

fn setup(commands: &mut Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    let white_material = materials.add(Color::rgb(1.0, 1.0, 1.0).into());

    commands
        .spawn(Camera2dBundle::default())
        .with(CameraConfig::default())
        .spawn(SpriteBundle {
            material: white_material,
            sprite: Sprite::new(Vec2::new(100.0, 100.0)),
            ..Default::default()
        });
}

struct CameraConfig {
    pan_speed: f32,
    zoom_step: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            pan_speed: 500.0,
            zoom_step: 0.05,
        }
    }
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mouse_wheel_events: Res<Events<MouseWheel>>,
    mut mouse_wheel_reader: Local<EventReader<MouseWheel>>,
    mut query: Query<(&CameraConfig, &mut Transform)>,
) {
    let mut pan_direction = Vec3::zero();
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

    let mut zoom_amount = 0;

    for ev in mouse_wheel_reader.iter(&mouse_wheel_events) {
        let MouseWheel { y, .. } = *ev;
        if y > 0.0 {
            zoom_amount += 1;
        } else if y < 0.0 {
            zoom_amount -= 1;
        }
    }

    for (config, mut transform) in query.iter_mut() {
        let zoom_factor = (1.0 + config.zoom_step).powi(zoom_amount);
        transform.scale.x /= zoom_factor;
        transform.scale.y /= zoom_factor;

        let pan_amount = transform.scale * pan_direction * config.pan_speed * time.delta_seconds();
        transform.translation += pan_amount;
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
