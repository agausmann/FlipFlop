pub mod board;
pub mod circuit;
pub mod counter;
pub mod cursor;
pub mod depot;
pub mod direction;
pub mod instance;
pub mod rect;
pub mod screen_vertex;
pub mod simulation;
pub mod viewport;

use crate::circuit::Circuit;
use crate::circuit::ComponentType;
use crate::counter::Counter;
use crate::cursor::{CursorManager, CursorState};
use crate::direction::Direction;
use crate::viewport::Viewport;
use anyhow::Context;
use futures_executor::block_on;
use glam::Vec2;
use std::sync::Arc;
use std::time::Instant;
use wgpu_glyph::ab_glyph::FontArc;
use wgpu_glyph::{GlyphBrushBuilder, Section, Text};
use winit::event::{
    ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorIcon, Window, WindowBuilder};

const HELP_TEXT: &str = "\
Controls (press F1 to show/hide):
Camera Pan - WASD or arrow keys
    or middle click and drag
Camera Zoom - Scroll or PgUp/PgDn
Place Component - Left click
Place Wire - Left click and drag
Remove Component/Wire - Right click
Rotate Component - R
1 - Pin/Wire
2 - Flip
3 - Flop
";

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
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
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
        let render_format = surface
            .get_preferred_format(&adapter)
            .context("failed to select render format")?;
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

    fn reconfigure(&self) {
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.render_format,
                width: self.window.inner_size().width,
                height: self.window.inner_size().height,
                present_mode: wgpu::PresentMode::Fifo,
            },
        )
    }
}

struct State {
    gfx: GraphicsContext,
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
    draw_help: bool,
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    })
}

impl State {
    async fn new(window: Window) -> anyhow::Result<Self> {
        let gfx = Arc::new(GraphicsContextInner::new(window).await?);
        gfx.reconfigure();
        let depth_texture = create_depth_texture(&gfx);
        let depth_texture_view = depth_texture.create_view(&Default::default());

        let fira_sans = FontArc::try_from_slice(include_bytes!("fonts/FiraSans-Regular.ttf"))?;
        let glyph_brush =
            GlyphBrushBuilder::using_font(fira_sans).build(&gfx.device, gfx.render_format);
        let staging_belt = wgpu::util::StagingBelt::new(1024);
        let local_pool = futures_executor::LocalPool::new();
        let local_spawner = local_pool.spawner();

        let viewport = Viewport::new(&gfx);

        let circuit = Circuit::new(&gfx, &viewport);
        let cursor_manager = CursorManager::new(&gfx, &viewport);

        Ok(Self {
            gfx,
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
            draw_help: true,
        })
    }

    fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.should_close = true;
            }
            WindowEvent::Resized(..) | WindowEvent::ScaleFactorChanged { .. } => {
                self.reconfigure();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let position = Vec2::new(position.x as f32, position.y as f32);
                self.viewport.cursor_moved(position);
            }
            WindowEvent::MouseInput { button, state, .. } => match (button, state) {
                (MouseButton::Middle, ElementState::Pressed) => {
                    self.cursor_manager.start_pan(&self.viewport);
                    self.gfx.window.set_cursor_icon(CursorIcon::Grabbing);
                }
                (MouseButton::Middle, ElementState::Released) => {
                    match self.cursor_manager.current_state() {
                        CursorState::Pan { .. } => {
                            self.cursor_manager.end();
                            self.gfx.window.set_cursor_icon(CursorIcon::Default);
                        }
                        _ => {}
                    }
                }
                (MouseButton::Left, ElementState::Pressed) => {
                    match self.cursor_manager.place_type() {
                        ComponentType::Pin => {
                            self.cursor_manager.start_place_wire(&self.viewport);
                        }
                        other_type => {
                            self.circuit.place_component(
                                other_type,
                                self.viewport.cursor().tile(),
                                self.cursor_manager.place_orientation(),
                            );
                        }
                    }
                }
                (MouseButton::Left, ElementState::Released) => {
                    match self.cursor_manager.current_state() {
                        &CursorState::PlaceWire {
                            start_position,
                            end_position,
                            ..
                        } => {
                            if start_position == end_position {
                                if self.circuit.component_at(start_position)
                                    == Some(ComponentType::Pin)
                                {
                                    self.circuit.delete_component(start_position);
                                } else {
                                    self.circuit.place_component(
                                        ComponentType::Pin,
                                        start_position,
                                        Direction::East,
                                    );
                                }
                            } else {
                                self.circuit.place_wire(start_position, end_position);
                            }
                            self.cursor_manager.end();
                        }
                        _ => {}
                    }
                }
                (MouseButton::Right, ElementState::Pressed) => {
                    match &self.cursor_manager.current_state() {
                        &CursorState::Normal => {
                            let position = self.viewport.cursor().tile();
                            self.circuit.delete_all_at(position);
                        }
                        _ => {}
                    }
                }
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } => match &self.cursor_manager.current_state() {
                CursorState::Normal => {
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(_x, y) => y,
                        MouseScrollDelta::PixelDelta(position) => position.y as f32 / 16.0,
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
                        VirtualKeyCode::Up | VirtualKeyCode::W => {
                            self.viewport.camera_mut().pan_up = pressed;
                        }
                        VirtualKeyCode::Down | VirtualKeyCode::S => {
                            self.viewport.camera_mut().pan_down = pressed;
                        }
                        VirtualKeyCode::Left | VirtualKeyCode::A => {
                            self.viewport.camera_mut().pan_left = pressed;
                        }
                        VirtualKeyCode::Right | VirtualKeyCode::D => {
                            self.viewport.camera_mut().pan_right = pressed;
                        }
                        VirtualKeyCode::PageUp => {
                            self.viewport.camera_mut().zoom_in = pressed;
                        }
                        VirtualKeyCode::PageDown => {
                            self.viewport.camera_mut().zoom_out = pressed;
                        }
                        VirtualKeyCode::Key1 if pressed => {
                            self.cursor_manager.set_place_type(ComponentType::Pin);
                        }
                        VirtualKeyCode::Key2 if pressed => {
                            self.cursor_manager.set_place_type(ComponentType::Flip);
                        }
                        VirtualKeyCode::Key3 if pressed => {
                            self.cursor_manager.set_place_type(ComponentType::Flop);
                        }
                        VirtualKeyCode::R if pressed => {
                            self.cursor_manager.set_place_orientation(
                                self.cursor_manager.place_orientation().right(),
                            );
                        }
                        VirtualKeyCode::F1 if pressed => {
                            self.draw_help = !self.draw_help;
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

        self.cursor_manager
            .update(&mut self.viewport, &self.circuit);
        self.viewport.update(dt);
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        self.frame_counter.tick();

        let frame = loop {
            match self.gfx.surface.get_current_texture() {
                Ok(frame) => break frame,
                Err(wgpu::SurfaceError::Lost) => {
                    self.reconfigure();
                }
                Err(wgpu::SurfaceError::Timeout) | Err(wgpu::SurfaceError::Outdated) => {
                    return Ok(());
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        };

        let frame_view = frame.texture.create_view(&Default::default());

        let mut encoder = self.gfx.device.create_command_encoder(&Default::default());

        {
            self.circuit.draw(
                &self.viewport,
                &mut encoder,
                &frame_view,
                &self.depth_texture_view,
            );
            self.cursor_manager.draw(
                &self.viewport,
                &mut encoder,
                &frame_view,
                &self.depth_texture_view,
            );
        }

        let size = self.gfx.window.inner_size();
        self.glyph_brush.queue(Section {
            screen_position: (0.0, 0.0),
            bounds: (size.width as f32 / 2.0, size.height as f32),
            text: vec![Text::new(&self.debug_text())
                .with_color([1.0, 1.0, 1.0, 1.0])
                .with_scale(18.0)],
            ..Default::default()
        });
        if self.draw_help {
            self.glyph_brush.queue(Section {
                screen_position: (size.width as f32 / 2.0, 0.0),
                bounds: (size.width as f32 / 2.0, size.height as f32),
                text: vec![Text::new(HELP_TEXT)
                    .with_color([1.0, 1.0, 1.0, 1.0])
                    .with_scale(18.0)],
                ..Default::default()
            });
        }
        self.glyph_brush
            .draw_queued(
                &self.gfx.device,
                &mut self.staging_belt,
                &mut encoder,
                &frame_view,
                size.width,
                size.height,
            )
            .expect("Text draw error");
        self.staging_belt.finish();

        self.gfx.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        use futures_util::task::SpawnExt;
        self.local_spawner
            .spawn(self.staging_belt.recall())
            .expect("Recall error");
        self.local_pool.run_until_stalled();

        Ok(())
    }

    fn debug_text(&self) -> String {
        let fps = self.frame_counter.rate();
        let cursor_pos = <(f32, f32)>::from(self.viewport.cursor().screen_position);
        let world_pos = <(f32, f32)>::from(self.viewport.cursor().world_position);
        let cursor_tile = <(i32, i32)>::from(self.viewport.cursor().tile());
        let tile_debug_info = self.circuit.tile_debug_info(self.viewport.cursor().tile());

        format!(
            "FPS: {:.0}\n\
            Cursor: {:.0?}\n\
            World: {:.2?}\n\
            Tile: {:?}\n\
            {}",
            fps, cursor_pos, world_pos, cursor_tile, tile_debug_info,
        )
    }

    fn reconfigure(&mut self) {
        self.gfx.reconfigure();
        self.depth_texture = create_depth_texture(&self.gfx);
        self.depth_texture_view = self.depth_texture.create_view(&Default::default());
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // The window decorations provided by winit when using wayland do not match the native system
    // theme, so fallback to X11 via XWayland if possible.
    std::env::set_var("WINIT_UNIX_BACKEND", "x11");

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
