pub mod stage;
pub mod dynamic_state;
pub mod bind_point;

pub use bind_point::PipelineBindPoint;
pub use dynamic_state::DynamicState;

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
	push_constants: P,
}

pub struct PushConstant<S, T> {
	shader_stages: PhantomData<S>,
	data_type: PhantomData<fn() -> T>,
}


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

pub struct GraphicsPipelineBuilder<S> {
	input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo,
	tessellation_state: vk::PipelineTessellationStateCreateInfo,
	viewport_state: vk::PipelineViewportStateCreateInfo,
	rasterization_state: vk::PipelineRasterizationStateCreateInfo,
	multisample_state: vk::PipelineMultisampleStateCreateInfo,
	depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo,
	color_blend_state: vk::PipelineColorBlendStateCreateInfo,
	viewports: Vec<vk::Viewport>,
	scissors: Vec<vk::Rect2D>,
	sample_mask: Box<u32>,
	state: PhantomData<S>,
}

pub struct PrimitiveTopology;
pub struct PrimitiveRestart;
pub struct Viewport;
pub struct Scissor;
pub struct RasterizerDiscard;
pub struct PolygonMode;
pub struct CullMode;
pub struct FrontFace;
pub struct DepthClamp;
pub struct DepthBias;
pub struct LineWidth;
pub struct SampleCount;
pub struct SampleShading;
pub struct SampleMask;
pub struct AlphaToCoverage;
pub struct AlphaToOne;
pub struct DepthTest;
pub struct DepthWrite;
pub struct DepthBounds;
pub struct StencilTest;
pub struct LogicOpOrColorBlend;
pub struct BlendConstant;
pub struct ReadyToBuild;

pub struct GraphicsPipelineDynamicStateBuilder<D> {
	dynamic_states: PhantomData<D>,
	dynamic_statess: Vec<vk::DynamicState>,
}

pub struct GraphicsPipelineSubpassSelecter<A, S, S0> {
	render_pass: Arc<render_pass::RenderPass<A, S>>,
	subpass_index: u32,
	subpass: PhantomData<S0>,
}


impl PipelineLayout<(), ()> {
	pub fn builder() -> PipelineLayoutBuilderPushConstants<()> {
		PipelineLayoutBuilderPushConstants {
			ranges: Vec::new(),
			push_constants: (),
		}
	}
}


impl<P, L> Drop for PipelineLayout<P, L> {
	fn drop(&mut self) { unsafe { self.device.handle.destroy_pipeline_layout(self.handle, None) } }
}

