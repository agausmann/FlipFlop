use crate::instance::InstanceManager;
use crate::viewport::Viewport;
use crate::GraphicsContext;
use bytemuck::{Pod, Zeroable};
use glam::{IVec2, Vec2, Vec4};
use once_cell::sync::Lazy;
use std::convert::TryInto;
use wgpu::util::DeviceExt;

pub use crate::instance::Handle;

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
    color: [f32; 4],
}

static INSTANCE_ATTRIBUTES: Lazy<[wgpu::VertexAttribute; 3]> =
    Lazy::new(|| {
        wgpu::vertex_attr_array![
            1 => Float32x2,
            2 => Float32x2,
            3 => Float32x4,
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

    fn new(rect: &Rect) -> Self {
        Self {
            position: rect.position.into(),
            size: rect.size.into(),
            color: rect.color.into(),
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

pub struct RectRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    instances: InstanceManager<Instance>,
}

impl RectRenderer {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let bind_group_layout = gfx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("RectRenderer.bind_group_layout"),
                entries: &[],
            },
        );

        let pipeline_layout = gfx.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("RectRenderer.pipeline_layout"),
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
                    "/shaders/rect.vert.spv"
                )));
        let fragment_module =
            gfx.device
                .create_shader_module(&wgpu::include_spirv!(concat!(
                    env!("OUT_DIR"),
                    "/shaders/rect.frag.spv"
                )));
        let render_pipeline = gfx.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("RectRenderer.render_pipeline"),
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
                    label: Some("RectRenderer.vertex_buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsage::VERTEX,
                });
        let index_buffer =
            gfx.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("RectRenderer.index_buffer"),
                    contents: bytemuck::cast_slice(INDICES),
                    usage: wgpu::BufferUsage::INDEX,
                });

        let bind_group =
            gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("RectRenderer.bind_group"),
                layout: &bind_group_layout,
                entries: &[],
            });

        let instances = InstanceManager::new(gfx);

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            bind_group,
            instances,
        }
    }

    pub fn insert(&mut self, rect: &Rect) -> Handle {
        self.instances.insert(Instance::new(rect))
    }

    pub fn update(&mut self, handle: &Handle, rect: &Rect) {
        self.instances.update(handle, Instance::new(rect));
    }

    pub fn remove(&mut self, handle: &Handle) -> bool {
        self.instances.remove(handle)
    }

    pub fn draw<'a>(
        &'a mut self,
        viewport: &'a Viewport,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        let instance_count = self.instances.len();
        let instance_buffer = match self.instances.buffer() {
            Some(buffer) => buffer,
            None => return,
        };

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.slice(..),
            wgpu::IndexFormat::Uint16,
        );
        render_pass.set_bind_group(0, viewport.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.bind_group, &[]);
        render_pass.draw_indexed(
            0..INDICES.len().try_into().unwrap(),
            0,
            0..instance_count.try_into().expect("too many instances"),
        );
    }
}

pub struct Rect {
    pub position: Vec2,
    pub size: Vec2,
    pub color: Vec4,
}

pub struct Wire {
    pub start: IVec2,
    pub end: IVec2,
    pub is_powered: bool,
}

impl From<Wire> for Rect {
    fn from(wire: Wire) -> Self {
        let position = wire.start;
        let size = wire.end - wire.start;

        // Ensure size is positive so WIRE_RADIUS offset will work correctly.
        let abs_size = size.abs();
        let abs_position = position - (abs_size - size) / 2;
        Self {
            position: abs_position.as_f32() + Vec2::splat(0.5 - WIRE_RADIUS),
            size: abs_size.as_f32() + Vec2::splat(2.0 * WIRE_RADIUS),
            color: wire_color(wire.is_powered),
        }
    }
}

pub struct Pin {
    pub position: IVec2,
    pub is_powered: bool,
}

impl From<Pin> for Rect {
    fn from(pin: Pin) -> Self {
        Self {
            position: pin.position.as_f32() + Vec2::splat(0.5 - PIN_RADIUS),
            size: Vec2::splat(2.0 * PIN_RADIUS),
            color: wire_color(pin.is_powered),
        }
    }
}

fn wire_color(is_powered: bool) -> Vec4 {
    if is_powered {
        Vec4::new(0.8, 0.0, 0.0, 1.0)
    } else {
        Vec4::new(0.0, 0.0, 0.0, 1.0)
    }
}
