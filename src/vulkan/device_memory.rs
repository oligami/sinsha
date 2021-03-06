pub mod alloc;
pub mod buffer;
pub mod image;

pub use memory_property::MemoryProperty;

use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::*;

use super::*;
use alloc::*;
use std::ptr;
use std::mem;
use std::slice;
use std::sync::Mutex;
use std::alloc::Layout;
use std::ops::{ RangeBounds, Bound };

pub struct VkAlloc;

pub struct DataIdent;
pub struct ImageIdent;

impl VkAlloc {
    pub fn alloc_data(&mut self, size: u64, usage: (), flags: ()) -> DataIdent {
        unimplemented!()
    }

    pub fn alloc_image(&mut self, size: u64, usage: (), flags: ()) -> ImageIdent {
        unimplemented!()
    }
}


pub struct DeviceMemory<I, D, A> where D: Borrow<Device<I>> {
    _instance: PhantomData<I>,
    device: D,
    handle: vk::DeviceMemory,
    type_index: u32,
    allocator: A,
    size: u64,
}

pub struct DeviceMemoryMapper<I, D, A, M> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>>,
{
    _marker: PhantomData<(I, D, A)>,
    device_memory: M,
    address: usize,
}

#[derive(Debug)]
pub enum MemoryErr {
    Vk(vk::Result),
    NoValidMemoryTypeIndex,
}


impl<I, D, A> DeviceMemory<I, D, A> where I: Borrow<Vulkan>, D: Borrow<Device<I>>, A: Allocator {
    pub fn with_allocator(
        device: D,
        allocator: A,
        memory_properties: vk::MemoryPropertyFlags,
    ) -> Result<Self, MemoryErr> {
        let type_index = Self::compatible_memory_type_indices(&device, memory_properties)
            .into_iter()
            .next()
            .ok_or(MemoryErr::NoValidMemoryTypeIndex)?;
        let size = allocator.size();

        let handle = unsafe {
            let info = vk::MemoryAllocateInfo::builder()
                .allocation_size(size)
                .memory_type_index(type_index);

            device.borrow().handle.allocate_memory(&*info, None)?
        };

        let memory = Self {
            _instance: PhantomData,
            device,
            handle,
            type_index,
            size: allocator.size(),
            allocator,
        };

        Ok(memory)
    }

    fn compatible_memory_type_indices(device: &D, flags: vk::MemoryPropertyFlags) -> Vec<u32> {
        device.borrow()
            .instance.borrow().physical_devices[device.borrow().physical_device_index]
            .memory_types
            .iter()
            .enumerate()
            .fold(Vec::new(), |mut indices, (index, memory_type)| {
                if memory_type.property_flags.contains(flags) {
                    indices.push(index as u32);
                }
                indices
            })
    }

    #[inline]
    pub fn size(&self) -> u64 { self.size }
}

impl<I, D, A> Drop for DeviceMemory<I, D, A> where D: Borrow<Device<I>> {
    fn drop(&mut self) { unsafe { self.device.borrow().handle.free_memory(self.handle, None) } }
}

impl<I, D, A, M> DeviceMemoryMapper<I, D, A, M> where
    I: Borrow<Vulkan>,
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>>,
{
    pub unsafe fn map_whole_size(device_memory: M) -> DeviceMemoryMapper<I, D, A, M> {
        let size = device_memory.borrow().size;
        Self::map(device_memory, 0..size)
    }

    pub unsafe fn map<R>(device_memory: M, range: R) -> DeviceMemoryMapper<I, D, A, M> where
        R: RangeBounds<u64>,
    {
        let contains_host_visible_flag = device_memory.borrow().device.borrow().instance.borrow()
            .physical_devices[device_memory.borrow().device.borrow().physical_device_index]
            .memory_types[device_memory.borrow().type_index as usize]
            .property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
        debug_assert!(contains_host_visible_flag);
        let start = match range.start_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n + 1,
            Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            Bound::Included(n) => *n,
            Bound::Excluded(n) => *n - 1,
            Bound::Unbounded => device_memory.borrow().size,
        };

        let address = device_memory.borrow().device.borrow().handle
            .map_memory(
                device_memory.borrow().handle,
                start,
                end - start,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap() as usize;

        DeviceMemoryMapper { _marker: PhantomData, device_memory, address }
    }

    #[inline]
    pub fn as_ptr<T>(&self) -> *const T { self.address as *const T }
    #[inline]
    pub fn as_mut_ptr<T>(&self) -> *mut T { self.address as *mut T }
}

impl<I, D, A, M> Drop for DeviceMemoryMapper<I, D, A, M> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, A>>
{
    fn drop(&mut self) {
        unsafe {
            self.device_memory.borrow().device.borrow().handle
                .unmap_memory(self.device_memory.borrow().handle);
        }
    }
}

impl From<vk::Result> for MemoryErr {
    fn from(v: vk::Result) -> Self { MemoryErr::Vk(v) }
}


mod memory_property {
    use ash::vk;

    pub struct MemoryProperty {
        flags: vk::MemoryPropertyFlags,
    }

    impl MemoryProperty {
        pub fn vk_flags(&self) -> vk::MemoryPropertyFlags { self.flags }
        pub fn empty() -> Self { Self { flags: vk::MemoryPropertyFlags::empty() } }
        pub fn device_local(&mut self) -> &mut Self {
            self.flags |= vk::MemoryPropertyFlags::DEVICE_LOCAL; self
        }
        pub fn host_visible(&mut self) -> &mut Self {
            self.flags |= vk::MemoryPropertyFlags::HOST_VISIBLE; self
        }
        pub fn host_coherent(&mut self) -> &mut Self {
            self.flags |= vk::MemoryPropertyFlags::HOST_COHERENT; self
        }
        pub fn host_cached(&mut self) -> &mut Self {
            self.flags |= vk::MemoryPropertyFlags::HOST_CACHED; self
        }
        pub fn lazily_allocated(&mut self) -> &mut Self {
            self.flags |= vk::MemoryPropertyFlags::LAZILY_ALLOCATED; self
        }
        pub fn protected(&mut self) -> &mut Self {
            self.flags |= vk::MemoryPropertyFlags::PROTECTED; self
        }
    }
}