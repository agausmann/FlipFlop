use crate::board::{self, BoardRenderer};
use crate::depot::{self, Depot};
use crate::direction::{Direction, Relative};
use crate::rect::{self, Color, RectRenderer, WireConnection};
use crate::simulation::Simulation;
use crate::viewport::Viewport;
use crate::GraphicsContext;
use glam::IVec2;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

pub struct Circuit {
    board_renderer: BoardRenderer,
    rect_renderer: RectRenderer,
    _root_board: board::Handle,
    tiles: HashMap<IVec2, Tile>,
    components: Depot<Component>,
    wires: Depot<Wire>,
    simulation: Simulation,
}

impl Circuit {
    pub fn new(gfx: &GraphicsContext, viewport: &Viewport) -> Self {
        let mut board_renderer = BoardRenderer::new(gfx, viewport);
        let _root_board = board_renderer.insert(&board::Board {
            position: IVec2::new(-10_000, -10_000),
            size: IVec2::new(20_000, 20_000),
            color: [0.1, 0.1, 0.1, 1.0],
            z_index: 0,
        });

        Self {
            board_renderer,
            rect_renderer: RectRenderer::new(gfx, viewport),
            _root_board,
            tiles: HashMap::new(),
            components: Depot::new(),
            wires: Depot::new(),
            simulation: Simulation::new(),
        }
    }

    pub fn draw(
        &mut self,
        viewport: &Viewport,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
    ) {
        // XXX
        self.simulation.tick();
        self.rect_renderer.update_cluster_states(&self.simulation);

        self.board_renderer
            .draw(viewport, encoder, frame_view, depth_view);
        self.rect_renderer
            .draw(viewport, encoder, frame_view, depth_view);
    }

    pub fn tile_debug_info(&self, pos: IVec2) -> TileDebugInfo {
        TileDebugInfo { circuit: self, pos }
    }

    pub fn tile(&self, pos: IVec2) -> Option<&Tile> {
        self.tiles.get(&pos)
    }

    pub fn component_at(&self, pos: IVec2) -> Option<ComponentType> {
        self.component(pos).map(|component| component.get_type())
    }

