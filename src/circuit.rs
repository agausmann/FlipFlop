use crate::board::{self, BoardRenderer};
use crate::viewport::Viewport;
use crate::wire::{self, WireRenderer};
use crate::GraphicsContext;
use glam::IVec2;
use std::collections::HashMap;

pub struct Circuit {
    board_renderer: BoardRenderer,
    wire_renderer: WireRenderer,
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
        Self {
            board_renderer,
            wire_renderer: WireRenderer::new(gfx, viewport),
            last_id: 0,
            tiles: HashMap::new(),
            pins: HashMap::new(),
            wires: HashMap::new(),
        }
    }

    pub fn draw<'a>(
        &'a mut self,
        viewport: &'a Viewport,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        self.board_renderer.draw(viewport, render_pass);
        self.wire_renderer.draw(viewport, render_pass);
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
        if let Some(&tile) = self.tiles.get(&position) {
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
        if let Some(&tile) = self.tiles.get(&position) {
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
        let instance = self.wire_renderer.insert(
            &wire::Pin {
                position,
                is_powered: power_sources > 0,
            }
            .into(),
        );
        let id = self.make_id();
        self.tiles.entry(position).or_default().pin = Some(id);
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
        let instance = self.wire_renderer.insert(
            &wire::Wire {
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
        }
        self.wires.insert(id, wire);
        true
    }

    fn remove_pin(&mut self, pin_id: u64) -> Pin {
        let pin = self.pins.remove(&pin_id).unwrap();
        let tile = self.tiles.get_mut(&pin.position).unwrap();
        tile.pin = None;
        self.wire_renderer.remove(&pin.instance);
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
        }
        self.wire_renderer.remove(&wire.instance);
        wire
    }

    fn make_id(&mut self) -> u64 {
        self.last_id += 1;
        self.last_id
    }
}

#[derive(Default, Clone, Copy)]
pub struct Tile {
    pub pin: Option<u64>,
    pub wires: [Option<u64>; 4],
}

pub struct Pin {
    pub position: IVec2,
    pub instance: crate::wire::Handle,
    pub power_sources: u32,
}

pub struct Wire {
    pub start: IVec2,
    pub end: IVec2,
    pub instance: crate::wire::Handle,
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
