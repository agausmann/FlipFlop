use std::time::{Duration, Instant};

const UPDATE_INTERVAL: Duration = Duration::from_millis(200);

pub struct Counter {
    since: Instant,
    count: usize,
    rate: f32,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            since: Instant::now(),
            count: 0,
            rate: 0.0,
        }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        let interval = now - self.since;
        if interval >= UPDATE_INTERVAL {
            self.since = now;
            self.rate = (self.count as f32) / interval.as_secs_f32();
            self.count = 0;
        }
        self.count += 1;
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }
}
