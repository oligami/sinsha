use ash::version::DeviceV1_0;
use ash::vk;
use ash::vk::StructureType;
use ash::Device;
use std::ptr;

use crate::linear_algebra::{XY, RGBA};
use crate::vulkan_api::*;

use std::mem;
use std::default::Default;
use std::ffi::CString;
use std::collections::HashMap;

pub trait GuiDraw {
	fn draw(
		&self,
		device: &Device,
		pipeline_layout: &vk::PipelineLayout,
		command_buffer: &Vec<vk::CommandBuffer>,
		image_index: &usize,
	);
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
	color: RGBA,
	position: XY,
	texture: XY,
}


#[derive(Copy, Clone, Debug)]
pub struct Extent2D<T> {
	pub top: T,
	pub bottom: T,
	pub right: T,
	pub left: T,
}

pub struct Rect2Ds {
	/// (usize: for vertex, descriptor set, and push constant, usize: for texture)
	tag_and_idxs: HashMap<&'static str, (usize, usize)>,
	positions: Vec<(Extent2D<f32>, Extent2D<i32>, XY)>,
	vertex_buffers: BuffersWithMemory,
	textures: ImagesWithMemory,
	descriptor_sets: DescriptorSets,
	push_constants: Vec<RGBA>,
}

pub struct Rect2DsBuilder {
	builders: Vec<Rect2DBuilder>,
	image_pathes: Vec<&'static str>,
}

struct Rect2DBuilder {
	tag: &'static str,
	extent: Extent2D<i32>,
	origin: XY,
	color: [RGBA; 4],
	tex_coord: [XY; 4],
	image_idx: usize,
	sampler: vk::Sampler,
	color_weight: RGBA,
}

pub struct Font {
	image: ImagesWithMemory,
}


impl Default for Vertex {
	fn default() -> Self {
		Self {
			color: RGBA::default(),
			position: XY::zero(),
			texture: XY::zero(),
		}
	}
}

impl Default for Extent2D<i32> {
	fn default() -> Self {
		Self {
			top: 0,
			bottom: 0xff,
			left: 0,
			right: 0xff,
		}
	}
}

impl Extent2D<i32> {
	fn normalize(&self, frame: XY, origin: XY) -> Extent2D<f32> {
		let frame = frame * XY::new(0.5, 0.5);
		let top_left = XY::new(self.top as f32, self.left as f32) / frame + origin;
		let bottom_right = XY::new(self.bottom as f32, self.right as f32) / frame + origin;

		Extent2D {
			top: top_left.y,
			bottom: bottom_right.y,
			left: top_left.x,
			right: bottom_right.x,
		}
	}
}

impl Extent2D<f32> {
	pub fn to_positions(&self) -> [XY; 4] {
		[
			XY::new(self.left, self.top),
			XY::new(self.left, self.bottom),
			XY::new(self.right, self.bottom),
			XY::new(self.right, self.top),
		]
	}

	pub fn contain(&self, point: &XY) -> bool {
		self.top <= point.y && point.y <= self.bottom
		&& self.left <= point.x && point.x <= self.right
	}
}

pub const TOP_LEFT: XY = XY::new(-1.0, -1.0);
pub const TOP_CENTER: XY = XY::new(0.0, -1.0);
pub const TOP_RIGHT: XY = XY::new(1.0, -1.0);
pub const LEFT_CENTER: XY = XY::new(-1.0, 0.0);
pub const CENTER: XY = XY::new(0.0, 0.0);
pub const RIGHT_CENTER: XY = XY::new(-1.0, 0.0);
pub const BOTTOM_LEFT: XY = XY::new(-1.0, 1.0);
pub const BOTTOM_CENTER: XY = XY::new(0.0, 1.0);
pub const BOTTOM_RIGHT: XY = XY::new(1.0, 1.0);

