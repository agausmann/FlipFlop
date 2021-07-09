use crate::circuit::ComponentType;
use crate::direction::Direction;
use crate::rect::{self, RectRenderer};
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::{IVec2, Vec2};

pub struct CursorManager {
    rect_renderer: RectRenderer,
    current_state: CursorState,
    place_type: ComponentType,
    place_orientation: Direction,
}

impl CursorManager {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        Self {
            rect_renderer: RectRenderer::new(gfx, viewport),
            current_state: CursorState::Normal,
            place_type: ComponentType::Pin,
            place_orientation: Direction::North,
        }
    }

    pub fn current_state(&self) -> &CursorState {
        &self.current_state
    }

    pub fn update(&mut self, viewport: &mut Viewport) {
        match &mut self.current_state {
            CursorState::Normal => {}
            CursorState::Pan { last_position } => {
                let position = viewport.cursor().screen_position;
                let delta = (position - *last_position) * Vec2::new(1.0, -1.0);
                let camera = viewport.camera_mut();
                camera.pan -= delta / camera.zoom;

                *last_position = position;
            }
            CursorState::PlaceWire {
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
                        color: Default::default(),
                    }
                    .into(),
                );
                self.rect_renderer.update(
                    end_pin,
                    &rect::Pin {
                        position: *end_position,
                        color: Default::default(),
                    }
                    .into(),
                );
                self.rect_renderer.update(
                    wire,
                    &rect::Wire {
                        start: *start_position,
                        end: *end_position,
                        start_connection: Default::default(),
                        end_connection: Default::default(),
                        color: Default::default(),
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
        self.replace(CursorState::Pan {
            last_position: viewport.cursor().screen_position,
        });
    }

    pub fn start_place_wire(&mut self, viewport: &Viewport) {
        let start_position = viewport.cursor().tile();
        let start_pin = self.rect_renderer.insert(
            &rect::Pin {
                position: start_position,
                color: Default::default(),
            }
            .into(),
        );
        let end_pin = self.rect_renderer.insert(
            &rect::Pin {
                position: start_position,
                color: Default::default(),
            }
            .into(),
        );
        let wire = self.rect_renderer.insert(
            &rect::Wire {
                start: start_position,
                end: start_position,
                start_connection: Default::default(),
                end_connection: Default::default(),
                color: Default::default(),
            }
            .into(),
        );
        self.replace(CursorState::PlaceWire {
            start_position,
            end_position: start_position,
            start_pin,
            end_pin,
            wire,
        })
    }

    pub fn end(&mut self) {
        self.replace(CursorState::Normal);
    }

    pub fn place_type(&self) -> ComponentType {
        self.place_type
    }

    pub fn place_orientation(&self) -> Direction {
        self.place_orientation
    }

    pub fn set_place_type(&mut self, ty: ComponentType) {
        self.place_type = ty;
    }

    pub fn set_place_orientation(&mut self, direction: Direction) {
        self.place_orientation = direction;
    }

    fn replace(&mut self, new_state: CursorState) {
        match &self.current_state {
            CursorState::Normal => {}
            CursorState::Pan { .. } => {}
            CursorState::PlaceWire {
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
        self.current_state = new_state;
    }
}

pub enum CursorState {
    Normal,
    Pan {
        last_position: Vec2,
    },
    PlaceWire {
        start_position: IVec2,
        end_position: IVec2,
        start_pin: rect::Handle,
        end_pin: rect::Handle,
        wire: rect::Handle,
    },
}
