pub mod stage;
pub mod dynamic_state;

use super::*;
use super::stage::OneShaderStage;
use super::descriptor::DescriptorSetLayout;

pub struct PipelineLayout<P, L> {
	device: Arc<Device>,
	handle: vk::PipelineLayout,
	push_constants: PhantomData<P>,
	set_layouts: PhantomData<L>,
}

pub struct PipelineLayoutBuilderPushConstants<P> {
	ranges: Vec<vk::PushConstantRange>,
	offset: u32,
	push_constants: PhantomData<fn() -> P>,
}

pub struct PushConstant<T>(PhantomData<fn() -> T>);

pub struct PipelineLayoutBuilderSetLayout<P, H, L> {
	push_constant_ranges: Vec<vk::PushConstantRange>,
	push_constants: PhantomData<P>,
	set_layout_count: u32,
	set_layout_handles: H,
	set_layouts: PhantomData<L>,
}

pub struct ComputePipeline<P, L> {
	handle: vk::Pipeline,
	layout: Arc<PipelineLayout<P, L>>,
}

pub struct GraphicsPipeline<A, S, P, L, V, D> {
	handle: vk::Pipeline,
	render_pass: Arc<render_pass::RenderPass<A, S>>,
	layout: Arc<PipelineLayout<P, L>>,
	vertex_or_mesh: PhantomData<V>,
	dynamic_states: PhantomData<D>,
}

pub struct PipelineShaderStagesBuilder<P, L> {
	layout: Arc<PipelineLayout<P, L>>,
	shader_stages: Vec<vk::PipelineShaderStageCreateInfo>,
	shader_stage_holders: Vec<ShaderStageHolder>,
}

struct ShaderStageHolder {
	invoke_fn_name: CString,
	specialization_info: Option<Box<vk::SpecializationInfo>>,
	data_and_maps: Option<Specializations>,
}

pub struct Specializations {
	maps: Vec<vk::SpecializationMapEntry>,
	data: Vec<u8>,
}

pub struct GraphicsPipelineBuilder<P, L> {
	layout: Arc<PipelineLayout<P, L>>,
}

pub struct GraphicsPipelineBuilderS {
	vertex_input_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
	vertex_input_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
	topology: vk::PrimitiveTopology,
	primitive_restart: vk::Bool32,
	tessellation_path_control_points: u32,
	viewports: Vec<vk::Viewport>,
	scissors: Vec<vk::Rect2D>,
}

pub struct GraphicsPipelineBuilderDynamicState<D> {
	dynamic_state: PhantomData<D>,
}

pub struct GraphicsPipelineBuilderSubpassSelect<A, S, I> {
	render_pass: Arc<render_pass::RenderPass<A, S>>,
	subpass_index: u32,
	subpass: PhantomData<I>,
}


impl PipelineLayout<(), ()> {
	pub fn builder() -> PipelineLayoutBuilderPushConstants<()> {
		PipelineLayoutBuilderPushConstants {
			ranges: Vec::new(),
			offset: 0,
			push_constants: PhantomData,
		}
	}
}

impl<P, L> PipelineLayout<P, L> {
	pub fn graphics_pipeline_builder(self: Arc<Self>) -> GraphicsPipelineBuilder<P, L> {
		GraphicsPipelineBuilder { layout: self }
	}
}

impl<P, L> Drop for PipelineLayout<P, L> {
	fn drop(&mut self) { unsafe { self.device.handle.destroy_pipeline_layout(self.handle, None) } }
}

impl<P> PipelineLayoutBuilderPushConstants<P> {
	pub fn push_constant<T, S>(
		mut self,
		data_type: PushConstant<T>,
		stage: S,
	) -> PipelineLayoutBuilderPushConstants<(P, T)> where S: OneShaderStage {
		let align = std::mem::align_of::<T>() as u32;
		let offset = if self.offset % align == 0 {
			self.offset
		} else {
			align * (self.offset / align + 1)
		};

		let push_constant = vk::PushConstantRange {
			stage_flags: S::shader_stages(),
			offset,
			size: std::mem::size_of::<T>() as u32,
		};

		println!("size: {}", std::mem::size_of::<T>());

		self.ranges.push(push_constant);
		PipelineLayoutBuilderPushConstants {
			ranges: self.ranges,
			offset: offset + std::mem::size_of::<T>() as u32,
			push_constants: PhantomData,
		}
	}

	pub fn descriptor_set_layout(self) -> PipelineLayoutBuilderSetLayout<P, (), ()> {
		PipelineLayoutBuilderSetLayout {
			push_constant_ranges: self.ranges,
			push_constants: PhantomData,
			set_layout_count: 0,
			set_layout_handles: (),
			set_layouts: PhantomData,
		}
	}
}

impl PushConstant<()> {
	pub fn new<T>() -> PushConstant<T> { PushConstant(PhantomData) }
}

