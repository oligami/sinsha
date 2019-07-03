pub mod stage;
pub mod dynamic_state;

use super::*;
use super::stage::OneShaderStage;
use super::descriptor::DescriptorSetLayout;
use dynamic_state::DynamicState;

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
pub struct Alpha;
pub struct DepthTest;
pub struct DepthWrite;
pub struct DepthBounds;
pub struct DepthCompare;
pub struct StencilTest;
pub struct LogicOp;
pub struct ColorBlend;
pub struct BlendConstant;

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
			offset: 0,
			push_constants: PhantomData,
		}
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
			state: PhantomData,
		}
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

impl<T, U> From<GraphicsPipelineBuilder<T>> for GraphicsPipelineBuilder<U> {
	fn from(builder: GraphicsPipelineBuilder<T>) -> GraphicsPipelineBuilder<U> {
		GraphicsPipelineBuilder {
			input_assembly_state: builder.input_assembly_state,
			tessellation_state: builder.tessellation_state,
			viewport_state: builder.viewport_state,
			rasterization_state: builder.rasterization_state,
			multisample_state: builder.multisample_state,
			depth_stencil_state: builder.depth_stencil_state,
			color_blend_state: builder.color_blend_state,
			viewports: builder.viewports,
			scissors: builder.scissors,
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
	) -> GraphicsPipelineBuilder<SampleMask> {
		self.multisample_state.rasterization_samples = sample_count;
		self.into()
	}
}

impl GraphicsPipelineBuilder {
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

	pub fn build<V, D, A, S, S0>(
		self,
		shader_stages: PipelineShaderStagesBuilder<P, L>,
		_vertex: V,
		(primitive_topology, primitive_restart): (vk::PrimitiveTopology, vk::Bool32),
		patch_control_points: Option<u32>,
		viewport: vk::Viewport,
		scissor: vk::Rect2D,
		rasterizer_discard: vk::Bool32,
		polygon_mode: vk::PolygonMode,
		cull_mode: vk::CullModeFlags,
		front_face: vk::FrontFace,
		depth_clamp: vk::Bool32,
		depth_bias: Option<(f32, f32, f32)>,
		line_width: Option<f32>,
		sample_count: vk::SampleCountFlags,
		sample_shading: Option<f32>,
		sample_mask: Option<u32>,
		alpha: (vk::Bool32, vk::Bool32),
		depth_test: vk::Bool32,
		depth_write: vk::Bool32,
		depth_bounds: Option<std::ops::Range<f32>>,
		depth_compare: vk::CompareOp,
		stencil_test: Option<(vk::StencilOpState, vk::StencilOpState)>,
		logic_op: Option<vk::LogicOp>,
		color_blend_attachments: Vec<vk::PipelineColorBlendAttachmentState>,
		blend_constants: Option<[f32; 4]>,
		dynamic_state: GraphicsPipelineDynamicStateBuilder<D>,
		subpass: GraphicsPipelineSubpassSelecter<A, S, S0>,
	) {
		let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo {
			s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
			topology: primitive_topology,
			primitive_restart_enable: primitive_restart,
		};

		let tessellation_state = patch_control_points.map(|p| {
			vk::PipelineTessellationStateCreateInfo {
				s_type: StructureType::PIPELINE_TESSELLATION_STATE_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::PipelineTessellationStateCreateFlags::empty(),
				patch_control_points: p,
			}
		});

		let viewport_state = vk::PipelineViewportStateCreateInfo {
			s_type: StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineViewportStateCreateFlags::empty(),
			viewport_count: 1,
			p_viewports: &viewport as _,
			scissor_count: 1,
			p_scissors: &scissor as _,
		};

		let (depth_bias, constant, clamp, slope) = match depth_bias {
			Some((a, b, c)) => (vk::TRUE, a, b, c),
			None => (vk::FALSE, 0.0, 0.0, 0.0),
		};

		let rasterization_state = vk::PipelineRasterizationStateCreateInfo {
			s_type: StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineRasterizationStateCreateFlags::empty(),
			rasterizer_discard_enable: rasterizer_discard,
			polygon_mode,
			cull_mode,
			front_face,
			depth_clamp_enable: depth_clamp,
			depth_bias_enable: depth_bias,
			depth_bias_constant_factor: constant,
			depth_bias_clamp: clamp,
			depth_bias_slope_factor: slope,
			line_width: line_width.unwrap_or(1.0),
		};

		let (sample_shading, min_sample_shading) = match sample_shading {
			Some(min) => (vk::TRUE, min),
			None => (vk::FALSE, 0.0),
		};

		let multisample_state = vk::PipelineMultisampleStateCreateInfo {
			s_type: StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineMultisampleStateCreateFlags::empty(),
			rasterization_samples: sample_count,
			sample_shading_enable: sample_shading,
			min_sample_shading,
			p_sample_mask: sample_mask.as_ref().map(|r| r as _).unwrap_or(ptr::null()),
			alpha_to_coverage_enable: alpha.0,
			alpha_to_one_enable: alpha.1,
		};

		let (depth_bounds, min_depth, max_depth) = match depth_bounds {
			Some(range) => (vk::TRUE, range.start, range.end),
			None => (vk::FALSE, 0.0, 0.0),
		};

		let (stencil_test, front, back) = match stencil_test {
			Some((front, back)) => (vk::TRUE, front, back),
			None => (vk::FALSE, vk::StencilOpState::default(), vk::StencilOpState::default()),
		};

		let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
			s_type: StructureType::PIPELINE_DEPTH_STENCIL_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
			depth_test_enable: depth_test,
			depth_write_enable: depth_write,
			depth_compare_op: depth_compare,
			depth_bounds_test_enable: depth_bounds,
			min_depth_bounds: min_depth,
			max_depth_bounds: max_depth,
			stencil_test_enable: stencil_test,
			front,
			back,
		};

		let (logic_op, op) = match logic_op {
			Some(op) => (vk::TRUE, op),
			None => (vk::FALSE, vk::LogicOp::CLEAR),
		};

		let color_blend_state = vk::PipelineColorBlendStateCreateInfo {
			s_type: StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineColorBlendStateCreateFlags::empty(),
			logic_op_enable: logic_op,
			logic_op: op,
			attachment_count: color_blend_attachments.len() as u32,
			p_attachments: color_blend_attachments.as_ptr(),
			blend_constants: blend_constants.unwrap_or([0.0; 4]),
		};

		let dynamic_state = vk::PipelineDynamicStateCreateInfo {
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
			p_input_assembly_state: &input_assembly_state as _,
			p_tessellation_state: tessellation_state.as_ref().map(|t| t as _).unwrap_or(ptr::null()),
			p_viewport_state: &viewport_state as _,
			p_rasterization_state: &rasterization_state as _,
			p_multisample_state: &multisample_state as _,
			p_depth_stencil_state: &depth_stencil_state as _,
			p_color_blend_state: &color_blend_state as _,
			p_dynamic_state: &dynamic_state as _,
			layout: self.layout.handle,
			render_pass: subpass.render_pass.handle(),
			subpass: subpass.subpass_index,
			base_pipeline_handle: vk::Pipeline::null(),
			base_pipeline_index: -1,
		};
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
