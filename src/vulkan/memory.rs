use super::*;

use std::borrow::Borrow;
use std::ops::BitAnd;

/// TODO: delete borrow of Vulkan.
pub struct DeviceMemory<V: Borrow<V>> {
    vulkan: V,
    handle: vk::DeviceMemory,
    size: u64,
    memory_type: usize,
}

pub struct DeviceMemoryBuilder {
    size: u64,
    memory_type_bits: MemoryTypeBits,
    memory_properties: vk::MemoryPropertyFlags,
}

#[derive(Copy, Clone, Debug)]
pub struct MemoryTypeBits(u32);


/// TODO: delete borrow of Vulkan.
pub struct Buffer<V: Borrow<Vulkan>> {
    vulkan: V,
    handle: vk::Buffer,
    size: u64,
    align: u64,
    offset: u64,
    usage: vk::BufferUsageFlags,
    compatible_memory_type_bits: MemoryTypeBits,
}

pub struct BufferBuilder {
    size: u64,
    usage: vk::BufferUsageFlags,
}

pub struct Image<V: Borrow<Vulkan>> {
    vulkan: V,
    handle: vk::Image,
    extent: vk::Extent3D,
    usage: vk::ImageUsageFlags,
}

impl<V: Borrow<Vulkan>> DeviceMemory<V> {
    pub fn builder() -> DeviceMemoryBuilder {
        DeviceMemoryBuilder {
            size: 0x1000,
            memory_type_bits: MemoryTypeBits(1),
            memory_properties: vk::MemoryPropertyFlags::empty(),
        }
    }
}

impl DeviceMemoryBuilder {
    pub fn size(&mut self, size: u64) -> &mut Self {
        self.size = size;
        self
    }

    pub fn memory_type_bits(&mut self, memory_type_bits: MemoryTypeBits) -> &mut Self {
        self.memory_type = memory_type_bits;
        self
    }

    pub fn properties(&mut self, properties: vk::MemoryPropertyFlags) -> &mut Self {
        self.memory_properties = properties;
        self
    }

    pub fn build<V: Borrow<Vulkan>>(&self, vulkan: V) -> Result<DeviceMemory<V>, u32> {
        let vulkan_ref = vulkan.borrow();

        // Search compatible memory type index.
        let memory_type_index = vulkan_ref.physical_device.memory_types
            .iter()
            .enumerate()
            .position(|(i, memory_type)| {
                let properties_satiffied = memory_type
                    .property_flags
                    .contain(self.memory_properties);

                let memory_type_bits_satisfied = (1 << i as u32) & self.memory_type_bits.0 != 0;

                properties_satiffied && memory_type_bits_satisfied
            })? as u32;

        // Allocate.
        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(self.size)
            .memory_type_index(memory_type_index)
            .build();

        let handle = unsafe { vulkan_ref.device.allocate_memory(&info, None).unwrap() };

        DeviceMemory {
            vulkan,
            handle,
            size: self.size,
            memory_type: self.memory_type as usize,
        }
    }
}

impl BitAnd for MemoryTypeBits {
    type Output = Self;
    fn bitand(self, rhs: MemoryTypeBits) -> Self::Output {
        MemoryTypeBits(self.0 & rhs.0)
    }
}

impl BufferBuilder {
    pub fn compatible_memory_type_bits(&self, vulkan: &Vulkan) -> MemoryTypeBits {
        let info = vk::BufferCreateInfo::builder()
            .size(self.size)
            .usage(self.usage)
            .sharing_mode(unimplemented!())
            .queue_family_indices(unimplemented!())
            .build();
        let handle = unsafe { vulkan.device.create_buffer(&info, None).unwrap() };
        let requirements = unsafe { vulkan.device.get_buffer_memory_requirements(handle) };
        unsafe { vulkan.device.destroy_buffer(handle, None); }

        MemoryTypeBits(requirements.memory_type_bits)
    }

    pub fn build<V: Borrow<Vulkan>>(&self, vulkan: V) -> Buffer<V> {
        let device_ref = &vulkan.borrow().device;

        let info = vk::BufferCreateInfo::builder()
            .size(self.size)
            .usage(self.usage)
            .sharing_mode(unimplemented!())
            .queue_family_indices(unimplemented!())
            .build();

        let handle = unsafe { device_ref.create_buffer(&info, None).unwrap() };

        let requirements = unsafe { device_ref.get_buffer_memory_requirements(handle) };

        Buffer {
            vulkan,
            handle,
            size: requirements.size,
            align: requirements.alignment,
            offset: unimplemented!(),
            usage: self.usage,
            compatible_memory_type_bits: MemoryTypeBits(requirements.memory_type_bits),
        }

    }
}