use crate::board::{self, BoardRenderer};
use crate::direction::{Direction, Relative};
use crate::rect::{self, RectRenderer};
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::IVec2;
use std::collections::HashMap;

pub struct Circuit {
    board_renderer: BoardRenderer,
    rect_renderer: RectRenderer,
    last_id: u64,
    tiles: HashMap<IVec2, Tile>,
    pins: HashMap<u64, Pin>,
    wires: HashMap<u64, Wire>,
}

impl Circuit {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let mut board_renderer = BoardRenderer::new(gfx, &viewport);
        board_renderer.insert(&board::Board {
            position: IVec2::new(-10_000, -10_000),
            size: IVec2::new(20_000, 20_000),
            color: [0.1, 0.1, 0.1, 1.0],
            z_index: 0,
        });
        let mut rect_renderer = RectRenderer::new(gfx, viewport);
        let flip = Flip {
            position: IVec2::new(1, 0),
            orientation: Direction::East,
            power_sources: 0,
            output_powered: true,
        };
        rect_renderer.insert(&flip.body());
        rect_renderer.insert(&flip.input());
        rect_renderer.insert(&flip.output());
        let flop = Flop {
            position: IVec2::new(0, 1),
            orientation: Direction::North,
            power_sources: 0,
            output_powered: false,
        };
        rect_renderer.insert(&flop.body());
        rect_renderer.insert(&flop.input());
        rect_renderer.insert(&flop.output());

        Self {
            board_renderer,
            rect_renderer,
            last_id: 0,
            tiles: HashMap::new(),
            pins: HashMap::new(),
            wires: HashMap::new(),
        }
    }

    pub fn draw(
        &mut self,
        viewport: &Viewport,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) {
        self.board_renderer
            .draw(viewport, encoder, frame_view, depth_view);
        self.rect_renderer
            .draw(viewport, encoder, frame_view, depth_view);
    }

    pub fn tile(&self, pos: IVec2) -> Option<&Tile> {
        self.tiles.get(&pos)
    }

    pub fn pin(&self, id: u64) -> Option<&Pin> {
        self.pins.get(&id)
    }

    pub fn wire(&self, id: u64) -> Option<&Wire> {
        self.wires.get(&id)
    }

    pub fn place_wire(&mut self, start: IVec2, end: IVec2) {
        self.place_pin(start);
        self.place_pin(end);

        // Split the wire at every tile where a pin is present.
        let pin_points: Vec<IVec2> = tiles(start, end)
            .filter(|pos| {
                self.tiles.get(&pos).and_then(|tile| tile.pin).is_some()
            })
            .collect();

        for v in pin_points.windows(2) {
            let sub_start = v[0];
            let sub_end = v[1];
            self.insert_wire(sub_start, sub_end);
        }
    }

    pub fn place_pin(&mut self, position: IVec2) {
        let tile = self.tiles.entry(position).or_default();
        if tile.pin.is_some() {
            return;
        }

        let wires = tile.wires.clone();
        for &wire_id in wires.iter().flatten() {
            let wire = self.remove_wire(wire_id);
            self.insert_wire(wire.start, position);
            self.insert_wire(position, wire.end);
        }

        self.insert_pin(position);
    }

    pub fn delete_pin(&mut self, position: IVec2) {
        if let Some(tile) = self.tiles.get(&position).cloned() {
            if let Some(pin_id) = tile.pin {
                self.remove_pin(pin_id);
            }
            let mut endpoints = Vec::new();
            for &wire_id in tile.wires.iter().flatten() {
                let wire = self.remove_wire(wire_id);
                if wire.start == position {
                    endpoints.push(wire.end);
                } else if wire.end == position {
                    endpoints.push(wire.start);
                } else {
                    panic!("wire is not connected to this tile");
                }
            }
            for i in 1..endpoints.len() {
                for j in 0..i {
                    if endpoints[i].x == endpoints[j].x
                        || endpoints[i].y == endpoints[j].y
                    {
                        self.insert_wire(endpoints[i], endpoints[j]);
                    }
                }
            }
        }
    }

    pub fn delete_all_at(&mut self, position: IVec2) {
        if let Some(tile) = self.tiles.get(&position).cloned() {
            if let Some(pin_id) = tile.pin {
                self.remove_pin(pin_id);
            }
            for &wire_id in tile.wires.iter().flatten() {
                self.remove_wire(wire_id);
            }
        }
    }

    fn insert_pin(&mut self, position: IVec2) -> bool {
        if let Some(tile) = self.tiles.get(&position) {
            if tile.pin.is_some() {
                return false;
            }
        }
        let power_sources = 0; //TODO detect
        let instance = self.rect_renderer.insert(
            &rect::Pin {
                position,
                is_powered: power_sources > 0,
            }
            .into(),
        );
        let id = self.make_id();
        let tile = self.tiles.entry(position).or_default();
        tile.pin = Some(id);
        tile.update_crossover(position, &mut self.rect_renderer);
        self.pins.insert(
            id,
            Pin {
                position,
                instance,
                power_sources,
            },
        );
        true
    }

    fn insert_wire(&mut self, start: IVec2, end: IVec2) -> bool {
        // Lexicographically order the start/end points to ensure "backwards" duplicates
        // get caught.
        if <[i32; 2]>::from(start) > <[i32; 2]>::from(end) {
            return self.insert_wire(end, start);
        }

        // Either a wire's start and end X coordinates need to be the same,
        // or their Y coordinates need to be the same, but not both.
        // (If both, then the wire would be zero-length, which is not useful.)
        assert!(
            (start.x == end.x) ^ (start.y == end.y),
            "Illegal wire start and end positions"
        );

        if let Some(tile) = self.tiles.get(&start) {
            for &id in tile.wires.iter().flatten() {
                let wire = &self.wires[&id];
                if wire.start == start && wire.end == end {
                    return false;
                }
            }
        }

        let power_sources = 0; //TODO detect
        let instance = self.rect_renderer.insert(
            &rect::Wire {
                start,
                end,
                is_powered: power_sources > 0,
            }
            .into(),
        );
        let id = self.make_id();
        let wire = Wire {
            start,
            end,
            instance,
            power_sources,
        };
        for pos in wire.tiles() {
            let tile = self.tiles.entry(pos).or_default();
            let mut inserted = false;
            for slot in tile.wires.iter_mut() {
                if slot.is_none() {
                    inserted = true;
                    *slot = Some(id);
                    break;
                }
            }
            assert!(inserted);
            tile.update_crossover(pos, &mut self.rect_renderer);
        }
        self.wires.insert(id, wire);
        true
    }

    fn remove_pin(&mut self, pin_id: u64) -> Pin {
        let pin = self.pins.remove(&pin_id).unwrap();
        let tile = self.tiles.get_mut(&pin.position).unwrap();
        tile.pin = None;
        tile.update_crossover(pin.position, &mut self.rect_renderer);
        self.rect_renderer.remove(&pin.instance);
        pin
    }

    fn remove_wire(&mut self, wire_id: u64) -> Wire {
        let wire = self.wires.remove(&wire_id).unwrap();
        for tile_pos in wire.tiles() {
            let tile = self.tiles.get_mut(&tile_pos).unwrap();
            let mut removed = false;
            for slot in &mut tile.wires {
                if *slot == Some(wire_id) {
                    removed = true;
                    *slot = None;
                }
            }
            assert!(removed);
            tile.update_crossover(tile_pos, &mut self.rect_renderer);
        }
        self.rect_renderer.remove(&wire.instance);
        wire
    }

    fn make_id(&mut self) -> u64 {
        self.last_id += 1;
        self.last_id
    }
}