impl GuiDraw for Rect2Ds {
	/// This function must be called,
	/// after the command buffer has entered in the valid render pass and been bound gui pipeline.
	/// The pipeline layout must be valid.
	fn draw(
		&self,
		device: &Device,
		pipeline_layout: &vk::PipelineLayout,
		command_buffer: &Vec<vk::CommandBuffer>,
		&image_index: &usize,
	) {
		unsafe {
			device.cmd_bind_vertex_buffers(
				command_buffer[image_index],
				0,
				&[self.vertex_buffers.raw_handle()],
				&[0],
			);

			self.push_constants
				.iter()
				.enumerate()
				.for_each(|(i, push_constant)| {
					device.cmd_bind_descriptor_sets(
						command_buffer[image_index],
						vk::PipelineBindPoint::GRAPHICS,
						*pipeline_layout,
						0,
						&[self.descriptor_sets.get(i, image_index)],
						&[],
					);
					device.cmd_push_constants(
						command_buffer[image_index],
						*pipeline_layout,
						vk::ShaderStageFlags::VERTEX,
						0,
						push_constant.to_ref_u8_slice(),
					);
					device.cmd_draw(command_buffer[image_index], 4, 1, (4 * i) as _, 0);
				});
		}
	}
}

impl Rect2Ds {
	pub fn start_builder() -> Rect2DsBuilder {
		Rect2DsBuilder::start()
	}

	pub fn valid_tag(&self, tag: &'static str) -> bool {
		self.tag_and_idxs.contains_key(tag)
	}

	pub fn extent(&self, tag: &'static str) -> Extent2D<f32> {
		let &(idx, _) = self.tag_and_idxs.get(tag).unwrap();
		self.positions[idx].0
	}

	pub fn contain(&self, tag: &'static str, point: &XY) -> bool {
		self.extent(tag).contain(&point)
	}

	pub fn update_color_weight(&mut self, tag: &'static str, color_weight: RGBA) {
		let &(idx, _) = self.tag_and_idxs.get(tag).unwrap();
		self.push_constants[idx] = color_weight;
	}

	// TODO: CommandRecorder should be provided by a argument because multiple operation should be integlated in one command.
	pub fn deal_with_window_resize(&mut self, vulkan: &Vulkan) {
		let data_infos = self.positions
			.iter_mut()
			.map(|(extent_f32, extent_i32, origin)| {
				*extent_f32 = extent_i32.normalize(vulkan.swapchain.render_xy(), *origin);
				BufferDataInfo::new(extent_f32.to_positions().to_vec())
			})
			.collect();

		let staging_buffers = BuffersWithMemory::visible_coherent(
			&vulkan.physical_device,
			&vulkan.device,
			data_infos,
			vk::BufferUsageFlags::TRANSFER_SRC,
			vk::SharingMode::EXCLUSIVE,
			vk::MemoryPropertyFlags::empty(),
		);

		let (_, _, region_infos) = (0..self.positions.len() * 4)
			.fold(
				(0, mem::size_of::<RGBA>() as _, Vec::with_capacity(self.positions.len() * 4)),
				|(src_offset, dst_offset, mut region_infos), _| {
					region_infos.push(
						vk::BufferCopy {
							src_offset,
							dst_offset,
							size: mem::size_of::<XY>() as _,
						}
					);

					(
						src_offset + mem::size_of::<XY>() as vk::DeviceSize,
						dst_offset + mem::size_of::<Vertex>() as vk::DeviceSize,
						region_infos
					)
				}
			);

		let mut command_recorder = vulkan
			.command_recorder()
			.begin_recording();

		command_recorder.transfer(&staging_buffers, &self.vertex_buffers, &region_infos[..]);

		let command_recorded = command_recorder.end_recording();

		vulkan.submit_command_recorder(
			&command_recorded,
			vk::PipelineStageFlags::empty(),
			&[],
			&[],
			&vk::Fence::null()
		);

		vulkan.queue_wait_idle();
		vulkan.destroy(staging_buffers);
		vulkan.destroy(command_recorded);
	}
}

