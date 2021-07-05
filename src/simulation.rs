use std::collections::HashMap;

pub struct Simulation {
    // Tracks unused cluster indexes so they can be re-used.
    num_clusters: usize,
    free_clusters: Vec<usize>,

    powered: Vec<bool>,
    was_powered: Vec<bool>,

    // Flip connections are multi-sets, because there's nothing stopping
    // the player from adding multiple flips/flops and so we need to keep
    // track of how many there are.
    flips: Vec<HashMap<usize, usize>>,
    flops: Vec<HashMap<usize, usize>>,

    manual_power: Vec<usize>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            num_clusters: 0,
            free_clusters: Vec::new(),
            powered: Vec::new(),
            was_powered: Vec::new(),
            flips: Vec::new(),
            flops: Vec::new(),
            manual_power: Vec::new(),
        }
    }

    /// Allocates a new cluster ID that is not currently being used.
    pub fn alloc_cluster(&mut self) -> usize {
        if let Some(id) = self.free_clusters.pop() {
            id
        } else {
            let id = self.num_clusters;
            self.num_clusters += 1;

            self.powered.push(false);
            self.was_powered.push(false);
            self.flips.push(HashMap::new());
            self.flops.push(HashMap::new());
            self.manual_power.push(0);

            id
        }
    }

    /// Frees the given cluster, allowing the ID to be re-used.
    pub fn free_cluster(&mut self, id: usize) {
        assert!(self.flips[id].is_empty());
        assert!(self.flops[id].is_empty());
        assert!(self.manual_power[id] == 0);
        self.free_clusters.push(id);
    }

    pub fn add_flip(&mut self, inp: usize, out: usize) {
        *self.flips[out].entry(inp).or_insert(0) += 1;
    }

    pub fn add_flop(&mut self, inp: usize, out: usize) {
        *self.flops[out].entry(inp).or_insert(0) += 1;
    }

    pub fn remove_flip(&mut self, inp: usize, out: usize) {
        let count = self.flips[out].get_mut(&inp).unwrap();
        *count -= 1;
        if *count == 0 {
            self.flips[out].remove(&inp);
        }
    }

    pub fn remove_flop(&mut self, inp: usize, out: usize) {
        let count = self.flops[out].get_mut(&inp).unwrap();
        *count -= 1;
        if *count == 0 {
            self.flops[out].remove(&inp);
        }
    }

    pub fn power(&mut self, id: usize) {
        self.manual_power[id] += 1;
    }

    pub fn unpower(&mut self, id: usize) {
        self.manual_power[id] -= 1;
    }

    pub fn is_powered(&mut self, id: usize) -> bool {
        self.powered[id]
    }

    pub fn tick(&mut self) {
        std::mem::swap(&mut self.powered, &mut self.was_powered);

        for i in 0..self.num_clusters {
            self.powered[i] = self.manual_power[i] > 0
                || self.flips[i].iter().any(|(&id, _)| !self.was_powered[id])
                || self.flops[i].iter().any(|(&id, _)| self.was_powered[id]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Simulation;

    #[test]
    fn feedback_flip() {
        let mut sim = Simulation::new();

        let cluster = sim.alloc_cluster();
        sim.add_flip(cluster, cluster);
        assert!(!sim.is_powered(cluster));
        for _ in 0..10 {
            sim.tick();
            assert!(sim.is_powered(cluster));
            sim.tick();
            assert!(!sim.is_powered(cluster));
        }
        sim.remove_flip(cluster, cluster);
        sim.tick();
        assert!(!sim.is_powered(cluster));
        sim.free_cluster(cluster);
    }

    #[test]
    fn sr_latch_astable() {
        let mut sim = Simulation::new();

        let a = sim.alloc_cluster();
        let b = sim.alloc_cluster();
        sim.add_flip(a, b);
        sim.add_flip(b, a);

        for _ in 0..10 {
            sim.tick();
            assert!(sim.is_powered(a));
            assert!(sim.is_powered(b));
            sim.tick();
            assert!(!sim.is_powered(a));
            assert!(!sim.is_powered(a));
        }
        sim.remove_flip(a, b);
        sim.remove_flip(b, a);
        sim.tick();
        assert!(!sim.is_powered(a));
        assert!(!sim.is_powered(a));
        sim.free_cluster(a);
        sim.free_cluster(b);
    }

    #[test]
    fn sr_latch_stable() {
        let mut sim = Simulation::new();

        let a = sim.alloc_cluster();
        let b = sim.alloc_cluster();
        sim.add_flip(a, b);
        sim.tick();
        sim.add_flip(b, a);

        for _ in 0..10 {
            sim.tick();
            assert!(!sim.is_powered(a));
            assert!(sim.is_powered(b));
        }

        sim.power(a);
        sim.tick();
        sim.tick();
        sim.unpower(a);

        for _ in 0..10 {
            sim.tick();
            assert!(sim.is_powered(a));
            assert!(!sim.is_powered(b));
        }

        sim.remove_flip(a, b);
        sim.remove_flip(b, a);
        sim.free_cluster(a);
        sim.free_cluster(b);
    }

    #[test]
    fn sr_latch_enter_astable() {
        let mut sim = Simulation::new();

        let a = sim.alloc_cluster();
        let b = sim.alloc_cluster();
        sim.add_flip(a, b);
        sim.tick();
        sim.add_flip(b, a);

        for _ in 0..10 {
            sim.tick();
            assert!(!sim.is_powered(a));
            assert!(sim.is_powered(b));
        }

        sim.power(a);
        sim.tick();
        sim.unpower(a);

        for _ in 0..10 {
            sim.tick();
            assert!(!sim.is_powered(a));
            assert!(!sim.is_powered(b));
            sim.tick();
            assert!(sim.is_powered(a));
            assert!(sim.is_powered(b));
        }

        sim.remove_flip(a, b);
        sim.remove_flip(b, a);
        sim.free_cluster(a);
        sim.free_cluster(b);
    }
}
