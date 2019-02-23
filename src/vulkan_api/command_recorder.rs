use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan_api::*;
use crate::vulkan_api::shaders::gui::GuiDraw;

use std::ptr;
use std::marker::PhantomData;

pub struct CommandRecorder<'a, T> {
	device: &'a Device,
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

impl<'a, T> CommandRecorder<'a, T> {
	fn transit_into<U>(self) -> CommandRecorder<'a, U> {
		CommandRecorder {
			device: self.device,
			queue: self.queue,
			command_pool: self.command_pool,
			command_buffer: self.command_buffer,
			marker: PhantomData::<U>,
		}
	}
}

impl<'a> CommandRecorder<'a, Uninitialized> {
	pub(in crate::vulkan_api) fn new(
		physical_device: &PhysicalDevice,
		device: &'a Device,
		queue: &'a vk::Queue,
	) -> Self {
		unsafe {
			let command_pool = device
				.create_command_pool(
					&vk::CommandPoolCreateInfo {
						s_type: StructureType::COMMAND_POOL_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::CommandPoolCreateFlags::empty(),
						queue_family_index: physical_device.queue_family_idx,
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
				device,
				queue,
				command_pool,
				command_buffer,
				marker: PhantomData::<Uninitialized>,
			}
		}
	}
}

impl<'a> CommandRecorder<'a, Uninitialized> {
	pub fn begin_recording(self) -> CommandRecorder<'a, Natural> {
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

impl<'a> CommandRecorder<'a, Natural> {
	pub fn transfer(
		self,
		src_buffer: &BuffersWithMemory,
		dst_buffer: &BuffersWithMemory,
		region_infos: &[vk::BufferCopy],
	) -> Self {
		unsafe {
			self.device.cmd_copy_buffer(
				self.command_buffer,
				src_buffer.buffer(),
				dst_buffer.buffer(),
				region_infos,
			);
		}

		self
	}
}

impl<'a> CommandRecorder<'a, Natural> {
	pub fn end_recording(self) -> CommandRecorder<'a, End> {
		unsafe {
			self.device
				.end_command_buffer(self.command_buffer)
				.unwrap();
		}

		self.transit_into()
	}
}

impl<'a> CommandRecorder<'a, End> {
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

impl<T> VkDestroy for CommandRecorder<'_, T> {
	fn destroy(self, device: &Device) {
		unsafe {
			device.destroy_command_pool(self.command_pool, None);
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