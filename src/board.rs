use crate::colored::Colored;
use crate::uv_sprite::{UvRect, UvSpriteBundle};
use crate::{GameAssets, Tile};
use bevy::prelude::*;

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(board_update.system())
            .add_system(attach_sprite.system());
    }
}

#[derive(Default, Bundle)]
pub struct BoardBundle {
    pub board: Board,
    pub colored: Colored,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Board {
    pub start: Tile,
    pub end: Tile,
    pub z: f32,
}

impl Board {
    fn position(&self) -> Vec3 {
        let start: Vec2 = self.start.into();
        let end: Vec2 = self.end.into();

        ((start + end) / 2.0).extend(self.z)
    }

    fn size(&self) -> Vec2 {
        let start: Vec2 = self.start.into();
        let end: Vec2 = self.end.into();

        (end - start).abs() + Vec2::splat(1.0)
    }
}

fn attach_sprite(
    commands: &mut Commands,
    game_assets: Res<GameAssets>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(Entity, &Board, &Colored), Without<Sprite>>,
) {
    for (entity, board, colored) in query.iter() {
        commands.set_current_entity(entity);
        commands.with_bundle(UvSpriteBundle {
            sprite: Sprite::new(board.size()),
            uv_rect: UvRect {
                min: Vec2::zero(),
                max: board.size(),
            },
            material: materials.add(ColorMaterial::modulated_texture(
                game_assets.board_texture.clone(),
                colored.color,
            )),
            ..Default::default()
        });
    }
}

fn board_update(
    mut query: Query<(&Board, &mut Sprite, &mut UvRect, &mut Transform), Changed<Board>>,
) {
    for (board, mut sprite, mut uv_rect, mut transform) in query.iter_mut() {
        sprite.size = board.size();

        uv_rect.min = Vec2::zero();
        uv_rect.max = board.size();

        *transform = Transform {
            translation: board.position(),
            scale: board.size().extend(1.0),
            ..Default::default()
        }
    }
}
