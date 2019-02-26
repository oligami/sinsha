use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan_api::PhysicalDevice;
use crate::vulkan_api::shaders::*;
use crate::vulkan_api::command_recorder::CommandRecorder;

use std::ptr;
use std::mem;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

pub trait Bytes {
	fn into_bytes(self) -> Vec<u8>;
	fn to_ref_bytes(&self) -> &Vec<u8>;
}

pub struct BuffersWithMemory<'device> {
	device: &'device Device,
	buffer: vk::Buffer,
	memory: vk::DeviceMemory,
	offsets: Vec<vk::DeviceSize>,
}

pub struct BufferDataInfo {
	byte_data: Vec<u8>,
}

pub struct Image {
	raw_handle: vk::Image,
	view: vk::ImageView,
	format: vk::Format,
	layout: Vec<vk::ImageLayout>,
	extent: vk::Extent3D,
	aspect_mask: vk::ImageAspectFlags,
	mip_levels: u32,
	array_layers: u32,
}

pub struct ImagesWithMemory<'device> {
	device: &'device Device,
	images: Vec<Image>,
	memory: vk::DeviceMemory,
}

pub struct ImagesIter<'device, 'item> {
	ptr: *const Image,
	end: *const Image,
	_marker: PhantomData<&'item ImagesWithMemory<'device>>,
}

pub struct ImagesIterMut<'device, 'item> {
	ptr: *mut Image,
	end: *mut Image,
	_marker: PhantomData<&'item mut ImagesWithMemory<'device>>,
}

pub struct Blit(vk::ImageBlit);


impl<'device> BuffersWithMemory<'device> {
	pub unsafe fn empty(
		physical_device: &PhysicalDevice,
		device: &'device Device,
		data_size: vk::DeviceSize,
		buffer_usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Self {
		let buffer = create_buffer(
			physical_device,
			device,
			data_size,
			buffer_usage,
			sharing_mode,
		);

		let requirements = device.get_buffer_memory_requirements(buffer);

		let memory = device
			.allocate_memory(
				&vk::MemoryAllocateInfo {
					s_type: StructureType::MEMORY_ALLOCATE_INFO,
					p_next: ptr::null(),
					allocation_size: requirements.size,
					memory_type_index: find_memory_type_index(
						physical_device,
						requirements.memory_type_bits,
						memory_properties,
					).unwrap(),
				},
				None,
			)
			.unwrap();

		device.bind_buffer_memory(buffer, memory, 0).unwrap();

		Self {
			device,
			buffer,
			memory,
			offsets: Vec::new(),
		}
	}

	pub fn visible_coherent<T>(
		physical_device: &PhysicalDevice,
		device: &'device Device,
		data_of_buffers: Vec<T>,
		buffer_usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Self where T: Bytes {
		let data_size = data_of_buffers
			.iter()
			.fold(0, |size, data_of_buffer| size + data_of_buffer.to_ref_bytes().len()) as _;

		let mut empty_buffer = unsafe {
			Self::empty(
				physical_device,
				device,
				data_size,
				buffer_usage,
				sharing_mode,
				memory_properties
					| vk::MemoryPropertyFlags::HOST_COHERENT
					| vk::MemoryPropertyFlags::HOST_COHERENT,
			)
		};

		unsafe {
			let access_to_memory = device
				.map_memory(empty_buffer.memory, 0, data_size, vk::MemoryMapFlags::empty())
				.unwrap() as *mut u8;
			data_of_buffers
				.into_iter()
				.fold(access_to_memory, |mem_addr, data_of_buffer| {
					let ref_bytes = data_of_buffer.to_ref_bytes();
					empty_buffer.offsets
						.push(empty_buffer.offsets.last().unwrap_or(&0) + ref_bytes.len() as u64);
					mem_addr.copy_from(ref_bytes.as_ptr(), ref_bytes.len());
					(mem_addr as usize + ref_bytes.len()) as *mut u8
				});
		}

		empty_buffer
	}

	pub fn device_local<T>(
		physical_device: &PhysicalDevice,
		device: &'device Device,
		data_of_buffers: Vec<T>,
		buffer_usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
		command_buffer: vk::CommandBuffer,
	) -> (Self, Self) where T: Bytes {
		// TODO: calculate size of buffer twice. one is below and the other is BuffersWithMemory::visible_coherent()
		let (size, offsets) = data_of_buffers
			.iter()
			.fold(
				(0, Vec::new()),
				|(offset, mut offsets), data_of_buffer| {
					offsets.push(offset);
					(offset + data_of_buffer.to_ref_bytes().len() as u64, offsets)
				}
			);

		let staging_buffer = BuffersWithMemory::visible_coherent(
			physical_device,
			device,
			data_of_buffers,
			vk::BufferUsageFlags::TRANSFER_SRC,
			sharing_mode,
			vk::MemoryPropertyFlags::empty(),
		);

		let mut device_local_buffer = unsafe {
			BuffersWithMemory::empty(
				physical_device,
				device,
				size,
				buffer_usage | vk::BufferUsageFlags::TRANSFER_DST,
				sharing_mode,
				vk::MemoryPropertyFlags::DEVICE_LOCAL,
			)
		};
		device_local_buffer.offsets = offsets;

		// transfer data to device local buffer from staging buffer.
		unsafe {
			device.cmd_copy_buffer(
				command_buffer,
				staging_buffer.buffer,
				device_local_buffer.buffer,
				&[
					vk::BufferCopy {
						src_offset: 0,
						dst_offset: 0,
						size,
					}
				],
			);
		}

		(device_local_buffer, staging_buffer)
	}

	pub fn raw_handle(&self) -> vk::Buffer {
		self.buffer
	}

	pub fn memory(&self) -> vk::DeviceMemory {
		self.memory
	}

	pub fn offsets(&self) -> &Vec<vk::DeviceSize> {
		&self.offsets
	}
}

impl Drop for BuffersWithMemory<'_> {
	fn drop(&mut self) {
		unsafe {
			self.device.destroy_buffer(self.buffer, None);
			self.device.free_memory(self.memory, None);
		}
	}
}


impl BufferDataInfo {
	pub fn new<T>(mut data: Vec<T>) -> Self {
		let size_per_element = mem::size_of::<T>();
		let byte_data;
		unsafe {
			byte_data = Vec::from_raw_parts(
				data.as_mut_ptr() as *mut u8,
				data.len() * size_per_element,
				data.capacity() * size_per_element,
			);

			// This operation is definitely needed because data inside of `data: Vec<T>` would be
			// lost by destructor of it running at the end of this function.
			mem::forget(data);
		};

		Self {
			byte_data,
		}
	}
}

impl Bytes for BufferDataInfo {
	fn into_bytes(self) -> Vec<u8> {
		self.byte_data
	}

