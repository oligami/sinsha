use super::*;

pub struct Memory;

pub enum MemoryType {
    DeviceLocal,
    HostVisible,
}

pub enum Lifetime {
    Short,
    Long,
}

impl Memory {
    pub fn alloc_data(&mut self, memory_type: MemoryType, lifetime: Lifetime) {}
    pub fn alloc_image(&mut self, memory_type: MemoryType, lifetime: Lifetime) {}
    pub fn flush(&mut self) {}
    pub fn dealloc_data(&mut self) {}
    pub fn dealloc_image(&mut self) {}
}