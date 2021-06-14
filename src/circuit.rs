use crate::board::{self, BoardRenderer};
use crate::depot::{self, Depot};
use crate::direction::Direction;
use crate::rect::{self, RectRenderer};
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::IVec2;
use std::collections::HashMap;

pub struct Circuit {
    board_renderer: BoardRenderer,
    rect_renderer: RectRenderer,
    tiles: HashMap<IVec2, Tile>,
    components: Depot<Component>,
    wires: Depot<Wire>,
}

impl Circuit {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let mut board_renderer = BoardRenderer::new(gfx, viewport);
        board_renderer.insert(&board::Board {
            position: IVec2::new(-10_000, -10_000),
            size: IVec2::new(20_000, 20_000),
            color: [0.1, 0.1, 0.1, 1.0],
            z_index: 0,
        });

        Self {
            board_renderer,
            rect_renderer: RectRenderer::new(gfx, viewport),
            tiles: HashMap::new(),
            components: Depot::new(),
            wires: Depot::new(),
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

    pub fn place_wire(&mut self, start: IVec2, end: IVec2) {
        self.place_component(ComponentType::Pin, start, Direction::East);
        self.place_component(ComponentType::Pin, end, Direction::East);

        // Split the wire at every tile where a component is present.
        let split_points: Vec<IVec2> = wire_tiles(start, end)
            .filter(|pos| {
                self.tiles
                    .get(&pos)
                    .and_then(|tile| tile.component)
                    .is_some()
            })
            .collect();

        for v in split_points.windows(2) {
            let sub_start = v[0];
            let sub_end = v[1];
            self.insert_wire(sub_start, sub_end);
        }
    }

    pub fn place_component(
        &mut self,
        ty: ComponentType,
        position: IVec2,
        orientation: Direction,
    ) {
        let tile = self.tiles.entry(position).or_default();
        if tile.component.is_some() {
            return;
        }

        // Logically split wires that pass over this tile,
        // so they connect through the pin.
        // TODO adapt this logic for the placement rules of other components.
        let wires = tile.wires.clone();
        if let Some(wire_id) = wires.north {
            if wires.north == wires.south {
                let wire = self.remove_wire(wire_id);
                self.insert_wire(wire.start, position);
                self.insert_wire(position, wire.end);
            }
        }
        if let Some(wire_id) = wires.east {
            if wires.east == wires.west {
                let wire = self.remove_wire(wire_id);
                self.insert_wire(wire.start, position);
                self.insert_wire(position, wire.end);
            }
        }

        self.insert_component(ty, position, orientation);
    }

    pub fn delete_component(&mut self, position: IVec2) {
        if let Some(tile) = self.tiles.get(&position).cloned() {
            if let Some(component_id) = tile.component {
                self.remove_component(component_id);
            }

            // Merge opposite wires into a single one passing over this tile.
            // TODO adapt this logic for the placement rules of other components.
            let wires = tile.wires.clone();
            let north = wires.north.map(|id| self.remove_wire(id));
            let east = wires.east.map(|id| self.remove_wire(id));
            let south = wires.south.map(|id| self.remove_wire(id));
            let west = wires.west.map(|id| self.remove_wire(id));

            if let (Some(north), Some(south)) = (north, south) {
                self.insert_wire(south.start, north.end);
            }
            if let (Some(east), Some(west)) = (east, west) {
                self.insert_wire(west.start, east.end);
            }
        }
    }

    pub fn delete_all_at(&mut self, position: IVec2) {
        if let Some(tile) = self.tiles.get(&position).cloned() {
            if let Some(component_id) = tile.component {
                self.remove_component(component_id);
            }

            let wires = tile.wires.clone();
            if let Some(id) = wires.north {
                self.remove_wire(id);
            }
            if let Some(id) = wires.south {
                if wires.south != wires.north {
                    self.remove_wire(id);
                }
            }
            if let Some(id) = wires.east {
                self.remove_wire(id);
            }
            if let Some(id) = wires.west {
                if wires.west != wires.east {
                    self.remove_wire(id);
                }
            }
        }
    }

    fn insert_component(
        &mut self,
        ty: ComponentType,
        position: IVec2,
        orientation: Direction,
    ) -> bool {
        if let Some(tile) = self.tiles.get(&position) {
            if tile.component.is_some() {
                return false;
            }
        }
        let data = match ty {
            ComponentType::Pin => {
                let power_sources = 0; //TODO detect
                let state = Pin { power_sources };
                let sprite = self.rect_renderer.insert(&Default::default());
                ComponentData::Pin(state, sprite)
            }
            ComponentType::Flip => {
                let power_sources = 0; //TODO detect
                let state = Flip {
                    power_sources,
                    output_powered: power_sources == 0,
                };
                let body = self.rect_renderer.insert(&Default::default());
                let input = self.rect_renderer.insert(&Default::default());
                let output = self.rect_renderer.insert(&Default::default());
                let sprite = FlipSprite {
                    body,
                    input,
                    output,
                };
                ComponentData::Flip(state, sprite)
            }
            ComponentType::Flop => {
                let power_sources = 0; //TODO detect
                let state = Flop {
                    power_sources,
                    output_powered: power_sources > 0,
                };
                let body = self.rect_renderer.insert(&Default::default());
                let input = self.rect_renderer.insert(&Default::default());
                let output = self.rect_renderer.insert(&Default::default());
                let sprite = FlopSprite {
                    body,
                    input,
                    output,
                };
                ComponentData::Flop(state, sprite)
            }
        };
        let component = Component {
            data,
            position,
            orientation,
        };
        component.update_sprite(&mut self.rect_renderer);

        let id = self.components.insert(component);
        let tile = self.tiles.entry(position).or_default();
        tile.component = Some(id);
        tile.update_crossover(position, &mut self.rect_renderer);
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
            for &id in tile.wires.as_array().iter().flatten() {
                let wire = &self.wires[&id];
                if wire.start == start && wire.end == end {
                    return false;
                }
            }
        }

        let power_sources = 0; //TODO detect
        let instance = self.rect_renderer.insert(&Default::default());
        let id = self.wires.insert(Wire {
            start,
            end,
            instance,
            power_sources,
        });
        let wire = self.wires.get(&id);
        wire.update_sprite(&mut self.rect_renderer);
        for pos in wire.tiles() {
            let tile = self.tiles.entry(pos).or_default();
            if pos != wire.start {
                *tile.wires.get_mut(wire.direction().opposite()) = Some(id);
            }
            if pos != wire.end {
                *tile.wires.get_mut(wire.direction()) = Some(id);
            }
            tile.update_crossover(pos, &mut self.rect_renderer);
        }
        true
    }

