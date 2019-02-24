use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan_api::PhysicalDevice;
use crate::vulkan_api::shaders::*;
use crate::vulkan_api::command_recorder::CommandRecorder;
use crate::vulkan_api::VkDestroy;

use std::ptr;
use std::mem;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

pub trait Bytes {
	fn into_bytes(self) -> Vec<u8>;
	fn to_ref_bytes(&self) -> &Vec<u8>;
}

pub struct BuffersWithMemory {
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

pub struct ImagesWithMemory {
	images: Vec<Image>,
	memory: vk::DeviceMemory,
}

pub struct ImagesIter<'a> {
	ptr: *const Image,
	end: *const Image,
	_marker: PhantomData<&'a ImagesWithMemory>,
}

pub struct ImagesIterMut<'a> {
	ptr: *mut Image,
	end: *mut Image,
	_marker: PhantomData<&'a mut ImagesWithMemory>,
}

pub struct ResourceLoader<'a> {
	physical_device: &'a PhysicalDevice,
	device: &'a Device,
	shaders: &'a Shaders,
	queue: &'a vk::Queue,
	command_pool: vk::CommandPool,
	command_buffer: vk::CommandBuffer,
	fence: vk::Fence,
	staging_buffers: Vec<BuffersWithMemory>,
	device_local_buffers: Vec<BuffersWithMemory>,
	images: Vec<ImagesWithMemory>,
}

pub struct Blit(vk::ImageBlit);


