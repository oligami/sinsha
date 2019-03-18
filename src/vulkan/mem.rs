use ash::vk;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::VkCore;

use std::io;
use std::ptr;
use std::ops;
use std::fmt;
use std::slice;
use std::error::Error;
use std::path::Path;

pub struct MemoryBlock<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::DeviceMemory,
	type_index: u32,
	size: vk::DeviceSize,
	stack_offset: vk::DeviceSize,
	resources: Vec<Resource>,
	buffers: Vec<Buffer>,
	images: Vec<Image>,
}

enum Resource {
	Buffer(usize),
	Image(usize),
}

pub (in crate::vulkan) struct Buffer {
	pub (in crate::vulkan) raw_handle: vk::Buffer,
	pub (in crate::vulkan) data_blocks: Vec<Data>,
	pub (in crate::vulkan) range: ops::Range<vk::DeviceSize>,
}

pub (in crate::vulkan) struct Data {
	pub (in crate::vulkan) range: ops::Range<vk::DeviceSize>,
}

pub (in crate::vulkan) struct Image {
	pub (in crate::vulkan) raw_handle: vk::Image,
	pub (in crate::vulkan) extent: vk::Extent3D,
	pub (in crate::vulkan) format: vk::Format,
	pub (in crate::vulkan) layout: Vec<vk::ImageLayout>,
	pub (in crate::vulkan) aspect_mask: vk::ImageAspectFlags,
	pub (in crate::vulkan) mip_levels: u32,
	pub (in crate::vulkan) array_layers: u32,
	pub (in crate::vulkan) view: vk::ImageView,
	pub (in crate::vulkan) range: ops::Range<vk::DeviceSize>,
}

pub struct BufferIndex(usize);
pub struct DataIndex(BufferIndex, usize);
pub struct ImageIndex(usize);

pub struct BufferAccess<'vk_core, 'memory> {
	memory: &'memory mut MemoryBlock<'vk_core>,
	rw: io::Cursor<&'memory mut [u8]>,
	buffer_index: BufferIndex,
}

#[derive(Debug)]
pub enum AllocErr {
	VkErr(vk::Result),
	NoValidMemoryTypeIndex,
	InvalidMemoryTypeIndex,
	OutOfMemoryBlock,
	OutOfBuffer,
}

impl Error for AllocErr {}
impl fmt::Display for AllocErr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			AllocErr::VkErr(err) => write!(f, "vk::Result: {}", err),
			AllocErr::NoValidMemoryTypeIndex => write!(f, "Valid memory type was not found."),
			AllocErr::InvalidMemoryTypeIndex => write!(f, "Memory type is invalid."),
			AllocErr::OutOfMemoryBlock => write!(f, "Out of memory block."),
			AllocErr::OutOfBuffer => write!(f, "Out of buffer in memory block."),
		}
	}
}

impl<'vk_core> MemoryBlock<'vk_core> {
    pub fn new(
		vk_core: &'vk_core VkCore,
		type_index: u32,
		size: vk::DeviceSize,
	) -> Result<Self, AllocErr> {
		unsafe {
			let raw_handle = vk_core.device
				.allocate_memory(
					&vk::MemoryAllocateInfo {
						s_type: StructureType::MEMORY_ALLOCATE_INFO,
						p_next: ptr::null(),
						allocation_size: size,
						memory_type_index: type_index as u32,
					},
					None,
				)
				.map_err(|err| AllocErr::VkErr(err))?;

			Ok(
				Self {
					vk_core,
					raw_handle,
					type_index,
					size,
					stack_offset: 0,
					resources: Vec::new(),
					buffers: Vec::new(),
					images: Vec::new(),
				}
			)
		}
	}