    fn remove_component(&mut self, component_id: depot::Handle) -> Component {
        let component = self.components.remove(&component_id);
        let tile = self.tiles.get_mut(&component.position).unwrap();
        tile.component = None;
        tile.update_crossover(component.position, &mut self.rect_renderer);
        match &component.data {
            ComponentData::Pin(_, instance) => {
                self.rect_renderer.remove(instance);
            }
            ComponentData::Flip(_, sprite) => {
                self.rect_renderer.remove(&sprite.body);
                self.rect_renderer.remove(&sprite.input);
                self.rect_renderer.remove(&sprite.output);
            }
            ComponentData::Flop(_, sprite) => {
                self.rect_renderer.remove(&sprite.body);
                self.rect_renderer.remove(&sprite.input);
                self.rect_renderer.remove(&sprite.output);
            }
        }
        component
    }

    fn remove_wire(&mut self, wire_id: depot::Handle) -> Wire {
        let wire = self.wires.remove(&wire_id);
        for tile_pos in wire.tiles() {
            let tile = self.tiles.get_mut(&tile_pos).unwrap();
            if tile_pos != wire.start {
                assert!(
                    tile.wires.get(wire.direction().opposite())
                        == Some(wire_id)
                );
                *tile.wires.get_mut(wire.direction().opposite()) = None;
            }
            if tile_pos != wire.end {
                assert!(tile.wires.get(wire.direction()) == Some(wire_id));
                *tile.wires.get_mut(wire.direction()) = None;
            }
            tile.update_crossover(tile_pos, &mut self.rect_renderer);
        }
        self.rect_renderer.remove(&wire.instance);
        wire
    }
}

#[derive(Default, Clone)]
pub struct Tile {
    pub component: Option<depot::Handle>,
    pub crossover: Option<rect::Handle>,
    pub wires: TileWires,
}

