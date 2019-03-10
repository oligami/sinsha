use ash::vk;
use ash::version::DeviceV1_0;
use ash::vk::StructureType;

use crate::vulkan::*;

use std::ptr;
use std::mem;
use std::ops;
use std::slice;
use std::marker::PhantomData;

pub struct CommandBuffers<'vk_core> {
	vk_core: &'vk_core VkCore,
	pool: vk::CommandPool,
	raw_buffers: Vec<vk::CommandBuffer>,
}

pub struct CommandRecorder<'vk_core, 'cmd_buf> {
	command_buffers: &'cmd_buf mut CommandBuffers<'vk_core>,
	index: usize,
}

pub struct InRenderPass;
pub struct Gui;

/// The number of this struct's command buffers should be as much as that of swapchain's.
pub struct GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, T> {
	command_recorder: CommandRecorder<'vk_core, 'cmd_buf>,
	vk_graphic: &'vk_graphic VkGraphic<'vk_core>,
	_marker: PhantomData<T>,
}

impl<'vk_core> CommandBuffers<'vk_core> {
	pub fn new(
		vk_core: &'vk_core VkCore,
		cmd_pool_flags: vk::CommandPoolCreateFlags,
		cmd_buf_level: vk::CommandBufferLevel,
		buffer_count: u32,
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
					command_buffer_count: buffer_count,
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

	pub fn queue_submit(
		&self,
		index: usize,
		ref wait_dst_stage_mask: vk::PipelineStageFlags,
		wait_semaphores: &[VkSemaphore],
		signal_semaphores: &[VkSemaphore],
		signal_fence: Option<&VkFence<'vk_core>>,
	) -> Result<(), vk::Result> {
		unsafe {
			self.vk_core.device.end_command_buffer(self[index])?;

			self.vk_core.device
				.queue_submit(
					self.vk_core.queue,
					&[vk::SubmitInfo {
						s_type: StructureType::SUBMIT_INFO,
						p_next: ptr::null(),
						command_buffer_count: 1,
						p_command_buffers: ops::Index::index(self, index) as _,
						p_wait_dst_stage_mask: wait_dst_stage_mask as *const _,
						wait_semaphore_count: wait_semaphores.len() as u32,
						p_wait_semaphores: wait_semaphores.as_ptr() as *const _,
						signal_semaphore_count: signal_semaphores.len() as u32,
						p_signal_semaphores: signal_semaphores.as_ptr() as *const _,
					}],
					signal_fence.map(|f| f.raw_handle).unwrap_or(vk::Fence::null()),
				)?;

			Ok(())
		}
	}
}

impl ops::Index<usize> for CommandBuffers<'_> {
	type Output = vk::CommandBuffer;
	fn index(&self, index: usize) -> &vk::CommandBuffer { &self.raw_buffers[index] }
}

impl ops::IndexMut<usize> for CommandBuffers<'_> {
	fn index_mut(&mut self, index: usize) -> &mut vk::CommandBuffer { &mut self.raw_buffers[index] }
}

