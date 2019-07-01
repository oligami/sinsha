use ash::vk::ShaderStageFlags;

pub trait ShaderStages {
	fn shader_stages() -> ShaderStageFlags;
}

pub trait OneShaderStage: ShaderStages {}

#[derive(Copy, Clone)]
pub struct Empty;
impl ShaderStages for Empty {
	fn shader_stages() -> ShaderStageFlags { ShaderStageFlags::empty() }
}

macro_rules! impl_shader_stage {
	($($name:ident, $flag:ident,)*) => {
		$(
			#[derive(Copy, Clone)]
			pub struct $name<S>(pub S) where S: ShaderStages;
			impl<S> ShaderStages for $name<S> where S: ShaderStages {
				fn shader_stages() -> ShaderStageFlags {
					ShaderStageFlags::$flag | S::shader_stages()
				}
			}
		)*
	};
}

impl_shader_stage!(
	Vertex, VERTEX,
	TessellationControl, TESSELLATION_CONTROL,
	TessellationEvaluation, TESSELLATION_EVALUATION,
	Geometry, GEOMETRY,
	Fragment, FRAGMENT,
	Compute, COMPUTE,
	AllGraphics, ALL_GRAPHICS,
	All, ALL,
	RaygenNv, RAYGEN_NV,
	AnyHitNv, ANY_HIT_NV,
	ClosestHitNv, CLOSEST_HIT_NV,
	MissNv, MISS_NV,
	IntersectionNv, INTERSECTION_NV,
	CallableNv, CALLABLE_NV,
	TaskNv, TASK_NV,
	MeshNv, MESH_NV,
);

macro_rules! impl_one_shader_stage {
	($($name: ident,)*) => {$( impl OneShaderStage for $name<Empty> {}  )*};
}

impl_one_shader_stage!(
	Vertex,
	TessellationControl,
	TessellationEvaluation,
	Geometry,
	Fragment,
	Compute,
	RaygenNv,
	AnyHitNv,
	ClosestHitNv,
	MissNv,
	IntersectionNv,
	CallableNv,
	TaskNv,
	MeshNv,
);

