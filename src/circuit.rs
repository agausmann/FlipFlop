use crate::board::{self, BoardRenderer};
use crate::depot::{self, Depot};
use crate::direction::Direction;
use crate::rect::{self, RectRenderer, WireConnection};
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

    pub fn can_place_wire(&mut self, start: IVec2, end: IVec2) -> bool {
        let wire_direction = wire_direction(start, end);

        // All the tiles on the wire's path must allow the wire.
        for tile_pos in wire_tiles(start, end) {
            let tile = match self.tile(tile_pos) {
                Some(x) => x,
                None => {
                    // Wires can always be placed on empty tiles.
                    continue;
                }
            };
            if let Some(component_id) = &tile.component {
                let component = self.components.get(component_id);
                match component.get_type() {
                    ComponentType::Pin => {
                        // Wires can always be placed across pins.
                    }
                    ComponentType::Flip => {
                        // Wires can be placed across flips if it connects to _either_ the input or
                        // the output, but not both.

                        // If the flip is not at the start or end of the wire, the wire can be
                        // placed across the flip if it only connects to the input pin; the output
                        // pin must not be in the path of the wire.
                        let illegal_directions = [
                            component.orientation,
                            component.orientation.opposite(),
                        ];
                        if tile_pos != start
                            && tile_pos != end
                            && illegal_directions.contains(&wire_direction)
                        {
                            return false;
                        }

                        // If the flip is at the start or end of the wire, then it is always legal.
                    }
                    ComponentType::Flop => {
                        // Wires can _never_ be placed across flops.
                        // (The flop must only be at the start or end of the wire).
                        if tile_pos != start && tile_pos != end {
                            return false;
                        }

                        // Wire endpoints can only connect to the input or the output of a flop;
                        // the other sides are illegal.
                        let illegal_directions = [
                            component.orientation.left(),
                            component.orientation.right(),
                        ];
                        if illegal_directions.contains(&wire_direction) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn place_wire(&mut self, start: IVec2, end: IVec2) -> bool {
        if !self.can_place_wire(start, end) {
            return false;
        }

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
        true
    }

    pub fn can_place_component(
        &self,
        ty: ComponentType,
        position: IVec2,
        orientation: Direction,
    ) -> bool {
        let tile = match self.tile(position) {
            Some(x) => x,
            None => {
                // An empty tile is always legal.
                return true;
            }
        };

        // Components cannot be placed on a tile that already has a component.
        if tile.component.is_some() {
            return false;
        }

        match ty {
            ComponentType::Pin => {
                // Pins have no special rules.
            }
            ComponentType::Flip => {
                // Flips can be placed if there is no wire on the output side.
                if tile.wires.get(orientation).is_some() {
                    return false;
                }
            }
            ComponentType::Flop => {
                // Flops cannot be placed on any location that has a wire.
                if tile.wires.count() != 0 {
                    return false;
                }
            }
        }
        true
    }

    pub fn place_component(
        &mut self,
        ty: ComponentType,
        position: IVec2,
        orientation: Direction,
    ) -> bool {
        if !self.can_place_component(ty, position, orientation) {
            return false;
        }

        let tile = self.tiles.entry(position).or_default();

        // Logically split wires that pass over this tile,
        // so they connect through the pin.
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

        &self.insert_component(ty, position, orientation);
        true
    }

    pub fn delete_component(&mut self, position: IVec2) {
        if let Some(tile) = self.tiles.get(&position).cloned() {
            let component = match tile.component {
                Some(id) => self.remove_component(id),
                None => return,
            };

            let north = tile.wires.north.map(|id| self.remove_wire(id));
            let east = tile.wires.east.map(|id| self.remove_wire(id));
            let south = tile.wires.south.map(|id| self.remove_wire(id));
            let west = tile.wires.west.map(|id| self.remove_wire(id));

            match component.get_type() {
                ComponentType::Pin => {
                    // Convert pin to crossover; merge opposite wires.

                    if let (Some(north), Some(south)) = (north, south) {
                        self.insert_wire(south.start, north.end);
                    }
                    if let (Some(east), Some(west)) = (east, west) {
                        self.insert_wire(west.start, east.end);
                    }
                }
                ComponentType::Flip => {}
                ComponentType::Flop => {}
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
                let state = PinState { power_sources };
                let sprite = PinSprite {
                    pin: self.rect_renderer.insert(&Default::default()),
                };
                ComponentData::Pin(state, sprite)
            }
            ComponentType::Flip => {
                let power_sources = 0; //TODO detect
                let state = FlipState {
                    power_sources,
                    output_powered: power_sources == 0,
                };
                let sprite = FlipSprite {
                    body: self.rect_renderer.insert(&Default::default()),
                    input: self.rect_renderer.insert(&Default::default()),
                    output: self.rect_renderer.insert(&Default::default()),
                };
                ComponentData::Flip(state, sprite)
            }
            ComponentType::Flop => {
                let power_sources = 0; //TODO detect
                let state = FlopState {
                    power_sources,
                    output_powered: power_sources > 0,
                };
                let sprite = FlopSprite {
                    body: self.rect_renderer.insert(&Default::default()),
                    input: self.rect_renderer.insert(&Default::default()),
                    output: self.rect_renderer.insert(&Default::default()),
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
        let start_connection = self
            .component_at(start)
            .map(|component| component.connection_type(wire.direction()))
            .unwrap_or(Default::default());
        let end_connection = self
            .component_at(end)
            .map(|component| {
                component.connection_type(wire.direction().opposite())
            })
            .unwrap_or(Default::default());
        wire.update_sprite(
            start_connection,
            end_connection,
            &mut self.rect_renderer,
        );
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
            ComponentData::Pin(_, sprite) => {
                self.rect_renderer.remove(&sprite.pin);
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

    fn component_at(&self, position: IVec2) -> Option<&Component> {
        self.tile(position)
            .and_then(|tile| tile.component)
            .map(|id| self.components.get(&id))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    fn get_type(&self) -> ComponentType {
        match &self.data {
            ComponentData::Pin(..) => ComponentType::Pin,
            ComponentData::Flip(..) => ComponentType::Flip,
            ComponentData::Flop(..) => ComponentType::Flop,
        }
    }

    fn connection_type(&self, direction: Direction) -> WireConnection {
        match self.get_type() {
            ComponentType::Pin => WireConnection::Pin,
            ComponentType::Flip => {
                if direction == self.orientation {
                    WireConnection::Output
                } else {
                    WireConnection::Pin
                }
            }
            ComponentType::Flop => {
                if direction == self.orientation {
                    WireConnection::Output
                } else {
                    WireConnection::SidePin
                }
            }
        }
    }

    fn update_sprite(&self, rect_renderer: &mut RectRenderer) {
        match &self.data {
            ComponentData::Pin(state, sprite) => {
                rect_renderer.update(
                    &sprite.pin,
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
    Pin(PinState, PinSprite),
    Flip(FlipState, FlipSprite),
    Flop(FlopState, FlopSprite),
}

struct PinState {
    power_sources: u32,
}

struct PinSprite {
    pin: rect::Handle,
}

struct FlipState {
    power_sources: u32,
    output_powered: bool,
}

struct FlipSprite {
    body: rect::Handle,
    input: rect::Handle,
    output: rect::Handle,
}

struct FlopState {
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
        wire_direction(self.start, self.end)
    }

    fn update_sprite(
        &self,
        start_connection: WireConnection,
        end_connection: WireConnection,
        rect_renderer: &mut RectRenderer,
    ) {
        rect_renderer.update(
            &self.instance,
            &rect::Wire {
                start: self.start,
                end: self.end,
                start_connection,
                end_connection,
                is_powered: self.power_sources > 0,
            }
            .into(),
        );
    }
}

fn wire_direction(start: IVec2, end: IVec2) -> Direction {
    if start.x == end.x {
        if start.y < end.y {
            Direction::North
        } else {
            Direction::South
        }
    } else {
        if start.x < end.x {
            Direction::East
        } else {
            Direction::West
        }
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
