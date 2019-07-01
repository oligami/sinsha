pub mod usage;
use usage::BufferUsage;

use super::*;

pub struct Buffer<MA, BA, P, U>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage
{
	memory: Arc<DeviceMemory<MA, P>>,
	identifier: MA::Identifier,
	handle: vk::Buffer,
	allocator: Mutex<BA>,
	_usage: PhantomData<U>,
}

pub struct Data<MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage,
		  D: ?Sized
{
	buffer: Arc<Buffer<MA, BA, P, U>>,
	identifier: BA::Identifier,
	range: ops::Range<u64>,
	_type: PhantomData<fn() -> D>,
}

pub struct DataAccess<'vk_data, MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage,
		  D: ?Sized
{
	data: &'vk_data Data<MA, BA, P, U, D>,
	memory_pointer: usize,
	_no_send_or_sync: PhantomData<*mut ()>,
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

impl<MA, BA, P, U> Buffer<MA, BA, P, U>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage
{
	pub fn new<T>(
		memory: Arc<DeviceMemory<MA, P>>,
		queue: Arc<Queue<T>>,
		allocator: BA,
		_usage: U,
	) -> Result<Arc<Self>, BufferErr> where MA: Allocator {
		let device = &memory.device.handle;
		let handle = unsafe {
			let info = vk::BufferCreateInfo {
				s_type: StructureType::BUFFER_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::BufferCreateFlags::empty(),
				size: allocator.size(),
				sharing_mode: vk::SharingMode::EXCLUSIVE,
				usage: U::buffer_usage(),
				queue_family_index_count: 1,
				p_queue_family_indices: &queue.family_index as *const _,
			};

			device.create_buffer(&info, None)?
		};

		let memory_requirements = unsafe {
			device.get_buffer_memory_requirements(handle)
		};

		if 1 << memory.type_index & memory_requirements.memory_type_bits == 0 {
			return Err(BufferErr::IncompatibleMemoryTypeIndex);
		}

		let layout = Layout::from_size_align(
			memory_requirements.size as usize,
			memory_requirements.alignment as usize,
		).unwrap();

		let (range, identifier) = memory.allocator.lock().unwrap().alloc(layout)?;

		unsafe { device.bind_buffer_memory(handle, memory.handle, range.start)? }

		let buffer = Self {
			memory,
			identifier,
			handle,
			allocator: Mutex::new(allocator),
			_usage: PhantomData,
		};

		Ok(Arc::new(buffer))
	}
}

impl<MA, BA, P, U> Destroy for Buffer<MA, BA, P, U>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage
{
	type Ok = ();
	type Error = Infallible;

	unsafe fn destroy(self) -> Result<Self::Ok, Self::Error>{
		unsafe { self.memory.device.handle.destroy_buffer(self.handle, None); }
		self.memory.allocator.lock().unwrap().dealloc(&self.identifier);
		Ok(())
	}
}

impl From<vk::Result> for BufferErr {
	fn from(v: vk::Result) -> Self { BufferErr::Vk(v) }
}
impl From<alloc::AllocErr> for BufferErr {
	fn from(a: alloc::AllocErr) -> Self { BufferErr::Allocator(a) }
}


impl< MA, BA, P, U, D> Data<MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage,
		  D: ?Sized
{
	pub fn new(
		buffer: Arc<Buffer<MA, BA, P, U>>,
		value: &D,
	) -> Result<Self, DataErr> {
		let layout = Layout::for_value(value);
		let (range, identifier) = buffer.allocator.lock().unwrap().alloc(layout)?;

		Ok(Self { buffer, identifier, range, _type: PhantomData })
	}

	pub fn access(&self) -> DataAccess<MA, BA, P, U, D> where P: memory_property::HostVisible {
		let mut access_lock = self.buffer.memory.access.lock().unwrap();
		let memory_pointer = if access_lock.count != 0 {
			access_lock.count += 1;
			access_lock.pointer
		} else {
			let new_memory_pointer = unsafe {
				self.buffer.memory.device.handle
					.map_memory(
						self.buffer.memory.handle,
						0,
						self.buffer.memory.size,
						vk::MemoryMapFlags::empty(),
					)
					.unwrap()
			} as usize;

			access_lock.pointer = new_memory_pointer;
			access_lock.count = 1;

			new_memory_pointer
		} + self.range.start as usize;

		DataAccess { data: self, memory_pointer, _no_send_or_sync: PhantomData }
	}
}


impl<MA, BA, P, U, D> Destroy for Data<MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage,
		  D: ?Sized
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

impl From<alloc::AllocErr> for DataErr {
	fn from(a: alloc::AllocErr) -> Self { DataErr::Allocator(a) }
}

impl<MA, BA, P, U, D> DataAccess<'_, MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage,
		  D: Sized,
{
	/// NOTE: AsRef trait may be better.
	pub fn as_ref(&self) -> &D {
		unsafe { (self.memory_pointer as *const D).as_ref().unwrap() }
	}

	/// NOTE: AsMut trait may be better.
	pub fn as_mut(&mut self) -> &mut D {
		unsafe { (self.memory_pointer as *mut D).as_mut().unwrap() }
	}
}

impl<MA, BA, P, U, D> DataAccess<'_, MA, BA, P, U, [D]>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage,
{
	/// NOTE: AsRef trait may be better.
	pub fn as_ref(&self) -> &[D] {
		unsafe {
			let ptr = self.memory_pointer as *const D;
			slice::from_raw_parts(ptr, self.len())
		}
	}

	/// NOTE: AsMut trait may be better.
	pub fn as_mut(&mut self) -> &mut [D] {
		unsafe {
			let ptr = self.memory_pointer as *mut D;
			slice::from_raw_parts_mut(ptr, self.len())
		}
	}

	fn len(&self) -> usize {
		(self.data.range.end - self.data.range.start) as usize / mem::size_of::<D>()
	}
}

impl<MA, BA, P, U, D> Drop for DataAccess<'_, MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperty,
		  U: BufferUsage,
		  D: ?Sized
{
	fn drop(&mut self) {
		let mut access_lock = self.data.buffer.memory.access.lock().unwrap();
		access_lock.count -= 1;

		// If there is no access to memory, unmap this memory.
		if access_lock.count == 0 {
			unsafe {
				self.data.buffer.memory.device.handle
					.unmap_memory(self.data.buffer.memory.handle);
			}

			if cfg!(debug_assertions) {
				println!("unmap memory!");
			}
		};
	}
}