#[derive(Default, Clone)]
pub struct Tile {
    pub pin: Option<u64>,
    pub crossover: Option<rect::Handle>,
    pub wires: [Option<u64>; 4],
}

impl Tile {
    fn update_crossover(
        &mut self,
        position: IVec2,
        renderer: &mut RectRenderer,
    ) {
        let wire_count = self.wires.iter().flatten().count();
        if self.pin.is_some() || wire_count < 2 {
            if let Some(handle) = self.crossover.take() {
                renderer.remove(&handle);
            }
        } else if wire_count >= 2 && self.crossover.is_none() {
            let handle = renderer.insert(&rect::Crossover { position }.into());
            self.crossover = Some(handle);
        }
    }
}

pub struct Pin {
    pub position: IVec2,
    pub instance: rect::Handle,
    pub power_sources: u32,
}

pub struct Flip {
    pub position: IVec2,
    pub orientation: Direction,
    pub power_sources: u32,
    pub output_powered: bool,
}

impl Flip {
    fn body(&self) -> rect::Rect {
        rect::Body {
            position: self.position,
        }
        .into()
    }

    fn input(&self) -> rect::Rect {
        rect::Pin {
            position: self.position,
            is_powered: self.power_sources > 0,
        }
        .into()
    }

    fn output(&self) -> rect::Rect {
        rect::Output {
            position: self.position,
            orientation: self.orientation,
            is_powered: self.output_powered,
        }
        .into()
    }
}

pub struct FlipSprite {
    pub body: rect::Handle,
    pub input: rect::Handle,
    pub output: rect::Handle,
}

pub struct Flop {
    pub position: IVec2,
    pub orientation: Direction,
    pub power_sources: u32,
    pub output_powered: bool,
}

impl Flop {
    fn body(&self) -> rect::Rect {
        rect::Body {
            position: self.position,
        }
        .into()
    }

    fn input(&self) -> rect::Rect {
        rect::SidePin {
            position: self.position,
            orientation: self.orientation.rotate(Relative::Opposite),
            is_powered: self.power_sources > 0,
        }
        .into()
    }

    fn output(&self) -> rect::Rect {
        rect::Output {
            position: self.position,
            orientation: self.orientation,
            is_powered: self.output_powered,
        }
        .into()
    }
}

pub struct FlopSprite {
    pub body: rect::Handle,
    pub input: rect::Handle,
    pub output: rect::Handle,
}

pub struct Wire {
    pub start: IVec2,
    pub end: IVec2,
    pub instance: rect::Handle,
    pub power_sources: u32,
}

impl Wire {
    fn tiles(&self) -> impl Iterator<Item = IVec2> {
        tiles(self.start, self.end)
    }
}

fn tiles(start: IVec2, end: IVec2) -> impl Iterator<Item = IVec2> {
    let delta = end - start;
    // Either X or Y is zero, so the "normalized" vector is clamping the
    // non-zero element, and length is the nonzero element + 0.
    let ray = delta.clamp(IVec2::splat(-1), IVec2::splat(1));
    let len = delta.x.abs() + delta.y.abs();

    (0..=len).map(move |i| start + ray * i)
}
