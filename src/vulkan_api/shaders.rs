pub mod gui;

use ash::Device;
use ash::version::DeviceV1_0;
use ash::vk;
use ash::vk::StructureType;

use std::ptr;
use std::fs;
use std::path::Path;
use std::convert::AsRef;
use std::error::Error;
use std::boxed::Box;

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

pub struct DescriptorSets {
	pub descriptor_sets: Vec<vk::DescriptorSet>,
	pub descriptor_pool: vk::DescriptorPool,
	sets_per_obj: usize,
}

impl Shaders {
	pub fn load(device: &Device, render_pass: vk::RenderPass) -> Self {
		Self {
			gui: self::gui::load(device, render_pass),
		}
	}

	pub fn unload(self, device: &Device) {
		unsafe {
			device.destroy_pipeline(self.gui.pipeline, None);
			device.destroy_pipeline_layout(self.gui.pipeline_layout, None);
			device.destroy_descriptor_set_layout(self.gui.descriptor_set_layout, None);
		}
	}
}

impl DescriptorSets {
	pub unsafe fn new(
		descriptor_sets: Vec<vk::DescriptorSet>,
		descriptor_pool: vk::DescriptorPool,
		sets_per_obj: usize,
	) -> Self {
		Self {
			descriptor_sets,
			descriptor_pool,
			sets_per_obj,
		}
	}

	pub unsafe fn gui(
		device: &Device,
		descriptor_set_layout: vk::DescriptorSetLayout,
		descriptor_set_per_obj: usize,
		textures: &[vk::DescriptorImageInfo],
	) -> Self {
		self::gui::create_descriptor_sets(
			device,
			descriptor_set_layout,
			descriptor_set_per_obj,
			textures,
		)
	}

	pub fn get(&self, obj_index: usize, image_index: usize) -> vk::DescriptorSet {
		self.descriptor_sets[obj_index * self.sets_per_obj + image_index]
	}
}


