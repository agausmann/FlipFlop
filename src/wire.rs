use crate::direction::Direction;
use crate::wire_colored::WireColored;
use crate::Tile;
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
    pub start: Tile,
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
}

impl Default for Wire {
    fn default() -> Self {
        Self {
            start: Tile::new(0, 0),
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

fn update_wire(mut query: Query<(&Wire, &mut Transform), Changed<Wire>>) {
    for (wire, mut transform) in query.iter_mut() {
        *transform = wire.transform();
    }
}
