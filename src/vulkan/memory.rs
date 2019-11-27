mod allocator;

use super::*;
use allocator::{ Allocator, AllocErr };

use std::borrow::Borrow;
use std::ops::BitAnd;
use std::ops::Range;
use std::alloc::Layout;
use std::marker::PhantomData;

pub enum MemAllocErr {
    NoValidMemoryType,
}
pub enum BufAllocErr {
    VkErr(vk::Result),
    InnerAllocatorErr(AllocErr),
    IncompatibleMemoryType,
}
pub enum ImageAllocErr {}
pub enum DataAllocErr {}

pub trait DeviceMemory {
    type Allocator: Allocator;
    fn handle(&self) -> vk::DeviceMemory;
    fn capacity(&self) -> u64;
    fn alloc_buffer<B: Buffer, A: Allocator>(
        &self, info: &vk::BufferCreateInfo, flags: <Self::Allocator as Allocator>::Flags, allocator: A,
    ) -> Result<B, BufAllocErr>;
    fn alloc_image<I: Image>(&self, info: ()) -> Result<I, ImageAllocErr>;
}

pub trait Buffer {
    // これいらないかも
    fn handle(&self) -> vk::Buffer;
    fn offset(&self) -> u64;
    fn capacity(&self) -> u64;
    fn alloc_data<T, D: Data<T>>(&self, info: ()) -> Result<D, DataAllocErr>;
}

// D: Data<T> とか impl Data<T> などの楽な記述ができるよう、trait にした。
pub trait Data<T> where T: ?Sized {
    fn buffer_handle(&self) -> vk::Buffer;
    fn offset(&self) -> u64;
    fn size(&self) -> u64;
    fn len(&self) -> u64;
}

pub struct DevMem<V: Borrow<Vulkan>, A> {
    vulkan: V,
    allocator: A,
    handle: vk::DeviceMemory,
    capacity: u64,
    memory_type_bits: u32,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
}

pub struct DevMemBuilder {
    memory_type_bits: u32,
    memory_properties: vk::MemoryPropertyFlags,
}

pub struct Buf<V, M, Aed, Aor> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, Aed>>,
    Aed: Allocator,
{
    vulkan: PhantomData<V>,
    memory: M,
    allocated: PhantomData<Aed>,
    handle: vk::Buffer,
    offset: u64,
    capacity: u64,
    layout: Layout,
    usage: vk::BufferUsageFlags,
    allocator: Aor,
}

// TODO: mip level と array layer とかも必要？他には？
pub trait Image {
    fn handle(&self) -> vk::Image;
    // TODO: 返り値は Option の方がいいかも。あと、複数返せないと Depth と Stencil がうまく使えない。
    fn view(&self) -> vk::ImageView;
    fn extent(&self) -> vk::Extent3D;
}

impl<V: Borrow<Vulkan>, A> DevMem<V, A> {
    pub fn builder() -> DevMemBuilder {
        DevMemBuilder {
            memory_type_bits: !0,
            memory_properties: vk::MemoryPropertyFlags::empty(),
        }
    }
}

impl From<vk::Result> for BufAllocErr {
    fn from(err: vk::Result) -> Self { BufAllocErr::VkErr(err) }
}

impl From<AllocErr> for BufAllocErr {
    fn from(err: AllocErr) -> Self { BufAllocErr::InnerAllocatorErr(err) }
}

impl<V: Borrow<Vulkan>, Aed: Allocator> DeviceMemory for DevMem<V, Aed> {
    type Allocator = Aed;
    fn handle(&self) -> vk::DeviceMemory { self.handle }
    fn capacity(&self) -> u64 { self.allocator.capacity() }

    fn alloc_buffer<B: Buffer, Aor: Allocator>(
        &self, info: &vk::BufferCreateInfo, flags: Aed::Flags, allocator: Aor,
    ) -> Result<B, BufAllocErr> {
        let ref_vulkan = self.vulkan.borrow();

        let raw_buffer = unsafe { ref_vulkan.device.create_buffer(&info, None)? };
        let requirements = unsafe { ref_vulkan.device.get_buffer_memory_requirements(raw_buffer) };

        let memory_type_compatible = requirements.memory_type_bits & self.memory_type_bits != 0;
        if !memory_type_compatible { return Err(BufAllocErr::IncompatibleMemoryType); }

        let layout = Layout::from_size_align(info.size as usize,
                                             requirements.alignment as usize).unwrap();
        let offset = unsafe { self.allocator.alloc(layout, flags)? };

        let buffer = Buf {
            vulkan: PhantomData,
            memory: self.clone(),
            allocated: PhantomData,
            handle: raw_buffer,
            offset,
            capacity: allocator.capacity(),
            layout,
            usage: info.usage,
            allocator,
        };

        Ok(buffer)
    }

    fn alloc_image<I: Image>(&self, info: ()) -> Result<I, ImageAllocErr> {
        unimplemented!()
    }
}

impl<V: Borrow<Vulkan>, A> Drop for DevMem<V, A> {
    fn drop(&mut self) {
        unsafe { self.vulkan.borrow().device.free_memory(self.handle, None); }
    }
}

impl DevMemBuilder {
    pub fn memory_type_bits(&mut self, memory_type_bits: u32) -> &mut Self {
        self.memory_type_bits = self.memory_type_bits & memory_type_bits;
        self
    }

    pub fn properties(&mut self, properties: vk::MemoryPropertyFlags) -> &mut Self {
        self.memory_properties = properties;
        self
    }

    pub fn build<V, A>(&self, vulkan: V, allocator: A) -> Result<DevMem<V, A>, MemAllocErr>
        where V: Borrow<Vulkan>, A: Allocator,
    {
        let ref_vulkan = vulkan.borrow();
        // Search compatible memory type index.
        let memory_properties = unsafe {
            ref_vulkan.instance.get_physical_device_memory_properties(ref_vulkan.physical_device)
        };

        let memory_type_index = memory_properties
            .memory_types[0..memory_properties.memory_type_count as usize]
            .iter()
            .enumerate()
            .position(|(i, memory_type)| {
                let properties_satisfied = memory_type
                    .property_flags
                    .contains(self.memory_properties);

                let memory_type_bits_satisfied = (1 << i as u32) & self.memory_type_bits != 0;

                properties_satisfied && memory_type_bits_satisfied
            })
            .ok_or(MemAllocErr::NoValidMemoryType)? as u32;

        // Allocate.
        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(allocator.capacity())
            .memory_type_index(memory_type_index)
            .build();

        let handle = unsafe { ref_vulkan.device.allocate_memory(&info, None).unwrap() };

        Ok(DevMem {
            vulkan,
            capacity: allocator.capacity(),
            allocator,
            handle,
            memory_type_bits: 1 << memory_type_index,
            memory_properties,
        })
    }
}

impl<V, M, Aed, Aor> Buffer for Buf<V, M, Aed, Aor> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, Aed>>,
    Aed: Allocator,
{
    fn handle(&self) -> vk::Buffer { self.handle }
    fn offset(&self) -> u64 { self.offset }
    fn capacity(&self) -> u64 { self.capacity }
    fn alloc_data<T, D: Data<T>>(&self, info: ()) -> Result<D, DataAllocErr> {
        unimplemented!()
    }
}

impl<V, M, Aed, Aor> Drop for Buf<V, M, Aed, Aor> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, Aed>>,
    Aed: Allocator,
{
    fn drop(&mut self) {
        unsafe {
            self.memory.borrow().vulkan.borrow().device.destroy_buffer(self.handle, None);
            self.memory.borrow().allocator.dealloc(self.offset, self.layout);
        }
    }
}

