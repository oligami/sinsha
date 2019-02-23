use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan_api::PhysicalDevice;
use crate::vulkan_api::shaders::*;
use crate::vulkan_api::VkDestroy;

use std::ptr;
use std::mem;
use crate::vulkan_api::command_recorder::CommandRecorder;
use crate::vulkan_api::command_recorder::Natural;

pub struct BuffersWithMemory {
	buffer: vk::Buffer,
	memory: vk::DeviceMemory,
}

pub struct BufferDataInfo {
	byte_data: Vec<u8>,
}

pub struct Image {
	raw_handle: vk::Image,
	offset: vk::DeviceSize,
	size: vk::DeviceSize,
	view: vk::ImageView,
	format: vk::Format,
	layout: vk::ImageLayout,
	extent: vk::Extent3D,
	subresource_range: vk::ImageSubresourceRange,
}

pub struct ImageDataInfo {
	bytes: Vec<u8>,
	width: u32,
	height: u32,
}

pub struct ImagesWithMemory {
	images: Vec<Image>,
	memory: vk::DeviceMemory,
}

pub struct ImagesWithMemoryBuilder<'a> {
	physical_device: &'a PhysicalDevice,
	device: &'a Device,
	image_infos: Vec<ImageInfo>,
}

struct ImageInfo {
	image: Image,
	bytes: Vec<u8>,
	usage: vk::ImageUsageFlags,
	sharing_mode: vk::SharingMode,
	image_type: vk::ImageType,
	view_type: vk::ImageViewType,
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
		}
	}

	pub fn visible_coherent(
		physical_device: &PhysicalDevice,
		device: &Device,
		buffer_data_infos: Vec<BufferDataInfo>,
		buffer_usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Self {
		let buffer_data = buffer_data_infos
			.into_iter()
			.fold(Vec::new(), |mut buffer_data, mut info| {
				buffer_data.append(&mut info.byte_data);
				buffer_data
			});

		let data_size = buffer_data.len() as vk::DeviceSize;

		let empty_buffer = unsafe {
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
			access_to_memory.copy_from(buffer_data.as_ptr(), buffer_data.len());
			device.unmap_memory(empty_buffer.memory);
		}

		empty_buffer
	}

	pub fn device_local(
		physical_device: &PhysicalDevice,
		device: &Device,
		data_infos: Vec<BufferDataInfo>,
		buffer_usage: vk::BufferUsageFlags,
		sharing_mode: vk::SharingMode,
		command_buffer: vk::CommandBuffer,
	) -> (Self, Self) {
		let size = data_infos
			.iter()
			.fold(0, |offset, data_info| {
				offset + data_info.byte_data.len() as vk::DeviceSize
			});

		let staging_buffer = BuffersWithMemory::visible_coherent(
			physical_device,
			device,
			data_infos,
			vk::BufferUsageFlags::TRANSFER_SRC,
			sharing_mode,
			vk::MemoryPropertyFlags::empty(),
		);

		let device_local_buffer = unsafe {
			BuffersWithMemory::empty(
				physical_device,
				device,
				size,
				buffer_usage | vk::BufferUsageFlags::TRANSFER_DST,
				sharing_mode,
				vk::MemoryPropertyFlags::DEVICE_LOCAL,
			)
		};

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

	pub fn buffer(&self) -> vk::Buffer {
		self.buffer
	}

	pub fn memory(&self) -> vk::DeviceMemory {
		self.memory
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


impl Image {
	/// vk::ImageView is null handle.
	pub unsafe fn new(
		physical_device: &PhysicalDevice,
		device: &Device,
		extent: vk::Extent3D,
		format: vk::Format,
		usage: vk::ImageUsageFlags,
		sharing_mode: vk::SharingMode,
		initial_layout: vk::ImageLayout,
		sample_count: vk::SampleCountFlags,
		subresource_range: vk::ImageSubresourceRange,
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
					mip_levels: subresource_range.level_count,
					array_layers: subresource_range.layer_count,
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
			layout: initial_layout,
			subresource_range,
			offset: 0,
			size: 0,
		}
	}

	pub fn raw_handle(&self) -> vk::Image {
		self.raw_handle
	}

	pub fn view(&self) -> vk::ImageView {
		self.view
	}

	pub fn layout(&self) -> vk::ImageLayout {
		self.layout
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

impl ImageDataInfo {
	pub unsafe fn new_uninit(bytes: Vec<u8>, width: u32, height: u32) -> Self {
		Self {
			bytes,
			width,
			height,
		}
	}

	pub fn new(path: &'static str) -> Self {
		let image = image_crate::open(path).unwrap().to_rgba();
		let (width, height) = image.dimensions();
		let bytes = image.into_raw();

		Self {
			bytes,
			width,
			height,
		}
	}
}

impl ImagesWithMemory {
	pub unsafe fn new(
		physical_device: &PhysicalDevice,
		device: &Device,
		mut images: Vec<Image>,
		memory_properties: vk::MemoryPropertyFlags,
	) -> Self {
		let (allocation_size, required_memory_type) = images
			.iter_mut()
			.fold(
				(0, 0),
				|(allocation_size, required_memory_type), image| {
					let requirements = device.get_image_memory_requirements(image.raw_handle);
					image.size = requirements.size;
					(
						allocation_size + requirements.size,
						required_memory_type | requirements.memory_type_bits
					)
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
			.fold(0, |offset, image| {
				device.bind_image_memory(image.raw_handle(), memory, offset).unwrap();
				image.offset = offset;
				offset + image.size
			});

		Self {
			images,
			memory,
		}
	}

	pub unsafe fn attach_image_views(
		&mut self,
		device: &Device,
		view_type: Vec<vk::ImageViewType>,
	) {
		debug_assert_eq!(self.images.len(), view_type.len());

		self.images
			.iter_mut()
			.zip(view_type.into_iter())
			.for_each(|(image, view_type)| {
				image.view = device
					.create_image_view(
						&vk::ImageViewCreateInfo {
							s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
							p_next: ptr::null(),
							flags: vk::ImageViewCreateFlags::empty(),
							image: image.raw_handle,
							format: image.format,
							components: vk::ComponentMapping {
								r: vk::ComponentSwizzle::IDENTITY,
								g: vk::ComponentSwizzle::IDENTITY,
								b: vk::ComponentSwizzle::IDENTITY,
								a: vk::ComponentSwizzle::IDENTITY,
							},
							subresource_range: image.subresource_range,
							view_type,
						},
						None,
					)
					.unwrap();
			});
	}

	pub fn attach_barriers(
		&mut self,
		device: &Device,
		idx: usize,
		new_layout: vk::ImageLayout,
		pipeline_barriers: (vk::PipelineStageFlags, vk::PipelineStageFlags),
		access_masks: (vk::AccessFlags, vk::AccessFlags),
		command_buffer: vk::CommandBuffer,
	) {
		unsafe {
			device
				.cmd_pipeline_barrier(
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
							image: self.get(idx).raw_handle,
							old_layout: self.get(idx).layout,
							new_layout,
							src_access_mask: access_masks.0,
							dst_access_mask: access_masks.1,
							src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
							dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
							subresource_range: self.get(idx).subresource_range,
						}
					],
				);

			self.images[idx].layout = new_layout;
		}
	}

	pub fn attach_barriers_all(
		&mut self,
		device: &Device,
		new_layout: vk::ImageLayout,
		pipeline_barriers: (vk::PipelineStageFlags, vk::PipelineStageFlags),
		access_masks: (vk::AccessFlags, vk::AccessFlags),
		command_buffer: vk::CommandBuffer,
	) {
		let image_barriers: Vec<_> = self.images
			.iter_mut()
			.map(|mut image| {
				let image_barrier = vk::ImageMemoryBarrier {
					s_type: StructureType::IMAGE_MEMORY_BARRIER,
					p_next: ptr::null(),
					image: image.raw_handle,
					old_layout: image.layout,
					new_layout,
					src_access_mask: access_masks.0,
					dst_access_mask: access_masks.1,
					src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
					dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
					subresource_range: image.subresource_range,
				};
				image.layout = new_layout;

				image_barrier
			})
			.collect();

		unsafe {
			device.cmd_pipeline_barrier(
				command_buffer,
				pipeline_barriers.0,
				pipeline_barriers.1,
				vk::DependencyFlags::BY_REGION,
				&[],
				&[],
				&image_barriers[..],
			)
		}
	}

	pub fn init_data(
		&self,
		image_data_infos: Vec<ImageDataInfo>,
		command_recorder: CommandRecorder<Natural>,
	) -> (CommandRecorder<Natural>, BuffersWithMemory) {
		unimplemented!()
	}

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
						Image::new(
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
							vk::ImageSubresourceRange {
								aspect_mask: vk::ImageAspectFlags::COLOR,
								layer_count: 1,
								base_array_layer: 0,
								level_count: 1,
								base_mip_level: 0,
							},
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
			ImagesWithMemory::new(
				physical_device,
				device,
				images,
				vk::MemoryPropertyFlags::DEVICE_LOCAL,
			)
		};

		unsafe {
			textures.attach_image_views(
				device,
				vec![vk::ImageViewType::TYPE_2D; textures.images.len()]
			);

			textures.attach_barriers_all(
				device,
				vk::ImageLayout::TRANSFER_DST_OPTIMAL,
				(vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER),
				(vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE),
				command_buffer,
			);

			textures.images
				.iter()
				.zip(offsets.into_iter())
				.for_each(|(image, offset)| {
					device.cmd_copy_buffer_to_image(
						command_buffer,
						staging_buffer.buffer,
						image.raw_handle,
						image.layout,
						&[
							vk::BufferImageCopy {
								buffer_offset: offset,
								buffer_row_length: 0, // represents no padding bytes
								buffer_image_height: 0, // represents no padding bytes
								image_offset: vk::Offset3D {
									x: 0,
									y: 0,
									z: 0,
								},
								image_extent: image.extent,
								image_subresource: vk::ImageSubresourceLayers {
									aspect_mask: vk::ImageAspectFlags::COLOR,
									mip_level: 0,
									layer_count: 1,
									base_array_layer: 0,
								}
							}
						],
					)
				});

			textures.attach_barriers_all(
				device,
				vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
				(
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::FRAGMENT_SHADER,
				),
				(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::SHADER_READ),
				command_buffer,
			)
		}

		(textures, staging_buffer)
	}

	#[inline]
	pub fn get(&self, idx: usize) -> &Image {
		&self.images[idx]
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

impl<'a> ImagesWithMemoryBuilder<'a> {
	pub fn start(physical_device: &'a PhysicalDevice, device: &'a Device) -> Self {
		Self {
			physical_device,
			device,
			image_infos: Vec::new(),
		}
	}
}

impl ImagesWithMemoryBuilder<'_> {
	pub fn next(
		mut self,
		image_data_info: ImageDataInfo,
		format: vk::Format,
		usage: vk::ImageUsageFlags,
		sharing_mode: vk::SharingMode,
		initial_layout: vk::ImageLayout,
		sample_count: vk::SampleCountFlags,
		subresource_range: vk::ImageSubresourceRange,
		image_type: vk::ImageType,
		view_type: vk::ImageViewType,
	) -> Self {
		self.image_infos.push(
			ImageInfo {
				image: unsafe {
					Image::new(
						self.physical_device,
						self.device,
						vk::Extent3D {
							width: image_data_info.width,
							height: image_data_info.height,
							depth: 1,
						},
						format,
						usage,
						sharing_mode,
						initial_layout,
						sample_count,
						subresource_range,
					)
				},
				usage,
				sharing_mode,
				image_type,
				view_type,
				bytes: image_data_info.bytes,
			}
		);

		self
	}

	pub fn next_same_settings(mut self, image_data_info: ImageDataInfo) -> Self {
		let pre_image_info = self.image_infos.last().unwrap();
		self.image_infos.push(
			ImageInfo {
				image: unsafe {
					Image::new(
						self.physical_device,
						self.device,
						vk::Extent3D {
							width: image_data_info.width,
							height: image_data_info.height,
							depth: 1,
						},
						pre_image_info.image.format,
						pre_image_info.usage,
						pre_image_info.sharing_mode,
						pre_image_info.image.layout,
						pre_image_info.image.sample_count,
						pre_image_info.image.subresource_range,
					)
				},
				usage: pre_image_info.usage,
				sharing_mode: pre_image_info.sharing_mode,
				image_type: pre_image_info.image_type,
				view_type: pre_image_info.view_type,
				bytes: image_data_info.bytes,
			}
		);

		self
	}

	pub fn build(self, memory_properties: vk::MemoryPropertyFlags) -> ImagesWithMemory {
		unimplemented!()
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

	// TODO: generic or shader specific and create descriptor sets at the same time.
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

