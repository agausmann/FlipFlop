use crate::GraphicsContext;
use bytemuck::{Pod, Zeroable};
use once_cell::sync::{Lazy, OnceCell};
use std::convert::TryInto;
use std::ops::Range;
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

static BUFFER_LAYOUTS: Lazy<[wgpu::VertexBufferLayout<'static>; 1]> =
    Lazy::new(|| [Vertex::buffer_layout()]);

static INSTANCE: OnceCell<ScreenVertexShader> = OnceCell::new();

pub struct ScreenVertexShader {
    pub vertex_module: wgpu::ShaderModule,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

impl ScreenVertexShader {
    pub fn get(gfx: &GraphicsContext) -> &'static Self {
        INSTANCE.get_or_init(|| Self::new(gfx))
    }

    pub fn vertex_state(&self) -> wgpu::VertexState {
        wgpu::VertexState {
            module: &self.vertex_module,
            entry_point: "main",
            buffers: &*BUFFER_LAYOUTS,
        }
    }

    pub fn primitive_state(&self) -> wgpu::PrimitiveState {
        wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: None,
            clamp_depth: false,
            polygon_mode: Default::default(),
            conservative: false,
        }
    }

    pub fn index_format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint16
    }

    pub fn indices(&self) -> Range<u32> {
        0..INDICES.len().try_into().unwrap()
    }

    pub fn base_vertex(&self) -> i32 {
        0
    }

    pub fn instances(&self) -> Range<u32> {
        0..1
    }

    fn new(gfx: &GraphicsContext) -> Self {
        let vertex_module = gfx
            .device
            .create_shader_module(&wgpu::include_spirv!(concat!(
                env!("OUT_DIR"),
                "/shaders/screen.vert.spv"
            )));
        let vertex_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ScreenVertexShader.vertex_buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsage::VERTEX,
            });
        let index_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ScreenVertexShader.index_buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsage::INDEX,
            });

        Self {
            vertex_module,
            vertex_buffer,
            index_buffer,
        }
    }
}
