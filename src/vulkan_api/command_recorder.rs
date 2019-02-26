use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan_api::*;
use crate::vulkan_api::shaders::gui::GuiDraw;

use std::ptr;
use std::marker::PhantomData;

pub struct CommandRecorder<'a, 'device, T> {
	physical_device: &'a PhysicalDevice,
	device: &'device Device,
	queue: &'a vk::Queue,
	command_buffer: vk::CommandBuffer,
	command_pool: vk::CommandPool,
	marker: PhantomData<T>,
}

pub struct GraphicCommandRecorder<'a, T> {
	device: &'a Device,
	queue: &'a vk::Queue,
	image_index: usize,
	command_buffers: &'a Vec<vk::CommandBuffer>,
	swapchain: &'a Swapchain,
	shaders: &'a Shaders,
	render_pass: &'a vk::RenderPass,
	framebuffers: &'a Vec<vk::Framebuffer>,
	marker: PhantomData<T>,
}

pub struct Uninitialized;
pub struct Natural;
pub struct InRenderPass;
pub struct GuiPipeline;
pub struct End;

impl<'a, 'device, T> CommandRecorder<'a, 'device, T> {
	fn transit_into<U>(self) -> CommandRecorder<'a, 'device, U> {
		CommandRecorder {
			physical_device: self.physical_device,
			device: self.device,
			queue: self.queue,
			command_pool: self.command_pool,
			command_buffer: self.command_buffer,
			marker: PhantomData::<U>,
		}
	}
}

impl<'a, 'device> CommandRecorder<'a, 'device, Uninitialized> {
	pub(in crate::vulkan_api) fn new(
		physical_device: &'a PhysicalDevice,
		device: &'device Device,
		queue: &'a vk::Queue,
	) -> Self {
		unsafe {
			let command_pool = device
				.create_command_pool(
					&vk::CommandPoolCreateInfo {
						s_type: StructureType::COMMAND_POOL_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::CommandPoolCreateFlags::empty(),
						queue_family_index: physical_device.queue_family_index,
					},
					None,
				)
				.unwrap();

			let command_buffer = device
				.allocate_command_buffers(
					&vk::CommandBufferAllocateInfo {
						s_type: StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
						p_next: ptr::null(),
						command_pool,
						command_buffer_count: 1,
						level: vk::CommandBufferLevel::PRIMARY,
					}
				)
				.unwrap()[0];

			Self {
				physical_device,
				device,
				queue,
				command_pool,
				command_buffer,
				marker: PhantomData::<Uninitialized>,
			}
		}
	}
}

impl<'a, 'device> CommandRecorder<'a, 'device, Uninitialized> {
	pub fn begin_recording(self) -> CommandRecorder<'a, 'device, Natural> {
		unsafe {
			self.device
				.begin_command_buffer(
					self.command_buffer,
					&vk::CommandBufferBeginInfo {
						s_type: StructureType::COMMAND_BUFFER_BEGIN_INFO,
						p_next: ptr::null(),
						flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
						p_inheritance_info: ptr::null(),
					},
				)
				.unwrap();
		}

		self.transit_into()
	}
}

impl<'a, 'device> CommandRecorder<'a, 'device, Natural> {
	pub fn transfer<T>(
		&self,
		src_buffer: &BufferWithMemory<T>,
		dst_buffer: &BufferWithMemory<NonRw>,
		region_infos: &[vk::BufferCopy],
	) {
		unsafe {
			self.device.cmd_copy_buffer(
				self.command_buffer,
				src_buffer.raw_handle(),
				dst_buffer.raw_handle(),
				region_infos,
			);
		}
	}

	pub fn buffer_to_image<T>(
		&self,
		src_buffer: &BufferWithMemory<T>,
		dst_image: &Image,
		region_infos: &[vk::BufferImageCopy],
	) {
		unsafe {
			self.device.cmd_copy_buffer_to_image(
				self.command_buffer,
				src_buffer.raw_handle(),
				dst_image.raw_handle(),
				dst_image.layout(region_infos[0].image_subresource.mip_level),
				region_infos,
			);
		}
	}