	pub fn with_buffer(
		vk_core: &'vk_core VkCore,
		memory_size: vk::DeviceSize,
		memory_properties: vk::MemoryPropertyFlags,
		buffer_size: vk::DeviceSize,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
	) -> Result<(Self, BufferIndex), AllocErr> {
		unsafe {
			let device = &vk_core.device;
			let physical_device = &vk_core.physical_device;
			let buffer = device
				.create_buffer(
					&vk::BufferCreateInfo {
						s_type: StructureType::BUFFER_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::BufferCreateFlags::empty(),
						size: buffer_size,
						usage,
						sharing_mode,
						queue_family_index_count: physical_device.queue_family_index_count(),
						p_queue_family_indices: physical_device.queue_family_index_ptr(),
					},
					None,
				)
				.map_err(|err| AllocErr::VkErr(err))?;

			let memory_requirements = device.get_buffer_memory_requirements(buffer);
			eprintln!("Memory requirements of a buffer: {:?}", memory_requirements);

			let memory_type_index =
				find_memory_type_index(
					vk_core,
					memory_requirements.memory_type_bits,
					memory_properties,
				)
				.ok_or(AllocErr::NoValidMemoryTypeIndex)
				.map_err(|err| {
					device.destroy_buffer(buffer, None);
					err
				})?;
			eprintln!("Memory type index of a buffer: {}", memory_type_index);

			let mut memory_block = Self::new(
				vk_core,
				memory_type_index,
				memory_size,
			)?;

			device
				.bind_buffer_memory(buffer, memory_block.raw_handle, 0)
				.map_err(|err| {
					device.destroy_buffer(buffer, None);
					AllocErr::VkErr(err)
				})?;

			let buffer = Buffer {
				raw_handle: buffer,
				data_blocks: Vec::new(),
				range: 0..memory_requirements.size,
			};

			memory_block.stack_offset = memory_requirements.size;
			memory_block.buffers.push(buffer);
			memory_block.resources.push(Resource::Buffer(0));

			Ok((memory_block, BufferIndex(0)))
		}
	}

