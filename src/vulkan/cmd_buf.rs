use ash::vk;
use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan::*;

use std::ptr;
use std::slice;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

struct Commandbuffers<'vk_core> {
	vk_core: &'vk_core VkCore,
	pool: vk::CommandPool,
	raw_buffers: Vec<vk::CommandBuffer>,
}

pub struct CommandRecorder<'vk_core, 'cmd_buf> {
	command_buffers: &'cmd_buf mut Commandbuffers<'vk_core>,
	index: usize,
}

/// The number of this struct's command buffers should be as much as that of swapchain's.
pub struct GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, T> {
	command_recorder: CommandRecorder<'vk_core, 'cmd_buf>,
	graphic: &'vk_graphic VkGraphic<'vk_core>,
	_marker: PhantomData<T>,
}

pub struct Uninitialized;
pub struct General;
pub struct Natural;
pub struct InRenderPass;
pub struct GuiPipeline;
pub struct End;

impl<'vk_core> Commandbuffers<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		cmd_pool_flags: vk::CommandPoolCreateFlags,
		cmd_buf_level: vk::CommandBufferLevel,
	) -> Result<Self, vk::Result> {
		unsafe {
			let command_pool = vk_core.device.create_command_pool(
				&vk::CommandPoolCreateInfo {
					s_type: StructureType::COMMAND_POOL_CREATE_INFO,
					p_next: ptr::null(),
					flags: cmd_pool_flags,
					queue_family_index: vk_core.physical_device.queue_family_index,
				},
				None,
			)?;

			let command_buffers = vk_core.device.allocate_command_buffers(
				&vk::CommandBufferAllocateInfo {
					s_type: StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
					p_next: ptr::null(),
					command_pool,
					level: cmd_buf_level,
					command_buffer_count: 1,
				}
			)?;

			Ok(
				Self {
					vk_core,
					pool: command_pool,
					raw_buffers: command_buffers,
				}
			)
		}
	}

	pub fn recorder<'cmd_buf>(
		&'cmd_buf mut self,
		index: usize,
		cmd_buf_usage: vk::CommandBufferUsageFlags,
	) -> Result<CommandRecorder<'vk_core, 'cmd_buf>, vk::Result> {
		unsafe {
			self.vk_core.device.begin_command_buffer(
				self.raw_buffers[index],
				&vk::CommandBufferBeginInfo {
					s_type: StructureType::COMMAND_BUFFER_BEGIN_INFO,
					p_next: ptr::null(),
					flags: cmd_buf_usage,
					p_inheritance_info: ptr::null(),
				},
			)?;

			Ok(CommandRecorder { command_buffers: self, index })
		}
	}

	pub fn graphic_recorder<'vk_graphic, 'cmd_buf>(
		&'cmd_buf mut self,
		vk_graphic: &'vk_graphic VkGraphic,
		index: usize,
		cmd_buf_usage: vk::CommandBufferUsageFlags,
	) -> Result<GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, General>, vk::Result> {
		let cmd_rec = self.recorder(index, cmd_buf_usage)?;
		unimplemented!()
	}
}

impl Index<usize> for Commandbuffers {
	type Output = vk::CommandBuffer;
	fn index(&self, index: usize) -> &vk::CommandBuffer {
		&self.raw_buffers[index]
	}
}

impl IndexMut<usize> for Commandbuffers {
	fn index_mut(&mut self, index: usize) -> &mut vk::CommandBuffer {
		&mut self.raw_buffers[index]
	}
}

impl Drop for Commandbuffers<'_> {
	fn drop(&mut self) { unsafe { self.vk_core.device.destroy_command_pool(self.pool, None); } }
}

impl<'vk_core, 'cmd_buf> CommandRecorder<'vk_core, 'cmd_buf> {
	pub fn buffer_to_buffer(
		&mut self,
		src_buffer: &Buffer,
		dst_buffer: &Buffer,
		regions: &[vk::BufferCopy],
	) -> &mut Self {
		unsafe {
			self.command_buffers.vk_core.device.cmd_copy_buffer(
				self.command_buffers[self.index],
				src_buffer.raw_handle(),
				dst_buffer.raw_handle(),
				regions,
			)
		}
		self
	}

	pub fn buffer_to_image(
		&mut self,
		src_buffer: &Buffer,
		(dst_image, mip_level): (&Image, u32),
		regions: &[vk::BufferImageCopy],
	) -> &mut Self {
		unsafe {
			regions.iter()
				.for_each(|region| {
					self.command_buffers.vk_core.device.cmd_copy_buffer_to_image(
						self.command_buffers[self.index],
						src_buffer.raw_handle(),
						dst_image.raw_handle(),
						dst_image.layout(region.image_subresource.mip_level),
						slice::from_ref(region),
					);
				});
		}
		self
	}

	pub fn queue_submit(
		self,
		ref wait_dst_stage_mask: vk::PipelineStageFlags,
		wait_semaphores: &[vk::Semaphore],
		signal_semaphores: &[vk::Semaphore],
		&signal_fence: &vk::Fence,
	) -> Result<(), vk::Result> {
		unsafe {
			self.command_buffers.vk_core.device
				.end_command_buffer(self.command_buffers[self.index]);

			self.command_buffers.vk_core.device
				.queue_submit(
					self.command_buffers.vk_core.queue,
					&[vk::SubmitInfo {
						s_type: StructureType::SUBMIT_INFO,
						p_next: ptr::null(),
						command_buffer_count: 1,
						p_command_buffers: self.command_buffers[self.index] as *const _,
						p_wait_dst_stage_mask: wait_dst_stage_mask as *const _,
						wait_semaphore_count: wait_semaphores.len() as u32,
						p_wait_semaphores: wait_semaphores.as_ptr(),
						signal_semaphore_count: signal_semaphores.len() as u32,
						p_signal_semaphores: signal_semaphores.as_ptr(),
					}],
					signal_fence,
				)?;

			Ok(())
		}
	}
}

fn mip() {
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
}

impl Drop for CommandRecorder<'_, '_> {
	fn drop(&mut self) {
		unsafe {
			self.command_buffers.vk_core.device.destroy_command_pool(self.command_buffers.pool);
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