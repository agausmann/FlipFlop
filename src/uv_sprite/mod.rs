mod render;

use self::render::{UvSpriteRenderGraphBuilder, UV_SPRITE_PIPELINE_HANDLE};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::pipeline::RenderPipeline;
use bevy::render::render_graph::base::MainPass;
use bevy::render::render_graph::RenderGraph;
use bevy::render::renderer::RenderResources;
use bevy::sprite::{SpritePlugin, QUAD_HANDLE};

#[derive(Default)]
pub struct UvSpritePlugin;

impl Plugin for UvSpritePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(SpritePlugin).register_type::<UvRect>();

        let resources = app.resources_mut();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_sprite_graph(resources);
    }
}

#[derive(Bundle)]
pub struct UvSpriteBundle {
    pub sprite: Sprite,
    pub uv_rect: UvRect,
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
    pub main_pass: MainPass,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for UvSpriteBundle {
    fn default() -> Self {
        Self {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                UV_SPRITE_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            main_pass: MainPass,
            draw: Default::default(),
            sprite: Default::default(),
            uv_rect: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(RenderResources, TypeUuid, Reflect)]
#[uuid = "3ff02b75-5b71-493f-924e-8d15fc6f2970"]
pub struct UvRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Default for UvRect {
    fn default() -> Self {
        Self {
            min: Vec2::zero(),
            max: Vec2::one(),
        }
    }
}
