use std::collections::HashMap;

pub struct Simulation {
    // Tracks unused cluster indexes so they can be re-used.
    num_clusters: u32,
    free_clusters: Vec<u32>,

    is_powered: Vec<bool>,
    was_powered: Vec<bool>,

    // Flip connections are multi-sets, because there's nothing stopping
    // the player from adding multiple flips/flops and so we need to keep
    // track of how many there are.
    flips: Vec<HashMap<u32, u32>>,
    flops: Vec<HashMap<u32, u32>>,

    manual_power: Vec<u32>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            num_clusters: 0,
            free_clusters: Vec::new(),
            is_powered: Vec::new(),
            was_powered: Vec::new(),
            flips: Vec::new(),
            flops: Vec::new(),
            manual_power: Vec::new(),
        }
    }

    pub fn num_clusters(&self) -> u32 {
        self.num_clusters
    }

    /// Allocates a new cluster ID that is not currently being used.
    pub fn alloc_cluster(&mut self) -> u32 {
        if let Some(id) = self.free_clusters.pop() {
            id
        } else {
            let id = self.num_clusters;
            self.num_clusters += 1;

            self.is_powered.push(false);
            self.was_powered.push(false);
            self.flips.push(HashMap::new());
            self.flops.push(HashMap::new());
            self.manual_power.push(0);

            id
        }
    }

    /// Frees the given cluster, allowing the ID to be re-used.
    pub fn free_cluster(&mut self, id: u32) {
        let index = cluster_array_index(id);
        assert!(self.flips[index].is_empty());
        assert!(self.flops[index].is_empty());
        assert!(self.manual_power[index] == 0);
        self.free_clusters.push(id);
    }

    pub fn add_flip(&mut self, inp: u32, out: u32) {
        let out = cluster_array_index(out);
        *self.flips[out].entry(inp).or_insert(0) += 1;
    }

    pub fn add_flop(&mut self, inp: u32, out: u32) {
        let out = cluster_array_index(out);
        *self.flops[out].entry(inp).or_insert(0) += 1;
    }

    pub fn remove_flip(&mut self, inp: u32, out: u32) {
        let out = cluster_array_index(out);
        let count = self.flips[out].get_mut(&inp).unwrap();
        *count -= 1;
        if *count == 0 {
            self.flips[out].remove(&inp);
        }
    }

    pub fn remove_flop(&mut self, inp: u32, out: u32) {
        let out = cluster_array_index(out);
        let count = self.flops[out].get_mut(&inp).unwrap();
        *count -= 1;
        if *count == 0 {
            self.flops[out].remove(&inp);
        }
    }

    pub fn power(&mut self, id: u32) {
        let id = cluster_array_index(id);
        self.manual_power[id] += 1;
    }

    pub fn unpower(&mut self, id: u32) {
        let id = cluster_array_index(id);
        self.manual_power[id] -= 1;
    }

    pub fn is_powered(&self, id: u32) -> bool {
        let id = cluster_array_index(id);
        self.is_powered[id]
    }

    pub fn was_powered(&self, id: u32) -> bool {
        let id = cluster_array_index(id);
        self.was_powered[id]
    }

    pub fn set_powered(&mut self, id: u32, powered: bool) {
        let id = cluster_array_index(id);
        self.is_powered[id] = powered;
    }

    pub fn tick(&mut self) {
        std::mem::swap(&mut self.is_powered, &mut self.was_powered);

        for i in 0..self.num_clusters {
            let i = cluster_array_index(i);
            self.is_powered[i] = self.manual_power[i] > 0
                || self.flips[i].iter().any(|(&id, _)| !self.was_powered(id))
                || self.flops[i].iter().any(|(&id, _)| self.was_powered(id));
        }
    }
}

fn cluster_array_index(idx: u32) -> usize {
    idx.try_into().unwrap()
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
