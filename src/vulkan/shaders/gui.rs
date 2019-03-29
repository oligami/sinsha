use ash::vk;
use ash::Device;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::vulkan::*;
use crate::linear_algebra::{XY, RGBA};

use std::mem;
use std::ptr;
use std::ops;
use std::slice;
use std::ffi::CString;
use std::collections::HashMap;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
	position: XY,
	texture: XY,
}

#[repr(C)]
pub struct PushConstant(RGBA, XY);

#[derive(Copy, Clone, Debug)]
pub struct Extent2D<T> {
	pub top: T,
	pub bottom: T,
	pub right: T,
	pub left: T,
}

impl Default for Vertex {
	fn default() -> Self {
		Self {
			position: XY::zero(),
			texture: XY::zero(),
		}
	}
}

impl Vertex {
	pub const fn new(position: XY, texture: XY) -> Self {
		Self { position, texture }
	}
}

impl PushConstant {
	pub fn new(rgba: RGBA, xy: XY) -> Self { PushConstant(rgba, xy) }
}

impl AsRef<[u8]> for PushConstant {
	fn as_ref(&self) -> &[u8] {
		unsafe {
			let ptr = self as *const _ as *const u8;
			slice::from_raw_parts(ptr, mem::size_of::<Self>())
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


/// The ownership of this struct never be obtained outside of this module.
/// Only references can be obtained. Thus, lifetime doesn't need.
pub struct DescriptorSets {
	raw_handles: Vec<vk::DescriptorSet>,
}

pub struct DescriptorPool<'vk_core> {
	vk_core: &'vk_core VkCore,
	raw_handle: vk::DescriptorPool,
	sets_vec: Vec<DescriptorSets>,
}

impl ops::Index<usize> for DescriptorSets {
	type Output = vk::DescriptorSet;
	fn index(&self, index: usize) -> &Self::Output { &self.raw_handles[index] }
}

impl<'vk_core> DescriptorPool<'vk_core> {
	pub fn new(
		vk_graphic: &VkGraphic<'vk_core>,
		textures: &[(&VkImageView<'vk_core>, &VkSampler)],
	) -> Result<Self, vk::Result> {
		unsafe {
			let total_sets_num = textures.len() * vk_graphic.images_num();

			let pool_sizes = [vk::DescriptorPoolSize {
				ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
				descriptor_count: vk_graphic.images_num() as u32,
			}];

			let raw_handle = vk_graphic.vk_core.device
				.create_descriptor_pool(
					&vk::DescriptorPoolCreateInfo {
						s_type: StructureType::DESCRIPTOR_POOL_CREATE_INFO,
						p_next: ptr::null(),
						flags: vk::DescriptorPoolCreateFlags::empty(),
						max_sets: total_sets_num as u32,
						pool_size_count: pool_sizes.len() as u32,
						p_pool_sizes: pool_sizes.as_ptr(),
					},
					None,
				)?;

			// NOTE: Allocate many times may make performance worse.
			// Allocate once and operate Vec may be better.
			let mut sets_vec = Vec::with_capacity(textures.len());
			let set_layouts = vec![
				vk_graphic.shaders.gui.descriptor_set_layout;
				vk_graphic.images_num()
			];
			for _i in 0..textures.len() {
				let sets = vk_graphic.vk_core.device
					.allocate_descriptor_sets(
						&vk::DescriptorSetAllocateInfo {
							s_type: StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
							p_next: ptr::null(),
							descriptor_pool: raw_handle,
							descriptor_set_count: set_layouts.len() as u32,
							p_set_layouts: set_layouts.as_ptr(),
						}
					)?;

				sets_vec.push(DescriptorSets { raw_handles: sets });
			}

			let mut write_sets = Vec::with_capacity(total_sets_num);
			let image_write_infos = textures
				.iter()
				.fold(
					Vec::with_capacity(total_sets_num),
					|mut infos, (image_view, sampler)| {
						infos.push(
							vk::DescriptorImageInfo {
								image_view: image_view.raw_handle,
								image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
								sampler: sampler.raw_handle,
							}
						);
						infos
					}
				);
			for (image_write, sets) in image_write_infos.iter().zip(sets_vec.iter()) {
				for &set in sets.raw_handles.iter() {
					write_sets.push(
						vk::WriteDescriptorSet {
							s_type: StructureType::WRITE_DESCRIPTOR_SET,
							p_next: ptr::null(),
							dst_set: set,
							dst_binding: 0,
							dst_array_element: 0,
							descriptor_count: 1,
							descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
							p_image_info: image_write as *const _,
							p_buffer_info: ptr::null(),
							p_texel_buffer_view: ptr::null(),
						}
					);
				}
			}
			vk_graphic.vk_core.device.update_descriptor_sets(&write_sets[..], &[]);

			Ok(Self { vk_core: vk_graphic.vk_core, raw_handle, sets_vec })
		}
	}
}

impl ops::Index<usize> for DescriptorPool<'_> {
	type Output = DescriptorSets;
	fn index(&self, index: usize) -> &Self::Output { &self.sets_vec[index] }
}

impl Drop for DescriptorPool<'_> {
	fn drop(&mut self) {
		unsafe { self.vk_core.device.destroy_descriptor_pool(self.raw_handle, None); }
	}
}

pub fn load(device: &Device, render_pass: vk::RenderPass, width_height_ratio: &f32) -> Shader {
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
				size: mem::size_of::<PushConstant>() as u32,
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

	let vertex_specialization_constants = [
		vk::SpecializationMapEntry {
			constant_id: 0,
			offset: 0,
			size: mem::size_of::<f32>(),
		}
	];

	let vertex_specialization_constants = vk::SpecializationInfo {
		map_entry_count: vertex_specialization_constants.len() as _,
		p_map_entries: vertex_specialization_constants.as_ptr(),
		data_size: mem::size_of::<f32>(),
		p_data: width_height_ratio as *const _ as _,
	};

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
		// Represents `position: XY` and `texture: XY` in Vertex
		vk::VertexInputAttributeDescription {
			binding: 0,
			location: 0,
			format: vk::Format::R32G32B32A32_SFLOAT,
			offset: 0,
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

	let info = vk::GraphicsPipelineCreateInfo {
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
	};

	let pipeline = unsafe {
		device
			.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
			.expect("Failed to create pipeline.")[0]
	};

	unsafe {
		device.destroy_shader_module(vert_shader_module, None);
		device.destroy_shader_module(frag_shader_module, None);
	}

	Shader {
		pipeline,
		pipeline_layout,
		descriptor_set_layout,
	}
}
