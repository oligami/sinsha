pub mod alloc;
pub mod buffer;
pub mod image;
pub mod memory_property;

pub use alloc::*;
pub use buffer::*;
pub use image::*;
pub use memory_property::MemoryProperty;

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


pub struct VkMemory<A, P> where A: Allocator, P: MemoryProperty {
	device: Arc<Device>,
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


impl<A, P> VkMemory<A, P> where A: Allocator, P: MemoryProperty {
	pub fn with_allocator(
		device: Arc<Device>,
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

	fn compatible_memory_type_indices<MP>(device: &Arc<Device>) -> Vec<u32>
		where MP: MemoryProperty
	{
		let flags = MP::memory_property();
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

impl From<vk::Result> for MemoryErr {
	fn from(v: vk::Result) -> Self { MemoryErr::Vk(v) }
}