impl BuffersWithMemory {
	pub unsafe fn empty(
		physical_device: &PhysicalDevice,
		device: &Device,
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
			buffer,
			memory,
			offsets: Vec::new(),
		}
	}

	pub fn visible_coherent<T>(
		physical_device: &PhysicalDevice,
		device: &Device,
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
		device: &Device,
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

impl VkDestroy for BuffersWithMemory {
	fn destroy(self, device: &Device) {
		unsafe {
			device.destroy_buffer(self.buffer, None);
			device.free_memory(self.memory, None);
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
					p_queue_family_indices: &physical_device.queue_family_idx as *const _,
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

impl VkDestroy for Image {
	fn destroy(self, device: &Device) {
		unsafe {
			device.destroy_image(self.raw_handle, None);
			device.destroy_image_view(self.view, None);
		}
	}
}

impl ImagesWithMemory {
	pub unsafe fn uninitialized(
		physical_device: &PhysicalDevice,
		device: &Device,
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
			images,
			memory,
		}
	}

	#[deprecated]
	pub fn textures(
		physical_device: &PhysicalDevice,
		device: &Device,
		pathes: Vec<&'static str>,
		sharing_mode: vk::SharingMode,
		command_buffer: vk::CommandBuffer,
	) -> (Self, BuffersWithMemory) {
		let cap_size = pathes.len();
		let (images, data, offsets, _) = pathes
			.into_iter()
			.fold(
				(
					Vec::with_capacity(cap_size),
					Vec::with_capacity(cap_size),
					Vec::with_capacity(cap_size),
					0
				),
				|(mut images, mut data, mut offsets, offset), path| {
					let image = image_crate::open(path).unwrap().to_rgba();
					let (width, height) = image.dimensions();
					let mut image_data = image.into_raw();

					let image = unsafe {
						Image::uninitialized(
							physical_device,
							device,
							vk::Extent3D {
								width,
								height,
								depth: 1,
							},
							vk::Format::R8G8B8A8_UNORM,
							vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
							sharing_mode,
							vk::ImageLayout::UNDEFINED,
							vk::SampleCountFlags::TYPE_1,
							vk::ImageAspectFlags::COLOR,
							1,
							1,
							vk::ImageType::TYPE_2D,
						)
					};

					images.push(image);
					offsets.push(offset);
					let offset = offset + image_data.len() as vk::DeviceSize;
					data.append(&mut image_data);

					(images, data, offsets, offset)
				}
			);

		let staging_buffer = BuffersWithMemory::visible_coherent(
			physical_device,
			device,
			vec![BufferDataInfo::new(data)],
			vk::BufferUsageFlags::TRANSFER_SRC,
			vk::SharingMode::EXCLUSIVE,
			vk::MemoryPropertyFlags::empty(),
		);

		let mut textures = unsafe {
			ImagesWithMemory::uninitialized(
				physical_device,
				device,
				images,
				vk::MemoryPropertyFlags::DEVICE_LOCAL,
			)
		};

		(textures, staging_buffer)
	}

	#[inline]
	pub fn get(&self, idx: usize) -> &Image {
		&self.images[idx]
	}

	pub fn iter<'a>(&self) -> ImagesIter<'a> {
		ImagesIter {
			ptr: &self.images[0] as *const _,
			end: self.images.last().unwrap_or(&self.images[0]) as *const _,
			_marker: PhantomData::<&'a Self>,
		}
	}

	pub fn iter_mut(&mut self) -> ImagesIterMut {
		let ptr = &mut self.images[0] as *mut _;
		ImagesIterMut {
			ptr,
			end: self.images.last_mut().map(|mut_ref| mut_ref as *mut _).unwrap_or(ptr),
			_marker: PhantomData::<&mut Self>,
		}
	}
}

impl VkDestroy for ImagesWithMemory {
	fn destroy(self, device: &Device) {
		unsafe {
			device.free_memory(self.memory, None);
			self.images
				.into_iter()
				.for_each(|image| {
					image.destroy(device);
				});
		}
	}
}

impl Index<usize> for ImagesWithMemory {
	type Output = Image;
	fn index(&self, index: usize) -> &Self::Output {
		&self.images[index]
	}
}

impl IndexMut<usize> for ImagesWithMemory {
	fn index_mut(&mut self, index: usize) -> &mut Image {
		&mut self.images[index]
	}
}

impl<'a> Iterator for ImagesIter<'a> {
	type Item = &'a Image;
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

impl<'a> Iterator for ImagesIterMut<'a> {
	type Item = &'a mut Image;
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

impl<'a> ResourceLoader<'a> {
	pub fn new(
		physical_device: &'a PhysicalDevice,
		device: &'a Device,
		queue: &'a vk::Queue,
		shaders: &'a Shaders,
	) -> Self {
		let command_pool = unsafe {
			device
				.create_command_pool(
					&vk::CommandPoolCreateInfo {
						s_type: StructureType::COMMAND_POOL_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::CommandPoolCreateFlags::TRANSIENT,
						queue_family_index: physical_device.queue_family_idx,
					},
					None,
				)
				.unwrap()
		};

		let command_buffer = unsafe {
			device
				.allocate_command_buffers(
					&vk::CommandBufferAllocateInfo {
						s_type: StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
						p_next: ptr::null(),
						level: vk::CommandBufferLevel::PRIMARY,
						command_pool,
						command_buffer_count: 1,
					}
				)
				.unwrap()[0]
		};

		unsafe {
			device
				.begin_command_buffer(
					command_buffer,
					&vk::CommandBufferBeginInfo {
						s_type: StructureType::COMMAND_BUFFER_BEGIN_INFO,
						p_next: ptr::null(),
						flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
						p_inheritance_info: ptr::null(),
					},
				)
				.unwrap();
		}

		let fence = unsafe {
			device
				.create_fence(
					&vk::FenceCreateInfo {
						s_type: StructureType::FENCE_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::FenceCreateFlags::empty(),
					},
					None,
				)
				.unwrap()
		};

		Self {
			physical_device,
			device,
			queue,
			shaders,
			command_pool,
			command_buffer,
			fence,
			staging_buffers: Vec::new(),
			device_local_buffers: Vec::new(),
			images: Vec::new(),
		}
	}
}

impl ResourceLoader<'_> {
	pub fn device_local_buffer(
		mut self,
		data_infos: Vec<BufferDataInfo>,
		buffer_usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
	) -> Self {
		let (device_local_buffer, staging_buffer) = BuffersWithMemory::device_local(
			self.physical_device,
			self.device,
			data_infos,
			buffer_usage,
			sharing_mode,
			self.command_buffer,
		);

		self.device_local_buffers.push(device_local_buffer);
		self.staging_buffers.push(staging_buffer);

		self
	}

	pub fn image_for_texture(
		mut self,
		pathes: Vec<&'static str>,
		sharing_mode: vk::SharingMode,
	) -> Self {
		let (images, staging_buffer) = ImagesWithMemory::textures(
			self.physical_device,
			self.device,
			pathes,
			sharing_mode,
			self.command_buffer,
		);

		self.images.push(images);
		self.staging_buffers.push(staging_buffer);

		self
	}

	pub fn execute(self) -> Self {
		unsafe {
			self.device.end_command_buffer(self.command_buffer).unwrap();

			self.device
				.queue_submit(
					*self.queue,
					&[
						vk::SubmitInfo {
							s_type: StructureType::SUBMIT_INFO,
							p_next: ptr::null(),
							command_buffer_count: 1,
							p_command_buffers: &self.command_buffer as *const _,
							wait_semaphore_count: 0,
							p_wait_semaphores: ptr::null(),
							signal_semaphore_count: 0,
							p_signal_semaphores: ptr::null(),
							p_wait_dst_stage_mask: ptr::null(),
						}
					],
					self.fence,
				)
				.unwrap();
		}

		self
	}

	pub fn finish(mut self) -> (Vec<BuffersWithMemory>, Vec<ImagesWithMemory>) {
		unsafe {
			self.device.wait_for_fences(&[self.fence], true, !0_u64).unwrap();

			self.device.destroy_fence(self.fence, None);
			self.device.destroy_command_pool(self.command_pool, None);

			let mut staging_buffers = Vec::new();
			mem::swap(&mut staging_buffers, &mut self.staging_buffers);
			staging_buffers
				.into_iter()
				.for_each(|staging_buffer| staging_buffer.destroy(self.device));
		}

		(self.device_local_buffers, self.images)
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
				p_queue_family_indices: &physical_device.queue_family_idx as *const _,
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

