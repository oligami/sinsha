pub mod alloc;

use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::*;

use self::alloc::Allocator;

use std::io;
use std::fs::File;
use std::ptr;
use std::ops;
use std::fmt;
use std::mem;
use std::slice;
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::error::Error;
use std::path::Path;
use std::sync::{ Arc, Mutex };
use std::alloc::Layout;


pub use self::memory_type::*;
mod memory_type {
	use ash::vk::MemoryPropertyFlags as MP;

	pub trait MemoryProperties {
		fn flags() -> MP;
	}

	pub struct DeviceLocalFlag;
	pub struct HostVisibleFlag;
	pub struct HostCoherentFlag;

	macro_rules! impl_memory_properties {
		($marker_struct:ty, $flag_bits:expr) => {
			impl MemoryProperties for $marker_struct {
				fn flags() -> MP { $flag_bits }
			}
		};
	}

	impl_memory_properties!(DeviceLocalFlag, MP::DEVICE_LOCAL);
	impl_memory_properties!(HostVisibleFlag, MP::HOST_VISIBLE);
	impl_memory_properties!(HostCoherentFlag, MP::HOST_COHERENT);

	impl<T, U> MemoryProperties for (T, U)
		where T: MemoryProperties,
			  U: MemoryProperties
	{
		fn flags() -> MP { T::flags() | U::flags() }
	}
}

pub use self::usage::*;
mod usage {
	use ash::vk::BufferUsageFlags as BU;

	pub trait BufferUsage {
		fn flags() -> BU;
	}

	pub trait ImageUsage {
		fn flags() -> ash::vk::ImageUsageFlags;
	}


	pub struct Source;
	pub struct Destination;
	pub struct Vertex;
	pub struct Index;
	pub struct Uniform;

	pub struct Sampled;
	pub struct ColorAttach;
	pub struct DepthStencilAttach;
	pub struct TransientAttach;
	pub struct InputAttach;

	macro_rules! impl_buffer_usage {
		($flag:ty, $bits:expr) => {
			impl BufferUsage for $flag {
				fn flags() -> BU { $bits }
			}
		};
	}

	impl_buffer_usage!(Source, BU::TRANSFER_SRC);
	impl_buffer_usage!(Destination, BU::TRANSFER_DST);
	impl_buffer_usage!(Vertex, BU::VERTEX_BUFFER);
	impl_buffer_usage!(Index, BU::INDEX_BUFFER);
}

pub struct VkMemory<A, P> where A: Allocator, P: MemoryProperties {
	device: Arc<VkDevice>,
	handle: vk::DeviceMemory,
	type_index: u32,
	allocator: Mutex<A>,
	size: u64,
	access: Mutex<MemoryAccess>,
	_properties: PhantomData<P>,
}

struct MemoryAccess {
	count: usize,
	pointer: usize,
}

pub struct VkBuffer<MA, BA, P, U>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
		  U: BufferUsage
{
	memory: Arc<VkMemory<MA, P>>,
	identifier: MA::Identifier,
	handle: vk::Buffer,
	allocator: Mutex<BA>,
	_usage: PhantomData<U>,
}

pub struct VkData<MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
		  U: BufferUsage,
		  D: ?Sized
{
	buffer: Arc<VkBuffer<MA, BA, P, U>>,
	identifier: BA::Identifier,
	range: ops::Range<u64>,
	_type: PhantomData<fn() -> D>,
}

pub struct VkDataAccess<'vk_data, MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
		  U: BufferUsage,
		  D: ?Sized
{
	data: &'vk_data VkData<MA, BA, P, U, D>,
	memory_pointer: usize,
}

#[derive(Debug)]
pub enum MemoryErr {
	Vk(vk::Result),
	NoValidMemoryTypeIndex,
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

pub trait VertexBuffer {
	type VertexType: Vertex;
	fn vk_buffer_id(&self) -> vk::Buffer;
	fn offset(&self) -> u64;
	fn count(&self) -> u64;
}

pub trait Vertex {
	fn input_descriptions() -> VertexInputDescriptions;
}

pub struct VertexInputDescriptions {
	binding: vk::VertexInputBindingDescription,
	attributes_count: usize,
	attributes: [vk::VertexInputAttributeDescription; 8],
}

pub struct VertexInputState<'desc> {
	input_state: vk::PipelineVertexInputStateCreateInfo,
	_marker: PhantomData<&'desc VertexInputDescriptions>,
}