impl VkDestroy for Rect2Ds {
	fn destroy(self, device: &Device) {
		self.descriptor_sets.destroy(device);
		self.textures.destroy(device);
		self.vertex_buffers.destroy(device);
	}
}

impl Rect2DsBuilder {
	pub fn start() -> Self {
		Self {
			builders: Vec::new(),
			image_pathes: Vec::new(),
		}
	}

	pub fn next(
		mut self,
		tag: &'static str,
		image_path: &'static str,
		sampler: vk::Sampler,
	) -> Self {
		self.builders.push(
			Rect2DBuilder {
				tag,
				extent: Extent2D::default(),
				origin: CENTER,
				color: [RGBA::default(); 4],
				tex_coord: [
					XY::new(0_f32, 0_f32),
					XY::new(0_f32, 1_f32),
					XY::new(1_f32, 1_f32),
					XY::new(1_f32, 0_f32),
				],
				image_idx: self.image_pathes.len(),
				sampler,
				color_weight: RGBA::default(),
			}
		);
		self.image_pathes.push(image_path);

		self
	}

	pub fn next_same_texture(mut self, tag: &'static str) -> Self {
		let sampler = self.builders.last().unwrap().sampler;
		self.builders.push(
			Rect2DBuilder {
				tag,
				extent: Extent2D::default(),
				origin: CENTER,
				color: [RGBA::default(); 4],
				tex_coord: [
					XY::new(0_f32, 0_f32),
					XY::new(0_f32, 1_f32),
					XY::new(1_f32, 1_f32),
					XY::new(1_f32, 0_f32),
				],
				image_idx: self.image_pathes.len() - 1,
				sampler,
				color_weight: RGBA::default(),
			}
		);

		self
	}

	pub fn extent(mut self, extent: Extent2D<i32>) -> Self {
		self.builders.last_mut().unwrap().extent = extent;
		self
	}

	pub fn origin(mut self, origin: XY) -> Self {
		self.builders.last_mut().unwrap().origin = origin;
		self
	}

	pub fn color(mut self, color: [RGBA; 4]) -> Self {
		self.builders.last_mut().unwrap().color = color;
		self
	}

	pub fn tex_coord(mut self, tex_coord: [XY; 4]) -> Self {
		self.builders.last_mut().unwrap().tex_coord = tex_coord;
		self
	}

	pub fn color_weight(mut self, color_weight: RGBA) -> Self {
		self.builders.last_mut().unwrap().color_weight = color_weight;
		self
	}

	pub fn build(self, vulkan: &Vulkan) -> Rect2Ds {
		let builders_num = self.builders.len();
		let (buffer_infos, positions, push_constants, tag_and_idxs) = self.builders
			.iter()
			.enumerate()
			.fold(
				(
					Vec::with_capacity(builders_num),
					Vec::with_capacity(builders_num),
					Vec::with_capacity(builders_num),
					HashMap::with_capacity(builders_num),
				),
				|
					(mut buffer_infos, mut positions, mut push_constants, mut tag_and_idxs),
					(i, builder)
				| {
					let extent = {
						let render_height = vulkan.swapchain.data.extent.height as f32;
						let render_width = vulkan.swapchain.data.extent.width as f32;
						let frame = XY::new(render_width, render_height);
						builder.extent.normalize(frame, builder.origin)
					};

					let vertices: Vec<_> = {
						let positions = extent.to_positions();

						builder.color
							.iter()
							.zip(builder.tex_coord.iter())
							.zip(positions.iter())
							.map(|((&color, &texture), &position)| {
								Vertex {
									position,
									color,
									texture,
								}
							})
							.collect()
					};

					buffer_infos.push(BufferDataInfo::new(vertices));

					positions.push((extent, builder.extent, builder.origin));
					push_constants.push(builder.color_weight);
					tag_and_idxs
						.insert(builder.tag, (i, builder.image_idx))
						.ok_or(())
						.expect_err("Same tag is prohibited.");

					(buffer_infos, positions, push_constants, tag_and_idxs)
				},
			);



		let (vertex_buffers, textures) = vulkan
			.resource_loader()
			.device_local_buffer(
				buffer_infos,
				vk::BufferUsageFlags::VERTEX_BUFFER,
				vk::SharingMode::EXCLUSIVE,
			)
			.image_for_texture(
				self.image_pathes,
				vk::SharingMode::EXCLUSIVE,
			)
			.execute()
			.finish();

		let vertex_buffers = vertex_buffers.into_iter().next().unwrap();
		let textures = textures.into_iter().next().unwrap();

		let descriptor_image_infos: Vec<_> = self.builders
			.iter()
			.map(|builder| {
				let image = textures.get(builder.image_idx);
				vk::DescriptorImageInfo {
					image_layout: image.layout(0),
					image_view: image.view(),
					sampler: builder.sampler,
				}
			})
			.collect();

		let descriptor_sets = vulkan.gui_descriptor_sets(&descriptor_image_infos[..]);

		Rect2Ds {
			tag_and_idxs,
			positions,
			vertex_buffers,
			textures,
			descriptor_sets,
			push_constants,
		}
	}
}

