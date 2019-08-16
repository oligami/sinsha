#![allow(unused)]

use ash::vk::PipelineStageFlags;

pub trait PipelineStages {
	fn pipeline_stages() -> PipelineStageFlags;
}

pub struct Empty;
impl PipelineStages for Empty {
	fn pipeline_stages() -> PipelineStageFlags { PipelineStageFlags::empty() }
}

macro_rules! impl_pipeline_stage {
	($($name: ident, $flag: ident,)*) => {$(
		pub struct $name<S>(pub S) where S: PipelineStages;
		impl<S> PipelineStages for $name<S> where S: PipelineStages {
			fn pipeline_stages() -> PipelineStageFlags {
				PipelineStageFlags::$flag | S::pipeline_stages()
			}
		}
	)*};
}

impl_pipeline_stage!(
	TopOfPipe, TOP_OF_PIPE,
	DrawIndirect, DRAW_INDIRECT,
	VertexInput, VERTEX_INPUT,
	VertexShader, VERTEX_SHADER,
	TessellationControlShader, TESSELLATION_CONTROL_SHADER,
	TessellationEvaluationShader, TESSELLATION_EVALUATION_SHADER,
	GeometryShader, GEOMETRY_SHADER,
	FragmentShader, FRAGMENT_SHADER,
	EarlyFragmentTests, EARLY_FRAGMENT_TESTS,
	lateFragmentTests, LATE_FRAGMENT_TESTS,
	ColorAttachmentOutput, COLOR_ATTACHMENT_OUTPUT,
	ComputeShader, COMPUTE_SHADER,
	Transfer, TRANSFER,
	BottomOfPipe, BOTTOM_OF_PIPE,
	Host, HOST,
	AllGraphics, ALL_GRAPHICS,
	AllCommands, ALL_COMMANDS,
	TransformFeedbackExt, TRANSFORM_FEEDBACK_EXT,
	ConditionalRenderingExt, CONDITIONAL_RENDERING_EXT,
	ComandProcessNvx, COMMAND_PROCESS_NVX,
	ShadingRateImageNv, SHADING_RATE_IMAGE_NV,
	RayTracingShaderNv, RAY_TRACING_SHADER_NV,
	AccelerationStructureBuildNv, ACCELERATION_STRUCTURE_BUILD_NV,
	TaskShaderNv, TASK_SHADER_NV,
	MeshShaderNV, MESH_SHADER_NV,
	FragmentDensityProdessExt, FRAGMENT_DENSITY_PROCESS_EXT,
);
