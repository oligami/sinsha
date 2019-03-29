pub mod gui;

use crate::vulkan::*;

use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk;
use ash::vk::StructureType;

use std::fs;
use std::ptr;
use std::path::Path;
use std::error::Error;

pub fn load_shader_module<P: AsRef<Path>>(
	device: &Device,
	path_to_spv: P
) -> Result<vk::ShaderModule, Box<dyn Error>> {
	let contents = fs::read(path_to_spv)?;
	let info = vk::ShaderModuleCreateInfo {
		s_type: StructureType::SHADER_MODULE_CREATE_INFO,
		p_next: ptr::null(),
		flags: vk::ShaderModuleCreateFlags::empty(),
		code_size: contents.len(),
		p_code: contents.as_ptr() as *const u32,
	};

	Ok(unsafe { device.create_shader_module(&info, None)? })
}

pub struct Shader {
	pub pipeline: vk::Pipeline,
	pub pipeline_layout: vk::PipelineLayout,
	pub descriptor_set_layout: vk::DescriptorSetLayout,
}

pub struct Shaders {
	pub gui: Shader,
}

impl Shaders {
	pub fn load(
		device: &Device,
		render_pass: vk::RenderPass,
		specialization_constants: (f32, )) -> Self {
		Self {
			gui: self::gui::load(device, render_pass, &specialization_constants.0),
		}
	}
}




