use crate::view::ViewTransform;
use bytemuck::{Pod, Zeroable};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicU64, Ordering};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
}

static VERTEX_ATTRIBUTES: Lazy<[wgpu::VertexAttribute; 1]> =
    Lazy::new(|| wgpu::vertex_attr_array![0 => Float2]);

impl Vertex {
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>().try_into().unwrap(),
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES[..],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Instance {
    position: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
}

static INSTANCE_ATTRIBUTES: Lazy<[wgpu::VertexAttribute; 3]> =
    Lazy::new(|| wgpu::vertex_attr_array![1 => Float2, 2 => Float2, 3 => Float4]);

impl Instance {
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>().try_into().unwrap(),
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES[..],
        }
    }

    fn new(board: &Board) -> Self {
        Self {
            position: board.position,
            size: board.size,
            color: board.color,
        }
    }
}

//XXX this belongs somewhere else
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Camera {
    pan: [f32; 2],
    zoom: f32,
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

const INSTANCE_BUFFER_SIZE: wgpu::BufferAddress = 1 * 1024 * 1024; // 1MB

pub struct BoardRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    texture_bind_group: wgpu::BindGroup,

    instances: Vec<Instance>,
    instance_to_handle: Vec<Handle>,
    handle_to_instance: HashMap<Handle, usize>,
    buffer_update: bool,
}

impl BoardRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        view_transform: &ViewTransform,
    ) -> Self {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("BoardRenderer.pipeline_layout"),
            bind_group_layouts: &[
                view_transform.bind_group_layout(),
                &texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let vertex_module = device.create_shader_module(&wgpu::include_spirv!(concat!(
            env!("OUT_DIR"),
            "/shaders/board.vert.spv"
        )));
        let fragment_module = device.create_shader_module(&wgpu::include_spirv!(concat!(
            env!("OUT_DIR"),
            "/shaders/board.frag.spv"
        )));
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("BoardRenderer.render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_module,
                entry_point: "main",
                buffers: &[Vertex::buffer_layout(), Instance::buffer_layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    color_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BoardRenderer.vertex_buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BoardRenderer.index_buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsage::INDEX,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("BoardRenderer.instance_buffer"),
            size: INSTANCE_BUFFER_SIZE,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let board_image = image::load_from_memory(include_bytes!("textures/board.png"))
            .expect("failed to load board texture")
            .into_rgba8();
        let size = wgpu::Extent3d {
            width: board_image.width(),
            height: board_image.height(),
            ..Default::default()
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("BoardRenderer.texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &board_image,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * size.width,
                rows_per_image: size.height,
            },
            size,
        );
        let texture_view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("BoardRenderer.sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            texture,
            texture_view,
            sampler,
            texture_bind_group,

            instances: Vec::new(),
            instance_to_handle: Vec::new(),
            handle_to_instance: HashMap::new(),
            buffer_update: false,
        }
    }

    pub fn insert(&mut self, board: &Board) -> Handle {
        let handle = Handle::new();
        self.update(&handle, board);
        handle
    }

    pub fn update(&mut self, handle: &Handle, board: &Board) {
        self.buffer_update = true;
        if let Some(&index) = self.handle_to_instance.get(handle) {
            self.instances[index] = Instance::new(board);
        } else {
            let index = self.instances.len();
            self.instances.push(Instance::new(board));
            self.instance_to_handle.push(handle.clone());
            self.handle_to_instance.insert(handle.clone(), index);
        }
    }

    pub fn remove(&mut self, handle: &Handle) -> bool {
        // If the handle exists for this renderer:
        if let Some(index) = self.handle_to_instance.remove(handle) {
            self.buffer_update = true;

            self.instances.swap_remove(index);

            let removed_handle = self.instance_to_handle.swap_remove(index);
            debug_assert!(removed_handle == *handle);

            // Update handle association for the instance that was swapped to this location.
            let affected_handle = &self.instance_to_handle[index];
            self.handle_to_instance
                .insert(affected_handle.clone(), index);

            true
        } else {
            false
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        view_transform: &'a ViewTransform,
        queue: &wgpu::Queue,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        if self.buffer_update {
            self.buffer_update = false;
            let src_bytes: &[u8] = bytemuck::cast_slice(&self.instances);
            queue.write_buffer(&self.instance_buffer, 0, src_bytes);
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_bind_group(0, view_transform.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.draw_indexed(
            0..INDICES.len().try_into().unwrap(),
            0,
            0..self.instances.len().try_into().expect("too many instances"),
        );
    }
}

pub struct Board {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    fn new() -> Self {
        let val = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        // Prevent overflow:
        if val == u64::MAX {
            panic!("max handle reached - how on earth did you do that?!")
        }
        Self(val)
    }
}
