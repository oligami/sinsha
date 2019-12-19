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
    BufferCreationErr(vk::Result),
    BufferBindingErr(vk::Result),
    InnerAllocatorErr(AllocErr),
    IncompatibleMemoryType { memory_type_bits: u32, required_type_bits: u32 },
}
pub enum ImageAllocErr {}
pub enum DataAllocErr {
    InnerAllocatorErr(AllocErr),
}

pub trait DeviceMemory {
    // Required.
    fn vulkan(&self) -> &Vulkan;
    fn handle(&self) -> vk::DeviceMemory;
    fn capacity(&self) -> u64;
    fn properties(&self) -> vk::MemoryPropertyFlags;
    fn type_bits(&self) -> u32;

//    fn alloc_buffer<B: Buffer>(this: B::BorrowOfDeviceMemory,
//                               info: &vk::BufferCreateInfo,
//                               allocator: B::Allocator,
//                               custom: B::CustomCreationArgument) -> Result<B, BufAllocErr> {
//        debug_assert_eq!(info.size, allocator.capacity(),
//                         "Size of vk::Buffer differs from capacity of Allocator.");
//
//        let ref_this = this.borrow();
//        let ref_vulkan = ref_this.vulkan();
//
//        let buffer_handle = unsafe {
//            ref_vulkan.device
//                .create_buffer(info, None)
//                .map_err(|e| BufAllocErr::BufferCreationErr(e))?
//        };
//
//        let requirements = unsafe {
//            ref_vulkan.device.get_buffer_memory_requirements(buffer_handle)
//        };
//
//        let type_compatible = requirements.memory_type_bits & ref_this.type_bits() != 0;
//        if !type_compatible {
//            let err = BufAllocErr::IncompatibleMemoryType {
//                memory_type_bits: ref_this.type_bits(),
//                required_type_bits: requirements.memory_type_bits,
//            };
//            return Err(err);
//        }
//
//        let layout = Layout::from_size_align(info.size as usize,
//                                             requirements.alignment as usize).unwrap();
//
//        let offset = unsafe {
//            ref_this
//                .alloc(layout)
//                .map_err(|e| BufAllocErr::InnerAllocatorErr(e))?
//        };
//
//        unsafe {
//            ref_vulkan.device
//                .bind_buffer_memory(buffer_handle, ref_this.handle(), offset)
//                .map_err(|e| BufAllocErr::BufferBindingErr(e))?
//        };
//
//        let buffer = unsafe {
//            B::from_raw_parts(this, buffer_handle, info, offset, layout, allocator, custom)
//        };
//
//        Ok(buffer)
//    }
}

pub trait BufferAllocator {
    type Allocator: Allocator;
    type DeviceMemory: DeviceMemory;

    // Required.
    fn allocator(&self) -> &Self::Allocator;
    fn device_memory(&self) -> &Self::DeviceMemory;

    // Provided.
    fn allocate_buffer(&self, unbound_buffer: ) -> vk::Buffer {

    }
}

// TODO: mip level と array layer とかも必要？他には？
pub trait Image {
    fn handle(&self) -> vk::Image;
    // TODO: 返り値は Option の方がいいかも。あと、複数返せないと Depth と Stencil がうまく使えない。
    fn view(&self) -> vk::ImageView;
    fn extent(&self) -> vk::Extent3D;
}

pub trait Buffer {
    fn vulkan(&self) -> &Vulkan;
    fn memory(&self) -> vk::DeviceMemory;
    fn handle(&self) -> vk::Buffer;
    fn usage(&self) -> vk::BufferUsageFlags;
    fn offset(&self) -> u64;
    fn layout(&self) -> Layout;

//    // Provided.
//    /// T must be Copy because T must not have data in heap.
//    fn alloc_data<T: Sized + Copy, D: Data<T>>(this: D::BorrowOfBuffer,
//                                               custom: D::CustomCreationArgument,
//    ) -> Result<D, DataAllocErr> {
//        let layout = Layout::new::<T>();
//        let ref_buffer = this.borrow();
//        let offset = unsafe {
//            ref_buffer
//                .alloc(layout)
//                .map_err(|e| { DataAllocErr::InnerAllocatorErr(e)})?
//        };
//
//        let data =  unsafe {
//            D::from_raw_parts(this, offset, layout.size() as u64, 1, custom)
//        };
//
//        Ok(data)
//    }
//
//    /// T must be Copy because T must not have data in heap.
//    fn alloc_array<T: Sized + Copy, D: Data<[T]>>(this: D::BorrowOfBuffer,
//                                                  custom: D::CustomCreationArgument,
//    ) -> Result<D, DataAllocErr> {
//        unimplemented!()
//    }
}

// D: Data<T> とか impl Data<T> などの楽な記述ができるよう、trait にした。
pub trait Data<T> where T: ?Sized {
    type ParentBuffer: Buffer;
    type BorrowOfBuffer: Borrow<Self::ParentBuffer>;
    type CustomCreationArgument;

    unsafe fn from_raw_parts(buffer: Self::BorrowOfBuffer,
                             offset: u64,
                             size: u64,
                             len: u64,
                             custom: Self::CustomCreationArgument) -> Self;
    fn buffer(&self) -> &Self::ParentBuffer;
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
    memory_properties: vk::MemoryPropertyFlags,
}

