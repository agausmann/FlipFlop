use crate::direction::Direction;
use crate::ivec::Vec2i;
use crate::pin::Pin;
use crate::wire::Wire;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct Circuit {
    pub tile_pins: HashSet<Vec2i>,
    pub wire_endpoints: HashSet<(Vec2i, Vec2i)>,
    pub tile_wires: HashMap<Vec2i, TileWire>,
    pub entity_wires: HashMap<Entity, Wire>,
}

impl Circuit {
    pub fn add_pin(&mut self, pin: Pin, commands: &mut Commands) {
        if self.tile_pins.insert(pin.position) {
            let position = pin.position;
            commands.spawn((pin,));

            let tile_wires = self.tile_wires.entry(position).or_default().clone();

            match (tile_wires.up, tile_wires.down) {
                (Some(entity_a), Some(entity_b)) if entity_a == entity_b => {
                    let wire = self.remove_wire_ll(entity_a, commands);
                    self.add_wire_ll(
                        Wire {
                            start: wire.start,
                            direction: wire.direction,
                            length: wire.tile_index(position),
                            z: wire.z,
                        },
                        commands,
                    );
                    self.add_wire_ll(
                        Wire {
                            start: position,
                            direction: wire.direction,
                            length: wire.length - wire.tile_index(position),
                            z: wire.z,
                        },
                        commands,
                    );
                }
                _ => {}
            }

            match (tile_wires.right, tile_wires.left) {
                (Some(entity_a), Some(entity_b)) if entity_a == entity_b => {
                    let wire = self.remove_wire_ll(entity_a, commands);
                    self.add_wire_ll(
                        Wire {
                            start: wire.start,
                            direction: wire.direction,
                            length: wire.tile_index(position),
                            z: wire.z,
                        },
                        commands,
                    );
                    self.add_wire_ll(
                        Wire {
                            start: position,
                            direction: wire.direction,
                            length: wire.length - wire.tile_index(position),
                            z: wire.z,
                        },
                        commands,
                    );
                }
                _ => {}
            }
        }
    }

    pub fn remove_pin(&mut self, position: Vec2i, commands: &mut Commands) {
        todo!()
    }

    pub fn add_wire(&mut self, wire: Wire, commands: &mut Commands) {
        let mut pins = Vec::new();
        pins.push(0);
        for i in 1..wire.length {
            if self.tile_pins.contains(&wire.nth_tile(i)) {
                pins.push(i);
            }
        }
        pins.push(wire.length);

        for v in pins.windows(2) {
            let (a, b) = (v[0], v[1]);
            let start = wire.nth_tile(a);
            let end = wire.nth_tile(b);
            if !self.wire_endpoints.contains(&(start, end))
                || !self.wire_endpoints.contains(&(end, start))
            {
                self.add_wire_ll(
                    Wire {
                        start,
                        direction: wire.direction,
                        length: b - a,
                        ..Default::default()
                    },
                    commands,
                );
            }
        }
    }

    pub fn remove_wire(&mut self, wire: Wire, commands: &mut Commands) {
        todo!()
    }

    fn add_wire_ll(&mut self, wire: Wire, commands: &mut Commands) {
        self.add_pin(
            Pin {
                position: wire.start,
                ..Default::default()
            },
            commands,
        );
        self.add_pin(
            Pin {
                position: wire.end(),
                ..Default::default()
            },
            commands,
        );
        self.wire_endpoints.insert((wire.start, wire.end()));
        self.wire_endpoints.insert((wire.end(), wire.start));
        commands.spawn((wire.clone(),));
        let wire_entity = commands.current_entity().unwrap();
        self.entity_wires.insert(wire_entity, wire.clone());

        *self
            .tile_wires
            .entry(wire.start)
            .or_default()
            .get_mut(wire.direction) = Some(wire_entity);
        *self
            .tile_wires
            .entry(wire.end())
            .or_default()
            .get_mut(wire.direction.opposite()) = Some(wire_entity);

        for i in 1..wire.length {
            let tile_wires = self.tile_wires.entry(wire.nth_tile(i)).or_default();
            *tile_wires.get_mut(wire.direction) = Some(wire_entity);
            *tile_wires.get_mut(wire.direction.opposite()) = Some(wire_entity);
        }
    }

    fn remove_wire_ll(&mut self, entity: Entity, commands: &mut Commands) -> Wire {
        let wire = self
            .entity_wires
            .remove(&entity)
            .expect("Wire does not exist");
        commands.despawn(entity);
        *self
            .tile_wires
            .get_mut(&wire.start)
            .unwrap()
            .get_mut(wire.direction) = None;
        *self
            .tile_wires
            .get_mut(&wire.end())
            .unwrap()
            .get_mut(wire.direction.opposite()) = None;
        for i in 1..wire.length {
            let tile_wires = self.tile_wires.get_mut(&wire.nth_tile(i)).unwrap();
            *tile_wires.get_mut(wire.direction) = None;
            *tile_wires.get_mut(wire.direction.opposite()) = None;
        }

        self.wire_endpoints.remove(&(wire.start, wire.end()));
        self.wire_endpoints.remove(&(wire.end(), wire.start));

        wire
    }
}

#[derive(Debug, Clone, Default)]
pub struct TileWire {
    pub up: Option<Entity>,
    pub down: Option<Entity>,
    pub left: Option<Entity>,
    pub right: Option<Entity>,
}

impl TileWire {
    fn get_mut(&mut self, direction: Direction) -> &mut Option<Entity> {
        match direction {
            Direction::Up => &mut self.up,
            Direction::Down => &mut self.down,
            Direction::Left => &mut self.left,
            Direction::Right => &mut self.right,
        }
    }
}