	#[deprecated]
	/// deprecated until impl data transfer.
	pub fn load_textures<'b>(
		&self,
		pathes: &[&'b str],
		mip_enable: &[bool],
	) -> (ImagesWithMemory, BufferWithMemory<Rw>) {
		debug_assert_eq!(pathes.len(), mip_enable.len());

		let (data_of_images, images): (Vec<_>, Vec<_>) = pathes
			.iter()
			.zip(mip_enable.iter())
			.map(|(path, &mip_enable)| {
				let image = image_crate::open(path).unwrap().to_rgba();
				let (width, height) = image.dimensions();
				let data = image.into_raw();

				let (image_usage, mip_levels) = if mip_enable {
					(
						vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC
							| vk::ImageUsageFlags::SAMPLED,
						Image::maximum_mip_level(width, height),
					)
				} else {
					(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED, 1)
				};

				let image = unsafe {
					Image::uninitialized(
						self.physical_device,
						self.device,
						vk::Extent3D {
							width,
							height,
							depth: 1,
						},
						vk::Format::R8G8B8A8_UNORM,
						image_usage,
						vk::SharingMode::EXCLUSIVE,
						vk::ImageLayout::UNDEFINED,
						vk::SampleCountFlags::TYPE_1,
						vk::ImageAspectFlags::COLOR,
						mip_levels,
						1,
						vk::ImageType::TYPE_2D,
					)
				};

				(BufferDataInfo::new(data), image)
			})
			.unzip();

		let staging_buffers = BufferWithMemory::visible_coherent(
			self.physical_device,
			self.device,
			data_of_images,
			vk::BufferUsageFlags::TRANSFER_SRC,
			vk::SharingMode::EXCLUSIVE,
			vk::MemoryPropertyFlags::empty(),
		);

		let mut images_with_memory = unsafe {
			ImagesWithMemory::uninitialized(
				self.physical_device,
				self.device,
				images,
				vk::MemoryPropertyFlags::DEVICE_LOCAL,
			)
		};

		images_with_memory
			.iter_mut()
			.for_each(|image| {
				image.attach_image_view(
					self.device,
					vk::ImageViewType::TYPE_2D,
					vk::ComponentMapping {
						r: vk::ComponentSwizzle::IDENTITY,
						g: vk::ComponentSwizzle::IDENTITY,
						b: vk::ComponentSwizzle::IDENTITY,
						a: vk::ComponentSwizzle::IDENTITY,
					},
				);

				image.attach_barriers(
					self.device,
					&self.command_buffer,
					0..image.mip_levels(),
					vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					(vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER),
					(vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE),
				);
			});

		//TODO: transfer buffer to image

		mip_enable
			.iter()
			.zip(images_with_memory.iter_mut())
			.take_while(|(mip_enable, image)| **mip_enable)
			.for_each(|(_mip_enable, image)| {
				unsafe {
					let base_extent = image.extent(0);
					let mut mip_width = base_extent.width as i32;
					let mut mip_height = base_extent.height as i32;

					for dst_mip_level in 1..image.mip_levels() {
						let src_mip_level = dst_mip_level - 1;

						image.attach_barriers(
							self.device,
							&self.command_buffer,
							(src_mip_level..dst_mip_level),
							vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
							(vk::PipelineStageFlags::empty(), vk::PipelineStageFlags::empty()),
							(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::TRANSFER_READ),
						);

						self.device.cmd_blit_image(
							self.command_buffer,
							image.raw_handle(),
							vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
							image.raw_handle(),
							vk::ImageLayout::TRANSFER_DST_OPTIMAL,
							&[vk::ImageBlit {
								src_offsets: [
									vk::Offset3D { x: 0, y: 0, z: 0 },
									vk::Offset3D { x: mip_width, y: mip_height, z: 1 },
								],
								src_subresource: vk::ImageSubresourceLayers {
									aspect_mask: image.aspect_mask(),
									base_array_layer: 0,
									layer_count: image.array_layers(),
									mip_level: src_mip_level,
								},
								dst_offsets: [
									vk::Offset3D { x: 0, y: 0, z: 0 },
									vk::Offset3D { x: mip_width / 2, y: mip_height / 2, z: 1 },
								],
								dst_subresource: vk::ImageSubresourceLayers {
									aspect_mask: image.aspect_mask(),
									base_array_layer: 0,
									layer_count: image.array_layers(),
									mip_level: dst_mip_level,
								}
							}],
							vk::Filter::LINEAR,
						);

						mip_width /= 2;
						mip_height /= 2;
					}
				}
			});

		images_with_memory
			.iter_mut()
			.for_each(|image| {
				let mip_levels = image.mip_levels();
				image.attach_barriers(
					self.device,
					&self.command_buffer,
					0..mip_levels,
					vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
					(vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER),
					(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::SHADER_READ),
				)
			});

		(images_with_memory, staging_buffers)
	}
}

