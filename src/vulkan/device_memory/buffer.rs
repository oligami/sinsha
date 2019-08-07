pub mod usage;
use usage::BufferUsage;

use super::*;
use memory_property::*;


use std::ops;

pub struct Buffer<I, D, M, BA, P, DA, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage
{
    _marker: PhantomData<(I, D, P)>,
    memory: M,
    handle: vk::Buffer,
    identifier: BA::Identifier,
    range: ops::Range<u64>,
    allocator: Mutex<DA>,
    _usage: PhantomData<U>,
}

pub trait BufferAbs {
    type Instance: Borrow<Instance> + Deref<Target = Instance>;
    type Device: Borrow<Device<Self::Instance>> + Deref<Target = Device<Self::Instance>>;
    type Memory: Borrow<DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::BufferAllocator,
        Self::MemoryProperty
    >> + Deref<Target = DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::BufferAllocator,
        Self::MemoryProperty
    >>;
    type BufferAllocator: Allocator;
    type DataAllocator: Allocator;
    type MemoryProperty: MemoryProperty;
    type Usage: BufferUsage;

    fn instance(&self) -> &Instance;
    fn device(&self) -> &Device<Self::Instance>;
    fn memory(&self) -> &DeviceMemory<Self::Instance, Self::Device, Self::BufferAllocator, Self::MemoryProperty>;
    fn handle(&self) -> vk::Buffer;
    fn offset(&self) -> u64;
    fn size(&self) -> u64;
}

pub struct Data<I, D, M, B, BA, P, DA, U, T>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          B: Borrow<Buffer<I, D, M, BA, P, DA, U>> + Deref<Target = Buffer<I, D, M, BA, P, DA, U>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage,
          T: ?Sized,
{
    _marker: PhantomData<(I, D, M, BA, U, P)>,
    buffer: B,
    identifier: DA::Identifier,
    range: ops::Range<u64>,
    _type: PhantomData<fn() -> T>,
}

pub trait DataAbs {
    type Instance: Borrow<Instance> + Deref<Target = Instance>;
    type Device: Borrow<Device<Self::Instance>> + Deref<Target = Device<Self::Instance>>;
    type Memory: Borrow<DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::BufferAllocator,
        Self::MemoryProperty
    >> + Deref<Target = DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::BufferAllocator,
        Self::MemoryProperty
    >>;
    type Buffer: Borrow<Buffer<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::BufferAllocator,
        Self::MemoryProperty,
        Self::DataAllocator,
        Self::Usage,
    >> + Deref<Target = Buffer<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::BufferAllocator,
        Self::MemoryProperty,
        Self::DataAllocator,
        Self::Usage,
    >>;
    type BufferAllocator: Allocator;
    type DataAllocator: Allocator;
    type MemoryProperty: MemoryProperty;
    type Usage: BufferUsage;
    type Type: ?Sized;

    fn instance(&self) -> &Instance;
    fn device(&self) -> &Device<Self::Instance>;
    fn memory(&self) -> &DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::BufferAllocator,
        Self::MemoryProperty
    >;
    fn buffer(&self) -> &Buffer<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::BufferAllocator,
        Self::MemoryProperty,
        Self::DataAllocator,
        Self::Usage,
    >;
    fn handle(&self) -> vk::Buffer;
    fn offset(&self) -> u64;
    fn size(&self) -> u64;
}

pub struct DataMapper<'data, D, T> where D: DataAbs<Type = T>, D::MemoryProperty: HostVisible, T: ?Sized {
    data: &'data D,
    pointer: usize,
    _marker: PhantomData<T>,
    // make this struct Not Send Nor Sync.
    _ptr: PhantomData<*const ()>,
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

impl<I, D, M, BA, P, DA, U> Buffer<I, D, M, BA, P, DA, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage
{
    pub unsafe fn new(
        memory: M,
        queue_families: &[u32],
        allocator: DA,
        _usage: U,
    ) -> Result<Self, BufferErr> {
        let device = &memory.device.handle;
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
                usage: U::buffer_usage(),
                queue_family_index_count: queue_families.len() as u32,
                p_queue_family_indices: queue_families.as_ptr(),
            };

