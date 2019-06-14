pub mod alloc;
pub mod buffer;
pub mod image;

pub use alloc::*;
pub use buffer::*;
pub use image::*;

use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::*;

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


pub use self::memory_type::MemoryProperties;
pub mod memory_type {
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

#[derive(Debug)]
pub enum MemoryErr {
	Vk(vk::Result),
	NoValidMemoryTypeIndex,
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