impl<'a, 'device> CommandRecorder<'a, 'device, Natural> {
	pub fn end_recording(self) -> CommandRecorder<'a, 'device, End> {
		unsafe {
			self.device
				.end_command_buffer(self.command_buffer)
				.unwrap();
		}

		self.transit_into()
	}
}

impl<'a, 'device> CommandRecorder<'a, 'device, End> {
	pub fn submit(
		&self,
		wait_dst_stage_mask: vk::PipelineStageFlags,
		wait_semaphores: &[vk::Semaphore],
		signal_semaphores: &[vk::Semaphore],
		&fence: &vk::Fence,
	) {
		unsafe {
			self.device
				.queue_submit(
					*self.queue,
					&[
						vk::SubmitInfo {
							s_type: StructureType::SUBMIT_INFO,
							p_next: ptr::null(),
							command_buffer_count: 1,
							p_command_buffers: &self.command_buffer as *const _,
							wait_semaphore_count: wait_semaphores.len() as u32,
							p_wait_semaphores: wait_semaphores.as_ptr(),
							signal_semaphore_count: signal_semaphores.len() as u32,
							p_signal_semaphores: signal_semaphores.as_ptr(),
							p_wait_dst_stage_mask: &wait_dst_stage_mask as *const _,
						}
					],
					fence,
				)
				.unwrap();
		}
	}
}

impl<'a> GraphicCommandRecorder<'a, Uninitialized> {
	pub(in crate::vulkan_api) fn new(
		device: &'a Device,
		queue: &'a vk::Queue,
		image_idx: usize,
		command_buffers: &'a Vec<vk::CommandBuffer>,
		swapchain: &'a Swapchain,
		shaders: &'a Shaders,
		render_pass: &'a vk::RenderPass,
		framebuffers: &'a Vec<vk::Framebuffer>,
	) -> Self {
		Self {
			device,
			queue,
			image_index: image_idx,
			command_buffers,
			swapchain,
			shaders,
			render_pass,
			framebuffers,
			marker: PhantomData::<Uninitialized>,
		}
	}
}

impl<T> GraphicCommandRecorder<'_, T> {
	pub fn index(&self) -> usize {
		self.image_index
	}
}

impl<'a, T> GraphicCommandRecorder<'a, T> {
	fn transit_into<U>(self) -> GraphicCommandRecorder<'a, U> {
		GraphicCommandRecorder {
			device: self.device,
			queue: self.queue,
			image_index: self.image_index,
			command_buffers: self.command_buffers,
			swapchain: self.swapchain,
			shaders: self.shaders,
			render_pass: self.render_pass,
			framebuffers: self.framebuffers,
			marker: PhantomData::<U>,
		}
	}
}

impl<'a> GraphicCommandRecorder<'a, Uninitialized> {
	pub fn begin_recording(self) -> GraphicCommandRecorder<'a, Natural> {
		unsafe {
			self.device
				.begin_command_buffer(
					self.command_buffers[self.image_index],
					&vk::CommandBufferBeginInfo {
						s_type: StructureType::COMMAND_BUFFER_BEGIN_INFO,
						p_next: ptr::null(),
						flags: vk::CommandBufferUsageFlags::SIMULTANEOUS_USE,
						p_inheritance_info: ptr::null(),
					}
				)
				.unwrap();
		}

		self.transit_into()
	}
}

