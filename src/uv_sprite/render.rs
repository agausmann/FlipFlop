use super::UvRect;
use bevy::asset::{Assets, HandleUntyped};
use bevy::ecs::Resources;
use bevy::reflect::TypeUuid;
use bevy::render::{
    pipeline::{
        BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
        CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, PipelineDescriptor,
        RasterizationStateDescriptor, StencilStateDescriptor, StencilStateFaceDescriptor,
    },
    render_graph::{base, RenderGraph, RenderResourcesNode},
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat,
};
use bevy::sprite::Sprite;

pub const UV_SPRITE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 0xf16acef4e15cce3a);

pub fn build_uv_sprite_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilStateDescriptor {
                front: StencilStateFaceDescriptor::IGNORE,
                back: StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        color_states: vec![ColorStateDescriptor {
            format: TextureFormat::default(),
            color_blend: BlendDescriptor {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendDescriptor {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("uv_sprite.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("uv_sprite.frag"),
            ))),
        })
    }
}

pub mod node {
    pub const UV_SPRITE: &str = "uv_sprite";
}

pub trait UvSpriteRenderGraphBuilder {
    fn add_sprite_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl UvSpriteRenderGraphBuilder for RenderGraph {
    fn add_sprite_graph(&mut self, resources: &Resources) -> &mut Self {
        self.add_system_node(node::UV_SPRITE, RenderResourcesNode::<Sprite>::new(true));
        self.add_system_node(node::UV_SPRITE, RenderResourcesNode::<UvRect>::new(true));
        self.add_node_edge(node::UV_SPRITE, base::node::MAIN_PASS)
            .unwrap();

        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        pipelines.set_untracked(
            UV_SPRITE_PIPELINE_HANDLE,
            build_uv_sprite_pipeline(&mut shaders),
        );
        self
    }
}