impl<P, H, L> PipelineLayoutBuilderSetLayout<P, H, L> {
	pub fn set_layout<L1>(
		self,
		set_layout: Arc<DescriptorSetLayout<L1>>,
	) -> PipelineLayoutBuilderSetLayout<P, (H, vk::DescriptorSetLayout), (L, L1)>
	{
		PipelineLayoutBuilderSetLayout {
			push_constant_ranges: self.push_constant_ranges,
			push_constants: self.push_constants,
			set_layout_count: self.set_layout_count + 1,
			set_layout_handles: (self.set_layout_handles, set_layout.handle()),
			set_layouts: PhantomData,
		}
	}

	pub fn build(self, device: Arc<Device>) -> Arc<PipelineLayout<P, L>> {
		let info = vk::PipelineLayoutCreateInfo {
			s_type: StructureType::PIPELINE_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineLayoutCreateFlags::empty(),
			set_layout_count: self.set_layout_count,
			p_set_layouts: &self.set_layout_handles as *const _ as *const _,
			push_constant_range_count: self.push_constant_ranges.len() as u32,
			p_push_constant_ranges: self.push_constant_ranges.as_ptr(),
		};

		let handle = unsafe { device.handle.create_pipeline_layout(&info, None).unwrap() };

		Arc::new(PipelineLayout {
			device,
			handle,
			set_layouts: self.set_layouts,
			push_constants: self.push_constants,
		})
	}
}

impl<A, S, P, L, V, D> Drop for GraphicsPipeline<A, S, P, L, V, D> {
	fn drop(&mut self) {
		unsafe { self.layout.device.handle.destroy_pipeline(self.handle, None); }
	}
}

impl<P, L> PipelineShaderStagesBuilder<P, L> {
	/// # Safety
	/// A SPIR-V file indicated by path may have incompatible contents.
	/// Especially, the pipeline layout, descriptor set layouts, push constants,
	/// specialization constants, the shader stage.
	pub unsafe fn shader_stage<Path, N, S>(
		mut self,
		path: Path,
		invoke_fn_name: N,
		stage_flag: S,
		specializations: Option<Specializations>,
	) -> Self where Path: AsRef<std::path::Path>, N: Into<Vec<u8>>, S: OneShaderStage {

		let module = {
			use std::io::Read;

			let mut spv = std::fs::File::open(path).unwrap();
			let mut buf = Vec::new();
			spv.read_to_end(&mut buf).unwrap();

			let info = vk::ShaderModuleCreateInfo {
				s_type: StructureType::SHADER_MODULE_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::ShaderModuleCreateFlags::empty(),
				code_size: buf.len(),
				p_code: buf.as_ptr() as _,
			};

			self.layout.device.handle.create_shader_module(&info, None).unwrap()
		};

		let invoke_fn_name = CString::new(invoke_fn_name).unwrap();

		let mut holder = ShaderStageHolder {
			invoke_fn_name,
			specialization_info: None,
			data_and_maps: None
		};

		let p_specialization_info = specializations
			.map(|s| {
				let info = vk::SpecializationInfo {
					map_entry_count: s.maps.len() as u32,
					p_map_entries: s.maps.as_ptr(),
					data_size: s.data.len(),
					p_data: s.data.as_ptr() as _,
				};

				let info = Box::new(info);
				let ptr = info.as_ref() as *const _;
				holder.specialization_info = Some(info);
				holder.data_and_maps = Some(s);

				ptr
			})
			.unwrap_or(ptr::null());

		let stage_info = vk::PipelineShaderStageCreateInfo {
			s_type: StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineShaderStageCreateFlags::empty(),
			stage: S::shader_stages(),
			module,
			p_name: holder.invoke_fn_name.as_ptr(),
			p_specialization_info,
		};

		self.shader_stages.push(stage_info);
		self.shader_stage_holders.push(holder);

		self
	}
}

impl Specializations {
	/// offset
	pub fn new<T>(data: T) -> Self {
		let data = unsafe {
			let ptr = &data as *const _ as *mut u8;
			let size = std::mem::size_of::<T>();
			Vec::from_raw_parts(ptr, size, size)
		};

		Self {
			maps: vec![],
			data,
		}
	}

	pub fn constant(mut self, id: u32, range: std::ops::Range<usize>) -> Self {
		self.maps.push(vk::SpecializationMapEntry {
			constant_id: id,
			offset: range.start as u32,
			size: range.end - range.start,
		});
		self
	}
}

impl<P, L> GraphicsPipelineBuilder<P, L> {
	pub fn dynamic_state(&self) -> GraphicsPipelineBuilderDynamicState<()> {
		GraphicsPipelineBuilderDynamicState { dynamic_state: PhantomData }
	}

	pub fn subpass_select<A, S>(
		&self,
		render_pass: Arc<render_pass::RenderPass<A, S>>,
	) -> GraphicsPipelineBuilderSubpassSelect<A, S, S> {
		GraphicsPipelineBuilderSubpassSelect {
			subpass_index: render_pass.subpass_count(),
			render_pass,
			subpass: PhantomData,
		}
	}
}