impl<P> PipelineLayoutBuilderPushConstants<P> {
	pub fn push_constant<S, T>(
		mut self,
		stage: S,
		data_type: PhantomData<T>,
	) -> PipelineLayoutBuilderPushConstants<(P, PushConstant<S, T>)>
		where S: shader::stage::ShaderStages
	{
		let size = std::mem::size_of::<T>() as u32;
		let size = if size % 4 != 0 { (size / 4 + 1) * 4 } else { size };

		let push_constant = vk::PushConstantRange {
			stage_flags: S::shader_stages(),
			offset: 0,
			size,
		};
		self.ranges.push(push_constant);

		let push_constant = PushConstant {
			shader_stages: PhantomData,
			data_type: PhantomData,
		};

		PipelineLayoutBuilderPushConstants {
			ranges: self.ranges,
			push_constants: (self.push_constants, push_constant),
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

impl GraphicsPipeline<(), (), (), (), (), ()> {
	pub fn builder() -> GraphicsPipelineBuilder<PrimitiveTopology> {
		GraphicsPipelineBuilder {
			input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo::default(),
			tessellation_state: vk::PipelineTessellationStateCreateInfo::default(),
			viewport_state: vk::PipelineViewportStateCreateInfo::default(),
			rasterization_state: vk::PipelineRasterizationStateCreateInfo::default(),
			multisample_state: vk::PipelineMultisampleStateCreateInfo::default(),
			depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo::default(),
			color_blend_state: vk::PipelineColorBlendStateCreateInfo::default(),
			viewports: Vec::new(),
			scissors: Vec::new(),
			sample_mask: Box::new(!0),
			state: PhantomData,
		}
	}

	pub fn dynamic_state_builder(&self) -> GraphicsPipelineDynamicStateBuilder<()> {
		GraphicsPipelineDynamicStateBuilder {
			dynamic_states: PhantomData,
			dynamic_statess: Vec::new(),
		}
	}

	pub fn subpass_selecter<A, S>(
		&self,
		render_pass: Arc<render_pass::RenderPass<A, S>>,
	) -> GraphicsPipelineSubpassSelecter<A, S, S> {
		GraphicsPipelineSubpassSelecter {
			subpass_index: render_pass.subpass_count() - 1,
			render_pass,
			subpass: PhantomData,
		}
	}
}

impl<A, S, P, L, V, D> GraphicsPipeline<A, S, P, L, V, D> {
	#[inline]
	pub fn handle(&self) -> vk::Pipeline { self.handle }
	#[inline]
	pub fn layout(&self) -> vk::PipelineLayout { self.layout.handle }
}

impl<A, S, P, L, V, D> Drop for GraphicsPipeline<A, S, P, L, V, D> {
	#[inline]
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

impl<T> GraphicsPipelineBuilder<T> {
	fn into<U>(self) -> GraphicsPipelineBuilder<U> {
		GraphicsPipelineBuilder {
			input_assembly_state: self.input_assembly_state,
			tessellation_state: self.tessellation_state,
			viewport_state: self.viewport_state,
			rasterization_state: self.rasterization_state,
			multisample_state: self.multisample_state,
			depth_stencil_state: self.depth_stencil_state,
			color_blend_state: self.color_blend_state,
			viewports: self.viewports,
			scissors: self.scissors,
			sample_mask: self.sample_mask,
			state: PhantomData,
		}
	}
}

impl GraphicsPipelineBuilder<PrimitiveTopology> {
	pub fn primitive_topology(
		mut self,
		topology: vk::PrimitiveTopology,
	) -> GraphicsPipelineBuilder<PrimitiveRestart> {
		self.input_assembly_state.topology = topology;
		self.into()
	}
}
impl GraphicsPipelineBuilder<PrimitiveRestart> {
	pub fn primitive_restart_enable(mut self) -> GraphicsPipelineBuilder<Viewport> {
		self.input_assembly_state.primitive_restart_enable = vk::TRUE;
		self.into()
	}

	pub fn primitive_restart_disable(mut self) -> GraphicsPipelineBuilder<Viewport> {
		self.input_assembly_state.primitive_restart_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<Viewport> {
	pub fn viewport(mut self, viewport: vk::Viewport) -> GraphicsPipelineBuilder<Scissor> {
		self.viewports.push(viewport);
		self.viewport_state.viewport_count = 1;
		self.viewport_state.p_viewports = self.viewports.as_ptr();
		self.into()
	}
}
impl GraphicsPipelineBuilder<Scissor> {
	pub fn scissor(mut self, scissor: vk::Rect2D) -> GraphicsPipelineBuilder<RasterizerDiscard> {
		self.scissors.push(scissor);
		self.viewport_state.scissor_count = 1;
		self.viewport_state.p_scissors = self.scissors.as_ptr();
		self.into()
	}
}
impl GraphicsPipelineBuilder<RasterizerDiscard> {
	pub fn rasterizer_discard_enable(mut self) -> GraphicsPipelineBuilder<PolygonMode> {
		self.rasterization_state.rasterizer_discard_enable = vk::TRUE;
		self.into()
	}

	pub fn rasterizer_discard_disable(mut self) -> GraphicsPipelineBuilder<PolygonMode> {
		self.rasterization_state.rasterizer_discard_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<PolygonMode> {
	pub fn polygon_mode(mut self, polygon_mode: vk::PolygonMode) -> GraphicsPipelineBuilder<CullMode> {
		self.rasterization_state.polygon_mode = polygon_mode;
		self.into()
	}
}
impl GraphicsPipelineBuilder<CullMode> {
	pub fn cull_mode(mut self, cull_mode: vk::CullModeFlags) -> GraphicsPipelineBuilder<FrontFace> {
		self.rasterization_state.cull_mode = cull_mode;
		self.into()
	}
}
impl GraphicsPipelineBuilder<FrontFace> {
	pub fn front_face(mut self, front_face: vk::FrontFace) -> GraphicsPipelineBuilder<DepthClamp> {
		self.rasterization_state.front_face = front_face;
		self.into()
	}
}
impl GraphicsPipelineBuilder<DepthClamp> {
	pub fn depth_clamp_enable(mut self) -> GraphicsPipelineBuilder<DepthBias> {
		self.rasterization_state.depth_clamp_enable = vk::TRUE;
		self.into()
	}

	pub fn depth_clamp_disable(mut self) -> GraphicsPipelineBuilder<DepthBias> {
		self.rasterization_state.depth_clamp_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<DepthBias> {
	pub fn depth_bias_enable(
		mut self, constant: f32, clamp: f32, slope: f32
	) -> GraphicsPipelineBuilder<LineWidth> {
		self.rasterization_state.depth_bias_enable = vk::TRUE;
		self.rasterization_state.depth_bias_constant_factor = constant;
		self.rasterization_state.depth_bias_clamp = clamp;
		self.rasterization_state.depth_bias_slope_factor = slope;
		self.into()
	}

	pub fn depth_bias_disable(mut self) -> GraphicsPipelineBuilder<LineWidth> {
		self.rasterization_state.depth_bias_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<LineWidth> {
	pub fn line_width(mut self, line_width: f32) -> GraphicsPipelineBuilder<SampleCount> {
		self.rasterization_state.line_width = line_width;
		self.into()
	}
}
impl GraphicsPipelineBuilder<SampleCount> {
	pub fn sample_count(
		mut self, sample_count: vk::SampleCountFlags
	) -> GraphicsPipelineBuilder<SampleShading> {
		self.multisample_state.rasterization_samples = sample_count;
		self.into()
	}
}
impl GraphicsPipelineBuilder<SampleShading> {
	pub fn sample_shading_enable(mut self, min: f32) -> GraphicsPipelineBuilder<SampleMask> {
		self.multisample_state.sample_shading_enable = vk::TRUE;
		self.multisample_state.min_sample_shading = min;
		self.into()
	}
	pub fn sample_shading_disable(mut self) -> GraphicsPipelineBuilder<SampleMask> {
		self.multisample_state.sample_shading_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<SampleMask> {
	pub fn sample_mask(mut self, mask: u32) -> GraphicsPipelineBuilder<AlphaToCoverage> {
		*self.sample_mask = mask;
		self.multisample_state.p_sample_mask = &*self.sample_mask as _;
		self.into()
	}
}
impl GraphicsPipelineBuilder<AlphaToCoverage> {
	pub fn alpha_to_coverage_enable(mut self) -> GraphicsPipelineBuilder<AlphaToOne> {
		self.multisample_state.alpha_to_coverage_enable = vk::TRUE;
		self.into()
	}

	pub fn alpha_to_coverage_disable(mut self) -> GraphicsPipelineBuilder<AlphaToOne> {
		self.multisample_state.alpha_to_coverage_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<AlphaToOne> {
	pub fn alpha_to_one_enable(mut self) -> GraphicsPipelineBuilder<DepthTest> {
		self.multisample_state.alpha_to_one_enable = vk::TRUE;
		self.into()
	}

	pub fn alpha_to_one_disable(mut self) -> GraphicsPipelineBuilder<DepthTest> {
		self.multisample_state.alpha_to_one_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<DepthTest> {
	pub fn depth_test_enable(mut self, op: vk::CompareOp) -> GraphicsPipelineBuilder<DepthWrite> {
		self.depth_stencil_state.depth_test_enable = vk::TRUE;
		self.depth_stencil_state.depth_compare_op = op;
		self.into()
	}

	pub fn depth_test_disable(mut self) -> GraphicsPipelineBuilder<DepthWrite> {
		self.depth_stencil_state.depth_test_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<DepthWrite> {
	pub fn depth_write_enable(mut self) -> GraphicsPipelineBuilder<DepthBounds> {
		self.depth_stencil_state.depth_write_enable = vk::TRUE;
		self.into()
	}

	pub fn depth_write_disable(mut self) -> GraphicsPipelineBuilder<DepthBounds> {
		self.depth_stencil_state.depth_write_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<DepthBounds> {
	pub fn depth_bounds_test_enable(
		mut self, bounds: std::ops::Range<f32>
	) -> GraphicsPipelineBuilder<StencilTest> {
		self.depth_stencil_state.depth_bounds_test_enable = vk::TRUE;
		self.depth_stencil_state.min_depth_bounds = bounds.start;
		self.depth_stencil_state.max_depth_bounds = bounds.end;
		self.into()
	}

	pub fn depth_bounds_test_disable(mut self) -> GraphicsPipelineBuilder<StencilTest> {
		self.depth_stencil_state.depth_bounds_test_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<StencilTest> {
	pub fn stencil_test_enable(
		mut self, front: vk::StencilOpState, back: vk::StencilOpState
	) -> GraphicsPipelineBuilder<LogicOpOrColorBlend> {
		self.depth_stencil_state.stencil_test_enable = vk::TRUE;
		self.depth_stencil_state.front = front;
		self.depth_stencil_state.back = back;
		self.into()
	}

	pub fn stencil_test_disable(mut self) -> GraphicsPipelineBuilder<LogicOpOrColorBlend> {
		self.depth_stencil_state.stencil_test_enable = vk::FALSE;
		self.into()
	}
}
impl GraphicsPipelineBuilder<LogicOpOrColorBlend> {
	pub fn logic_op_enable(mut self, op: vk::LogicOp) -> GraphicsPipelineBuilder<ReadyToBuild> {
		self.color_blend_state.logic_op_enable = vk::TRUE;
		self.color_blend_state.logic_op = op;
		self.into()
	}

	pub fn color_blend(
		mut self, attachments: Vec<vk::PipelineColorBlendAttachmentState>
	) -> GraphicsPipelineBuilder<ReadyToBuild> {
		self.color_blend_state.attachment_count = attachments.len() as u32;
		self.color_blend_state.p_attachments = attachments.as_ptr();
		self.into()
	}
}

impl GraphicsPipelineBuilder<ReadyToBuild> {
	pub fn build<P, L, D, A, S, S1, V>(
		mut self,
		shader_stages: PipelineShaderStagesBuilder<P, L>,
		dynamic_state: GraphicsPipelineDynamicStateBuilder<D>,
		subpass: GraphicsPipelineSubpassSelecter<A, S, S1>,
	) -> GraphicsPipeline<A, S, P, L, V, D> {
		let dynamic_state_info = vk::PipelineDynamicStateCreateInfo {
			s_type: StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineDynamicStateCreateFlags::empty(),
			dynamic_state_count: dynamic_state.dynamic_statess.len() as u32,
			p_dynamic_states: dynamic_state.dynamic_statess.as_ptr(),
		};

		let info = vk::GraphicsPipelineCreateInfo {
			s_type: StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineCreateFlags::empty(),
			stage_count: shader_stages.shader_stages.len() as u32,
			p_stages: shader_stages.shader_stages.as_ptr(),
			p_vertex_input_state: unimplemented!(),
			p_input_assembly_state: &self.input_assembly_state as _,
			p_tessellation_state: &self.tessellation_state as _,
			p_viewport_state: &self.viewport_state as _,
			p_rasterization_state: &self.rasterization_state as _,
			p_multisample_state: &self.multisample_state as _,
			p_depth_stencil_state: &self.depth_stencil_state as _,
			p_color_blend_state: &self.color_blend_state as _,
			p_dynamic_state: &dynamic_state_info as _,
			layout: shader_stages.layout.handle,
			render_pass: subpass.render_pass.handle(),
			subpass: subpass.subpass_index,
			base_pipeline_handle: vk::Pipeline::null(),
			base_pipeline_index: -1,
		};

		let handle = unsafe {
			shader_stages.layout.device.handle
				.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
				.unwrap()[0]

		};

		GraphicsPipeline {
			handle,
			layout: shader_stages.layout,
			render_pass: subpass.render_pass,
			vertex_or_mesh: PhantomData,
			dynamic_states: PhantomData,
		}
	}
}

impl<D> GraphicsPipelineDynamicStateBuilder<D> {
	pub fn dynamic_state<D1>(mut self, _factor: D1) -> GraphicsPipelineDynamicStateBuilder<(D, D1)>
		where D1: DynamicState,
	{
		self.dynamic_statess.push(D1::dynamic_state());

		GraphicsPipelineDynamicStateBuilder {
			dynamic_states: PhantomData,
			dynamic_statess: self.dynamic_statess,
		}
	}
}

impl<A, S, S1, S2> GraphicsPipelineSubpassSelecter<A, S, (S1, S2)> {
	pub fn before_subpass(self) -> GraphicsPipelineSubpassSelecter<A, S, S1> {
		GraphicsPipelineSubpassSelecter {
			render_pass: self.render_pass,
			subpass_index: self.subpass_index - 1,
			subpass: PhantomData,
		}
	}
}
