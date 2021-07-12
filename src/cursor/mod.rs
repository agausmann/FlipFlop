mod outline;

use self::outline::OutlineRenderer;
use crate::circuit::{wire_direction, Circuit, ComponentType};
use crate::direction::Direction;
use crate::rect::{self, Color, RectRenderer};
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::{IVec2, Vec2, Vec3, Vec4};

pub struct CursorManager {
    rect_renderer: RectRenderer,
    outline_renderer: OutlineRenderer,
    current_state: CursorState,
    place_sprite: Sprite,
    place_orientation: Direction,
}

impl CursorManager {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let mut rect_renderer = RectRenderer::new(gfx, viewport);
        let place_sprite = Sprite::new(ComponentType::Pin, &mut rect_renderer);
        let outline_renderer = OutlineRenderer::new(gfx, viewport);

        Self {
            rect_renderer,
            place_sprite,
            outline_renderer,
            current_state: CursorState::Normal,
            place_orientation: Direction::North,
        }
    }

    pub fn current_state(&self) -> &CursorState {
        &self.current_state
    }

    pub fn update(&mut self, viewport: &mut Viewport, circuit: &Circuit) {
        self.place_sprite.update(
            viewport.cursor().tile(),
            self.place_orientation,
            &self.current_state,
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

                if circuit.component_at(*start_position).is_some() {
                    start_pin.set(&Default::default());
                } else {
                    start_pin.set(
                        &rect::Pin {
                            position: *start_position,
                            color: Default::default(),
                        }
                        .into(),
                    );
                }
                if circuit.component_at(*end_position).is_some() {
                    end_pin.set(&Default::default());
                } else {
                    end_pin.set(
                        &rect::Pin {
                            position: *end_position,
                            color: Default::default(),
                        }
                        .into(),
                    );
                }
                let wire_direction = wire_direction(*start_position, *end_position);
                wire.set(
                    &rect::Wire {
                        start: *start_position,
                        end: *end_position,
                        start_connection: circuit
                            .wire_connection(*start_position, wire_direction)
                            .unwrap_or_default(),
                        end_connection: circuit
                            .wire_connection(*end_position, wire_direction.opposite())
                            .unwrap_or_default(),
                        color: Default::default(),
                    }
                    .into(),
                );
            }
        }

        let valid_place = match &self.current_state {
            &CursorState::PlaceWire {
                start_position,
                end_position,
                ..
            } => circuit.can_place_wire(start_position, end_position),
            _ => match self.place_type() {
                ComponentType::Pin => true,
                other_type => circuit.can_place_component(
                    other_type,
                    viewport.cursor().tile(),
                    self.place_orientation,
                ),
            },
        };

        let outline_color = if valid_place {
            Vec3::new(0.0, 0.0, 1.0)
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        };
        self.outline_renderer.set_outline_color(outline_color);
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
        self.outline_renderer
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
            self.place_sprite = Sprite::new(ty, &mut self.rect_renderer);
        }
    }

    pub fn set_place_orientation(&mut self, direction: Direction) {
        self.place_orientation = direction;
    }

    fn replace(&mut self, new_state: CursorState) {
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

enum Sprite {
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
    fn new(ty: ComponentType, renderer: &mut RectRenderer) -> Self {
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

    fn component_type(&self) -> ComponentType {
        match self {
            Self::Pin { .. } => ComponentType::Pin,
            Self::Flip { .. } => ComponentType::Flip,
            Self::Flop { .. } => ComponentType::Flop,
        }
    }

    fn update(&self, position: IVec2, orientation: Direction, current_state: &CursorState) {
        let visible = match current_state {
            CursorState::Normal => true,
            CursorState::Pan { .. } => false,
            CursorState::PlaceWire { .. } => false,
        };
        match self {
            Self::Pin { pin } => {
                if visible {
                    pin.set(
                        &rect::Pin {
                            position,
                            color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                        }
                        .into(),
                    );
                } else {
                    pin.set(&Default::default());
                }
            }
            Self::Flip {
                input,
                body,
                output,
            } => {
                if visible {
                    input.set(
                        &rect::Pin {
                            position,
                            color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                        }
                        .into(),
                    );
                    body.set(&rect::Body { position }.into());
                    output.set(
                        &rect::Output {
                            position,
                            orientation,
                            color: Color::Fixed(Vec4::new(1.0, 0.0, 0.0, 1.0)),
                        }
                        .into(),
                    );
                } else {
                    input.set(&Default::default());
                    body.set(&Default::default());
                    output.set(&Default::default());
                }
            }
            Self::Flop {
                input,
                body,
                output,
            } => {
                if visible {
                    input.set(
                        &rect::SidePin {
                            position,
                            orientation: orientation.opposite(),
                            color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                        }
                        .into(),
                    );
                    body.set(&rect::Body { position }.into());
                    output.set(
                        &rect::Output {
                            position,
                            orientation,
                            color: Color::Fixed(Vec4::new(0.0, 0.0, 0.0, 1.0)),
                        }
                        .into(),
                    );
                } else {
                    input.set(&Default::default());
                    body.set(&Default::default());
                    output.set(&Default::default());
                }
            }
        }
    }
}
