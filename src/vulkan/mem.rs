use ash::version::DeviceV1_0;
use ash::vk;
use ash::vk::StructureType;

use crate::vulkan::VkCore;

use std::ptr;
use std::ops;
use std::fmt;
use std::error::Error;
use std::path::Path;

pub struct VkAlloc<'vk_core> {
    vk_core: &'vk_core VkCore,
    memories: Vec<Memory>,
}

pub struct Memory {
	block_size: vk::DeviceSize,
    blocks: Vec<MemoryBlock>,
	resources: Vec<Resource>,
	buffers: Vec<Buffer>,
	images: Vec<Image>,
}

pub struct MemoryBlock {
    raw_handle: vk::DeviceMemory,
    size: vk::DeviceSize,
	stack_offset: vk::DeviceSize,
}

pub enum Resource {
	Buffer(usize),
	Image(usize),
}

pub struct Buffer {
	raw_handle: vk::Buffer,
	data_blocks: Vec<Data>,
	range: ops::Range<vk::DeviceSize>,
}

pub struct Data {
	range: ops::Range<vk::DeviceSize>,
}

pub struct Image {
	raw_handle: vk::Image,
	extent: vk::Extent3D,
	format: vk::Format,
	layout: Vec<vk::ImageLayout>,
	aspect_mask: vk::ImageAspectFlags,
	mip_levels: u32,
	array_layers: u32,
	view: vk::ImageView,
	range: ops::Range<vk::DeviceSize>,
}

pub struct BufferHandle {
	memory_type_index: usize,
	buffer_index: usize,
}

pub struct DataHandle {
	buffer_handle: BufferHandle,
	data_index: usize,
}

pub struct ImageHandle {
	memory_type_index: usize,
	image_index: usize,
}

#[derive(Debug)]
pub enum AllocErr {
	VkErr(vk::Result),
	NoValidMemoryType,
}

impl Error for AllocErr {}
impl fmt::Display for AllocErr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			AllocErr::VkErr(err) => write!(f, "vk::Result: {}", err),
			AllocErr::NoValidMemoryType => write!(f, "Valid memory type was not found."),
		}
	}
}

impl<'vk_core> VkAlloc<'vk_core> {
    pub fn new(vk_core: &'vk_core VkCore) -> Self {
		Self::with_block_size(vk_core, |memory_type| {
			if memory_type.property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL) {
				0x1000_0000
			} else {
				0x400_0000
			}
		})
	}

	pub fn with_block_size<F>(vk_core: &'vk_core VkCore, mut f: F) -> Self
		where F: FnMut(&vk::MemoryType) -> vk::DeviceSize
	{
		let memory_properties = &vk_core.physical_device.memory_properties;
		let mem_ty_count = memory_properties.memory_type_count as usize;
		let mut memories = Vec::with_capacity(mem_ty_count);
		for memory_type in memory_properties.memory_types[0..mem_ty_count].iter() {
			let memory = Memory {
				block_size: f(memory_type),
				blocks: Vec::new(),
				resources: Vec::new(),
				buffers: Vec::new(),
				images: Vec::new(),
			};

			memories.push(memory);
		}

		Self { vk_core, memories }
	}

	pub fn push_buffer(
		&mut self,
		size: vk::DeviceSize,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Result<BufferHandle, AllocErr> {
		unsafe {
			let device = &self.vk_core.device;
			let physical_device = &self.vk_core.physical_device;
			let buffer = device
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
				)
				.map_err(|err| AllocErr::VkErr(err))?;

			let memory_requirements = device.get_buffer_memory_requirements(buffer);
			eprintln!("Memory requirements of a buffer: {:?}", memory_requirements);

			let memory_type_index = find_memory_type_index(
				self.vk_core,
				memory_requirements.memory_type_bits,
				memory_properties,
			).ok_or(AllocErr::NoValidMemoryType)? as usize;

			let buffer_opt = match self.memories[memory_type_index].blocks.last_mut() {
				Some(block) => {
					// bind memory. be careful about alignment and offset.
					let new_offset = if block.stack_offset % memory_requirements.alignment != 0 {
						(block.stack_offset / memory_requirements.alignment + 1)
							* memory_requirements.alignment
					} else {
						block.stack_offset
					};

					let range_end = new_offset + memory_requirements.size;
					if range_end < block.size {
						block.stack_offset = new_offset;
						self.vk_core.device
							.bind_buffer_memory(buffer, block.raw_handle, block.stack_offset)
							.map_err(|err| AllocErr::VkErr(err))?;

						Some(
							Buffer {
								raw_handle: buffer,
								data_blocks: Vec::new(),
								range: block.stack_offset..range_end
							}
						)
					} else {
						None
					}
				}
				None => None,
			};

			let buffer = match buffer_opt {
				Some(buffer) => buffer,
				None => {
					let allocation_size = self.memories[memory_type_index].block_size
						.max(memory_requirements.size);

					let new_block = self.vk_core.device
						.allocate_memory(
							&vk::MemoryAllocateInfo {
								s_type: StructureType::MEMORY_ALLOCATE_INFO,
								p_next: ptr::null(),
								allocation_size,
								memory_type_index: memory_type_index as _,
							},
							None,
						)
						.map_err(|err| AllocErr::VkErr(err))?;

					self.memories[memory_type_index].blocks.push(
						MemoryBlock {
							raw_handle: new_block,
							size: allocation_size,
							stack_offset: 0,
						}
					);

					self.vk_core.device
						.bind_buffer_memory(buffer, new_block, 0)
						.map_err(|err| AllocErr::VkErr(err))?;

					Buffer {
						raw_handle: buffer,
						data_blocks: Vec::new(),
						range: 0..memory_requirements.size,
					}
				}
			};

			self.memories[memory_type_index].buffers.push(buffer);
			let buffer_index = self.memories[memory_type_index].buffers.len() - 1;
			self.memories[memory_type_index].resources.push(Resource::Buffer(buffer_index));

			Ok(BufferHandle { memory_type_index, buffer_index })
		}
	}

	pub fn push_image(
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
	) -> Result<ImageHandle, AllocErr> {
		unsafe {
			unimplemented!()
		}
	}

	pub fn push_data<D>(&mut self, data: D) -> Result<DataHandle, AllocErr> where D: AsRef<[u8]> {
		unimplemented!()
	}

	pub fn pop(&mut self) -> Resource {
		unimplemented!()
	}
}

impl Drop for VkAlloc<'_> {
	fn drop(&mut self) {
		unsafe {
			self.memories
				.iter()
				.for_each(|memory| {
					memory.blocks
						.iter()
						.for_each(|block| self.vk_core.device.free_memory(block.raw_handle, None));
					memory.buffers
						.iter()
						.for_each(|buffer| {
							self.vk_core.device.destroy_buffer(buffer.raw_handle, None);
						});
					memory.images
						.iter()
						.for_each(|image| {
							self.vk_core.device.destroy_image(image.raw_handle, None);
							self.vk_core.device.destroy_image_view(image.view, None);
						})
				})
		}
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