use crate::GraphicsContext;
use bytemuck::Pod;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct InstanceManager<T> {
    gfx: GraphicsContext,
    buffer: Option<wgpu::Buffer>,
    buffer_capacity: usize,

    instances: Vec<T>,
    instance_to_handle: Vec<Handle>,
    handle_to_instance: HashMap<Handle, usize>,
    buffer_update: bool,
}

impl<T> InstanceManager<T>
where
    T: Pod,
{
    pub fn new(gfx: &GraphicsContext) -> Self {
        Self {
            gfx: gfx.clone(),
            buffer: None,
            buffer_capacity: 0,

            instances: Vec::new(),
            instance_to_handle: Vec::new(),
            handle_to_instance: HashMap::new(),
            buffer_update: false,
        }
    }

    pub fn insert(&mut self, instance: T) -> Handle {
        let handle = Handle::new();
        self.update(&handle, instance);
        handle
    }

    pub fn update(&mut self, handle: &Handle, instance: T) {
        self.buffer_update = true;
        if let Some(&index) = self.handle_to_instance.get(handle) {
            self.instances[index] = instance;
        } else {
            let index = self.instances.len();
            self.instances.push(instance);
            self.instance_to_handle.push(handle.clone());
            self.handle_to_instance.insert(handle.clone(), index);
        }
    }

    pub fn remove(&mut self, handle: &Handle) -> bool {
        // If the handle exists for this renderer:
        if let Some(index) = self.handle_to_instance.remove(handle) {
            self.buffer_update = true;

            self.instances.swap_remove(index);

            let removed_handle = self.instance_to_handle.swap_remove(index);
            debug_assert!(removed_handle == *handle);

            if index != self.instances.len() {
                // Update handle association for the instance that was swapped to this location.
                let affected_handle = &self.instance_to_handle[index];
                self.handle_to_instance
                    .insert(affected_handle.clone(), index);
            }

            true
        } else {
            false
        }
    }

    pub fn buffer(&mut self) -> Option<&wgpu::Buffer> {
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

            let buffer =
                self.gfx.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!(
                        "{}.buffer",
                        std::any::type_name::<Self>()
                    )),
                    size: bytes.try_into().unwrap(),
                    usage: wgpu::BufferUsage::VERTEX
                        | wgpu::BufferUsage::COPY_DST,
                    mapped_at_creation: false,
                });
            self.buffer = Some(buffer);
            self.buffer_capacity = new_cap;
        }
    }
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    fn new() -> Self {
        let val = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        // Prevent overflow:
        if val == u64::MAX {
            panic!("max instance handle reached")
        }
        Self(val)
    }
}
