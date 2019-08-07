pub mod alloc;
pub mod buffer;
pub mod image;
pub mod memory_property;

pub use alloc::*;
pub use buffer::*;
pub use image::*;

pub use memory_property::{
    MemoryProperties,
    MemoryProperty,
    DeviceLocal,
    HostVisible,
    HostCoherent,
    HostCached,
    LazilyAllocated,
    Protected,
};

use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::*;

use super::*;
use std::ptr;
use std::mem;
use std::slice;
use std::sync::Mutex;
use std::alloc::Layout;

pub struct DeviceMemory<I, D, A, P>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          A: Allocator,
          P: MemoryProperty
{
    _instance: PhantomData<I>,
    device: D,
    handle: vk::DeviceMemory,
    type_index: u32,
    allocator: Mutex<A>,
    size: u64,
    access: Mutex<MemoryAccess>,
    _properties: PhantomData<P>,
}

pub trait DeviceMemoryAbs {
    type Instance: Borrow<Instance> + Deref<Target = Instance>;
    type Device: Borrow<Device<Self::Instance>> + Deref<Target = Device<Self::Instance>>;
    type Allocator: Allocator;
    type MemoryProperty: MemoryProperty;

    fn instance(&self) -> &Instance;
    fn device(&self) -> &Device<Self::Instance>;
    fn handle(&self) -> vk::DeviceMemory;
    fn size(&self) -> u64;
}

struct MemoryAccess {
    count: usize,
    pointer: usize,
}

#[derive(Debug)]
pub enum MemoryErr {
    Vk(vk::Result),
    NoValidMemoryTypeIndex,
}


impl<I, D, A, P> DeviceMemory<I, D, A, P>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          A: Allocator,
          P: MemoryProperty
{
    pub fn with_allocator(
        device: D,
        allocator: A,
        memory_properties: P,
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

            device.handle.allocate_memory(&*info, None)?
        };

        let memory = Self {
            _instance: PhantomData,
            device,
            handle,
            type_index,
            size: allocator.size(),
            allocator: Mutex::new(allocator),
            access: Mutex::new(MemoryAccess { count: 0, pointer: 0 }),
            _properties: PhantomData,
        };

        Ok(memory)
    }

    fn compatible_memory_type_indices<MP>(device: &D, _memory_property: MP) -> Vec<u32>
        where MP: MemoryProperty
    {
        let flags = MP::memory_property();
        device
            .instance.physical_devices[device.physical_device_index]
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
}

impl<I, D, A, P> DeviceMemoryAbs for DeviceMemory<I, D, A, P>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          A: Allocator,
          P: MemoryProperty
{
    type Instance = I;
    type Device = D;
    type Allocator = A;
    type MemoryProperty = P;

    #[inline]
    fn instance(&self) -> &Instance { &self.device.instance }
    #[inline]
    fn device(&self) -> &Device<Self::Instance> { &self.device }
    #[inline]
    fn handle(&self) -> vk::DeviceMemory { self.handle }
    #[inline]
    fn size(&self) -> u64 { self.size }
}

impl<I, D, A, P> Drop for DeviceMemory<I, D, A, P>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          A: Allocator,
          P: MemoryProperty
{
    fn drop(&mut self) {
        unsafe { self.device.handle.free_memory(self.handle, None) }
        // Ensure that there is no cpu access to this memory.
        debug_assert!(
            self.access.lock().unwrap().count == 0,
            "There is/are {} cpu access to this vk::DeviceMemory!",
            self.access.lock().unwrap().count,
        );
    }
}

impl<I, D, A, P> Destroy for DeviceMemory<I, D, A, P>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          A: Allocator,
          P: MemoryProperty
{
    type Ok = ();
    type Error = Infallible;

    unsafe fn destroy(self) -> Result<Self::Ok, Self::Error> {
        unsafe { self.device.handle.free_memory(self.handle, None) }

        // Ensure that there is no cpu access to this memory.
        debug_assert!(
            self.access.lock().unwrap().count == 0,
            "There is/are {} cpu access to this vk::DeviceMemory!",
            self.access.lock().unwrap().count,
        );

        Ok(())
    }
}

impl From<vk::Result> for MemoryErr {
    fn from(v: vk::Result) -> Self { MemoryErr::Vk(v) }
}