impl Tile {
    fn update_crossover(
        &mut self,
        position: IVec2,
        renderer: &mut RectRenderer,
    ) {
        let wire_count = self.wires.count();
        if self.component.is_some() || wire_count < 2 {
            if let Some(handle) = self.crossover.take() {
                renderer.remove(&handle);
            }
        } else if wire_count >= 2 && self.crossover.is_none() {
            let handle = renderer.insert(&rect::Crossover { position }.into());
            self.crossover = Some(handle);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TileWires {
    pub east: Option<depot::Handle>,
    pub north: Option<depot::Handle>,
    pub west: Option<depot::Handle>,
    pub south: Option<depot::Handle>,
}

impl TileWires {
    pub fn count(&self) -> usize {
        let mut sum = 0;
        if self.east.is_some() {
            sum += 1;
        }
        if self.north.is_some() {
            sum += 1;
        }
        if self.west.is_some() && self.west != self.east {
            sum += 1;
        }
        if self.south.is_some() && self.south != self.north {
            sum += 1;
        }
        sum
    }

    pub fn get(&self, direction: Direction) -> Option<depot::Handle> {
        match direction {
            Direction::East => self.east,
            Direction::North => self.north,
            Direction::West => self.west,
            Direction::South => self.south,
        }
    }

    pub fn get_mut(
        &mut self,
        direction: Direction,
    ) -> &mut Option<depot::Handle> {
        match direction {
            Direction::East => &mut self.east,
            Direction::North => &mut self.north,
            Direction::West => &mut self.west,
            Direction::South => &mut self.south,
        }
    }

    pub fn as_array(&self) -> [Option<depot::Handle>; 4] {
        [self.east, self.north, self.west, self.south]
    }
}

pub enum ComponentType {
    Pin,
    Flip,
    Flop,
}

struct Component {
    data: ComponentData,
    position: IVec2,
    orientation: Direction,
}

impl Component {
    fn update_sprite(&self, rect_renderer: &mut RectRenderer) {
        match &self.data {
            ComponentData::Pin(state, sprite) => {
                rect_renderer.update(
                    sprite,
                    &rect::Pin {
                        position: self.position,
                        is_powered: state.power_sources > 0,
                    }
                    .into(),
                );
            }
            ComponentData::Flip(state, sprite) => {
                rect_renderer.update(
                    &sprite.body,
                    &rect::Body {
                        position: self.position,
                    }
                    .into(),
                );
                rect_renderer.update(
                    &sprite.input,
                    &rect::Pin {
                        position: self.position,
                        is_powered: state.power_sources > 0,
                    }
                    .into(),
                );
                rect_renderer.update(
                    &sprite.output,
                    &rect::Output {
                        position: self.position,
                        orientation: self.orientation,
                        is_powered: state.output_powered,
                    }
                    .into(),
                );
            }
            ComponentData::Flop(state, sprite) => {
                rect_renderer.update(
                    &sprite.body,
                    &rect::Body {
                        position: self.position,
                    }
                    .into(),
                );
                rect_renderer.update(
                    &sprite.input,
                    &rect::SidePin {
                        position: self.position,
                        orientation: self.orientation.opposite(),
                        is_powered: state.power_sources > 0,
                    }
                    .into(),
                );
                rect_renderer.update(
                    &sprite.output,
                    &rect::Output {
                        position: self.position,
                        orientation: self.orientation,
                        is_powered: state.output_powered,
                    }
                    .into(),
                );
            }
        };
    }
}

enum ComponentData {
    Pin(Pin, rect::Handle),
    Flip(Flip, FlipSprite),
    Flop(Flop, FlopSprite),
}

struct Pin {
    power_sources: u32,
}

struct Flip {
    power_sources: u32,
    output_powered: bool,
}

struct FlipSprite {
    body: rect::Handle,
    input: rect::Handle,
    output: rect::Handle,
}

struct Flop {
    power_sources: u32,
    output_powered: bool,
}

struct FlopSprite {
    body: rect::Handle,
    input: rect::Handle,
    output: rect::Handle,
}

struct Wire {
    start: IVec2,
    end: IVec2,
    power_sources: u32,
    instance: rect::Handle,
}

impl Wire {
    fn tiles(&self) -> impl Iterator<Item = IVec2> {
        wire_tiles(self.start, self.end)
    }

    fn direction(&self) -> Direction {
        if self.start.x == self.end.x {
            Direction::North
        } else {
            Direction::East
        }
    }

    fn update_sprite(&self, rect_renderer: &mut RectRenderer) {
        rect_renderer.update(
            &self.instance,
            &rect::Wire {
                start: self.start,
                end: self.end,
                is_powered: self.power_sources > 0,
            }
            .into(),
        );
    }
}

fn wire_tiles(start: IVec2, end: IVec2) -> impl Iterator<Item = IVec2> {
    let delta = end - start;
    // Either X or Y is zero, so the "normalized" vector is clamping the
    // non-zero element, and length is the nonzero element + 0.
    let ray = delta.clamp(IVec2::splat(-1), IVec2::splat(1));
    let len = delta.x.abs() + delta.y.abs();

    (0..=len).map(move |i| start + ray * i)
}
