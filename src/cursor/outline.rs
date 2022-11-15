use crate::screen_vertex::ScreenVertexShader;
use crate::viewport::Viewport;
use crate::GraphicsContext;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

pub struct OutlineRenderer {
    gfx: GraphicsContext,
    screen_vertex_shader: &'static ScreenVertexShader,
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    depth_sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    uniforms: Uniforms,
}

impl OutlineRenderer {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let screen_vertex_shader = ScreenVertexShader::get(gfx);
        let bind_group_layout =
            gfx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("OutlineRenderer.bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout = gfx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("OutlineRenderer.pipeline_layout"),
                bind_group_layouts: &[viewport.bind_group_layout(), &bind_group_layout],
                push_constant_ranges: &[],
            });
        let fragment_module = gfx
            .device
            .create_shader_module(wgpu::include_wgsl!("cursor_outline.wgsl"));
        let render_pipeline = gfx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("OutlineRenderer.render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: screen_vertex_shader.vertex_state(),
                primitive: screen_vertex_shader.primitive_state(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gfx.render_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: Default::default(),
                    })],
                }),
                multiview: None,
            });

        let depth_sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("OutlineRenderer.depth_sampler"),
            ..Default::default()
        });
        let uniforms = Uniforms::default();
        let uniform_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("OutlineRenderer.uniform_buffer"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        Self {
            gfx: gfx.clone(),
            screen_vertex_shader,
            render_pipeline,
            bind_group_layout,
            depth_sampler,
            uniform_buffer,
            uniforms,
        }
    }

    pub fn draw(
        &mut self,
        viewport: &Viewport,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) {
        let bind_group = self
            .gfx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("OutlineRenderer.bind_group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.depth_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                ],
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("OutlineRenderer.render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.screen_vertex_shader.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            self.screen_vertex_shader.index_buffer.slice(..),
            self.screen_vertex_shader.index_format(),
        );
        render_pass.set_bind_group(0, viewport.bind_group(), &[]);
        render_pass.set_bind_group(1, &bind_group, &[]);
        render_pass.draw_indexed(
            self.screen_vertex_shader.indices(),
            self.screen_vertex_shader.base_vertex(),
            self.screen_vertex_shader.instances(),
        );
    }

    pub fn set_outline_color(&mut self, color: Vec3) {
        self.uniforms.outline_color = color.into();
        self.update_uniform_buffer();
    }

    fn update_uniform_buffer(&self) {
        self.gfx
            .queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Uniforms {
    outline_color: [f32; 3],
    padding: [u8; 4],
}

impl Default for Uniforms {
    fn default() -> Self {
        Self {
            outline_color: [0.0, 0.0, 1.0],
            padding: [0; 4],
        }
    }
}