impl<'a> GraphicCommandRecorder<'a, Natural> {
	pub fn begin_render_pass(
		self, clear_values: &[vk::ClearValue]
	) -> GraphicCommandRecorder<'a, InRenderPass> {
		unsafe {
			self.device
				.cmd_begin_render_pass(
					self.command_buffers[self.image_index],
					&vk::RenderPassBeginInfo {
						s_type: StructureType::RENDER_PASS_BEGIN_INFO,
						p_next: ptr::null(),
						render_pass: *self.render_pass,
						framebuffer: self.framebuffers[self.image_index],
						render_area: vk::Rect2D {
							offset: vk::Offset2D { x: 0, y: 0 },
							extent: self.swapchain.data.extent,
						},
						clear_value_count: clear_values.len() as u32,
						p_clear_values: clear_values.as_ptr(),
					},
					vk::SubpassContents::INLINE,
				);
			self.device
				.cmd_set_viewport(
					self.command_buffers[self.image_index],
					0,
					&[
						vk::Viewport {
							x: 0_f32,
							y: 0_f32,
							width: self.swapchain.data.extent.width as _,
							height: self.swapchain.data.extent.height as _,
							min_depth: 0_f32,
							max_depth: 1_f32,
						},
					],
				);
			self.device
				.cmd_set_scissor(
					self.command_buffers[self.image_index],
					0,
					&[
						vk::Rect2D {
							offset: vk::Offset2D { x: 0, y: 0 },
							extent: self.swapchain.data.extent,
						},
					],
				);
		}

		self.transit_into()
	}
}

impl<'a> GraphicCommandRecorder<'a, InRenderPass> {
	pub fn enter_gui_pipeline(self) -> GraphicCommandRecorder<'a, GuiPipeline> {
		unsafe {
			self.device.cmd_bind_pipeline(
				self.command_buffers[self.image_index],
				vk::PipelineBindPoint::GRAPHICS,
				self.shaders.gui.pipeline,
			);
		}

		self.transit_into()
	}
}

impl<'a> GraphicCommandRecorder<'a, GuiPipeline> {
	pub fn draw<D>(self, draw_objects: &D) -> Self where D: GuiDraw {
		draw_objects.draw(
			self.device,
			&self.shaders.gui.pipeline_layout,
			self.command_buffers,
			&self.image_index,
		);
		self
	}

	pub fn quit_gui_pipeline(self) -> GraphicCommandRecorder<'a, InRenderPass> {
		self.transit_into()
	}
}

impl<'a> GraphicCommandRecorder<'a, InRenderPass> {
	pub fn end_render_pass(self) -> GraphicCommandRecorder<'a, Natural> {
		unsafe {
			self.device.cmd_end_render_pass(self.command_buffers[self.image_index]);
		}

		self.transit_into()
	}
}

impl<'a> GraphicCommandRecorder<'a, Natural> {
	pub fn end_recording(self) -> GraphicCommandRecorder<'a, End> {
		unsafe {
			self.device.end_command_buffer(self.command_buffers[self.image_index]).unwrap();
		}

		self.transit_into()
	}
}

impl GraphicCommandRecorder<'_, End> {
	pub fn submit(
		self,
		wait_dst_stage_mask: &[vk::PipelineStageFlags],
		wait_semaphores: &[vk::Semaphore],
		signal_semaphores: &[vk::Semaphore],
		fence: vk::Fence,
	) {
		unsafe {
			self.device
				.queue_submit(
					*self.queue,
					&[
						vk::SubmitInfo {
							s_type: StructureType::SUBMIT_INFO,
							p_next: ptr::null(),
							p_wait_dst_stage_mask: wait_dst_stage_mask.as_ptr(),
							wait_semaphore_count: wait_semaphores.len() as u32,
							p_wait_semaphores: wait_semaphores.as_ptr(),
							signal_semaphore_count: signal_semaphores.len() as u32,
							p_signal_semaphores: signal_semaphores.as_ptr(),
							command_buffer_count: 1,
							p_command_buffers: &self.command_buffers[self.image_index] as *const _,
						}
					],
					fence,
				)
				.unwrap();
		}
	}
}