	pub fn with_image(
		vk_core: &'vk_core VkCore,
		memory_size: vk::DeviceSize,
		memory_properties: vk::MemoryPropertyFlags,
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
		view_type: vk::ImageViewType,
		components: vk::ComponentMapping,
	) -> Result<(Self, ImageIndex), AllocErr> {
		unsafe {
			let device = &vk_core.device;
			let physical_device = &vk_core.physical_device;

			// create image
			let image = device
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
				)
				.map_err(|err| AllocErr::VkErr(err))?;

			let destroy_image = || device.destroy_image(image, None);

			let memory_requirements = device.get_image_memory_requirements(image);
			eprintln!("Memory requirements of an image: {:?}", memory_requirements);

			let memory_type_index = find_memory_type_index(
				vk_core,
				memory_requirements.memory_type_bits,
				memory_properties,
			)
				.ok_or(AllocErr::NoValidMemoryTypeIndex)
				.map_err(|err| {
					destroy_image();
					err
				})?;
			eprintln!("Memory type index of an image: {}", memory_type_index);

			let mut memory_block = Self::new(
				vk_core,
				memory_type_index,
				memory_size,
			)
				.map_err(|err| {
					destroy_image();
					err
				})?;

			device
				.bind_image_memory(image, memory_block.raw_handle, 0)
				.map_err(|err| {
					destroy_image();
					AllocErr::VkErr(err)
				})?;

			let view = device
				.create_image_view(
					&vk::ImageViewCreateInfo {
						s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::ImageViewCreateFlags::empty(),
						image,
						format,
						view_type,
						components,
						subresource_range: vk::ImageSubresourceRange {
							aspect_mask,
							base_mip_level: 0,
							level_count: mip_levels,
							base_array_layer: 0,
							layer_count: array_layers,
						}
					},
					None,
				)
				.map_err(|err| {
					destroy_image();
					AllocErr::VkErr(err)
				})?;

			let image = Image {
				raw_handle: image,
				extent,
				format,
				layout: vec![initial_layout; mip_levels as usize],
				aspect_mask,
				mip_levels,
				array_layers,
				view,
				range: 0..memory_requirements.size,
			};

			let image_index = ImageIndex(0);
			memory_block.resources.push(Resource::Image(0));
			memory_block.images.push(image);
			memory_block.stack_offset = memory_requirements.size;

			Ok((memory_block, image_index))
		}
	}

	pub fn bind_buffer(
		&mut self,
		size: vk::DeviceSize,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
	) -> Result<BufferIndex, AllocErr> {
		unimplemented!()
	}

	pub fn bind_image(
		&mut self,
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
		view_type: vk::ImageViewType,
		components: vk::ComponentMapping,
	) -> Result<ImageIndex, AllocErr> {
		unsafe {
			let device = &self.vk_core.device;
			let physical_device = &self.vk_core.physical_device;

			// create image
			let image = device
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
				)
				.map_err(|err| AllocErr::VkErr(err))?;

			// examine memory requirements
			let memory_requirements = device.get_image_memory_requirements(image);
			eprintln!("Memory requirements of an image: {:?}", memory_requirements);
			if 1 << self.type_index & memory_requirements.memory_type_bits != 0 {
				return Err(AllocErr::InvalidMemoryTypeIndex);
			}

			// check alignment and size
			let alignment = memory_requirements.alignment;
			let new_offset = if self.stack_offset % alignment == 0 {
				self.stack_offset
			} else {
				(self.stack_offset / alignment + 1) * alignment
			};

			let destroy_image = || unsafe { device.destroy_image(image, None); };

			let range_end = new_offset + memory_requirements.size;
			if range_end > self.size {
				destroy_image();
				return Err(AllocErr::OutOfMemoryBlock);
			}

			// bind the image to the memory
			device
				.bind_image_memory(image, self.raw_handle, new_offset)
				.map_err(|err| {
					destroy_image();
					AllocErr::VkErr(err)
				})?;

			// create image view and image structure
			let view = device
				.create_image_view(
					&vk::ImageViewCreateInfo {
						s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::ImageViewCreateFlags::empty(),
						image,
						format,
						view_type,
						components,
						subresource_range: vk::ImageSubresourceRange {
							aspect_mask,
							base_mip_level: 0,
							level_count: mip_levels,
							base_array_layer: 0,
							layer_count: array_layers,
						}
					},
					None,
				)
				.map_err(|err| {
					destroy_image();
					AllocErr::VkErr(err)
				})?;

			let image = Image {
				raw_handle: image,
				extent,
				format,
				layout: vec![initial_layout; mip_levels as usize],
				aspect_mask,
				mip_levels,
				array_layers,
				view,
				range: new_offset..range_end,
			};

			// update data in the structure
			self.stack_offset = range_end;
			let image_index = self.images.len();
			self.resources.push(Resource::Image(image_index));
			self.images.push(image);

			Ok(ImageIndex(image_index))
		}
	}

	pub fn bind_data(
		&mut self,
		buffer_index: &BufferIndex,
		data_size: vk::DeviceSize,
	) -> Result<DataIndex, AllocErr> {
		let buffer = &mut self.buffers[buffer_index.0];
		let offset = buffer.data_blocks
			.last()
			.map(|data_block| data_block.range.end)
			.unwrap_or(0) + buffer.range.start;
		let range = offset..offset + data_size;
		if range.end > buffer.range.end {
			return Err(AllocErr::OutOfMemoryBlock)
		}
		let data_index = buffer.data_blocks.len();
		buffer.data_blocks.push(Data { range });

		Ok(DataIndex(BufferIndex(buffer_index.0), data_index))
	}

	pub fn access_buffer<'memory>(
		&'memory mut self,
		buffer_index: &BufferIndex,
	) -> Result<BufferAccess<'vk_core, 'memory>, vk::Result> {
		unsafe {
			let buffer = &self.buffers[buffer_index.0];
			let offset = buffer.range.start;
			let size = buffer.range.end - buffer.range.start;
			let ptr = self.vk_core.device
				.map_memory(self.raw_handle, offset, size, vk::MemoryMapFlags::empty())? as *mut u8;

			let rw = io::Cursor::new(slice::from_raw_parts_mut(ptr, size as _));

			Ok(BufferAccess { memory: self, rw, buffer_index: BufferIndex(buffer_index.0) })
		}
	}

	pub fn clear_last(&mut self) {
		unimplemented!()
	}

	pub fn clear_data(&mut self, buffer_index: usize) {
		self.buffers[buffer_index].data_blocks.clear();
	}

	pub fn clear(&mut self) {
		unsafe {
			self.buffers
				.iter()
				.for_each(|buffer| {
					self.vk_core.device.destroy_buffer(buffer.raw_handle, None);
				});
			self.images
				.iter()
				.for_each(|image| {
					self.vk_core.device.destroy_image(image.raw_handle, None);
					self.vk_core.device.destroy_image_view(image.view, None);
				})
		}

		self.resources.clear();
		self.buffers.clear();
		self.images.clear();
		self.stack_offset = 0;
	}

	#[inline]
	pub(in crate::vulkan)
	fn buffer_ref(&self, index: &BufferIndex) -> &Buffer { &self.buffers[index.0] }

	#[inline]
	pub(in crate::vulkan)
	fn data_ref(&self, index: &DataIndex) -> &Data {
		&self.buffer_of_data(index).data_blocks[index.1]
	}

	#[inline]
	pub(in crate::vulkan)
	fn buffer_of_data(&self, index: &DataIndex) -> &Buffer {
		&self.buffers[(index.0).0]
	}

	#[inline]
	pub(in crate::vulkan)
	fn image_ref(&self, index: &ImageIndex) -> &Image { &self.images[index.0] }

	#[inline]
	pub(in crate::vulkan)
	fn image_ref_mut(&mut self, index: &ImageIndex) -> &mut Image { &mut self.images[index.0] }
}

