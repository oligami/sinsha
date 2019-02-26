use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan_api::PhysicalDevice;
use crate::vulkan_api::shaders::*;
use crate::vulkan_api::command_recorder::CommandRecorder;

use std::io;
use std::fmt;
use std::ptr;
use std::mem;
use std::slice;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};
use std::error::Error;

/// This struct must be bound to memory otherwise destructor won't run.
pub struct Buffer {
	raw_handle: vk::Buffer,
	/// Buffer location in the bound memory.
	/// (range.start = range.end = 0) means that this buffer is not bound to any memory.
	range: Range<vk::DeviceSize>,
}

/// This struct must be bound to memory otherwise destructor won't run.
pub struct Image {
	raw_handle: vk::Image,
	/// if view is null, this image is not bound to any memory.
	view: vk::ImageView,
	format: vk::Format,
	layout: Vec<vk::ImageLayout>,
	extent: vk::Extent3D,
	aspect_mask: vk::ImageAspectFlags,
	mip_levels: u32,
	array_layers: u32,
}

pub struct Rw;
pub struct NonRw;

/// If T is Rw, memory properties always contain HOST_VISIBLE and HOST_COHERENT.
/// If T is NonRw, memory properties may contain HOST_VISIBLE and/or HOST_COHERENT.
/// NonRw doesn't means that buffer data is immutable but that buffer data can't be accessed by CPU.
pub struct Memory<'device> {
	device: &'device Device,
	raw_handle: vk::DeviceMemory,
	buffer: Buffer,
	images: Vec<Image>,
}

pub struct MemoryAccessor<'memory, 'device> {
	buffer: &'memory mut [u8],
	seeker: u64,
	memory: &'memory Memory<'device>,
}

#[derive(Debug)]
pub enum MemoryTypeError {
	NotFound,
}

impl Buffer {
	pub unsafe fn uninitialized(
		physical_device: &PhysicalDevice,
		device: &Device,
		size: vk::DeviceSize,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
	) -> Result<Self, vk::Result> {
		let raw_handle = device.create_buffer(
			&vk::BufferCreateInfo {
				s_type: StructureType::BUFFER_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::BufferCreateFlags::empty(),
				size,
				usage,
				sharing_mode,
				queue_family_index_count: 1,
				p_queue_family_indices: physical_device.queue_family_index_ptr(),
			},
			None,
		)?;

		Ok(
			Self {
				raw_handle,
				range: 0..0,
			}
		)
	}

	#[inline]
	pub fn size(&self) -> vk::DeviceSize {
		self.range.end - self.range.start
	}

	#[inline]
	pub fn bound_to_memory(&self) -> bool {
		self.size() != 0
	}
}

impl Drop for Buffer {
	fn drop(&mut self) { debug_assert!(self.bound_to_memory()); }
}

impl Image {
	pub unsafe fn uninitialized(
		physical_device: &PhysicalDevice,
		device: &Device,
		extent: vk::Extent3D,
		format: vk::Format,
		usage: vk::ImageUsageFlags,
		sharing_mode: vk::SharingMode,
		initial_layout: vk::ImageLayout,
		sample_count: vk::SampleCountFlags,
		aspect_mask: vk::ImageAspectFlags,
		mip_levels: u32,
		array_layers: u32,
		image_type: vk::ImageType,
	) -> Self {
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
					queue_family_index_count: 1,
					p_queue_family_indices: &physical_device.queue_family_index as *const _,
				},
				None,
			)
			.unwrap();

		Self {
			raw_handle: image,
			view: vk::ImageView::null(),
			extent,
			format,
			layout: vec![initial_layout; mip_levels as usize],
			aspect_mask,
			mip_levels,
			array_layers,
		}
	}

	pub fn attach_image_view(
		&mut self,
		device: &Device,
		view_type: vk::ImageViewType,
		component_mapping: vk::ComponentMapping,
	) {
		unsafe {
			debug_assert_eq!(self.view, vk::ImageView::null());
			let image_view = device
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
						components: component_mapping,
					},
					None,
				)
				.unwrap();

			self.view = image_view;
		}
	}

	#[inline]
	pub fn raw_handle(&self) -> vk::Image {
		self.raw_handle
	}

	#[inline]
	pub fn view(&self) -> vk::ImageView {
		self.view
	}

	#[inline]
	pub fn layout(&self, mip_level: u32) -> vk::ImageLayout {
		debug_assert!(mip_level <= self.mip_levels);
		self.layout[mip_level as usize]
	}

	#[inline]
	pub fn extent(&self, mip_level: u32) -> vk::Extent3D {
		debug_assert!(mip_level <= self.mip_levels);
		let divider = 2_u32.pow(mip_level);
		vk::Extent3D {
			width: self.extent.width / divider,
			height: self.extent.height / divider,
			depth: self.extent.depth,
		}
	}

	#[inline]
	pub fn aspect_mask(&self) -> vk::ImageAspectFlags {
		self.aspect_mask
	}

	#[inline]
	pub fn mip_levels(&self) -> u32 {
		self.mip_levels
	}

	#[inline]
	pub fn array_layers(&self) -> u32 {
		self.array_layers
	}

	pub fn maximum_mip_level(width: u32, height: u32) -> u32 {
		[width, height]
			.iter()
			.map(|&num| (num as f32).log2() as u32)
			.min()
			.unwrap_or(1)
	}

	#[inline]
	pub fn bound_to_memory(&self) -> bool {
		self.view != vk::ImageView::null()
	}
}

