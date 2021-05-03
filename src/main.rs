pub mod board;
pub mod circuit;
pub mod counter;
pub mod cursor;
pub mod instance;
pub mod rect;
pub mod viewport;

use crate::circuit::Circuit;
use crate::counter::Counter;
use crate::cursor::{CursorManager, CursorMode};
use crate::viewport::Viewport;
use anyhow::Context;
use futures_executor::block_on;
use glam::Vec2;
use std::sync::Arc;
use std::time::Instant;
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text};
use winit::event::{
    ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode,
    WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorIcon, Window, WindowBuilder};

pub type GraphicsContext = Arc<GraphicsContextInner>;

pub struct GraphicsContextInner {
    pub window: Window,
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub render_format: wgpu::TextureFormat,
    pub depth_format: wgpu::TextureFormat,
}

impl GraphicsContextInner {
    async fn new(window: Window) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .context("Failed to find a suitable adapter")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: Default::default(),
                    limits: Default::default(),
                },
                None,
            )
            .await
            .context("Failed to open device")?;

        // XXX does this produce incompatible formats on different backends?
        let render_format =
            adapter.get_swap_chain_preferred_format(&surface).unwrap();
        let depth_format = wgpu::TextureFormat::Depth32Float;

        Ok(Self {
            window,
            surface,
            device,
            queue,
            render_format,
            depth_format,
        })
    }
}

struct State {
    gfx: GraphicsContext,
    swap_chain: wgpu::SwapChain,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    staging_belt: wgpu::util::StagingBelt,
    local_pool: futures_executor::LocalPool,
    local_spawner: futures_executor::LocalSpawner,
    viewport: Viewport,
    frame_counter: Counter,
    should_close: bool,
    last_update: Instant,
    circuit: Circuit,
    cursor_manager: CursorManager,
}

fn create_swap_chain(gfx: &GraphicsContext) -> wgpu::SwapChain {
    gfx.device.create_swap_chain(
        &gfx.surface,
        &wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: gfx.render_format,
            width: gfx.window.inner_size().width,
            height: gfx.window.inner_size().height,
            //present_mode: wgpu::PresentMode::Fifo,
            present_mode: wgpu::PresentMode::Mailbox,
        },
    )
}
fn create_depth_texture(gfx: &GraphicsContext) -> wgpu::Texture {
    gfx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: wgpu::Extent3d {
            width: gfx.window.inner_size().width,
            height: gfx.window.inner_size().height,
            ..Default::default()
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: gfx.depth_format,
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT
            | wgpu::TextureUsage::SAMPLED,
    })
}

impl State {
    async fn new(window: Window) -> anyhow::Result<Self> {
        let gfx = Arc::new(GraphicsContextInner::new(window).await?);
        let swap_chain = create_swap_chain(&gfx);
        let depth_texture = create_depth_texture(&gfx);
        let depth_texture_view = depth_texture.create_view(&Default::default());

        let fira_sans = FontArc::try_from_slice(include_bytes!(
            "fonts/FiraSans-Regular.ttf"
        ))?;
        let glyph_brush = GlyphBrushBuilder::using_font(fira_sans)
            .build(&gfx.device, gfx.render_format);
        let staging_belt = wgpu::util::StagingBelt::new(1024);
        let local_pool = futures_executor::LocalPool::new();
        let local_spawner = local_pool.spawner();

        let viewport = Viewport::new(&gfx);

        let circuit = Circuit::new(&gfx, &viewport);
        let cursor_manager = CursorManager::new(&gfx, &viewport);

        Ok(Self {
            gfx,
            swap_chain,
            depth_texture,
            depth_texture_view,
            glyph_brush,
            staging_belt,
            local_pool,
            local_spawner,
            viewport,
            frame_counter: Counter::new(),
            should_close: false,
            last_update: Instant::now(),
            circuit,
            cursor_manager,
        })
    }

    fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.should_close = true;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let position = Vec2::new(position.x as f32, position.y as f32);
                self.viewport.cursor_moved(position);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                match (button, state) {
                    (MouseButton::Middle, ElementState::Pressed) => {
                        self.cursor_manager.start_pan(&self.viewport);
                        self.gfx.window.set_cursor_icon(CursorIcon::Grabbing);
                    }
                    (MouseButton::Middle, ElementState::Released) => {
                        match self.cursor_manager.current_mode() {
                            CursorMode::Pan { .. } => {
                                self.cursor_manager.end();
                                self.gfx
                                    .window
                                    .set_cursor_icon(CursorIcon::Default);
                            }
                            _ => {}
                        }
                    }
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.cursor_manager.start_place(&self.viewport);
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        match self.cursor_manager.current_mode() {
                            &CursorMode::Place {
                                start_position,
                                end_position,
                                ..
                            } => {
                                if start_position == end_position {
                                    if self
                                        .circuit
                                        .tile(start_position)
                                        .and_then(|tile| tile.pin)
                                        .is_some()
                                    {
                                        self.circuit.delete_pin(start_position);
                                    } else {
                                        self.circuit.place_pin(start_position);
                                    }
                                } else {
                                    self.circuit.place_wire(
                                        start_position,
                                        end_position,
                                    );
                                }
                                self.cursor_manager.end();
                            }
                            _ => {}
                        }
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        match &self.cursor_manager.current_mode() {
                            &CursorMode::Normal => {
                                let position = self.viewport.cursor().tile();
                                self.circuit.delete_all_at(position);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match &self
                .cursor_manager
                .current_mode()
            {
                CursorMode::Normal => {
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(_x, y) => y,
                        MouseScrollDelta::PixelDelta(position) => {
                            position.y as f32 / 16.0
                        }
                    };
                    let camera = self.viewport.camera_mut();
                    camera.set_zoom(camera.zoom * camera.zoom_step.powf(delta));
                }
                _ => {}
            },
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    let pressed = match input.state {
                        ElementState::Pressed => true,
                        ElementState::Released => false,
                    };

                    match keycode {
                        VirtualKeyCode::Up => {
                            self.viewport.camera_mut().pan_up = pressed;
                        }
                        VirtualKeyCode::Down => {
                            self.viewport.camera_mut().pan_down = pressed;
                        }
                        VirtualKeyCode::Left => {
                            self.viewport.camera_mut().pan_left = pressed;
                        }
                        VirtualKeyCode::Right => {
                            self.viewport.camera_mut().pan_right = pressed;
                        }
                        VirtualKeyCode::PageUp => {
                            self.viewport.camera_mut().zoom_in = pressed;
                        }
                        VirtualKeyCode::PageDown => {
                            self.viewport.camera_mut().zoom_out = pressed;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        let dt = now - self.last_update;
        self.last_update = now;

        self.cursor_manager.update(&mut self.viewport);
        self.viewport.update(dt);
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        self.frame_counter.tick();

        let frame = loop {
            match self.swap_chain.get_current_frame() {
                Ok(frame) => break frame.output,
                Err(wgpu::SwapChainError::Lost)
                | Err(wgpu::SwapChainError::Outdated) => {
                    self.swap_chain = create_swap_chain(&self.gfx);

                    self.depth_texture = create_depth_texture(&self.gfx);
                    self.depth_texture_view =
                        self.depth_texture.create_view(&Default::default());
                }
                Err(wgpu::SwapChainError::Timeout) => {
                    return Ok(());
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        };

        let mut encoder =
            self.gfx.device.create_command_encoder(&Default::default());

        {
            let mut render_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[wgpu::RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachment {
                            view: &self.depth_texture_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        },
                    ),
                });
            self.circuit.draw(&self.viewport, &mut render_pass);
            self.cursor_manager.draw(&self.viewport, &mut render_pass);
        }

        let size = self.gfx.window.inner_size();
        self.glyph_brush.queue(Section {
            screen_position: (0.0, 0.0),
            bounds: (size.width as f32, size.height as f32),
            text: vec![Text::new(&self.debug_text())
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_scale(24.0)],
            ..Default::default()
        });
        self.glyph_brush
            .draw_queued(
                &self.gfx.device,
                &mut self.staging_belt,
                &mut encoder,
                &frame.view,
                size.width,
                size.height,
            )
            .expect("Text draw error");
        self.staging_belt.finish();

        self.gfx.queue.submit(std::iter::once(encoder.finish()));

        use futures_util::task::SpawnExt;
        self.local_spawner
            .spawn(self.staging_belt.recall())
            .expect("Recall error");
        self.local_pool.run_until_stalled();

        Ok(())
    }

    fn debug_text(&self) -> String {
        let tile = self.circuit.tile(self.viewport.cursor().tile());
        format!(
            "FPS: {:.0}\nCursor: {:.0?}\nWorld: {:.2?}\nTile: {:?}\nPin: {:?}\nWires: {:?}",
            self.frame_counter.rate(),
            <(f32, f32)>::from(self.viewport.cursor().screen_position),
            <(f32, f32)>::from(self.viewport.cursor().world_position),
            <(i32, i32)>::from(self.viewport.cursor().tile()),
            tile.and_then(|tile| tile.pin),
            tile.map(|tile| tile.wires),
        )
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("FlipFlop")
        .build(&event_loop)?;

    let mut state = block_on(State::new(window))?;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(..) => {
                state.update();
                state.redraw().unwrap();
            }
            Event::WindowEvent { event, .. } => {
                state.handle_window_event(event);
            }
            Event::MainEventsCleared => {
                state.gfx.window.request_redraw();
            }
            _ => {}
        }
        if state.should_close {
            *control_flow = ControlFlow::Exit;
        }
    });
}
