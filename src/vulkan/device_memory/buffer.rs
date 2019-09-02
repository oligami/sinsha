pub use usage::BufferUsage;

use super::*;
use std::ops;

pub struct Buffer<I, D, M, BA, DA> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    BA: Allocator,
{
    _marker: PhantomData<(I, D)>,
    memory: M,
    handle: vk::Buffer,
    ident: BA::Identifier,
    offset: u64,
    size: u64,
    align: usize,
    allocator: DA,
}

pub struct Data<I, D, M, B, BA, DA, T> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    B: Borrow<Buffer<I, D, M, BA, DA>>,
    BA: Allocator,
    DA: Allocator,
    T: ?Sized,
{
    _marker: PhantomData<(I, D, M, BA, fn() -> T)>,
    buffer: B,
    ident: DA::Identifier,
    offset: u64,
    size: u64,
}

#[derive(Debug)]
pub enum BufferErr {
    Vk(vk::Result),
    IncompatibleMemoryTypeIndex,
    Allocator(alloc::AllocErr),
}

#[derive(Debug)]
pub enum DataErr {
    Allocator(alloc::AllocErr),
}

impl<I, D, M, BA, DA> Buffer<I, D, M, BA, DA> where
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    BA: Allocator,
    DA: Allocator,
{
    pub fn new(
        memory: M,
        queue_families: &[u32],
        allocator: DA,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self, BufferErr> {
        let device = &memory.borrow().device.borrow().handle;
        let sharing_mode = if queue_families.len() == 1 {
            vk::SharingMode::EXCLUSIVE
        } else {
            vk::SharingMode::CONCURRENT
        };
        let handle = {
            let info = vk::BufferCreateInfo {
                s_type: StructureType::BUFFER_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::BufferCreateFlags::empty(),
                size: allocator.size(),
                sharing_mode,
                usage,
                queue_family_index_count: queue_families.len() as u32,
                p_queue_family_indices: queue_families.as_ptr(),
            };

            unsafe { device.create_buffer(&info, None)? }
        };

        let memory_requirements = unsafe { device.get_buffer_memory_requirements(handle) };

        if 1 << memory.borrow().type_index & memory_requirements.memory_type_bits == 0 {
            return Err(BufferErr::IncompatibleMemoryTypeIndex);
        }

        let layout = Layout::from_size_align(
            memory_requirements.size as usize,
            memory_requirements.alignment as usize,
        ).unwrap();

        let (offset, ident) = match memory.borrow().allocator.alloc(layout) {
            Ok(ok) => ok,
            Err(e) => {
                unsafe { device.destroy_buffer(handle, None) };
                return Err(BufferErr::Allocator(e));
            }
        };
        unsafe { device.bind_buffer_memory(handle, memory.borrow().handle, offset)? };

        let buffer = Self {
            _marker: PhantomData,
            memory,
            handle,
            ident,
            offset,
            size: memory_requirements.size,
            align: memory_requirements.alignment as usize,
            allocator,
        };

        Ok(buffer)
    }

    #[inline]
    pub fn device_memory(&self) -> &DeviceMemory<I, D, BA> { &self.memory.borrow() }
    #[inline]
    pub fn handle(&self) -> vk::Buffer { self.handle }
    #[inline]
    pub fn offset(&self) -> u64 { self.offset }
    #[inline]
    pub fn size(&self) -> u64 { self.size }
}

impl<I, D, M, BA, DA> Drop for Buffer<I, D, M, BA, DA> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    BA: Allocator,
{
    #[inline]
    fn drop(&mut self) {
        unsafe { self.memory.borrow().device.borrow().handle.destroy_buffer(self.handle, None); }
        self.memory.borrow().allocator.dealloc(&self.ident);
    }
}

impl From<vk::Result> for BufferErr {
    fn from(v: vk::Result) -> Self { BufferErr::Vk(v) }
}
impl From<alloc::AllocErr> for BufferErr {
    fn from(a: alloc::AllocErr) -> Self { BufferErr::Allocator(a) }
}


impl<I, D, M, B, BA, DA> Data<I, D, M, B, BA, DA, ()> where
    I: Borrow<Instance>,
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    B: Borrow<Buffer<I, D, M, BA, DA>>,
    BA: Allocator,
    DA: Allocator,
{
    pub fn new<T>(buffer: B) -> Result<Data<I, D, M, B, BA, DA, T>, DataErr> where T: Sized {
        let align_of_t = mem::align_of::<T>();
        let align = if align_of_t < buffer.borrow().align { buffer.borrow().align } else { align_of_t };
        let size = mem::size_of::<T>();
        let layout = Layout::from_size_align(size, align).unwrap();

        let (offset, ident) = buffer.borrow().allocator.alloc(layout)?;

        Ok(Data { _marker: PhantomData, buffer, ident, offset, size: size as u64 })
    }

    pub fn new_slice<T>(buffer: B, len: usize) -> Result<Data<I, D, M, B, BA, DA, [T]>, DataErr>
        where T: Sized
    {
        let align_of_t = mem::align_of::<T>();
        let align = if align_of_t < buffer.borrow().align { buffer.borrow().align } else { align_of_t };
        let size = mem::size_of::<T>() * len;
        let layout = Layout::from_size_align(size, align).unwrap();

        let (offset, identifier) = buffer.borrow().allocator.alloc(layout)?;

        Ok(Data { _marker: PhantomData, buffer, ident: identifier, offset, size: size as u64 })
    }
}

impl<I, D, M, B, BA, DA, T> Data<I, D, M, B, BA, DA, T> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    B: Borrow<Buffer<I, D, M, BA, DA>>,
    BA: Allocator,
    DA: Allocator,
    T: ?Sized,
{
    #[inline]
    pub fn buffer(&self) -> &Buffer<I, D, M, BA, DA> { &self.buffer.borrow() }
    #[inline]
    pub fn offset_by_buffer(&self) -> u64 { self.offset }
    #[inline]
    pub fn offset_by_memory(&self) -> u64 { self.buffer.borrow().offset + self.offset }
    #[inline]
    pub fn size(&self) -> u64 { self.size }
}

impl<I, D, M, B, BA, DA, T> Data<I, D, M, B, BA, DA, [T]> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    B: Borrow<Buffer<I, D, M, BA, DA>>,
    BA: Allocator,
    DA: Allocator,
    T: Sized,
{
    #[inline]
    pub fn len(&self) -> u64 {
        self.size() / mem::size_of::<T>() as u64
    }
}

impl<I, D, M, B, BA, DA, T> Drop for Data<I, D, M, B, BA, DA, T> where
    D: Borrow<Device<I>>,
    M: Borrow<DeviceMemory<I, D, BA>>,
    B: Borrow<Buffer<I, D, M, BA, DA>>,
    BA: Allocator,
    DA: Allocator,
    T: ?Sized,
{
    #[inline]
    fn drop(&mut self) { self.buffer.borrow().allocator.dealloc(&self.ident); }
}

impl From<alloc::AllocErr> for DataErr {
    fn from(a: alloc::AllocErr) -> Self { DataErr::Allocator(a) }
}

mod usage {
    use ash::vk;
    pub struct BufferUsage {
        flags: vk::BufferUsageFlags,
    }

    impl BufferUsage {
        pub fn vk_flags(&self) -> vk::BufferUsageFlags { self.flags }
        pub fn empty() -> Self { BufferUsage { flags: vk::BufferUsageFlags::empty() } }
        pub fn transfer_src(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::TRANSFER_SRC; self
        }
        pub fn transfer_dst(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::TRANSFER_DST; self
        }
        pub fn uniform_texel_buffer(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER; self
        }
        pub fn storage_texel_buffer(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::STORAGE_TEXEL_BUFFER; self
        }
        pub fn uniform_buffer(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::UNIFORM_BUFFER; self
        }
        pub fn storage_buffer(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::STORAGE_BUFFER; self
        }
        pub fn index_buffer(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::INDEX_BUFFER; self
        }
        pub fn vertex_buffer(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::VERTEX_BUFFER; self
        }
        pub fn indirect_buffer(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::INDIRECT_BUFFER; self
        }
        pub fn transform_feedback_buffer_ext(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::TRANSFORM_FEEDBACK_BUFFER_EXT; self
        }
        pub fn transform_feedback_counter_buffer_ext(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::TRANSFORM_FEEDBACK_COUNTER_BUFFER_EXT; self
        }
        pub fn conditional_rendering_ext(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::CONDITIONAL_RENDERING_EXT; self
        }
        pub fn ray_tracing_nv(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::RAY_TRACING_NV; self
        }
        pub fn shader_device_address_ext(&mut self) -> &mut Self {
            self.flags |= vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS_EXT; self
        }
    }
}