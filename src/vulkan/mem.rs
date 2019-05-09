use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::VkCore;

use std::io;
use std::ptr;
use std::ops;
use std::fmt;
use std::mem;
use std::slice;
use std::marker::PhantomData;
use std::error::Error;
use std::path::Path;

pub trait Memory {
	fn alloc_sized<T>(&mut self) -> ();
	fn alloc_unsized<T>(&mut self, len: usize) -> ();
}

pub trait MemoryStack: Memory {
	fn push_buffer<T>(&mut self) -> ();
	fn pop_buffer(&mut self) -> ();
}

pub struct VkMemory<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::DeviceMemory,
	size: vk::DeviceSize,
	type_index: u32,
}

pub struct VkSubBuffer<T> {
	buffer_id: vk::Buffer,
	len: usize,
	_marker: PhantomData<T>,
}

pub struct VkBufferSlice<'vk_core, T> {
	vk_core: &'vk_core VkCore,
	id: vk::Buffer,
	len: usize,
	_marker: PhantomData<T>,
}

pub struct VkBuffer<'vk_core> {
	vk_core: &'vk_core VkCore,
	pub(in crate::vulkan) raw_handle: vk::Buffer,
}

pub struct VkImage<'vk_core> {
	vk_core: &'vk_core VkCore,
	pub(in crate::vulkan) raw_handle: vk::Image,
	pub(in crate::vulkan) extent: vk::Extent3D,
	pub(in crate::vulkan) format: vk::Format,
	pub(in crate::vulkan) aspect_mask: vk::ImageAspectFlags,
	pub(in crate::vulkan) mip_levels: u32,
	pub(in crate::vulkan) array_layers: u32,
}

pub struct VkImageView<'vk_core> {
	vk_core: &'vk_core VkCore,
	pub(in crate::vulkan) raw_handle: vk::ImageView,
}

pub struct VkMemoryBinder<'vk_core, 'mem> {
	memory: &'mem VkMemory<'vk_core>,
	stack_offset: vk::DeviceSize,
}

pub struct VkMemoryAccess<'vk_core, 'mem> {
	memory: &'mem mut VkMemory<'vk_core>,
	rw: io::Cursor<&'mem mut [u8]>,
}

#[derive(Debug)]
pub enum AllocErr {
	VkErr(vk::Result),
	NoValidMemoryTypeIndex,
	InvalidMemoryTypeIndex,
	OutOfMemory,
}

impl Error for AllocErr {}
impl fmt::Display for AllocErr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			AllocErr::VkErr(err) => write!(f, "vk::Result: {}", err),
			AllocErr::NoValidMemoryTypeIndex => write!(f, "Valid memory type was not found."),
			AllocErr::InvalidMemoryTypeIndex => write!(f, "Memory type is invalid."),
			AllocErr::OutOfMemory => write!(f, "Memory size is not enough."),
		}
	}
}

impl<'vk_core> VkMemory<'vk_core> {
    pub fn new(
		vk_core: &'vk_core VkCore,
		size: vk::DeviceSize,
		type_index: u32,
	) -> Result<Self, AllocErr> {
		unsafe {
			let raw_handle = vk_core.device
				.allocate_memory(
					&vk::MemoryAllocateInfo {
						s_type: StructureType::MEMORY_ALLOCATE_INFO,
						p_next: ptr::null(),
						allocation_size: size,
						memory_type_index: type_index,
					},
					None,
				)
				.map_err(|err| AllocErr::VkErr(err))?;

			Ok(
				Self {
					vk_core,
					raw_handle,
					size,
					type_index,
				}
			)
		}
	}



	pub fn new_by_buffer_properties(
		memory_size: vk::DeviceSize,
		memory_properties: vk::MemoryPropertyFlags,
		buffer: &VkBuffer<'vk_core>,
	) -> Result<Self, AllocErr> {
		unsafe {
			let device = &buffer.vk_core.device;

			let mem_reqs = device.get_buffer_memory_requirements(buffer.raw_handle);

			let result = find_memory_type_index(
				buffer.vk_core,
				mem_reqs.memory_type_bits,
				memory_properties,
			);
			let memory_type_index = result.ok_or(AllocErr::NoValidMemoryTypeIndex)?;

			let memory_block = Self::new(buffer.vk_core, memory_size, memory_type_index)?;

			Ok(memory_block)
		}
	}

