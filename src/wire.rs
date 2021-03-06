use crate::direction::Direction;
use crate::ivec::Vec2i;
use crate::wire_colored::WireColored;
use bevy::prelude::*;

const WIRE_WIDTH: f32 = 0.125;

pub struct WirePlugin;

impl Plugin for WirePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(crate::PRE_RENDER_SETUP, attach_sprite.system())
            .add_system_to_stage(crate::RENDER_SETUP, update_wire.system());
    }
}

#[derive(Debug, Clone)]
pub struct Wire {
    pub start: Vec2i,
    pub direction: Direction,
    pub length: i32,
    pub z: f32,
}

impl Wire {
    fn size(&self) -> Vec2 {
        Vec2::new((self.length as f32) + WIRE_WIDTH, WIRE_WIDTH)
    }

    fn transform(&self) -> Transform {
        Transform {
            translation: (Vec2::from(self.start)
                + Vec2::splat(0.5)
                + self.direction.vector() * (self.length as f32) / 2.0)
                .extend(self.z),
            rotation: self.direction.into(),
            ..Default::default()
        }
    }

    pub fn nth_tile(&self, index: i32) -> Vec2i {
        self.start + self.direction.int_vector() * index
    }

    pub fn tile_index(&self, tile: Vec2i) -> i32 {
        let delta = tile - self.start;
        let projected = delta * self.direction.int_vector();
        projected.x + projected.y
    }

    pub fn end(&self) -> Vec2i {
        self.start + self.direction.int_vector() * self.length
    }
}

impl Default for Wire {
    fn default() -> Self {
        Self {
            start: Vec2i::zero(),
            direction: Direction::Right,
            length: 1,
            z: 0.0,
        }
    }
}

fn attach_sprite(
    commands: &mut Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(Entity, &Wire), Added<Wire>>,
) {
    for (entity, wire) in query.iter() {
        commands.set_current_entity(entity);
        commands
            .with_bundle(SpriteBundle {
                sprite: Sprite::new(wire.size()),
                material: materials.add(Default::default()),
                transform: wire.transform(),
                ..Default::default()
            })
            .with(WireColored::default());
    }
}

fn update_wire(mut query: Query<(&Wire, &mut Sprite, &mut Transform), Changed<Wire>>) {
    for (wire, mut sprite, mut transform) in query.iter_mut() {
        sprite.size = wire.size();
        *transform = wire.transform();
    }
}