impl Drop for Image {
	fn drop(&mut self) { debug_assert!(self.bound_to_memory()); }
}

impl<'device> Memory<'device> {
	pub unsafe fn uninitialized(
		physical_device: &PhysicalDevice,
		device: &'device Device,
		(mut buffer, buffer_pos): (Buffer, usize),
		images: Vec<Image>,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Result<Self, Box<dyn Error>> {
		let mut memory_requirements: Vec<_> = images[..buffer_pos]
			.iter()
			.map(|image| device.get_image_memory_requirements(image.raw_handle))
			.collect();

		memory_requirements.reserve(images.len() - buffer_pos + 1);
		memory_requirements.push(device.get_buffer_memory_requirements(buffer.raw_handle));

		images[buffer_pos..]
			.iter()
			.for_each(|image| {
				memory_requirements.push(device.get_image_memory_requirements(image.raw_handle));
			});

		let (allocation_size, offsets, required_memory_type) = memory_requirements
			.into_iter()
			.fold(
				(0, Vec::with_capacity(images.len() + 1), 0),
				|(alloc_size, mut offsets, req_mem_ty), mem_req| {
					offsets.push(alloc_size);
					(alloc_size + mem_req.size, offsets, req_mem_ty & mem_req.memory_type_bits)
				}
			);

		let memory = device.allocate_memory(
			&vk::MemoryAllocateInfo {
				s_type: StructureType::MEMORY_ALLOCATE_INFO,
				p_next: ptr::null(),
				allocation_size,
				memory_type_index: find_memory_type_index(
					physical_device,
					required_memory_type,
					memory_properties,
				)?,
			},
			None,
		)?;

		images[..buffer_pos]
			.iter()
			.zip(offsets[..buffer_pos].iter())
			.for_each(|(image, &offset)| {
				device.bind_image_memory(image.raw_handle, memory, offset)?;
			});
		device.bind_buffer_memory(buffer.raw_handle, memory, offsets[buffer_pos]);
		buffer.range = offsets[buffer_pos]..offsets[buffer_pos + 1];
		images[buffer_pos..]
			.iter()
			.zip(offsets[buffer_pos + 1..].iter())
			.for_each(|(image, &offset)| {
				device.bind_image_memory(image.raw_handle, memory, offset)?;
			});

		Ok(
			Self {
				device,
				buffer,
				images,
				raw_handle: memory,
			}
		)
	}

	pub fn accessor(&mut self) {
		unimplemented!()
	}
}


impl fmt::Display for MemoryTypeError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			MemoryTypeError::NotFound => {
				write!(f, "Not found suitable memory type index in physical device.")
			},
		}
	}
}
impl Error for MemoryTypeError {}


fn find_memory_type_index(
	physical_device: &PhysicalDevice,
	required_memory_type_bits: u32,
	required_memory_property_flags: vk::MemoryPropertyFlags,
) -> Result<u32, MemoryTypeError> {
	for i in 0..physical_device.memory_properties.memory_type_count {
		if required_memory_type_bits & 1 << i != 0
			&& physical_device.memory_properties.memory_types[i as usize].property_flags
			& required_memory_property_flags
			== required_memory_property_flags
		{
			return Ok(i);
		}
	}
	Err(MemoryTypeError::NotFound)
}