	pub fn new_by_image_properties(
		memory_size: vk::DeviceSize,
		memory_properties: vk::MemoryPropertyFlags,
		image: &VkImage<'vk_core>,
	) -> Result<Self, AllocErr> {
		unsafe {
			let device = &image.vk_core.device;

			let mem_reqs = device.get_image_memory_requirements(image.raw_handle);

			let result = find_memory_type_index(
				image.vk_core,
				mem_reqs.memory_type_bits,
				memory_properties,
			);
			let memory_type_index = result.ok_or(AllocErr::NoValidMemoryTypeIndex)?;

			let memory_block = Self::new(image.vk_core, memory_size, memory_type_index)?;

			Ok(memory_block)
		}
	}

	pub fn binder(&mut self, offset: vk::DeviceSize) -> VkMemoryBinder {
		VkMemoryBinder { memory: self, stack_offset: offset }
	}

	pub fn access(&mut self) -> Result<VkMemoryAccess<'vk_core, '_>, AllocErr> {
		unsafe {
			let ptr = self.vk_core.device
				.map_memory(self.raw_handle, 0, self.size, vk::MemoryMapFlags::empty())
				.map_err(|err| AllocErr::VkErr(err))? as *mut u8;
			let rw = io::Cursor::new(slice::from_raw_parts_mut(ptr, self.size as usize));

			Ok(VkMemoryAccess { memory: self, rw })
		}
	}
}

impl Drop for VkMemory<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.free_memory(self.raw_handle, None) } }
}

impl<'vk_core> VkBuffer<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		size: vk::DeviceSize,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
	) -> Result<Self, vk::Result> {
		unsafe {
			let physical_device = &vk_core.physical_device;

			let raw_handle = vk_core.device
				.create_buffer(
					&vk::BufferCreateInfo {
						s_type: StructureType::BUFFER_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::BufferCreateFlags::empty(),
						size,
						usage,
						sharing_mode,
						queue_family_index_count: physical_device.queue_family_index_count(),
						p_queue_family_indices: physical_device.queue_family_index_ptr(),
					},
					None,
				)?;

			Ok(Self { vk_core, raw_handle })
		}
	}
}

impl Drop for VkBuffer<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_buffer(self.raw_handle, None) } }
}

impl<'vk_core> VkImage<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		image_type: vk::ImageType,
		extent: vk::Extent3D,
		format: vk::Format,
		usage: vk::ImageUsageFlags,
		sharing_mode: vk::SharingMode,
		initial_layout: vk::ImageLayout,
		sample_count: vk::SampleCountFlags,
		aspect_mask: vk::ImageAspectFlags,
		mip_levels: u32,
		array_layers: u32,
	) -> Result<Self, vk::Result> {
		unsafe {
			let physical_device = &vk_core.physical_device;

			let raw_handle = vk_core.device
				.create_image(
					&vk::ImageCreateInfo {
						s_type: StructureType::IMAGE_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::ImageCreateFlags::empty(),
						extent,
						format,
						usage,
						sharing_mode,
						initial_layout,
						samples: sample_count,
						tiling: vk::ImageTiling::OPTIMAL,
						image_type,
						mip_levels,
						array_layers,
						queue_family_index_count: physical_device.queue_family_index_count(),
						p_queue_family_indices: physical_device.queue_family_index_ptr(),
					},
					None,
				)?;

			Ok(
				VkImage {
					vk_core,
					raw_handle,
					extent,
					format,
					aspect_mask,
					mip_levels,
					array_layers,
				}
			)
		}
	}

	pub fn view(
		&self,
		view_type: vk::ImageViewType,
		components: vk::ComponentMapping,
	) -> Result<VkImageView<'vk_core>, vk::Result> {
		unsafe {
			let raw_handle = self.vk_core.device
				.create_image_view(
					&vk::ImageViewCreateInfo {
						s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::ImageViewCreateFlags::empty(),
						image: self.raw_handle,
						format: self.format,
						subresource_range: vk::ImageSubresourceRange {
							aspect_mask: self.aspect_mask,
							base_mip_level: 0,
							level_count: self.mip_levels,
							base_array_layer: 0,
							layer_count: self.array_layers,
						},
						view_type,
						components,
					},
					None,
				)?;

			Ok(VkImageView { vk_core: self.vk_core, raw_handle })
		}
	}

	pub fn extent(&self, mip_level: u32) -> vk::Extent3D {
		let mut extent = self.extent;
		for _ in 0..mip_level {
			extent.width /= 2;
			extent.height /= 2;
		}

		extent
	}
}

