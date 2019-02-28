use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan::*;

use std::io;
use std::fmt;
use std::ptr;
use std::mem;
use std::slice;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};
use std::error::Error;

/// This struct must be bound to memory otherwise destructor won't run.
pub struct Buffer<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::Buffer,
	/// Buffer location in the bound memory.
	/// (range.start = range.end = 0) means that this buffer is not bound to any memory.
	range: Range<vk::DeviceSize>,
}

/// This struct must be bound to memory otherwise destructor won't run.
pub struct Image<'vk_core> {
	vk_core: &'vk_core VkCore,
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

pub struct MemoryAllocator<'vk_core> {
	vk_core: &'vk_core VkCore,
	memory_properties: vk::MemoryPropertyFlags,
	buffer: Option<Buffer<'vk_core>>,
	images: Vec<(Image<'vk_core>, vk::ImageViewType, vk::ComponentMapping)>,
}

/// If T is Rw, memory properties always contain HOST_VISIBLE and HOST_COHERENT.
/// If T is NonRw, memory properties may contain HOST_VISIBLE and/or HOST_COHERENT.
/// NonRw doesn't means that buffer data is immutable but that buffer data can't be accessed by CPU.
/// Memory, Buffer and Images have same lifetime because they must be created by the same Device.
pub struct MemoryBlock<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::DeviceMemory,
	buffer: Option<Buffer<'vk_core>>,
	images: Vec<Image<'vk_core>>,
}

pub struct MemoryAccessor<'memory, 'vk_core> {
	buffer: &'memory mut [u8],
	seeker: u64,
	memory: &'memory MemoryBlock<'vk_core>,
}

#[derive(Debug)]
pub enum MemoryTypeError {
	NotFound,
}

impl<'vk_core> Buffer<'vk_core> {
	pub unsafe fn uninitialized(
		vk_core: &'vk_core VkCore,
		size: vk::DeviceSize,
		usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
	) -> Result<Self, vk::Result> {
		let raw_handle = vk_core.device.create_buffer(
			&vk::BufferCreateInfo {
				s_type: StructureType::BUFFER_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::BufferCreateFlags::empty(),
				size,
				usage,
				sharing_mode,
				queue_family_index_count: vk_core.physical_device.queue_family_index_count(),
				p_queue_family_indices: vk_core.physical_device.queue_family_index_ptr(),
			},
			None,
		)?;

		Ok(
			Self {
				vk_core,
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

impl Drop for Buffer<'_> {
	fn drop(&mut self) {
		unsafe {
			self.vk_core.device.destroy_buffer(self.raw_handle, None);
		}
	}
}

impl<'vk_core> Image<'vk_core> {
	pub unsafe fn uninitialized(
		vk_core: &'vk_core VkCore,
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
		let image = vk_core.device
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
					queue_family_index_count: vk_core.physical_device.queue_family_index_count(),
					p_queue_family_indices: vk_core.physical_device.queue_family_index_ptr(),
				},
				None,
			)
			.unwrap();

		Self {
			vk_core,
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
		unimplemented!()
	}
}

impl Drop for Image<'_> {
	fn drop(&mut self) {
		unsafe {
			self.vk_core.device.destroy_image(self.raw_handle, None);
			if self.view != vk::ImageView::null() {
				self.vk_core.device.destroy_image_view(self.view, None);
			}
		}
	}
}

impl<'vk_core> MemoryAllocator<'vk_core> {
	pub fn bind_buffer(&mut self, buffer: Buffer<'vk_core>) -> &mut Self {
		debug_assert!(self.buffer.is_none());
		self.buffer = Some(buffer);
		self
	}

	pub fn bind_image(
		&mut self,
		image: Image<'vk_core>,
		view_type: vk::ImageViewType,
		component_mapping: vk::ComponentMapping,
	) -> &mut Self {
		self.images.push((image, view_type, component_mapping));
		self
	}

	/// This method is unsafe because inner data is uninitialized.
	pub unsafe fn allocate(self) -> Result<MemoryBlock<'vk_core>, Box<dyn Error>> {
		let obj_num = self.images.len() + if self.buffer.is_some() { 1 } else { 0 };
		debug_assert!(obj_num > 0);
		let mut memory_requirements = Vec::with_capacity(obj_num);
		self.buffer.as_ref().map(|buffer| {
			memory_requirements.push(
				self.vk_core.device.get_buffer_memory_requirements(buffer.raw_handle)
			);
		});

		self.images.iter().for_each(|(image, _, _)| {
			memory_requirements.push(
				self.vk_core.device.get_image_memory_requirements(image.raw_handle)
			);
		});

		let (allocation_size, offsets, required_memory_type) = memory_requirements
			.into_iter()
			.fold(
				(0, Vec::with_capacity(obj_num), !0),
				|(alloc_size, mut offsets, req_mem_ty), mem_req| {
					dbg!(mem_req);
					offsets.push(alloc_size);
					(alloc_size + mem_req.size, offsets, req_mem_ty & mem_req.memory_type_bits)
				}
			);

		let memory = self.vk_core.device.allocate_memory(
			&vk::MemoryAllocateInfo {
				s_type: StructureType::MEMORY_ALLOCATE_INFO,
				p_next: ptr::null(),
				allocation_size,
				memory_type_index: find_memory_type_index(
					&self.vk_core.physical_device,
					required_memory_type,
					self.memory_properties,
				)?,
			},
			None,
		)?;

		let mut offset_index = 0;
		for buffer in self.buffer.iter() {
			self.vk_core.device.bind_buffer_memory(buffer.raw_handle, memory, 0)?;
			offset_index += 1;
		}

		for (image, _, _) in self.images.iter() {
			self.vk_core.device.bind_image_memory(image.raw_handle, memory, offsets[offset_index])?;
			offset_index += 1;
		}

		let mut images = Vec::with_capacity(self.images.len());
		for (mut image, view_type, component_mapping) in self.images.into_iter() {
			let image_view = self.vk_core.device.create_image_view(
				&vk::ImageViewCreateInfo {
					s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::ImageViewCreateFlags::empty(),
					image: image.raw_handle,
					format: image.format,
					subresource_range: vk::ImageSubresourceRange {
						aspect_mask: image.aspect_mask,
						base_mip_level: 0,
						level_count: image.mip_levels,
						base_array_layer: 0,
						layer_count: image.array_layers,
					},
					view_type,
					components: component_mapping,
				},
				None,
			)?;

			debug_assert_ne!(image.view, vk::ImageView::null());
			image.view = image_view;
			images.push(image);
		}

		Ok(
			MemoryBlock {
				vk_core: self.vk_core,
				raw_handle: memory,
				buffer: self.buffer,
				images,
			}
		)
	}
}

impl<'vk_core> MemoryBlock<'vk_core> {
	pub fn allocator(
		vk_core: &'vk_core VkCore,
		memory_properties: vk::MemoryPropertyFlags,
	) -> MemoryAllocator<'vk_core> {
		MemoryAllocator {
			vk_core,
			buffer: None,
			images: Vec::new(),
			memory_properties,
		}
	}

	pub fn accessor(&mut self) {
		unimplemented!()
	}
}

impl Drop for MemoryBlock<'_> {
	fn drop(&mut self) {
		unsafe {
			self.vk_core.device.free_memory(self.raw_handle, None);
		}
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