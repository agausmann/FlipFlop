use crate::circuit::ComponentType;
use crate::direction::Direction;
use crate::rect::{self, Color, RectRenderer};
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::{IVec2, Vec2, Vec4};

pub struct CursorManager {
    rect_renderer: RectRenderer,
    current_state: CursorState,
    place_sprite: Sprite,
    place_orientation: Direction,
}

impl CursorManager {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let mut rect_renderer = RectRenderer::new(gfx, viewport);
        let place_sprite = Sprite::new(ComponentType::Pin, &mut rect_renderer);

        Self {
            rect_renderer,
            place_sprite,
            current_state: CursorState::Normal,
            place_orientation: Direction::North,
        }
    }

    pub fn current_state(&self) -> &CursorState {
        &self.current_state
    }

    pub fn update(&mut self, viewport: &mut Viewport) {
        self.place_sprite.update(
            viewport.cursor().tile(),
            self.place_orientation,
            &mut self.rect_renderer,
        );
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
        self.place_sprite.component_type()
    }

    pub fn place_orientation(&self) -> Direction {
        self.place_orientation
    }

    pub fn set_place_type(&mut self, ty: ComponentType) {
        if ty != self.place_sprite.component_type() {
            self.place_sprite.remove(&mut self.rect_renderer);
            self.place_sprite = Sprite::new(ty, &mut self.rect_renderer);
        }
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

pub enum Sprite {
    Pin {
        pin: rect::Handle,
    },
    Flip {
        input: rect::Handle,
        body: rect::Handle,
        output: rect::Handle,
    },
    Flop {
        input: rect::Handle,
        body: rect::Handle,
        output: rect::Handle,
    },
}

impl Sprite {
    pub fn new(ty: ComponentType, renderer: &mut RectRenderer) -> Self {
        match ty {
            ComponentType::Pin => Self::Pin {
                pin: renderer.insert(&Default::default()),
            },
            ComponentType::Flip => Self::Flip {
                input: renderer.insert(&Default::default()),
                body: renderer.insert(&Default::default()),
                output: renderer.insert(&Default::default()),
            },
            ComponentType::Flop => Self::Flop {
                input: renderer.insert(&Default::default()),
                body: renderer.insert(&Default::default()),
                output: renderer.insert(&Default::default()),
            },
        }
    }

    pub fn component_type(&self) -> ComponentType {
        match self {
            Self::Pin { .. } => ComponentType::Pin,
            Self::Flip { .. } => ComponentType::Flip,
            Self::Flop { .. } => ComponentType::Flop,
        }
    }

    pub fn remove(&self, renderer: &mut RectRenderer) {
        match self {
            Self::Pin { pin } => {
                renderer.remove(&pin);
            }
            Self::Flip {
                input,
                body,
                output,
            } => {
                renderer.remove(&input);
                renderer.remove(&body);
                renderer.remove(&output);
            }
            Self::Flop {
                input,
                body,
                output,
            } => {
                renderer.remove(&input);
                renderer.remove(&body);
                renderer.remove(&output);
            }
        }
    }

    pub fn update(&self, position: IVec2, orientation: Direction, renderer: &mut RectRenderer) {
        match self {
            Self::Pin { pin } => {
                renderer.update(
                    pin,
                    &rect::Pin {
                        position,
                        color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                    }
                    .into(),
                );
            }
            Self::Flip {
                input,
                body,
                output,
            } => {
                renderer.update(
                    input,
                    &rect::Pin {
                        position,
                        color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                    }
                    .into(),
                );
                renderer.update(body, &rect::Body { position }.into());
                renderer.update(
                    output,
                    &rect::Output {
                        position,
                        orientation,
                        color: Color::Fixed(Vec4::new(1.0, 0.0, 0.0, 1.0)),
                    }
                    .into(),
                );
            }
            Self::Flop {
                input,
                body,
                output,
            } => {
                renderer.update(
                    input,
                    &rect::SidePin {
                        position,
                        orientation: orientation.opposite(),
                        color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                    }
                    .into(),
                );
                renderer.update(body, &rect::Body { position }.into());
                renderer.update(
                    output,
                    &rect::Output {
                        position,
                        orientation,
                        color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                    }
                    .into(),
                );
            }
        }
    }
}
