use bevy::prelude::*;
use std::collections::HashSet;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_resource(TickTimer(Timer::from_seconds(0.01, true)))
        .add_system(tick.system())
        .run();
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