	fn to_ref_bytes(&self) -> &Vec<u8> {
		&self.byte_data
	}
}


impl Image {
	/// vk::ImageView is null handle.
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

	/// This Image must have been bound to memory.
	pub fn attach_image_view(
		&mut self,
		device: &Device,
		view_type: vk::ImageViewType,
		component_mapping: vk::ComponentMapping,
	) {
		unsafe {
			debug_assert_ne!(self.view, vk::ImageView::null());
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

	/// Must be bound to memory.
	/// Image layout will be updated but actual layout is not updated yet.
	pub fn attach_barriers(
		&mut self,
		device: &Device,
		&command_buffer: &vk::CommandBuffer,
		mip_level_range: Range<u32>,
		new_layout: vk::ImageLayout,
		pipeline_barriers: (vk::PipelineStageFlags, vk::PipelineStageFlags),
		access_masks: (vk::AccessFlags, vk::AccessFlags),
	) {
		debug_assert!(mip_level_range.end <= self.mip_levels);
		let mip_level_range_idx = mip_level_range.start as usize..mip_level_range.end as usize;
		let old_layout = self.layout[mip_level_range_idx.start];
		self.layout[mip_level_range_idx]
			.iter_mut()
			.for_each(|layout| {
				debug_assert_eq!(old_layout, *layout);
				*layout = new_layout;
			});

		unsafe {
			device.cmd_pipeline_barrier(
				command_buffer,
				pipeline_barriers.0,
				pipeline_barriers.1,
				vk::DependencyFlags::BY_REGION,
				&[],
				&[],
				&[
					vk::ImageMemoryBarrier {
						s_type: StructureType::IMAGE_MEMORY_BARRIER,
						p_next: ptr::null(),
						image: self.raw_handle,
						old_layout,
						new_layout,
						src_access_mask: access_masks.0,
						dst_access_mask: access_masks.1,
						src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
						dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
						subresource_range: vk::ImageSubresourceRange {
							aspect_mask: self.aspect_mask,
							base_array_layer: 0,
							layer_count: self.array_layers,
							base_mip_level: mip_level_range.start,
							level_count: mip_level_range.end - mip_level_range.start,
						}
					}
				],
			)
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
}

impl<'device> ImagesWithMemory<'device> {
	pub unsafe fn uninitialized(
		physical_device: &PhysicalDevice,
		device: &'device Device,
		mut images: Vec<Image>,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Self {
		let memory_requirements: Vec<_> = images
			.iter()
			.map(|image| {
				device.get_image_memory_requirements(image.raw_handle)
			})
			.collect();

		let (allocation_size, required_memory_type) = memory_requirements
			.iter()
			.fold(
				(0, 0),
				|(alloc_size, mem_ty), requirement| {
					(alloc_size + requirement.size, mem_ty | requirement.memory_type_bits)
				}
			);

		let memory = device
			.allocate_memory(
				&vk::MemoryAllocateInfo {
					s_type: StructureType::MEMORY_ALLOCATE_INFO,
					p_next: ptr::null(),
					allocation_size,
					memory_type_index: find_memory_type_index(
						physical_device,
						required_memory_type,
						memory_properties,
					).unwrap(),
				},
				None,
			)
			.unwrap();

		images
			.iter_mut()
			.zip(memory_requirements.into_iter())
			.fold(0, |offset, (image, mem_req)| {
				device.bind_image_memory(image.raw_handle(), memory, offset).unwrap();
				offset + mem_req.size
			});

		Self {
			device,
			images,
			memory,
		}
	}

	#[inline]
	pub fn get(&self, idx: usize) -> &Image {
		&self.images[idx]
	}

	pub fn iter<'item>(&self) -> ImagesIter<'device, 'item> {
		ImagesIter {
			ptr: &self.images[0] as *const _,
			end: self.images.last().unwrap_or(&self.images[0]) as *const _,
			_marker: PhantomData::<&Self>,
		}
	}

	pub fn iter_mut<'item>(&mut self) -> ImagesIterMut<'device, 'item> {
		let ptr = &mut self.images[0] as *mut _;
		ImagesIterMut {
			ptr,
			end: self.images.last_mut().map(|mut_ref| mut_ref as *mut _).unwrap_or(ptr),
			_marker: PhantomData::<&mut Self>,
		}
	}
}

impl<'device> Index<usize> for ImagesWithMemory<'device> {
	type Output = Image;
	fn index(&self, index: usize) -> &Self::Output {
		&self.images[index]
	}
}

impl<'device> IndexMut<usize> for ImagesWithMemory<'device> {
	fn index_mut(&mut self, index: usize) -> &mut Image {
		&mut self.images[index]
	}
}

impl<'device, 'item> Iterator for ImagesIter<'device, 'item> {
	type Item = &'item Image;
	fn next(&mut self) -> Option<Self::Item> {
		unsafe {
			if self.ptr == self.end {
				None
			} else {
				self.ptr = (self.ptr as usize + mem::size_of::<Self::Item>()) as *const _;
				self.ptr.as_ref()
			}
		}
	}
}

impl<'device, 'item> Iterator for ImagesIterMut<'device, 'item> {
	type Item = &'item mut Image;
	fn next(&mut self) -> Option<Self::Item> {
		unsafe {
			if self.ptr == self.end {
				None
			} else {
				self.ptr = (self.ptr as usize + mem::size_of::<Self::Item>()) as *mut _;
				self.ptr.as_mut()
			}
		}
	}
}

#[inline]
unsafe fn create_buffer(
	physical_device: &PhysicalDevice,
	device: &Device,
	size: vk::DeviceSize,
	usage: vk::BufferUsageFlags,
	sharing_mode: vk::SharingMode,
) -> vk::Buffer {
	device
		.create_buffer(
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
		)
		.unwrap()
}


fn find_memory_type_index(
	physical_device: &PhysicalDevice,
	suitable_memory_type_bits: u32,
	required_memory_property_flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
	for i in 0..physical_device.memory_properties.memory_type_count {
		if suitable_memory_type_bits & 1 << i != 0
			&& physical_device.memory_properties.memory_types[i as usize].property_flags
			& required_memory_property_flags
			== required_memory_property_flags
		{
			return Some(i);
		}
	}
	None
}

