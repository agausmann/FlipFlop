use crate::circuit::ComponentType;
use crate::direction::Direction;
use crate::rect::{self, Color, RectRenderer};
use crate::screen_vertex::ScreenVertexShader;
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::{IVec2, Vec2, Vec4};

pub struct CursorManager {
    gfx: GraphicsContext,
    rect_renderer: RectRenderer,
    current_state: CursorState,
    place_sprite: Sprite,
    place_orientation: Direction,

    screen_vertex_shader: &'static ScreenVertexShader,
    outline_render_pipeline: wgpu::RenderPipeline,
    outline_bind_group_layout: wgpu::BindGroupLayout,
}

impl CursorManager {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let mut rect_renderer = RectRenderer::new(gfx, viewport);
        let place_sprite = Sprite::new(ComponentType::Pin, &mut rect_renderer);

        let screen_vertex_shader = ScreenVertexShader::get(gfx);
        let outline_bind_group_layout =
            gfx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CursorManager.outline_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Sampler {
                                filtering: false,
                                comparison: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });
        let outline_pipeline_layout =
            gfx.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("CursorManager.outline_pipeline_layout"),
                    bind_group_layouts: &[viewport.bind_group_layout(), &outline_bind_group_layout],
                    push_constant_ranges: &[],
                });
        let outline_fragment_module =
            gfx.device
                .create_shader_module(&wgpu::include_spirv!(concat!(
                    env!("OUT_DIR"),
                    "/shaders/cursor_outline.frag.spv"
                )));
        let outline_render_pipeline =
            gfx.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("CursorManager.outline_render_pipeline"),
                    layout: Some(&outline_pipeline_layout),
                    vertex: screen_vertex_shader.vertex_state(),
                    primitive: screen_vertex_shader.primitive_state(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &outline_fragment_module,
                        entry_point: "main",
                        targets: &[wgpu::ColorTargetState {
                            format: gfx.render_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: Default::default(),
                        }],
                    }),
                });

        Self {
            gfx: gfx.clone(),
            rect_renderer,
            place_sprite,
            screen_vertex_shader,
            outline_render_pipeline,
            outline_bind_group_layout,
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

        let outline_depth_sampler = self.gfx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("CursorManager.outline_depth_sampler"),
            ..Default::default()
        });

        let outline_bind_group = self
            .gfx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("CursorManager.outline_bind_group"),
                layout: &self.outline_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&outline_depth_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                ],
            });

        let mut outline_render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("CursorManager.outline_render_pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        outline_render_pass.set_pipeline(&self.outline_render_pipeline);
        outline_render_pass.set_vertex_buffer(0, self.screen_vertex_shader.vertex_buffer.slice(..));
        outline_render_pass.set_index_buffer(
            self.screen_vertex_shader.index_buffer.slice(..),
            self.screen_vertex_shader.index_format(),
        );
        outline_render_pass.set_bind_group(0, viewport.bind_group(), &[]);
        outline_render_pass.set_bind_group(1, &outline_bind_group, &[]);
        outline_render_pass.draw_indexed(
            self.screen_vertex_shader.indices(),
            self.screen_vertex_shader.base_vertex(),
            self.screen_vertex_shader.instances(),
        );
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