            device.create_buffer(&info, None)?
        };

        let memory_requirements = device.get_buffer_memory_requirements(handle);

        if 1 << memory.type_index & memory_requirements.memory_type_bits == 0 {
            return Err(BufferErr::IncompatibleMemoryTypeIndex);
        }

        let layout = Layout::from_size_align(
            memory_requirements.size as usize,
            memory_requirements.alignment as usize,
        ).unwrap();

        let (range, identifier) = memory.allocator.lock().unwrap().alloc(layout)?;
        device.bind_buffer_memory(handle, memory.handle, range.start)?;

        let buffer = Self {
            _marker: PhantomData,
            memory,
            handle,
            identifier,
            range,
            allocator: Mutex::new(allocator),
            _usage: PhantomData,
        };

        Ok(buffer)
    }
}

impl<I, D, M, BA, P, DA, U> BufferAbs for Buffer<I, D, M, BA, P, DA, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage
{
    type Instance = I;
    type Device = D;
    type Memory = M;
    type BufferAllocator = BA;
    type DataAllocator = DA;
    type MemoryProperty = P;
    type Usage = U;

    #[inline]
    fn instance(&self) -> &Instance { &self.memory.device.instance }
    #[inline]
    fn device(&self) -> &Device<Self::Instance> { &self.memory.device }
    #[inline]
    fn memory(&self) -> &DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::BufferAllocator,
        Self::MemoryProperty
    > { &self.memory }
    #[inline]
    fn handle(&self) -> vk::Buffer { self.handle }
    #[inline]
    fn offset(&self) -> u64 { self.range.start }
    #[inline]
    fn size(&self) -> u64 { self.range.end - self.range.start }
}

impl<I, D, M, BA, P, DA, U> Destroy for Buffer<I, D, M, BA, P, DA, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage
{
    type Ok = ();
    type Error = Infallible;

    unsafe fn destroy(self) -> Result<Self::Ok, Self::Error>{
        unsafe { self.device().handle.destroy_buffer(self.handle, None); }
        self.memory().allocator.lock().unwrap().dealloc(&self.identifier);
        Ok(())
    }
}

impl<I, D, M, BA, P, DA, U> Drop for Buffer<I, D, M, BA, P, DA, U>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage
{
    fn drop(&mut self) {
        unsafe { self.memory.device.handle.destroy_buffer(self.handle, None); }
        self.memory().allocator.lock().unwrap().dealloc(&self.identifier);
    }
}

impl From<vk::Result> for BufferErr {
    fn from(v: vk::Result) -> Self { BufferErr::Vk(v) }
}
impl From<alloc::AllocErr> for BufferErr {
    fn from(a: alloc::AllocErr) -> Self { BufferErr::Allocator(a) }
}


impl<I, D, M, B, BA, P, DA, U, T> Data<I, D, M, B, BA, P, DA, U, T>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          B: Borrow<Buffer<I, D, M, BA, P, DA, U>> + Deref<Target = Buffer<I, D, M, BA, P, DA, U>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage,
          T: Sized,
{
    pub unsafe fn new(
        buffer: B,
        _type: &T,
    ) -> Result<Self, DataErr> {
        let layout = Layout::new::<T>();
        let (range, identifier) = buffer.allocator.lock().unwrap().alloc(layout)?;

        Ok(Self { _marker: PhantomData, buffer, identifier, range, _type: PhantomData })
    }
}

impl<I, D, M, B, BA, P, DA, U, T> Data<I, D, M, B, BA, P, DA, U, T>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          B: Borrow<Buffer<I, D, M, BA, P, DA, U>> + Deref<Target = Buffer<I, D, M, BA, P, DA, U>>,
          BA: Allocator,
          P: HostVisible,
          DA: Allocator,
          U: BufferUsage,
          T: ?Sized,
{
    // TODO: Return type must be Result.
    pub fn mapper(&self) -> DataMapper<Self, T> {
        let mut access = self.memory().access.lock().unwrap();
        let memory_pointer = if access.count > 0 {
            access.count += 1;
            access.pointer
        } else {
            let new_memory_pointer = unsafe {
                let memory = self.memory();
                self.device().handle
                    .map_memory(memory.handle, 0, memory.size, vk::MemoryMapFlags::empty())
                    .unwrap() as usize

            };

            access.count = 1;
            access.pointer = new_memory_pointer;

            new_memory_pointer
        };

        DataMapper {
            data: &self,
            pointer: memory_pointer + self.offset() as usize,
            _marker: PhantomData,
            _ptr: PhantomData,
        }
    }
}

