use crate::GraphicsContext;
use bytemuck::Pod;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

pub struct InstanceManager<T> {
    gfx: GraphicsContext,
    buffer: Option<wgpu::Buffer>,
    buffer_capacity: usize,

    update_tx: mpsc::Sender<Update<T>>,
    update_rx: mpsc::Receiver<Update<T>>,
    instances: Vec<T>,
    instance_to_handle: Vec<u64>,
    handle_to_instance: HashMap<u64, usize>,
    buffer_update: bool,
}

impl<T> InstanceManager<T>
where
    T: Pod,
{
    pub fn new(gfx: &GraphicsContext) -> Self {
        let (update_tx, update_rx) = mpsc::channel();

        Self {
            gfx: gfx.clone(),
            buffer: None,
            buffer_capacity: 0,

            update_tx,
            update_rx,
            instances: Vec::new(),
            instance_to_handle: Vec::new(),
            handle_to_instance: HashMap::new(),
            buffer_update: false,
        }
    }

    pub fn insert(&mut self, instance: T) -> Handle<T> {
        let handle = Handle::new(self.update_tx.clone());
        handle.set(instance);
        handle
    }

    fn set(&mut self, handle: u64, instance: T) {
        self.buffer_update = true;

        if let Some(&index) = self.handle_to_instance.get(&handle) {
            self.instances[index] = instance;
        } else {
            let index = self.instances.len();
            self.instances.push(instance);
            self.instance_to_handle.push(handle);
            self.handle_to_instance.insert(handle, index);
        }
    }

    fn remove(&mut self, handle: u64) {
        self.buffer_update = true;

        let index = self.handle_to_instance.remove(&handle).unwrap();
        self.instances.swap_remove(index);

        let removed_handle = self.instance_to_handle.swap_remove(index);
        debug_assert!(removed_handle == handle);

        if index != self.instances.len() {
            // Update handle association for the instance that was swapped to this location.
            let affected_handle = self.instance_to_handle[index];
            self.handle_to_instance.insert(affected_handle, index);
        }
    }

    fn handle_updates(&mut self) {
        while let Ok(update) = self.update_rx.try_recv() {
            match update {
                Update::Set(handle, instance) => self.set(handle, instance),
                Update::Remove(handle) => self.remove(handle),
            }
        }
    }

    pub fn buffer(&mut self) -> Option<&wgpu::Buffer> {
        self.handle_updates();
        if self.buffer_update {
            self.buffer_update = false;

            self.ensure_capacity(self.instances.len());
            if let Some(buffer) = &self.buffer {
                let src_bytes: &[u8] = bytemuck::cast_slice(&self.instances);
                self.gfx.queue.write_buffer(buffer, 0, src_bytes);
            }
        }
        self.buffer.as_ref()
    }

    pub fn len(&self) -> usize {
        self.instances.len()
    }

    fn ensure_capacity(&mut self, cap: usize) {
        if cap > self.buffer_capacity {
            let new_cap = cap.checked_next_power_of_two().unwrap();
            let bytes = std::mem::size_of::<T>() * new_cap;

            let buffer = self.gfx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{}.buffer", std::any::type_name::<Self>())),
                size: bytes.try_into().unwrap(),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.buffer = Some(buffer);
            self.buffer_capacity = new_cap;
        }
    }
}

enum Update<T> {
    Set(u64, T),
    Remove(u64),
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0);

pub struct Handle<T> {
    id: u64,
    updates: mpsc::Sender<Update<T>>,
}

impl<T> Handle<T> {
    fn new(updates: mpsc::Sender<Update<T>>) -> Self {
        let id = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        // Prevent overflow:
        if id == u64::MAX {
            panic!("max instance handle reached")
        }
        Self { id, updates }
    }

    pub fn set(&self, instance: T) {
        self.updates.send(Update::Set(self.id, instance)).ok();
    }
}

impl<T> Drop for Handle<T> {
    fn drop(&mut self) {
        self.updates.send(Update::Remove(self.id)).ok();
    }
}