impl Font {
	pub fn new() -> Self {
		unimplemented!()
	}

	fn ascii_to_texture_coordinates(code: char) -> [XY; 4] {
		unimplemented!()
	}
}


pub fn load(
	device: &Device,
	render_pass: vk::RenderPass,
) -> shaders::Shader {
	let descriptor_set_layout = {
		let descriptor_set_bindings = [
			vk::DescriptorSetLayoutBinding {
				binding: 0,
				descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
				descriptor_count: 1,
				stage_flags: vk::ShaderStageFlags::FRAGMENT,
				p_immutable_samplers: ptr::null(),
			},
		];

		let info = vk::DescriptorSetLayoutCreateInfo {
			s_type: StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DescriptorSetLayoutCreateFlags::empty(),
			binding_count: descriptor_set_bindings.len() as u32,
			p_bindings: descriptor_set_bindings.as_ptr(),
		};

		unsafe {
			device
				.create_descriptor_set_layout(&info, None)
				.expect("Failed to create descriptor set layout.")
		}
	};

	let pipeline_layout = {
		let push_constant_ranges = [
			vk::PushConstantRange {
				stage_flags: vk::ShaderStageFlags::VERTEX,
				offset: 0,
				size: mem::size_of::<RGBA>() as u32,
			},
		];

		let info = vk::PipelineLayoutCreateInfo {
			s_type: StructureType::PIPELINE_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineLayoutCreateFlags::empty(),
			set_layout_count: 1,
			p_set_layouts: &descriptor_set_layout as *const _,
			push_constant_range_count: push_constant_ranges.len() as u32,
			p_push_constant_ranges: push_constant_ranges.as_ptr(),
		};

		unsafe {
			device
				.create_pipeline_layout(&info, None)
				.expect("Failed to create pipeline layout")
		}
	};

	let vert_shader_module = shaders::load_shader_module(device, "shaders/gui/vert.spv").unwrap();
	let frag_shader_module = shaders::load_shader_module(device, "shaders/gui/frag.spv").unwrap();

	let invoke_fn_name = CString::new("main").unwrap();

	let shader_infos = [
		vk::PipelineShaderStageCreateInfo {
			s_type: StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineShaderStageCreateFlags::empty(),
			stage: vk::ShaderStageFlags::VERTEX,
			module: vert_shader_module,
			p_name: invoke_fn_name.as_ptr(),
			p_specialization_info: ptr::null(),
		},
		vk::PipelineShaderStageCreateInfo {
			s_type: StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineShaderStageCreateFlags::empty(),
			stage: vk::ShaderStageFlags::FRAGMENT,
			module: frag_shader_module,
			p_name: invoke_fn_name.as_ptr(),
			p_specialization_info: ptr::null(),
		},
	];

	let bind_desc = vk::VertexInputBindingDescription {
		binding: 0,
		stride: mem::size_of::<Vertex>() as u32,
		input_rate: vk::VertexInputRate::VERTEX,
	};

	let attribute_desc = [
		// Represents `color: RGBA` in Vertex
		vk::VertexInputAttributeDescription {
			binding: 0,
			location: 0,
			format: vk::Format::R32G32B32A32_SFLOAT,
			offset: 0,
		},
		// Represents `position: XY` and `texture: XY` in Vertex
		vk::VertexInputAttributeDescription {
			binding: 0,
			location: 1,
			format: vk::Format::R32G32B32A32_SFLOAT,
			offset: mem::size_of::<RGBA>() as u32,
		},
	];

	let input_state = vk::PipelineVertexInputStateCreateInfo {
		s_type: StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::PipelineVertexInputStateCreateFlags::empty(),
		vertex_binding_description_count: 1,
		p_vertex_binding_descriptions: &bind_desc as *const _,
		vertex_attribute_description_count: attribute_desc.len() as u32,
		p_vertex_attribute_descriptions: attribute_desc.as_ptr(),
	};

	let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
		s_type: StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
		topology: vk::PrimitiveTopology::TRIANGLE_FAN,
		primitive_restart_enable: vk::FALSE,
	};

	let viewports = [vk::Viewport {
		x: 0.0,
		y: 0.0,
		width: 1280_f32,
		height: 720_f32,
		min_depth: 0.0,
		max_depth: 1.0,
	}];

	let scissors = [vk::Rect2D {
		offset: vk::Offset2D { x: 0, y: 0 },
		extent: vk::Extent2D { width: 1280, height: 720 },
	}];

	let viewport_state = vk::PipelineViewportStateCreateInfo {
		s_type: StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::PipelineViewportStateCreateFlags::empty(),
		viewport_count: viewports.len() as u32,
		p_viewports: viewports.as_ptr(),
		scissor_count: scissors.len() as u32,
		p_scissors: scissors.as_ptr()
	};

	let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
		s_type: StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::PipelineRasterizationStateCreateFlags::empty(),
		depth_clamp_enable: vk::FALSE,
		rasterizer_discard_enable: vk::FALSE,
		polygon_mode: vk::PolygonMode::FILL,
		cull_mode: vk::CullModeFlags::BACK,
		front_face: vk::FrontFace::COUNTER_CLOCKWISE,
		depth_bias_enable: vk::FALSE,
		depth_bias_constant_factor: 0.0,
		depth_bias_clamp: 0.0,
		depth_bias_slope_factor: 0.0,
		line_width: 1.0,
	};

	let multisample = vk::PipelineMultisampleStateCreateInfo {
		s_type: StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::PipelineMultisampleStateCreateFlags::empty(),
		rasterization_samples: vk::SampleCountFlags::TYPE_1,
		sample_shading_enable: vk::FALSE,
		min_sample_shading: 1.0,
		p_sample_mask: ptr::null(),
		alpha_to_coverage_enable: vk::FALSE,
		alpha_to_one_enable: vk::FALSE,
	};

	let color_blend_attachments = [vk::PipelineColorBlendAttachmentState {
		color_write_mask: vk::ColorComponentFlags::all(),
		blend_enable: vk::TRUE,
		src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
		dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
		color_blend_op: vk::BlendOp::ADD,
		src_alpha_blend_factor: vk::BlendFactor::ONE,
		dst_alpha_blend_factor: vk::BlendFactor::ZERO,
		alpha_blend_op: vk::BlendOp::ADD,
	}];

	let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
		s_type: StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::PipelineColorBlendStateCreateFlags::empty(),
		logic_op_enable: vk::FALSE,
		logic_op: vk::LogicOp::COPY,
		attachment_count: color_blend_attachments.len() as u32,
		p_attachments: color_blend_attachments.as_ptr(),
		blend_constants: [0.0; 4],
	};

	let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

	let dynamic_states = vk::PipelineDynamicStateCreateInfo {
		s_type: StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::PipelineDynamicStateCreateFlags::empty(),
		dynamic_state_count: dynamic_states.len() as u32,
		p_dynamic_states: dynamic_states.as_ptr(),
	};

	let info = [
		vk::GraphicsPipelineCreateInfo {
			s_type: StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineCreateFlags::empty(),
			stage_count: shader_infos.len() as u32,
			p_stages: shader_infos.as_ptr(),
			p_vertex_input_state: &input_state as *const _,
			p_input_assembly_state: &input_assembly as *const _,
			p_viewport_state: &viewport_state as *const _,
			p_tessellation_state: ptr::null(),
			p_rasterization_state: &rasterization_state as *const _,
			p_multisample_state: &multisample as *const _,
			p_depth_stencil_state: ptr::null(),
			p_color_blend_state: &color_blend_state as *const _,
			p_dynamic_state: &dynamic_states as *const _,
			layout: pipeline_layout,
			render_pass,
			subpass: 0,
			base_pipeline_handle: vk::Pipeline::null(),
			base_pipeline_index: -1,
		}
	];

	let pipeline = unsafe {
		device
			.create_graphics_pipelines(vk::PipelineCache::null(), &info, None)
			.expect("Failed to create pipeline.")[0]
	};

	unsafe {
		device.destroy_shader_module(vert_shader_module, None);
		device.destroy_shader_module(frag_shader_module, None);
	}

	shaders::Shader {
		pipeline,
		pipeline_layout,
		descriptor_set_layout,
	}
}

