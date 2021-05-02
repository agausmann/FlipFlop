use crate::viewport::Viewport;
use crate::GraphicsContext;
use bytemuck::{Pod, Zeroable};
use glam::{IVec2, Vec2};
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

static VERTEX_ATTRIBUTES: Lazy<[wgpu::VertexAttribute; 1]> = Lazy::new(|| {
    wgpu::vertex_attr_array![
        0 => Float32x2,
    ]
});

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
    is_powered: u32,
}

static INSTANCE_ATTRIBUTES: Lazy<[wgpu::VertexAttribute; 3]> =
    Lazy::new(|| {
        wgpu::vertex_attr_array![
            1 => Float32x2,
            2 => Float32x2,
            3 => Uint32,
        ]
    });

impl Instance {
    fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>().try_into().unwrap(),
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES[..],
        }
    }

    fn new(wire: &WireRect) -> Self {
        Self {
            position: wire.position.into(),
            size: wire.size.into(),
            is_powered: wire.is_powered as u32,
        }
    }
}

const WIRE_RADIUS: f32 = 1.0 / 16.0;
const PIN_RADIUS: f32 = 2.0 / 16.0;

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

pub struct WireRenderer {
    gfx: GraphicsContext,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,

    wire_color_buffer: wgpu::Buffer,

    instances: Vec<Instance>,
    instance_to_handle: Vec<Handle>,
    handle_to_instance: HashMap<Handle, usize>,
    buffer_update: bool,
}

impl WireRenderer {
    pub fn new(gfx: GraphicsContext, viewport: &Viewport) -> Self {
        let bind_group_layout = gfx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("WireRenderer.bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            },
        );

        let pipeline_layout = gfx.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("WireRenderer.pipeline_layout"),
                bind_group_layouts: &[
                    viewport.bind_group_layout(),
                    &bind_group_layout,
                ],
                push_constant_ranges: &[],
            },
        );
        let vertex_module =
            gfx.device
                .create_shader_module(&wgpu::include_spirv!(concat!(
                    env!("OUT_DIR"),
                    "/shaders/wire.vert.spv"
                )));
        let fragment_module =
            gfx.device
                .create_shader_module(&wgpu::include_spirv!(concat!(
                    env!("OUT_DIR"),
                    "/shaders/wire.frag.spv"
                )));
        let render_pipeline = gfx.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("WireRenderer.render_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_module,
                    entry_point: "main",
                    buffers: &[
                        Vertex::buffer_layout(),
                        Instance::buffer_layout(),
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    clamp_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: gfx.depth_format,
                    //depth_write_enabled: true,
                    //depth_compare: wgpu::CompareFunction::GreaterEqual,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_module,
                    entry_point: "main",
                    targets: &[wgpu::ColorTargetState {
                        format: gfx.render_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrite::ALL,
                    }],
                }),
            },
        );
        let vertex_buffer =
            gfx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("WireRenderer.vertex_buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsage::VERTEX,
                });
        let index_buffer =
            gfx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("WireRenderer.index_buffer"),
                    contents: bytemuck::cast_slice(INDICES),
                    usage: wgpu::BufferUsage::INDEX,
                });
        let instance_buffer =
            gfx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("WireRenderer.instance_buffer"),
                size: INSTANCE_BUFFER_SIZE,
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            });

        let wire_color_buffer =
            gfx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("WireRenderer.wire_color_buffer"),
                    contents: bytemuck::bytes_of(&WireColor::default()),
                    usage: wgpu::BufferUsage::UNIFORM
                        | wgpu::BufferUsage::COPY_DST,
                });

        let bind_group =
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("WireRenderer.bind_group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wire_color_buffer.as_entire_binding(),
                }],
            });

        Self {
            gfx,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            bind_group,

            wire_color_buffer,

            instances: Vec::new(),
            instance_to_handle: Vec::new(),
            handle_to_instance: HashMap::new(),
            buffer_update: false,
        }
    }

    pub fn insert(&mut self, wire: &WireRect) -> Handle {
        let handle = Handle::new();
        self.update(&handle, wire);
        handle
    }

    pub fn update(&mut self, handle: &Handle, wire: &WireRect) {
        self.buffer_update = true;
        if let Some(&index) = self.handle_to_instance.get(handle) {
            self.instances[index] = Instance::new(wire);
        } else {
            let index = self.instances.len();
            self.instances.push(Instance::new(wire));
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

            if index != self.instances.len() {
                // Update handle association for the instance that was swapped to this location.
                let affected_handle = &self.instance_to_handle[index];
                self.handle_to_instance
                    .insert(affected_handle.clone(), index);
            }

            true
        } else {
            false
        }
    }

    pub fn update_wire_color(&mut self, wire_color: &WireColor) {
        self.gfx.queue.write_buffer(
            &self.wire_color_buffer,
            0,
            bytemuck::bytes_of(wire_color),
        );
    }

    pub fn draw<'a>(
        &'a mut self,
        viewport: &'a Viewport,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        if self.buffer_update {
            self.buffer_update = false;
            let src_bytes: &[u8] = bytemuck::cast_slice(&self.instances);
            self.gfx
                .queue
                .write_buffer(&self.instance_buffer, 0, src_bytes);
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.set_bind_group(0, viewport.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.bind_group, &[]);
        render_pass.draw_indexed(
            0..INDICES.len().try_into().unwrap(),
            0,
            0..self.instances.len().try_into().expect("too many instances"),
        );
    }
}

pub struct WireRect {
    pub position: Vec2,
    pub size: Vec2,
    pub is_powered: bool,
}

pub struct Wire {
    pub start: IVec2,
    pub end: IVec2,
    pub is_powered: bool,
}

impl From<Wire> for WireRect {
    fn from(wire: Wire) -> Self {
        let position = wire.start;
        let size = wire.end - wire.start;

        // Ensure size is positive so WIRE_RADIUS offset will work correctly.
        let abs_size = size.abs();
        let abs_position = position - (abs_size - size) / 2;
        Self {
            position: abs_position.as_f32() + Vec2::splat(0.5 - WIRE_RADIUS),
            size: abs_size.as_f32() + Vec2::splat(2.0 * WIRE_RADIUS),
            is_powered: wire.is_powered,
        }
    }
}

pub struct Pin {
    pub position: IVec2,
    pub is_powered: bool,
}

impl From<Pin> for WireRect {
    fn from(pin: Pin) -> Self {
        Self {
            position: pin.position.as_f32() + Vec2::splat(0.5 - PIN_RADIUS),
            size: Vec2::splat(2.0 * PIN_RADIUS),
            is_powered: pin.is_powered,
        }
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct WireColor {
    pub off_color: [f32; 4],
    pub on_color: [f32; 4],
}

impl Default for WireColor {
    fn default() -> Self {
        Self {
            off_color: [0.0, 0.0, 0.0, 1.0],
            on_color: [0.8, 0.0, 0.0, 1.0],
        }
    }
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
