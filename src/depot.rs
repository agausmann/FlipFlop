use std::collections::HashMap;
use std::ops;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Depot<T> {
    items: HashMap<Handle, T>,
}

impl<T> Depot<T> {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    pub fn insert(&mut self, item: T) -> Handle {
        let handle = Handle::new();
        self.items.insert(handle.clone(), item);
        handle
    }

    pub fn get(&self, handle: &Handle) -> &T {
        self.items
            .get(handle)
            .expect("handle is invalid for this depot")
    }

    pub fn get_mut(&mut self, handle: &Handle) -> &mut T {
        self.items
            .get_mut(handle)
            .expect("handle is invalid for this depot")
    }

    pub fn remove(&mut self, handle: &Handle) -> T {
        self.items
            .remove(handle)
            .expect("handle is invalid for this depot")
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<'a, T> ops::Index<&'a Handle> for Depot<T> {
    type Output = T;

    fn index(&self, idx: &'a Handle) -> &Self::Output {
        self.get(idx)
    }
}

impl<'a, T> ops::IndexMut<&'a Handle> for Depot<T> {
    fn index_mut(&mut self, idx: &'a Handle) -> &mut Self::Output {
        self.get_mut(idx)
    }
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    fn new() -> Self {
        let val = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        // Prevent overflow:
        if val == u64::MAX {
            panic!("max depot handle reached")
        }
        Self(val)
    }
}
