use crate::assets::GameAssets;
use crate::ivec::Vec2i;
use crate::wire_colored::WireColored;
use bevy::prelude::*;

pub struct PinPlugin;

impl Plugin for PinPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(crate::RENDER_SETUP, update_pin.system())
            .add_system_to_stage(crate::PRE_RENDER_SETUP, attach_sprite.system());
    }
}

#[derive(Debug, Clone)]
pub struct Pin {
    pub position: Vec2i,
    pub z: f32,
}

impl Pin {
    fn transform(&self) -> Transform {
        Transform {
            translation: (Vec2::from(self.position) + Vec2::splat(0.5)).extend(self.z),
            ..Default::default()
        }
    }
}

impl Default for Pin {
    fn default() -> Self {
        Self {
            position: Vec2i::zero(),
            z: 0.0,
        }
    }
}

fn attach_sprite(
    commands: &mut Commands,
    game_assets: Res<GameAssets>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(Entity, &Pin), Added<Pin>>,
) {
    for (entity, pin) in query.iter() {
        commands.set_current_entity(entity);
        commands
            .with_bundle(SpriteBundle {
                sprite: Sprite::new(Vec2::new(1.0, 1.0)),
                material: materials.add(game_assets.pin_texture.clone().into()),
                transform: pin.transform(),
                ..Default::default()
            })
            .with(WireColored::default());
    }
}

fn update_pin(mut query: Query<(&Pin, &mut Transform), Changed<Pin>>) {
    for (pin, mut transform) in query.iter_mut() {
        *transform = pin.transform();
    }
}
