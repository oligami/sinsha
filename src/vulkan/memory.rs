mod allocator;

use super::*;

use std::borrow::Borrow;
use std::ops::BitAnd;

pub struct DeviceMemory {
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

pub struct Buffer {
    handle: vk::Buffer,
    requirements: vk::MemoryRequirements,
    usage: vk::BufferUsageFlags,
    compatible_memory_type_bits: MemoryTypeBits,
}

pub struct BufferBuilder {
    size: u64,
    usage: vk::BufferUsageFlags,
}

pub struct Image {
    handle: vk::Image,
    extent: vk::Extent3D,
    usage: vk::ImageUsageFlags,
}

impl DeviceMemory {
    pub fn builder() -> DeviceMemoryBuilder {
        DeviceMemoryBuilder {
            size: 0x1000,
            memory_type_bits: MemoryTypeBits(!0),
            memory_properties: vk::MemoryPropertyFlags::empty(),
        }
    }

    pub unsafe fn free(self, vulkan: &Vulkan) {
        vulkan.device.free_memory(self.handle, None);
    }
}

impl DeviceMemoryBuilder {
    pub fn size(&mut self, size: u64) -> &mut Self {
        self.size = size;
        self
    }

    pub fn memory_type_bits(&mut self, memory_type_bits: MemoryTypeBits) -> &mut Self {
        self.memory_type.0 = self.memory_type_bits.0 & memory_type_bits.0;
        self
    }

    pub fn properties(&mut self, properties: vk::MemoryPropertyFlags) -> &mut Self {
        self.memory_properties = properties;
        self
    }

    pub fn build(&self, vulkan: &Vulkan) -> Result<DeviceMemory, u32> {
        // Search compatible memory type index.
        let memory_properties = unsafe {
            vulkan.instance.get_physical_device_memory_properties(vulkan.physical_device)
        };

        let memory_type_index = memory_properties
            .memory_types[0..memory_properties.memory_type_count as usize]
            .iter()
            .enumerate()
            .position(|(i, memory_type)| {
                let properties_satiffied = memory_type
                    .property_flags
                    .contain(self.memory_properties);

                let memory_type_bits_satisfied = (1 << i as u32) & self.memory_type_bits.0 != 0;

                properties_satiffied && memory_type_bits_satisfied
            })
            .unwrap() as u32;

        // Allocate.
        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(self.size)
            .memory_type_index(memory_type_index)
            .build();

        let handle = unsafe { vulkan.device.allocate_memory(&info, None).unwrap() };

        Ok(
            DeviceMemory {
                handle,
                size: self.size,
                memory_type: memory_type_index as usize,
            }
        )
    }
}

impl Buffer {
    pub unsafe fn destroy(self, vulkan: &Vulkan) {
        vulkan.device.destroy_buffer(self.handle, None);
    }

    pub fn compatible_memory_type_bits(&self, vulkan: &Vulkan) -> MemoryTypeBits {
        MemoryTypeBits(self.requirements.memory_type_bits)
    }
}

impl BufferBuilder {
    pub fn size(&mut self, size: u64) -> &mut Self {
        self.size = size;
        self
    }

    pub fn usage(&mut self, usage: vk::BufferUsageFlags) -> &mut Self {
        self.usage = usage;
        self
    }

    pub fn build(&self, vulkan: &Vulkan) -> Buffer {
        let device = &vulkan.device;

        let info = vk::BufferCreateInfo::builder()
            .size(self.size)
            .usage(self.usage)
            .sharing_mode(unimplemented!())
            .queue_family_indices(unimplemented!())
            .build();

        let handle = unsafe { device.create_buffer(&info, None).unwrap() };

        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };

        Buffer {
            handle,
            requirements,
            usage: self.usage,
            compatible_memory_type_bits: MemoryTypeBits(requirements.memory_type_bits),
        }

    }
}