impl Drop for CommandBuffers<'_> {
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
		image_barriers: &mut [ImageMemoryBarrier],
	) -> &mut Self {
		unsafe {
			let image_barriers: Vec<_> = image_barriers
				.into_iter()
				.map(|image_barrier| {
					let barrier = image_barrier.vk();
					let image = image_barrier.image_mut();
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

	pub fn blit_image<SalR, DalR, SRx, SRy, SRz, DRx, DRy, DRz>(
		&mut self,
		(src_image, dst_image): (&Image, &Image),
		(src_mip_level, dst_mip_level): (u32, u32),
		(src_array_layer_range, dst_array_layer_range): (SalR, DalR),
		(src_extent, dst_extent): ((SRx, SRy, SRz), (DRx, DRy, DRz)),
		filter: vk::Filter,
	) -> &mut Self
		where SalR: ops::RangeBounds<u32>,
			  DalR: ops::RangeBounds<u32>,
			  SRx: ops::RangeBounds<u32>,
			  SRy: ops::RangeBounds<u32>,
			  SRz: ops::RangeBounds<u32>,
			  DRx: ops::RangeBounds<u32>,
			  DRy: ops::RangeBounds<u32>,
			  DRz: ops::RangeBounds<u32>,
	{
		fn bound_to_range<B>(bound: B, max: u32) -> ops::Range<u32> where B: ops::RangeBounds<u32> {
			let start = match bound.start_bound() {
				ops::Bound::Included(&n) => n,
				ops::Bound::Excluded(&n) => n + 1,
				ops::Bound::Unbounded => 0,
			};

			let end = match bound.end_bound() {
				ops::Bound::Included(&n) => n + 1,
				ops::Bound::Excluded(&n) => n,
				ops::Bound::Unbounded => max,
			};

			start..end
		}

		fn bounds_to_offsets<Rx, Ry, Rz>(
			(bound_x, bound_y, bound_z): (Rx, Ry, Rz),
			extent: vk::Extent3D,
		) -> [vk::Offset3D; 2]
			where Rx: ops::RangeBounds<u32>,
				  Ry: ops::RangeBounds<u32>,
				  Rz: ops::RangeBounds<u32>
		{
			let range_x = bound_to_range(bound_x, extent.width);
			let range_y = bound_to_range(bound_y, extent.height);
			let range_z = bound_to_range(bound_z, extent.depth);

			[
				vk::Offset3D {
					x: range_x.start as _,
					y: range_y.start as _,
					z: range_z.start as _,
				},
				vk::Offset3D {
					x: range_x.end as _,
					y: range_y.end as _,
					z: range_z.end as _,
				}
			]
		};

		let src_array_layer_range = bound_to_range(src_array_layer_range, src_image.array_layers());
		let dst_array_layer_range = bound_to_range(dst_array_layer_range, dst_image.array_layers());

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
						src_offsets: bounds_to_offsets(src_extent, src_image.extent(src_mip_level)),
						src_subresource: vk::ImageSubresourceLayers {
							aspect_mask: src_image.aspect_mask(),
							base_array_layer: src_array_layer_range.start,
							layer_count: src_array_layer_range.end - src_array_layer_range.start,
							mip_level: src_mip_level,
						},
						dst_offsets: bounds_to_offsets(dst_extent, dst_image.extent(dst_mip_level)),
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

	pub fn into_graphic<'vk_graphic>(
		self,
		vk_graphic: &'vk_graphic VkGraphic<'vk_core>,
		clear_values: [[f32; 4]; 1]
	) -> GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, InRenderPass> {
		unsafe {
			self.command_buffers.vk_core.device
				.cmd_begin_render_pass(
					self.command_buffers[self.index],
					&vk::RenderPassBeginInfo {
						s_type: StructureType::RENDER_PASS_BEGIN_INFO,
						p_next: ptr::null(),
						render_pass: vk_graphic.render_pass,
						framebuffer: vk_graphic.framebuffers[self.index],
						render_area: vk::Rect2D {
							offset: vk::Offset2D { x: 0, y: 0 },
							extent: vk_graphic.swapchain.data.extent,
						},
						clear_value_count: clear_values.len() as _,
						p_clear_values: clear_values.as_ptr() as *const _,
					},
					vk::SubpassContents::INLINE,
				);

			GraphicCommandRecorder {
				command_recorder: self,
				vk_graphic,
				_marker: PhantomData::<InRenderPass>,
			}
		}
	}

	pub fn end(self) -> Result<(), vk::Result> {
		unsafe {
			self.command_buffers.vk_core.device
				.end_command_buffer(self.command_buffers[self.index])?;
			Ok(())
		}
	}
}

impl Drop for CommandRecorder<'_, '_> {
	fn drop(&mut self) {
		panic!("CommandRecorder must not be dropped while recording commands. Use end() method.")
	}
}

impl<'vk_core, 'vk_graphic, 'cmd_buf, T> GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, T> {
	fn transit<U>(self) -> GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, U> {
		let pre = mem::ManuallyDrop::new(self);
		GraphicCommandRecorder {
			command_recorder: pre.command_recorder,
			vk_graphic: pre.vk_graphic,
			_marker: PhantomData::<U>,
		}
	}

	pub fn bind_gui_pipeline(self) -> GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, Gui> {
		unsafe {
			self.vk_graphic.vk_core.device
				.cmd_bind_pipeline(
					self.command_recorder.command_buffers[self.command_recorder.index],
					vk::PipelineBindPoint::GRAPHICS,
					self.vk_graphic.shaders.gui.pipeline,
				);

			self.transit()
		}
	}
}

impl<'vk_core, 'vk_graphic, 'cmd_buf> GraphicCommandRecorder<'vk_core, 'vk_graphic, 'cmd_buf, Gui> {
	pub fn draw(&mut self, rect2d: &gui::Rect2D) -> &mut Self {
		unsafe {
			self.vk_graphic.vk_core.device
				.cmd_bind_descriptor_sets(
					self.command_recorder.command_buffers[self.command_recorder.index],
					vk::PipelineBindPoint::GRAPHICS,
					self.vk_graphic.shaders.gui.pipeline_layout,
					0,
					&[rect2d.texture()[self.command_recorder.index]],
					&[],
				);
			self.vk_graphic.vk_core.device
				.cmd_bind_vertex_buffers(
					self.command_recorder.command_buffers[self.command_recorder.index],
					0,
					&[rect2d.vertex_buffer().raw_handle()],
					&[rect2d.vertex_buffer().range().start],
				);
			self.vk_graphic.vk_core.device
				.cmd_draw(
					self.command_recorder.command_buffers[self.command_recorder.index],
					4, 1, rect2d.vertex_offset(), 0,
				)
		}

		self
	}
}

impl<T> Drop for GraphicCommandRecorder<'_, '_, '_, T> {
	fn drop(&mut self) {
		panic!(
			"GraphicCommandRecorder must not be dropped while recording commands.Use end() method."
		)
	}
}
