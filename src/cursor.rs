use crate::rect::{self, RectRenderer};
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::{IVec2, Vec2};

pub struct CursorManager {
    rect_renderer: RectRenderer,
    current_mode: CursorMode,
}

impl CursorManager {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        Self {
            rect_renderer: RectRenderer::new(gfx, viewport),
            current_mode: CursorMode::Normal,
        }
    }

    pub fn current_mode(&self) -> &CursorMode {
        &self.current_mode
    }

    pub fn update(&mut self, viewport: &mut Viewport) {
        match &mut self.current_mode {
            CursorMode::Normal => {}
            CursorMode::Pan { last_position } => {
                let position = viewport.cursor().screen_position;
                let delta = (position - *last_position) * Vec2::new(1.0, -1.0);
                let camera = viewport.camera_mut();
                camera.pan -= delta / camera.zoom;

                *last_position = position;
            }
            CursorMode::Place {
                start_position,
                end_position,
                start_pin,
                end_pin,
                wire,
            } => {
                let delta = viewport.cursor().tile() - *start_position;

                let size;
                if delta.x.abs() > delta.y.abs() {
                    size = delta * IVec2::X;
                } else {
                    size = delta * IVec2::Y;
                }
                *end_position = *start_position + size;

                self.rect_renderer.update(
                    start_pin,
                    &rect::Pin {
                        position: *start_position,
                        is_powered: false,
                    }
                    .into(),
                );
                self.rect_renderer.update(
                    end_pin,
                    &rect::Pin {
                        position: *end_position,
                        is_powered: false,
                    }
                    .into(),
                );
                self.rect_renderer.update(
                    wire,
                    &rect::Wire {
                        start: *start_position,
                        end: *end_position,
                        is_powered: false,
                    }
                    .into(),
                );
            }
        }
    }

    pub fn draw(
        &mut self,
        viewport: &Viewport,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) {
        self.rect_renderer
            .draw(viewport, encoder, frame_view, depth_view);
    }

    pub fn start_pan(&mut self, viewport: &Viewport) {
        self.replace(CursorMode::Pan {
            last_position: viewport.cursor().screen_position,
        });
    }

    pub fn start_place(&mut self, viewport: &Viewport) {
        let start_position = viewport.cursor().tile();
        let start_pin = self.rect_renderer.insert(
            &rect::Pin {
                position: start_position,
                is_powered: false,
            }
            .into(),
        );
        let end_pin = self.rect_renderer.insert(
            &rect::Pin {
                position: start_position,
                is_powered: false,
            }
            .into(),
        );
        let wire = self.rect_renderer.insert(
            &rect::Wire {
                start: start_position,
                end: start_position,
                is_powered: false,
            }
            .into(),
        );
        self.replace(CursorMode::Place {
            start_position,
            end_position: start_position,
            start_pin,
            end_pin,
            wire,
        })
    }

    pub fn end(&mut self) {
        self.replace(CursorMode::Normal);
    }

    fn replace(&mut self, new_mode: CursorMode) {
        match &self.current_mode {
            CursorMode::Normal => {}
            CursorMode::Pan { .. } => {}
            CursorMode::Place {
                start_pin,
                end_pin,
                wire,
                ..
            } => {
                self.rect_renderer.remove(start_pin);
                self.rect_renderer.remove(end_pin);
                self.rect_renderer.remove(wire);
            }
        }
        self.current_mode = new_mode;
    }
}

pub enum CursorMode {
    Normal,
    Pan {
        last_position: Vec2,
    },
    Place {
        start_position: IVec2,
        end_position: IVec2,
        start_pin: rect::Handle,
        end_pin: rect::Handle,
        wire: rect::Handle,
    },
}
