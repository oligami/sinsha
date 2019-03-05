use ash::vk;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan::*;

use std::ptr;
use std::mem;
use std::slice;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

struct Commandbuffers<'vk_core> {
	vk_core: &'vk_core VkCore,
	pool: vk::CommandPool,
	raw_buffers: Vec<vk::CommandBuffer>,
}

pub struct CommandRecorder<'vk_core, 'cmd_buf> {
	command_buffers: &'cmd_buf mut Commandbuffers<'vk_core>,
	index: usize,
}

pub struct General;

/// The number of this struct's command buffers should be as much as that of swapchain's.
pub struct GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, T> {
	command_recorder: CommandRecorder<'vk_core, 'cmd_buf>,
	graphic: &'vk_graphic VkGraphic<'vk_core>,
	_marker: PhantomData<T>,
}

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
		vk_graphic: &'vk_graphic VkGraphic<'vk_core>,
		index: usize,
		cmd_buf_usage: vk::CommandBufferUsageFlags,
	) -> Result<GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, General>, vk::Result> {
		let cmd_rec = self.recorder(index, cmd_buf_usage)?;
		unimplemented!()
	}
}

impl Index<usize> for Commandbuffers<'_> {
	type Output = vk::CommandBuffer;
	fn index(&self, index: usize) -> &vk::CommandBuffer { &self.raw_buffers[index] }
}

impl IndexMut<usize> for Commandbuffers<'_> {
	fn index_mut(&mut self, index: usize) -> &mut vk::CommandBuffer { &mut self.raw_buffers[index] }
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

	pub fn barriers(
		&mut self,
		stage: (vk::PipelineStageFlags, vk::PipelineStageFlags),
		buffer_barriers: &[vk::BufferMemoryBarrier],
		image_barriers: Vec<(&mut Image<'vk_core>, vk::ImageMemoryBarrier)>,
	) -> &mut Self {
		unsafe {
			let image_barriers: Vec<_> = image_barriers
				.into_iter()
				.map(|(image, barrier)| {
					let mip_level_start = barrier.subresource_range.base_mip_level;
					let mip_level_end = mip_level_start + barrier.subresource_range.level_count;
					for mip_level in mip_level_start..mip_level_end {
						image.transit_layout(mip_level, barrier.new_layout);
					}
					barrier
				})
				.collect();

			self.command_buffers.vk_core.device
				.cmd_pipeline_barrier(
					self.command_buffers[self.index],
					stage.0,
					stage.1,
					vk::DependencyFlags::BY_REGION,
					&[],
					&buffer_barriers[..],
					&image_barriers[..],
				);
		}
		self
	}

	pub fn blit_image(
		&mut self,
		(src_image, dst_image): (&Image<'vk_core>, &Image<'vk_core>),
		(src_mip_level, dst_mip_level): (u32, u32),
		(src_array_layer_range, dst_array_layer_range): (Range<u32>, Range<u32>),
		(src_extent, dst_extent): (Range<(i32, i32, i32)>, Range<(i32, i32, i32)>),
		filter: vk::Filter,
	) -> &mut Self {
		unsafe {
			let src_image_extent =
			self.command_buffers.vk_core.device
				.cmd_blit_image(
					self.command_buffers[self.index],
					src_image.raw_handle(),
					src_image.layout(src_mip_level),
					dst_image.raw_handle(),
					dst_image.layout(dst_mip_level),
					&[vk::ImageBlit {
						src_offsets: [
							vk::Offset3D {
								x: src_extent.start.0,
								y: src_extent.start.1,
								z: src_extent.start.2,
							},
							vk::Offset3D {
								x: src_extent.end.0,
								y: src_extent.end.1,
								z: src_extent.end.2,
							},
						],
						src_subresource: vk::ImageSubresourceLayers {
							aspect_mask: src_image.aspect_mask(),
							base_array_layer: src_array_layer_range.start,
							layer_count: src_array_layer_range.end - src_array_layer_range.start,
							mip_level: src_mip_level,
						},
						dst_offsets: [
							vk::Offset3D {
								x: dst_extent.start.0,
								y: dst_extent.start.1,
								z: dst_extent.start.2,
							},
							vk::Offset3D {
								x: dst_extent.end.0,
								y: dst_extent.end.1,
								z: dst_extent.end.2,
							},
						],
						dst_subresource: vk::ImageSubresourceLayers {
							aspect_mask: dst_image.aspect_mask(),
							base_array_layer: dst_array_layer_range.start,
							layer_count: dst_array_layer_range.end - dst_array_layer_range.start,
							mip_level: dst_mip_level,
						}
					}],
					filter,
				);
		}

		self
	}

	pub fn queue_submit(
		self,
		ref wait_dst_stage_mask: vk::PipelineStageFlags,
		wait_semaphores: &[VkSemaphore],
		signal_semaphores: &[VkSemaphore],
		signal_fence: &VkFence,
	) -> Result<(), vk::Result> {
		unsafe {
			self.command_buffers.vk_core.device
				.end_command_buffer(self.command_buffers[self.index])?;

			self.command_buffers.vk_core.device
				.queue_submit(
					self.command_buffers.vk_core.queue,
					&[vk::SubmitInfo {
						s_type: StructureType::SUBMIT_INFO,
						p_next: ptr::null(),
						command_buffer_count: 1,
						p_command_buffers: self.command_buffers.index(self.index) as *const _,
						p_wait_dst_stage_mask: wait_dst_stage_mask as *const _,
						wait_semaphore_count: wait_semaphores.len() as u32,
						p_wait_semaphores: wait_semaphores.as_ptr() as *const _,
						signal_semaphore_count: signal_semaphores.len() as u32,
						p_signal_semaphores: signal_semaphores.as_ptr() as *const _,
					}],
					signal_fence.raw_handle,
				)?;

			Ok(())
		}
	}
}

impl Drop for CommandRecorder<'_, '_> {
	fn drop(&mut self) {
		unsafe {
			self.command_buffers.vk_core.device
				.destroy_command_pool(self.command_buffers.pool, None);
		}
	}
}