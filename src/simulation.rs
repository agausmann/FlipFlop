use bevy::prelude::*;
use std::collections::HashSet;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(TickTimer::default())
            .add_system(tick.system());
    }
}

#[derive(Default)]
pub struct TickTimer(Timer);

pub struct Wire {
    inputs: HashSet<Entity>,
    state: bool,
}

pub struct Gate {
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