pub struct DevMemBuilder {
    memory_type_bits: u32,
    memory_properties: vk::MemoryPropertyFlags,
}

pub struct Imag;

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

pub struct Dat<V, M, AedM, B, AedB, T> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, AedM>>,
    AedM: Allocator,
    B: Borrow<Buf<V, M, AedM, AedB>>,
    AedB: Allocator,
    T: ?Sized,
{
    vulkan: PhantomData<V>,
    memory: PhantomData<M>,
    buffer: B,
    allocated: PhantomData<(AedM, AedB)>,
    offset: u64,
    size: u64,
    len: u64,
    ty: PhantomData<T>,
}

impl<V: Borrow<Vulkan>, A> DevMem<V, A> {
    pub fn builder() -> DevMemBuilder {
        DevMemBuilder {
            memory_type_bits: !0,
            memory_properties: vk::MemoryPropertyFlags::empty(),
        }
    }
}

unsafe impl<V, A> Allocator for DevMem<V, A> where V: Borrow<Vulkan>, A: Allocator {
    fn capacity(&self) -> u64 { self.allocator.capacity() }
    unsafe fn alloc(&self, layout: Layout) -> Result<u64, AllocErr> { self.allocator.alloc(layout) }
    unsafe fn dealloc(&self, offset: u64, layout: Layout) { self.allocator.dealloc(offset, layout) }
}

impl<V, A> DeviceMemory for DevMem<V, A> where V: Borrow<Vulkan>, A: Allocator {
    fn vulkan(&self) -> &Vulkan { self.vulkan.borrow() }
    fn handle(&self) -> vk::DeviceMemory { self.handle }
    fn properties(&self) -> vk::MemoryPropertyFlags { self.memory_properties }
    fn type_bits(&self) -> u32 { self.memory_type_bits }
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
        let memory_properties = memory_properties
            .memory_types[memory_type_index as usize]
            .property_flags;

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

unsafe impl<V, M, Aed, Aor> Allocator for Buf<V, M, Aed, Aor> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, Aed>>,
    Aed: Allocator,
    Aor: Allocator,
{
    fn capacity(&self) -> u64 { self.allocator.capacity() }
    unsafe fn alloc(&self, layout: Layout) -> Result<u64, AllocErr> { self.allocator.alloc(layout) }
    unsafe fn dealloc(&self, offset: u64, layout: Layout) { self.allocator.dealloc(offset, layout) }
}

impl<V, M, Aed, Aor> Buffer for Buf<V, M, Aed, Aor> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, Aed>>,
    Aed: Allocator,
    Aor: Allocator,
{
    type DeviceMemory = DevMem<V, Aed>;
    type BorrowOfDeviceMemory = M;
    type Allocator = Aor;
    type CustomCreationArgument = ();

    unsafe fn from_raw_parts(memory: Self::BorrowOfDeviceMemory,
                             handle: vk::Buffer,
                             info: &vk::BufferCreateInfo,
                             offset: u64,
                             layout: Layout,
                             allocator: Self::Allocator,
                             custom: Self::CustomCreationArgument,
    ) -> Self {
        Self {
            vulkan: PhantomData,
            memory,
            allocated: PhantomData,
            handle,
            offset,
            layout,
            usage: info.usage,
            capacity: allocator.capacity(),
            allocator,
        }
    }
    fn memory(&self) -> &Self::DeviceMemory { &self.memory.borrow() }
    fn handle(&self) -> vk::Buffer { self.handle }
    fn usage(&self) -> vk::BufferUsageFlags { self.usage }
    fn offset(&self) -> u64 { self.offset }
    fn layout(&self) -> Layout { self.layout }
}

impl<V, M, Aed, Aor> Drop for Buf<V, M, Aed, Aor> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, Aed>>,
    Aed: Allocator,
{
    fn drop(&mut self) {
        unsafe {
            // De-allocation must be done after destroyed vk::Buffer
            // to ensure not to occur aliasing.
            self.memory.borrow().vulkan.borrow().device.destroy_buffer(self.handle, None);
            self.memory.borrow().allocator.dealloc(self.offset, self.layout);
        }
    }
}

impl<V, M, AedM, B, AedB, T> Data<T> for Dat<V, M, AedM, B, AedB, T> where
    V: Borrow<Vulkan>,
    M: Borrow<DevMem<V, AedM>>,
    AedM: Allocator,
    B: Borrow<Buf<V, M, AedM, AedB>>,
    AedB: Allocator,
    T: ?Sized,
{
    type ParentBuffer = Buf<V, M, AedM, AedB>;
    type BorrowOfBuffer = B;
    type CustomCreationArgument = ();

    unsafe fn from_raw_parts(buffer: Self::BorrowOfBuffer,
                             offset: u64,
                             size: u64,
                             len: u64,
                             custom: Self::CustomCreationArgument) -> Self
    {
        Self {
            vulkan: PhantomData,
            memory: PhantomData,
            buffer,
            allocated: PhantomData,
            offset,
            size,
            len,
            ty: PhantomData,
        }
    }
    fn buffer(&self) -> &Self::ParentBuffer { self.buffer.borrow() }
    fn offset(&self) -> u64 { self.offset }
    fn size(&self) -> u64 { self.size }
    fn len(&self) -> u64 { self.len }
}