    pub fn can_place_wire(&self, start: IVec2, end: IVec2) -> bool {
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
                        let illegal_directions =
                            [component.orientation, component.orientation.opposite()];
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
                        let illegal_directions =
                            [component.orientation.left(), component.orientation.right()];
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

    pub fn wire_connection(&self, position: IVec2, direction: Direction) -> Option<WireConnection> {
        self.component(position)
            .map(|component| component.connection_type(direction))
    }

    fn insert_component(
        &mut self,
        ty: ComponentType,
        position: IVec2,
        orientation: Direction,
    ) -> bool {
        if self
            .tile(position)
            .and_then(|tile| tile.component)
            .is_some()
        {
            return false;
        }
        let data = match ty {
            ComponentType::Pin => {
                let mut node = None;
                if let Some(tile) = self.tile(position).cloned() {
                    let directions = [
                        Direction::North,
                        Direction::East,
                        Direction::South,
                        Direction::West,
                    ];
                    node = directions
                        .iter()
                        .flat_map(|&dir| tile.wires.get(dir))
                        .map(|wire_handle| GraphNode::Wire(wire_handle))
                        .fold(None, |acc, next| match acc {
                            Some(current) => {
                                self.merge_clusters(current, next);
                                Some(current)
                            }
                            None => Some(next),
                        });
                }
                let cluster_index = match node {
                    Some(node) => self.cluster_id(&node),
                    None => self.simulation.alloc_cluster(),
                };

                let state = PinState { cluster_index };
                let sprite = PinSprite {
                    pin: self.rect_renderer.insert(&Default::default()),
                };
                ComponentData::Pin(state, sprite)
            }
            ComponentType::Flip => {
                let mut input_node = None;
                let mut output_node = None;
                if let Some(tile) = self.tile(position).cloned() {
                    let input_directions = [
                        orientation.right(),
                        orientation.opposite(),
                        orientation.left(),
                    ];
                    input_node = input_directions
                        .iter()
                        .flat_map(|&dir| tile.wires.get(dir))
                        .map(|wire_handle| GraphNode::Wire(wire_handle))
                        .fold(None, |acc, next| match acc {
                            Some(current) => {
                                self.merge_clusters(current, next);
                                Some(current)
                            }
                            None => Some(next),
                        });

                    output_node = tile
                        .wires
                        .get(orientation)
                        .map(|wire_handle| GraphNode::Wire(wire_handle));
                }
                let input_cluster_index = match input_node {
                    Some(node) => self.cluster_id(&node),
                    None => self.simulation.alloc_cluster(),
                };
                let output_cluster_index = match output_node {
                    Some(node) => self.cluster_id(&node),
                    None => self.simulation.alloc_cluster(),
                };

                self.simulation
                    .add_flip(input_cluster_index, output_cluster_index);

                let state = FlipState {
                    input_cluster_index,
                    output_cluster_index,
                };
                let sprite = FlipSprite {
                    body: self.rect_renderer.insert(&Default::default()),
                    input: self.rect_renderer.insert(&Default::default()),
                    output: self.rect_renderer.insert(&Default::default()),
                };
                ComponentData::Flip(state, sprite)
            }
            ComponentType::Flop => {
                let mut input_node = None;
                let mut output_node = None;
                if let Some(tile) = self.tile(position).cloned() {
                    input_node = tile
                        .wires
                        .get(orientation.opposite())
                        .map(|wire_handle| GraphNode::Wire(wire_handle));

                    output_node = tile
                        .wires
                        .get(orientation)
                        .map(|wire_handle| GraphNode::Wire(wire_handle));
                }
                let input_cluster_index = match input_node {
                    Some(node) => self.cluster_id(&node),
                    None => self.simulation.alloc_cluster(),
                };
                let output_cluster_index = match output_node {
                    Some(node) => self.cluster_id(&node),
                    None => self.simulation.alloc_cluster(),
                };

                self.simulation
                    .add_flop(input_cluster_index, output_cluster_index);

                let state = FlopState {
                    input_cluster_index,
                    output_cluster_index,
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
        component.update_sprite();

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

        let direction = wire_direction(start, end);
        let start_connection = self
            .component(start)
            .map(|component| component.connection_type(direction))
            .unwrap_or(Default::default());
        let end_connection = self
            .component(end)
            .map(|component| component.connection_type(direction.opposite()))
            .unwrap_or(Default::default());

        let mut node = None;
        if let Some(component_handle) = self.tile(start).and_then(|tile| tile.component) {
            let next = GraphNode::Component(component_handle, direction);
            match node {
                Some(current) => {
                    self.merge_clusters(current, next);
                }
                None => {
                    node = Some(next);
                }
            }
        }
        if let Some(component_handle) = self.tile(end).and_then(|tile| tile.component) {
            let next = GraphNode::Component(component_handle, direction.opposite());
            match node {
                Some(current) => {
                    self.merge_clusters(current, next);
                }
                None => {
                    node = Some(next);
                }
            }
        }
        let cluster_index = match node {
            Some(node) => self.cluster_id(&node),
            None => self.simulation.alloc_cluster(),
        };

        let instance = self.rect_renderer.insert(&Default::default());
        let id = self.wires.insert(Wire {
            start,
            end,
            start_connection,
            end_connection,
            instance,
            cluster_index,
        });
        let wire = self.wires.get(&id);
        wire.update_sprite();
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
        let component = self.components.get(&component_id);

        // Move/copy out to prevent lifetime errors
        let orientation = component.orientation;
        match &component.data {
            ComponentData::Pin(state, _sprite) => {
                if !self.has_neighbors(&GraphNode::Component(component_id, Direction::North)) {
                    self.simulation.free_cluster(state.cluster_index);
                }
            }
            ComponentData::Flip(state, _sprite) => {
                // Move/copy out to prevent lifetime errors
                let &FlipState {
                    input_cluster_index,
                    output_cluster_index,
                    ..
                } = state;

                self.simulation
                    .remove_flip(input_cluster_index, output_cluster_index);

                if !self.has_neighbors(&GraphNode::Component(component_id, orientation.opposite()))
                {
                    self.simulation.free_cluster(input_cluster_index);
                }
                if !self.has_neighbors(&GraphNode::Component(component_id, orientation)) {
                    self.simulation.free_cluster(output_cluster_index);
                }
            }
            ComponentData::Flop(state, _sprite) => {
                // Move/copy out to prevent lifetime errors
                let &FlopState {
                    input_cluster_index,
                    output_cluster_index,
                    ..
                } = state;

                self.simulation
                    .remove_flop(input_cluster_index, output_cluster_index);

                if !self.has_neighbors(&GraphNode::Component(component_id, orientation.opposite()))
                {
                    self.simulation.free_cluster(input_cluster_index);
                }
                if !self.has_neighbors(&GraphNode::Component(component_id, orientation)) {
                    self.simulation.free_cluster(output_cluster_index);
                }
            }
        }

        let component = self.components.remove(&component_id);
        let tile = self.tiles.get_mut(&component.position).unwrap();
        tile.component = None;
        tile.update_crossover(component.position, &mut self.rect_renderer);

        match &component.data {
            ComponentData::Pin(..) => {
                let directions = [
                    Direction::North,
                    Direction::East,
                    Direction::South,
                    Direction::West,
                ];
                self.split_all(component.position, &directions);
            }
            ComponentData::Flip(..) => {
                let input_directions = [
                    orientation.right(),
                    orientation.opposite(),
                    orientation.left(),
                ];
                self.split_all(component.position, &input_directions);
            }
            ComponentData::Flop(..) => {}
        }
        component
    }

    fn remove_wire(&mut self, wire_id: depot::Handle) -> Wire {
        let wire = self.wires.get(&wire_id);

        if !self.has_neighbors(&GraphNode::Wire(wire_id)) {
            self.simulation.free_cluster(wire.cluster_index);
        }

        let wire = self.wires.remove(&wire_id);
        for tile_pos in wire.tiles() {
            let tile = self.tiles.get_mut(&tile_pos).unwrap();
            if tile_pos != wire.start {
                assert_eq!(tile.wires.get(wire.direction().opposite()), Some(wire_id));
                *tile.wires.get_mut(wire.direction().opposite()) = None;
            }
            if tile_pos != wire.end {
                assert_eq!(tile.wires.get(wire.direction()), Some(wire_id));
                *tile.wires.get_mut(wire.direction()) = None;
            }
            tile.update_crossover(tile_pos, &mut self.rect_renderer);
        }

        let start_component = self.tile(wire.start).and_then(|tile| tile.component);
        let end_component = self.tile(wire.end).and_then(|tile| tile.component);
        if let (Some(start), Some(end)) = (start_component, end_component) {
            self.split_clusters(
                GraphNode::Component(start, wire.direction()),
                GraphNode::Component(end, wire.direction().opposite()),
            );
        }

        wire
    }

    fn component(&self, position: IVec2) -> Option<&Component> {
        self.tile(position)
            .and_then(|tile| tile.component)
            .map(|id| self.components.get(&id))
    }

    fn merge_clusters(&mut self, into: GraphNode, from: GraphNode) {
        let into_index = self.cluster_id(&into);
        let from_index = self.cluster_id(&from);
        if into_index == from_index {
            return;
        }
        let from_cluster = self.cluster_of(&from);
        for node in &from_cluster {
            match node {
                &GraphNode::Wire(handle) => {
                    let wire = self.wires.get_mut(&handle);
                    wire.cluster_index = into_index;
                    wire.update_sprite();
                }
                &GraphNode::Component(handle, direction) => {
                    let component = self.components.get_mut(&handle);
                    match &mut component.data {
                        ComponentData::Pin(state, _sprite) => {
                            state.cluster_index = into_index;
                        }
                        ComponentData::Flip(state, _sprite) => {
                            if direction == component.orientation {
                                // Output cluster changed:
                                self.simulation.remove_flip(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flip(state.input_cluster_index, into_index);
                                state.output_cluster_index = into_index;
                            } else {
                                // Input cluster changed
                                self.simulation.remove_flip(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flip(into_index, state.output_cluster_index);
                                state.input_cluster_index = into_index;
                            }
                        }
                        ComponentData::Flop(state, _sprite) => {
                            if direction == component.orientation {
                                // Output cluster changed:
                                self.simulation.remove_flop(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flop(state.input_cluster_index, into_index);
                                state.output_cluster_index = into_index;
                            } else if direction == component.orientation.opposite() {
                                // Input cluster changed:
                                self.simulation.remove_flop(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flop(into_index, state.output_cluster_index);
                                state.input_cluster_index = into_index;
                            } else {
                                unreachable!()
                            }
                        }
                    }
                    component.update_sprite();
                }
            }
        }

        self.simulation.set_powered(
            into_index,
            self.simulation.is_powered(into_index) || self.simulation.is_powered(from_index),
        );
        self.simulation.free_cluster(from_index);
    }

    fn split_clusters(&mut self, keep: GraphNode, split: GraphNode) {
        let keep_index = self.cluster_id(&keep);
        if keep_index != self.cluster_id(&split) {
            return;
        }
        let split_cluster = self.cluster_of(&split);
        if split_cluster.contains(&keep) {
            return;
        }
        let split_index = self.simulation.alloc_cluster();
        self.simulation
            .set_powered(split_index, self.simulation.is_powered(keep_index));

        for node in &split_cluster {
            match node {
                &GraphNode::Wire(handle) => {
                    let wire = self.wires.get_mut(&handle);
                    wire.cluster_index = split_index;
                    wire.update_sprite();
                }
                &GraphNode::Component(handle, direction) => {
                    let component = self.components.get_mut(&handle);
                    match &mut component.data {
                        ComponentData::Pin(state, _sprite) => {
                            state.cluster_index = split_index;
                        }
                        ComponentData::Flip(state, _sprite) => {
                            if direction == component.orientation {
                                // Output cluster changed:
                                self.simulation.remove_flip(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flip(state.input_cluster_index, split_index);
                                state.output_cluster_index = split_index;
                            } else {
                                // Input cluster changed
                                self.simulation.remove_flip(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flip(split_index, state.output_cluster_index);
                                state.input_cluster_index = split_index;
                            }
                        }
                        ComponentData::Flop(state, _sprite) => {
                            if direction == component.orientation {
                                // Output cluster changed:
                                self.simulation.remove_flop(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flop(state.input_cluster_index, split_index);
                                state.output_cluster_index = split_index;
                            } else if direction == component.orientation.opposite() {
                                // Input cluster changed:
                                self.simulation.remove_flop(
                                    state.input_cluster_index,
                                    state.output_cluster_index,
                                );
                                self.simulation
                                    .add_flop(split_index, state.output_cluster_index);
                                state.input_cluster_index = split_index;
                            } else {
                                unreachable!()
                            }
                        }
                    }
                    component.update_sprite();
                }
            }
        }
    }

    fn split_all(&mut self, position: IVec2, directions: &[Direction]) {
        //TODO optimize
        let tile = self.tile(position).unwrap().clone();
        let nodes: Vec<GraphNode> = directions
            .iter()
            .flat_map(|&dir| tile.wires.get(dir))
            .map(|wire_handle| GraphNode::Wire(wire_handle))
            .collect();

        for i in 1..nodes.len() {
            for j in 0..i {
                self.split_clusters(nodes[i], nodes[j]);
            }
        }
    }

    fn cluster_of(&self, node: &GraphNode) -> HashSet<GraphNode> {
        let id = self.cluster_id(node);

        let mut visited = HashSet::new();
        let mut queue = Vec::new();
        visited.insert(*node);
        queue.push(*node);
        while let Some(next) = queue.pop() {
            debug_assert_eq!(self.cluster_id(&next), id);

            self.neighbors(&next, |neighbor| {
                // Only enqueue if this is the first time it has been seen.
                if visited.insert(neighbor) {
                    queue.push(neighbor);
                }
            });
        }
        visited
    }

    fn cluster_id(&self, node: &GraphNode) -> u32 {
        match node {
            &GraphNode::Wire(handle) => self.wires.get(&handle).cluster_index,
            &GraphNode::Component(handle, direction) => {
                let component = self.components.get(&handle);
                match &component.data {
                    ComponentData::Pin(state, _sprite) => state.cluster_index,
                    ComponentData::Flip(state, _sprite) => {
                        if direction == component.orientation {
                            state.output_cluster_index
                        } else {
                            state.input_cluster_index
                        }
                    }
                    ComponentData::Flop(state, _sprite) => {
                        if direction == component.orientation {
                            state.output_cluster_index
                        } else if direction == component.orientation.opposite() {
                            state.input_cluster_index
                        } else {
                            unreachable!()
                        }
                    }
                }
            }
        }
    }

    fn neighbors<V>(&self, node: &GraphNode, mut visitor: V)
    where
        V: FnMut(GraphNode),
    {
        match node {
            &GraphNode::Wire(handle) => {
                let wire = self.wires.get(&handle);
                let start_tile = self.tile(wire.start).unwrap();
                if let Some(component_handle) = start_tile.component {
                    visitor(GraphNode::Component(component_handle, wire.direction()));
                }
                let end_tile = self.tile(wire.end).unwrap();
                if let Some(component_handle) = end_tile.component {
                    visitor(GraphNode::Component(
                        component_handle,
                        wire.direction().opposite(),
                    ));
                }
            }
            &GraphNode::Component(handle, direction) => {
                let component = self.components.get(&handle);
                let tile = self.tile(component.position).unwrap();
                let component_relatives: &[Relative] = match component.get_type() {
                    ComponentType::Pin => {
                        // All faces of a pin are connected.
                        &[
                            Relative::Same,
                            Relative::Right,
                            Relative::Opposite,
                            Relative::Left,
                        ]
                    }
                    ComponentType::Flip => {
                        // Flip input faces are connected, output face is not.
                        if direction == component.orientation {
                            &[Relative::Same]
                        } else {
                            &[Relative::Right, Relative::Opposite, Relative::Left]
                        }
                    }
                    ComponentType::Flop => {
                        // Flops have no faces connected to each other.
                        if let Some(wire_handle) = tile.wires.get(direction) {
                            visitor(GraphNode::Wire(wire_handle));
                        }
                        return;
                    }
                };
                for &rel in component_relatives {
                    if let Some(wire_handle) = tile.wires.get(component.orientation.rotate(rel)) {
                        visitor(GraphNode::Wire(wire_handle));
                    }
                }
            }
        }
    }

    fn has_neighbors(&self, node: &GraphNode) -> bool {
        let mut acc = false;
        self.neighbors(node, |_| acc = true);
        acc
    }
}

pub struct TileDebugInfo<'a> {
    circuit: &'a Circuit,
    pos: IVec2,
}

impl<'a> fmt::Display for TileDebugInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(tile) = self.circuit.tile(self.pos) {
            if let Some(component_handle) = tile.component {
                let component = self.circuit.components.get(&component_handle);
                match &component.data {
                    ComponentData::Pin(state, _sprite) => {
                        writeln!(f, "Component: Pin ({})", state.cluster_index)?;
                    }
                    ComponentData::Flip(state, _sprite) => {
                        writeln!(
                            f,
                            "Component: Flip ({} -> {})",
                            state.input_cluster_index, state.output_cluster_index,
                        )?;
                    }
                    ComponentData::Flop(state, _sprite) => {
                        writeln!(
                            f,
                            "Component: Flop ({} -> {})",
                            state.input_cluster_index, state.output_cluster_index,
                        )?;
                    }
                }
            }
            let directions = [
                Direction::East,
                Direction::West,
                Direction::North,
                Direction::South,
            ];
            for direction in directions {
                if let Some(wire_handle) = tile.wires.get(direction) {
                    let wire = self.circuit.wires.get(&wire_handle);
                    writeln!(f, "{:?} ({})", direction, wire.cluster_index)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct Tile {
    pub component: Option<depot::Handle>,
    pub crossover: Option<Rc<rect::Handle>>,
    pub wires: TileWires,
}

impl Tile {
    fn update_crossover(&mut self, position: IVec2, renderer: &mut RectRenderer) {
        let wire_count = self.wires.count();
        if self.component.is_some() || wire_count < 2 {
            self.crossover = None;
        } else if wire_count >= 2 && self.crossover.is_none() {
            let handle = renderer.insert(&rect::Crossover { position }.into());
            self.crossover = Some(Rc::new(handle));
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

    pub fn get_mut(&mut self, direction: Direction) -> &mut Option<depot::Handle> {
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

    fn update_sprite(&self) {
        match &self.data {
            ComponentData::Pin(state, sprite) => {
                sprite.pin.set(
                    &rect::Pin {
                        position: self.position,
                        color: Color::Wire {
                            cluster_index: state.cluster_index,
                            delayed: false,
                            inverted: false,
                        },
                    }
                    .into(),
                );
            }
            ComponentData::Flip(state, sprite) => {
                sprite.body.set(
                    &rect::Body {
                        position: self.position,
                    }
                    .into(),
                );
                sprite.input.set(
                    &rect::Pin {
                        position: self.position,
                        color: Color::Wire {
                            cluster_index: state.input_cluster_index,
                            delayed: false,
                            inverted: false,
                        },
                    }
                    .into(),
                );
                sprite.output.set(
                    &rect::Output {
                        position: self.position,
                        orientation: self.orientation,
                        color: Color::Wire {
                            cluster_index: state.input_cluster_index,
                            delayed: true,
                            inverted: true,
                        },
                    }
                    .into(),
                );
            }
            ComponentData::Flop(state, sprite) => {
                sprite.body.set(
                    &rect::Body {
                        position: self.position,
                    }
                    .into(),
                );
                sprite.input.set(
                    &rect::SidePin {
                        position: self.position,
                        orientation: self.orientation.opposite(),
                        color: Color::Wire {
                            cluster_index: state.input_cluster_index,
                            delayed: false,
                            inverted: false,
                        },
                    }
                    .into(),
                );
                sprite.output.set(
                    &rect::Output {
                        position: self.position,
                        orientation: self.orientation,
                        color: Color::Wire {
                            cluster_index: state.input_cluster_index,
                            delayed: true,
                            inverted: false,
                        },
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
    cluster_index: u32,
}

struct PinSprite {
    pin: rect::Handle,
}

struct FlipState {
    input_cluster_index: u32,
    output_cluster_index: u32,
}

struct FlipSprite {
    body: rect::Handle,
    input: rect::Handle,
    output: rect::Handle,
}

struct FlopState {
    input_cluster_index: u32,
    output_cluster_index: u32,
}

struct FlopSprite {
    body: rect::Handle,
    input: rect::Handle,
    output: rect::Handle,
}

struct Wire {
    start: IVec2,
    end: IVec2,
    start_connection: WireConnection,
    end_connection: WireConnection,
    cluster_index: u32,
    instance: rect::Handle,
}

impl Wire {
    fn tiles(&self) -> impl Iterator<Item = IVec2> {
        wire_tiles(self.start, self.end)
    }

    fn direction(&self) -> Direction {
        wire_direction(self.start, self.end)
    }

    fn update_sprite(&self) {
        self.instance.set(
            &rect::Wire {
                start: self.start,
                end: self.end,
                start_connection: self.start_connection,
                end_connection: self.end_connection,
                color: Color::Wire {
                    cluster_index: self.cluster_index,
                    delayed: false,
                    inverted: false,
                },
            }
            .into(),
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum GraphNode {
    Wire(depot::Handle),
    Component(depot::Handle, Direction),
}

pub fn wire_direction(start: IVec2, end: IVec2) -> Direction {
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
