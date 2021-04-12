use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, SquareMatrix, Vector2, Vector4, Zero};
use wgpu::util::DeviceExt;

#[derive(Clone)]
pub struct Camera {
    pub pan: Vector2<f32>,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: Vector2::zero(),
            zoom: 16.0,
        }
    }
}

pub struct ViewTransform {
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,

    buffer_updates: bool,
    size: Vector2<f32>,
    camera: Camera,
}

impl ViewTransform {
    pub fn new(device: &wgpu::Device, size: Vector2<f32>) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ViewTransform.uniform_buffer"),
            contents: bytemuck::bytes_of(&Uniforms::default()),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ViewTransform.bind_group_layout"),
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
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ViewTransform.bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            uniform_buffer,
            bind_group_layout,
            bind_group,

            buffer_updates: true,
            size,
            camera: Default::default(),
        }
    }

    pub fn window_resized(&mut self, size: Vector2<f32>) {
        self.buffer_updates = true;
        self.size = size;
    }

    pub fn camera_update(&mut self, camera: Camera) {
        self.buffer_updates = true;
        self.camera = camera;
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        if self.buffer_updates {
            self.buffer_updates = false;
            queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::bytes_of(&Uniforms::new(self.size, &self.camera)),
            );
        }
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        assert!(!self.buffer_updates, "ViewTransform buffer is stale");
        &self.bind_group
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view: [[f32; 4]; 4],
}

impl Uniforms {
    fn default() -> Self {
        Self {
            view: Matrix4::identity().into(),
        }
    }

    fn new(size: Vector2<f32>, camera: &Camera) -> Self {
        let proj = Matrix4 {
            x: 2.0 / size.x * Vector4::unit_x(),
            y: 2.0 / size.y * Vector4::unit_y(),
            z: Vector4::unit_z(),
            w: Vector4::unit_w(),
        };
        let view = Matrix4::from_nonuniform_scale(camera.zoom, camera.zoom, 1.0)
            * Matrix4::from_translation(camera.pan.extend(0.0));
        Self {
            view: (proj * view).into(),
        }
    }
}
