use ash::vk;
use ash::Device;
use ash::vk::StructureType;
use ash::version::DeviceV1_0;

use crate::linear_algebra::*;
use crate::vulkan::shaders::load_shader_module;

use std::ptr;
use std::mem;


pub struct Vertex {
	position: XYZ,
	normal: XYZ,
	color: RGBA,
}

pub fn load_layouts(
	device: &Device
) -> Result<(vk::DescriptorSetLayout, vk::PipelineLayout), vk::Result> {
	let descriptor_set_layout = {
		let bindings = [
			vk::DescriptorSetLayoutBinding {
				binding: 0,
				descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
				descriptor_count: 1,
				stage_flags: vk::ShaderStageFlags::VERTEX,
				p_immutable_samplers: ptr::null(),
			}
		];

		let info = vk::DescriptorSetLayoutCreateInfo {
			s_type: StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DescriptorSetLayoutCreateFlags::empty(),
			binding_count: bindings.len() as _,
			p_bindings: bindings.as_ptr(),
		};

		unsafe { device.create_descriptor_set_layout(&info, None)? }
	};

	let pipeline_layout = {
		let push_constants = [
			vk::PushConstantRange {
				stage_flags: vk::ShaderStageFlags::VERTEX,
				offset: 0,
				size: mem::size_of::<[f32; 5]>() as _,
			},
		];

		let info = vk::PipelineLayoutCreateInfo {
			s_type: StructureType::PIPELINE_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineLayoutCreateFlags::empty(),
			set_layout_count: 1,
			p_set_layouts: &descriptor_set_layout as *const _,
			push_constant_range_count: push_constants.len() as _,
			p_push_constant_ranges: push_constants.as_ptr(),
		};

		unsafe { device.create_pipeline_layout(&info, None)? }
	};

	Ok((descriptor_set_layout, pipeline_layout))
}

pub fn load_pipeline(
	device: &Device,
	render_pass: vk::RenderPass,
	descriptor_set_layout: vk::DescriptorSetLayout,
	pipeline_layout: vk::PipelineLayout,
	fov: f32,
	near: f32,
	far: f32,
	render_extent: vk::Extent2D,
) -> Result<vk::Pipeline, vk::Result> {
	let vert_shader_module = load_shader_module(device, "shaders/d3/vert.spv")?;
	let frag_shader_module = load_shader_module(device, "shaders/d3/frag.spv")?;

	let invoke_fn_name = std::ffi::CString::new("main").unwrap();

	let specialization_constants_map = [
		vk::SpecializationMapEntry {
			constant_id: 0,
			offset: 0,
			size: mem::size_of::<f32>(),
		},
		vk::SpecializationMapEntry {
			constant_id: 1,
			offset: mem::size_of::<f32>() as _,
			size: mem::size_of::<f32>(),
		},
		vk::SpecializationMapEntry {
			constant_id: 2,
			offset: mem::size_of::<[f32; 2]>() as _,
			size: mem::size_of::<f32>(),
		},
		vk::SpecializationMapEntry {
			constant_id: 2,
			offset: mem::size_of::<[f32; 3]>() as _,
			size: mem::size_of::<f32>(),
		},
	];
	let aspect_re = render_extent.height as f32 / render_extent.width as f32;
	let tan_fov_re = 1.0 / fov.tan();
	let specialization_data = [aspect_re, tan_fov_re, near, far];

	let specialization_info = vk::SpecializationInfo {
		map_entry_count: specialization_constants_map.len() as _,
		p_map_entries: specialization_constants_map.as_ptr(),
		data_size: mem::size_of::<[f32; 4]>(),
		p_data: specialization_data.as_ptr() as _,
	};

	let shader_infos = [
		vk::PipelineShaderStageCreateInfo {
			s_type: StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineShaderStageCreateFlags::empty(),
			stage: vk::ShaderStageFlags::VERTEX,
			module: vert_shader_module,
			p_name: invoke_fn_name.as_ptr(),
			p_specialization_info: &specialization_info as *const _,
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
		vk::VertexInputAttributeDescription {
			binding: 0,
			location: 0,
			format: vk::Format::R32G32B32_SFLOAT,
			offset: 0,
		},
		vk::VertexInputAttributeDescription {
			binding: 0,
			location: 1,
			format: vk::Format::R32G32B32_SFLOAT,
			offset: 0,
		},
		vk::VertexInputAttributeDescription {
			binding: 0,
			location: 2,
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
		topology: vk::PrimitiveTopology::TRIANGLE_LIST,
		primitive_restart_enable: vk::FALSE,
	};

	let viewports = [vk::Viewport {
		x: 0.0,
		y: 0.0,
		width: render_extent.width as f32,
		height: render_extent.height as f32,
		min_depth: 0.0,
		max_depth: 1.0,
	}];

	let scissors = [vk::Rect2D {
		offset: vk::Offset2D { x: 0, y: 0 },
		extent: render_extent,
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
		p_dynamic_state: ptr::null(),
		layout: pipeline_layout,
		render_pass,
		subpass: 0,
		base_pipeline_handle: vk::Pipeline::null(),
		base_pipeline_index: -1,
	};

	let pipeline = unsafe {
		device
			.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
			.map_err(|(_, err)| err)?[0]
	};

	unsafe {
		device.destroy_shader_module(vert_shader_module, None);
		device.destroy_shader_module(frag_shader_module, None);
	}

	Ok(pipeline)
}