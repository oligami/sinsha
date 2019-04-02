pub mod gui;
pub mod d3;

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
) -> Result<vk::ShaderModule, vk::Result> {
	let contents = fs::read(path_to_spv).unwrap();
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
	pub d3: Shader,
}

impl Shaders {
	pub fn load(
		device: &Device,
		render_pass: vk::RenderPass,
		render_extent: vk::Extent2D,
	) -> Result<Self, vk::Result> {
		let gui = {
			let (descriptor_set_layout, pipeline_layout) = gui::load_layouts(&device)?;
			let pipeline = gui::load_pipeline(
				device,
				render_pass,
				descriptor_set_layout,
				pipeline_layout,
				render_extent,
			)?;

			Shader { pipeline_layout, descriptor_set_layout, pipeline }
		};

		let d3 = {
			let (descriptor_set_layout, pipeline_layout) = d3::load_layouts(&device)?;
			let pipeline = d3::load_pipeline(
				device,
				render_pass,
				descriptor_set_layout,
				pipeline_layout,
				1.0,
				0.01,
				100.0,
				render_extent,
			)?;

			Shader { descriptor_set_layout, pipeline_layout, pipeline }
		};

		Ok(Self { gui, d3 })
	}

	pub fn reload(
		&mut self,
		device: &Device,
		render_pass: vk::RenderPass,
		render_extent: vk::Extent2D,
	) -> Result<(), vk::Result> {
		unsafe { device.destroy_pipeline(self.gui.pipeline, None); }
		self.gui.pipeline = gui::load_pipeline(
			device,
			render_pass,
			self.gui.descriptor_set_layout,
			self.gui.pipeline_layout,
			render_extent,
		)?;

		Ok(())
	}
}