pub fn create_descriptor_sets(
	device: &Device,
	descriptor_set_layout: vk::DescriptorSetLayout,
	sets_per_obj: usize,
	textures: &[vk::DescriptorImageInfo],
) -> DescriptorSets {
	let descriptor_set_count = sets_per_obj * textures.len();

	let descriptor_pool = {
		let pool_sizes = vec![
			vk::DescriptorPoolSize {
				ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
				descriptor_count: 1,
			}; descriptor_set_count
		];

		let info = vk::DescriptorPoolCreateInfo {
			s_type: StructureType::DESCRIPTOR_POOL_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DescriptorPoolCreateFlags::empty(),
			max_sets: descriptor_set_count as u32,
			pool_size_count: pool_sizes.len() as u32,
			p_pool_sizes: pool_sizes.as_ptr(),
		};

		unsafe {
			device
				.create_descriptor_pool(&info, None)
				.expect("Failed to create descriptor pool.")
		}
	};

	let set_layouts = vec![descriptor_set_layout; descriptor_set_count];

	let descriptor_sets = unsafe {
		device
			.allocate_descriptor_sets(
				&vk::DescriptorSetAllocateInfo {
					s_type: StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
					p_next: ptr::null(),
					descriptor_pool,
					descriptor_set_count: set_layouts.len() as u32,
					p_set_layouts: set_layouts.as_ptr(),
				}
			)
			.unwrap()
	};

	let write_infos: Vec<_> = textures
		.iter()
		.zip(descriptor_sets[..].chunks(sets_per_obj))
		.fold(Vec::new(), |mut write_infos, (image_info, descriptor_sets)| {
			descriptor_sets
				.iter()
				.for_each(|&descriptor_set| {
					write_infos.push(
						vk::WriteDescriptorSet {
							s_type: StructureType::WRITE_DESCRIPTOR_SET,
							p_next: ptr::null(),
							dst_set: descriptor_set,
							dst_binding: 0,
							dst_array_element: 0,
							descriptor_count: 1,
							descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
							p_image_info: image_info as *const _,
							p_buffer_info: ptr::null(),
							p_texel_buffer_view: ptr::null(),
						}
					);
				});
			write_infos
		});

	unsafe {
		device.update_descriptor_sets(&write_infos[..], &[]);
		DescriptorSets::new(descriptor_sets, descriptor_pool, sets_per_obj)
	}
}