impl Drop for MemoryBlock<'_> {
	fn drop(&mut self) {
		unsafe {
			self.vk_core.device.free_memory(self.raw_handle, None);
			self.buffers
				.iter()
				.for_each(|buffer| {
					self.vk_core.device.destroy_buffer(buffer.raw_handle, None);
				});
			self.images
				.iter()
				.for_each(|image| {
					self.vk_core.device.destroy_image(image.raw_handle, None);
					self.vk_core.device.destroy_image_view(image.view, None);
				})
		}
	}
}

impl BufferAccess<'_, '_> {
	pub fn write_data<D>(&mut self, data_index: &DataIndex, data: D) -> io::Result<()>
		where D: AsRef<[u8]>
	{
		debug_assert_eq!(self.buffer_index.0, (data_index.0).0);
		let data_block = &self.memory.buffers[self.buffer_index.0].data_blocks[data_index.1];
		let offset = data_block.range.start;
		if offset + data.as_ref().len() as u64 > data_block.range.end {
			let exceed = offset + data.as_ref().len() as u64 - data_block.range.end;
			let err = io::Error::new(
				io::ErrorKind::AddrNotAvailable,
				format!("data length exceeded: {}", exceed),
			);
			return Err(err);
		}

		io::Seek::seek(&mut self.rw, io::SeekFrom::Start(offset))?;
		io::Write::write_all(&mut self.rw, data.as_ref())?;
		Ok(())
	}
}

impl Drop for BufferAccess<'_, '_> {
	fn drop(&mut self) {
		unsafe { self.memory.vk_core.device.unmap_memory(self.memory.raw_handle) }
	}
}


fn find_memory_type_index(
	vk_core: &VkCore,
	memory_type_bits: u32,
	memory_properties: vk::MemoryPropertyFlags,
) -> Option<u32> {
	let p_device_mem_prop = &vk_core.physical_device.memory_properties;
	for index in 0..p_device_mem_prop.memory_type_count {
		let index_available = memory_type_bits & 1 << index != 0;
		let index_memory_properties = p_device_mem_prop.memory_types[index as usize].property_flags;
		let properties_satisfied = index_memory_properties & memory_properties == memory_properties;
		if index_available && properties_satisfied {
			return Some(index)
		}
	}
	None
}