impl<I, D, M, B, BA, P, DA, U, T> DataAbs for Data<I, D, M, B, BA, P, DA, U, T>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          B: Borrow<Buffer<I, D, M, BA, P, DA, U>> + Deref<Target = Buffer<I, D, M, BA, P, DA, U>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage,
          T: ?Sized
{
    type Instance = I;
    type Device = D;
    type Memory = M;
    type Buffer = B;
    type BufferAllocator = BA;
    type DataAllocator = DA;
    type MemoryProperty = P;
    type Usage = U;
    type Type = T;

    #[inline]
    fn instance(&self) -> &Instance { &self.buffer.memory.device.instance }
    #[inline]
    fn device(&self) -> &Device<Self::Instance> { &self.buffer.memory.device }
    #[inline]
    fn memory(&self) -> &DeviceMemory<
        Self::Instance,
        Self::Device,
        Self::BufferAllocator,
        Self::MemoryProperty
    > { &self.buffer.memory }
    #[inline]
    fn buffer(&self) -> &Buffer<
        Self::Instance,
        Self::Device,
        Self::Memory,
        Self::BufferAllocator,
        Self::MemoryProperty,
        Self::DataAllocator,
        Self::Usage,
    > { &self.buffer }
    #[inline]
    fn handle(&self) -> vk::Buffer { self.buffer.handle }
    #[inline]
    fn offset(&self) -> u64 { self.range.start }
    #[inline]
    fn size(&self) -> u64 { self.range.end - self.range.start }
}

impl<I, D, M, B, BA, P, DA, U, T> Destroy for Data<I, D, M, B, BA, P, DA, U, T>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          B: Borrow<Buffer<I, D, M, BA, P, DA, U>> + Deref<Target = Buffer<I, D, M, BA, P, DA, U>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage,
          T: ?Sized,
{
    type Ok = ();
    type Error = Infallible;

    /// This is not Vulkan API object.
    /// Destroying this will tell parent buffer to free memory domain of this.
    unsafe fn destroy(self) -> Result<Self::Ok, Self::Error> {
        self.buffer.allocator.lock().unwrap().dealloc(&self.identifier);
        Ok(())
    }
}

impl<I, D, M, B, BA, P, DA, U, T> Drop for Data<I, D, M, B, BA, P, DA, U, T>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          M: Borrow<DeviceMemory<I, D, BA, P>> + Deref<Target = DeviceMemory<I, D, BA, P>>,
          B: Borrow<Buffer<I, D, M, BA, P, DA, U>> + Deref<Target = Buffer<I, D, M, BA, P, DA, U>>,
          BA: Allocator,
          P: MemoryProperty,
          DA: Allocator,
          U: BufferUsage,
          T: ?Sized,
{
    fn drop(&mut self) {
        self.buffer().allocator.lock().unwrap().dealloc(&self.identifier);
    }
}

impl From<alloc::AllocErr> for DataErr {
    fn from(a: alloc::AllocErr) -> Self { DataErr::Allocator(a) }
}

impl<D, T> DataMapper<'_, D, T>
    where D: DataAbs<Type = T>,
          D::MemoryProperty: HostVisible,
          T: Sized,
{
    pub unsafe fn read(&self) -> T { ptr::read(self.pointer as *const T) }
    pub unsafe fn write(&self, data: T) { ptr::write(self.pointer as _, data) }
}

impl<D, T> DataMapper<'_, D, [T]>
    where D: DataAbs<Type = [T]>,
          D::MemoryProperty: HostVisible,
          T: Sized,
{
    pub unsafe fn read(&self) -> &[T] { unimplemented!() }
    pub unsafe fn write(&self, data: &[T]) { unimplemented!() }
}

impl<D, T> Drop for DataMapper<'_, D, T> where D: DataAbs<Type = T>, D::MemoryProperty: HostVisible, T: ?Sized {
    fn drop(&mut self) {
        let mut access = self.data.memory().access.lock().unwrap();
        access.count -= 1;

        if access.count == 0 {
            unsafe {
                self.data.device().handle
                    .unmap_memory(self.data.memory().handle);
            }
        }
    }
}

