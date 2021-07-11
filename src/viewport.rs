use crate::GraphicsContext;
use bytemuck::{Pod, Zeroable};
use glam::{IVec2, Mat4, Vec2, Vec3, Vec4};
use std::time::Duration;
use wgpu::util::DeviceExt;

pub struct Camera {
    pub pan: Vec2,
    pub zoom: f32,

    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub zoom_step: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,

    pub pan_up: bool,
    pub pan_down: bool,
    pub pan_left: bool,
    pub pan_right: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
}

impl Camera {
    fn new() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 16.0,

            pan_speed: 500.0,
            zoom_speed: 4.0,
            zoom_step: 1.1,
            min_zoom: 8.0,
            max_zoom: 64.0,

            pan_up: false,
            pan_down: false,
            pan_left: false,
            pan_right: false,
            zoom_in: false,
            zoom_out: false,
        }
    }

    fn update(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();
        let mut pan_delta = Vec2::ZERO;
        if self.pan_up {
            pan_delta += Vec2::Y;
        }
        if self.pan_down {
            pan_delta -= Vec2::Y;
        }
        if self.pan_right {
            pan_delta += Vec2::X;
        }
        if self.pan_left {
            pan_delta -= Vec2::X;
        }
        self.pan += dt * self.pan_speed / self.zoom * pan_delta;

        let mut zoom_factor = 1.0;
        if self.zoom_in {
            zoom_factor *= self.zoom_speed;
        }
        if self.zoom_out {
            zoom_factor /= self.zoom_speed;
        }
        self.set_zoom(self.zoom * zoom_factor.powf(dt));
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(self.min_zoom, self.max_zoom);
    }
}

pub struct Cursor {
    pub screen_position: Vec2,
    pub world_position: Vec2,
}

impl Cursor {
    fn new() -> Self {
        Self {
            screen_position: Vec2::ZERO,
            world_position: Vec2::ZERO,
        }
    }

    fn update(&mut self, gfx: &GraphicsContext, camera: &Camera) {
        let size = Vec2::new(
            gfx.window.inner_size().width as f32,
            gfx.window.inner_size().height as f32,
        );
        self.world_position =
            (self.screen_position - size / 2.0) * Vec2::new(1.0, -1.0) / camera.zoom + camera.pan;
    }

    pub fn tile(&self) -> IVec2 {
        self.world_position.floor().as_i32()
    }
}

pub struct Viewport {
    gfx: GraphicsContext,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    camera: Camera,
    cursor: Cursor,
}

impl Viewport {
    pub fn new(gfx: &GraphicsContext) -> Self {
        let uniform_buffer = gfx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Viewport.uniform_buffer"),
                contents: bytemuck::bytes_of(&Uniforms::default()),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            });
        let bind_group_layout =
            gfx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Viewport.bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Viewport.bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            gfx: gfx.clone(),
            uniform_buffer,
            bind_group_layout,
            bind_group,
            camera: Camera::new(),
            cursor: Cursor::new(),
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.camera.update(dt);
        self.cursor.update(&self.gfx, &self.camera);

        let size = Vec2::new(
            self.gfx.window.inner_size().width as f32,
            self.gfx.window.inner_size().height as f32,
        );
        self.gfx.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms::new(size, &self.camera)),
        );
    }

    pub fn cursor_moved(&mut self, position: Vec2) {
        self.cursor.screen_position = position;
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

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    view_size: [f32; 2],
}

impl Uniforms {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            view_size: [1.0, 1.0],
        }
    }

    fn new(size: Vec2, camera: &Camera) -> Self {
        let proj = Mat4::from_cols(
            2.0 / size.x * Vec4::X,
            2.0 / size.y * Vec4::Y,
            Vec4::Z,
            Vec4::W,
        );
        let view = Mat4::from_scale(Vec3::new(camera.zoom, camera.zoom, 1.0))
            * Mat4::from_translation(-camera.pan.extend(0.0));
        Self {
            view_proj: (proj * view).to_cols_array_2d(),
            view_size: size.into(),
        }
    }
}