impl Drop for VkImage<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_image(self.raw_handle, None) } }
}

impl Drop for VkImageView<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_image_view(self.raw_handle, None) } }
}

impl VkMemoryBinder<'_, '_> {
	pub fn bind_buffer(&mut self, buffer: &VkBuffer) -> Result<&mut Self, AllocErr> {
		unsafe {
			let device = &self.memory.vk_core.device;
			let memory_requirements = device.get_buffer_memory_requirements(buffer.raw_handle);
			let memory_range = self.memory_requirements_check(memory_requirements)?;
			self.stack_offset = memory_range.end;
			device
				.bind_buffer_memory(buffer.raw_handle, self.memory.raw_handle, memory_range.start)
				.map_err(|err| AllocErr::VkErr(err))?;

			Ok(self)
		}
	}

	pub fn bind_image(&mut self, image: &VkImage) -> Result<&mut Self, AllocErr> {
		unsafe {
			let device = &self.memory.vk_core.device;
			let memory_requirements = device.get_image_memory_requirements(image.raw_handle);
			let memory_range = self.memory_requirements_check(memory_requirements)?;
			self.stack_offset = memory_range.end;
			device
				.bind_image_memory(image.raw_handle, self.memory.raw_handle, memory_range.start)
				.map_err(|err| AllocErr::VkErr(err))?;

			Ok(self)
		}
	}

	fn memory_requirements_check(
		&self,
		memory_requirements: vk::MemoryRequirements
	) -> Result<ops::Range<vk::DeviceSize>, AllocErr> {
		if 1 << self.memory.type_index & memory_requirements.memory_type_bits == 0 {
			return Err(AllocErr::NoValidMemoryTypeIndex);
		}

		let alignment = memory_requirements.alignment;
		let new_stack_offset = if self.stack_offset % alignment == 0 {
			self.stack_offset
		} else {
			alignment * (self.stack_offset / alignment + 1)
		};

		let end = new_stack_offset + memory_requirements.size;
		if end > self.memory.size {
			return Err(AllocErr::OutOfMemory);
		}

		Ok(new_stack_offset..end)
	}
}

impl VkMemoryAccess<'_, '_> {
	pub fn write<B>(&mut self, offset: vk::DeviceSize, bytes: &B) -> io::Result<vk::DeviceSize>
		where B: ?Sized,
	{
		let bytes_len = mem::size_of_val(bytes) as u64;
		let end = offset + bytes_len;
		if end > self.memory.size {
			let exceeded = offset + bytes_len - self.memory.size;
			let err = io::Error::new(
				io::ErrorKind::AddrNotAvailable,
				format!("data is too long: {}", exceeded),
			);
			return Err(err);
		}

		let bytes = unsafe {
			slice::from_raw_parts(bytes as *const B as *const u8, bytes_len as usize)
		};
		io::Seek::seek(&mut self.rw, io::SeekFrom::Start(offset))?;
		io::Write::write_all(&mut self.rw, bytes)?;
		Ok(end)
	}
}

impl Drop for VkMemoryAccess<'_, '_> {
	fn drop(&mut self) {
		unsafe { self.memory.vk_core.device.unmap_memory(self.memory.raw_handle) }
	}
}


fn find_memory_type_index(
	vk_core: &VkCore,
	memory_type_bits: u32,
	memory_properties: vk::MemoryPropertyFlags,
) -> Option<u32> {
	let p_dev_mem_prop = &vk_core.physical_device.memory_properties;
	for index in 0..p_dev_mem_prop.memory_type_count {
		let index_available = memory_type_bits & 1 << index != 0;
		let index_memory_properties = p_dev_mem_prop.memory_types[index as usize].property_flags;
		let properties_satisfied = index_memory_properties & memory_properties == memory_properties;
		if index_available && properties_satisfied { return Some(index) }
	}
	None
}