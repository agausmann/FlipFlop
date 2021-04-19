use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, SquareMatrix, Vector2, Vector4, Zero};
use std::time::Duration;
use wgpu::util::DeviceExt;

pub struct Camera {
    pub pan: Vector2<f32>,
    pub zoom: f32,

    pub pan_speed: f32,
    pub zoom_speed: f32,

    pub pan_up: bool,
    pub pan_down: bool,
    pub pan_left: bool,
    pub pan_right: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
}

impl Camera {
    fn update(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();
        let mut pan_delta = Vector2::zero();
        if self.pan_up {
            pan_delta += Vector2::unit_y();
        }
        if self.pan_down {
            pan_delta -= Vector2::unit_y();
        }
        if self.pan_right {
            pan_delta += Vector2::unit_x();
        }
        if self.pan_left {
            pan_delta -= Vector2::unit_x();
        }
        self.pan += dt * self.pan_speed / self.zoom * pan_delta;

        let mut zoom_factor = 1.0;
        if self.zoom_in {
            zoom_factor *= self.zoom_speed;
        }
        if self.zoom_out {
            zoom_factor /= self.zoom_speed;
        }
        self.zoom *= zoom_factor.powf(dt);
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: Vector2::zero(),
            zoom: 16.0,

            pan_speed: 300.0,
            zoom_speed: 2.0,

            pan_up: false,
            pan_down: false,
            pan_left: false,
            pan_right: false,
            zoom_in: false,
            zoom_out: false,
        }
    }
}

pub struct Viewport {
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    camera: Camera,
}

impl Viewport {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Viewport.uniform_buffer"),
            contents: bytemuck::bytes_of(&Uniforms::default()),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Viewport.bind_group_layout"),
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
            label: Some("Viewport.bind_group"),
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
            camera: Default::default(),
        }
    }

    pub fn update(&mut self, dt: Duration, window: &winit::window::Window, queue: &wgpu::Queue) {
        self.camera.update(dt);

        let size = Vector2::new(
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        );
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms::new(size, &self.camera)),
        );
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

impl Uniforms {
    fn default() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
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
            * Matrix4::from_translation(-camera.pan.extend(0.0));
        Self {
            view_proj: (proj * view).into(),
        }
    }
}