pub trait GraphicPipeline {
	type Vertex: Vertex;
	type DescriptorSet;
	type PushConstant;
}

impl<A, P> VkMemory<A, P> where A: Allocator, P: MemoryProperties {
	pub fn with_allocator(
		device: Arc<VkDevice>,
		allocator: A,
		_memory_properties: P,
	) -> Result<Arc<Self>, MemoryErr> {
		let type_index = Self::compatible_memory_type_indices::<P>(&device)
			.into_iter()
			.next()
			.ok_or(MemoryErr::NoValidMemoryTypeIndex)?;
		let size = allocator.size();

		let handle = unsafe {
			let info = vk::MemoryAllocateInfo {
				s_type: StructureType::MEMORY_ALLOCATE_INFO,
				p_next: ptr::null(),
				allocation_size: size,
				memory_type_index: type_index,
			};

			device.handle.allocate_memory(&info, None)?
		};

		// This Arc should be drop by end of this function to make Weak can't be upgraded.
		let dummy_arc = Arc::new(0);
		let mapping = Mutex::new(Arc::downgrade(&dummy_arc));
		drop(dummy_arc);

		let memory = Self {
			device,
			handle,
			type_index,
			size: allocator.size(),
			allocator: Mutex::new(allocator),
			access: Mutex::new(MemoryAccess { count: 0, pointer: 0 }),
			_properties: PhantomData,
		};

		Ok(Arc::new(memory))
	}

	fn compatible_memory_type_indices<MP>(device: &Arc<VkDevice>) -> Vec<u32>
		where MP: MemoryProperties
	{
		let flags = MP::flags();
		device.instance.physical_devices[device.physical_device_index]
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

impl<A, P> Drop for VkMemory<A, P>
	where A: Allocator,
		  P: MemoryProperties
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

impl From<vk::Result> for MemoryErr {
	fn from(v: vk::Result) -> Self { MemoryErr::Vk(v) }
}

impl<MA, BA, P, U> VkBuffer<MA, BA, P, U>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
		  U: BufferUsage
{
	pub fn new<T>(
		memory: Arc<VkMemory<MA, P>>,
		queue: Arc<VkQueue<T>>,
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
				usage: U::flags(),
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

impl<MA, BA, P, U> Drop for VkBuffer<MA, BA, P, U>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
		  U: BufferUsage
{
	fn drop(&mut self) {
		unsafe { self.memory.device.handle.destroy_buffer(self.handle, None); }
		self.memory.allocator.lock().unwrap().dealloc(&self.identifier);
	}
}

impl From<vk::Result> for BufferErr {
	fn from(v: vk::Result) -> Self { BufferErr::Vk(v) }
}
impl From<alloc::AllocErr> for BufferErr {
	fn from(a: alloc::AllocErr) -> Self { BufferErr::Allocator(a) }
}


impl< MA, BA, P, U, D> VkData<MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
		  U: BufferUsage,
		  D: ?Sized
{
	pub fn new(
		buffer: Arc<VkBuffer<MA, BA, P, U>>,
		value: &D,
	) -> Result<Self, DataErr> {
		let layout = Layout::for_value(value);
		let (range, identifier) = buffer.allocator.lock().unwrap().alloc(layout)?;

		Ok(Self { buffer, identifier, range, _type: PhantomData })
	}

	// TODO: set flag bound (memory properties must contain HostVisible)
	pub fn access(&self) -> VkDataAccess<MA, BA, P, U, D> {
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

		VkDataAccess { data: self, memory_pointer }
	}
}


impl<MA, BA, P, U, D> Drop for VkData<MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
		  U: BufferUsage,
		  D: ?Sized
{
	fn drop(&mut self) { self.buffer.allocator.lock().unwrap().dealloc(&self.identifier); }
}

impl From<alloc::AllocErr> for DataErr {
	fn from(a: alloc::AllocErr) -> Self { DataErr::Allocator(a) }
}

impl<MA, BA, P, U, D> VkDataAccess<'_, MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
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

impl<MA, BA, P, U, D> VkDataAccess<'_, MA, BA, P, U, [D]>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
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

impl<MA, BA, P, U, D> Drop for VkDataAccess<'_, MA, BA, P, U, D>
	where MA: Allocator,
		  BA: Allocator,
		  P: MemoryProperties,
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