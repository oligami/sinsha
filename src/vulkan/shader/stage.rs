use ash::vk::ShaderStageFlags;

pub trait ShaderStage {
	fn shader_stage() -> ShaderStageFlags;
}

#[derive(Copy, Clone)]
pub struct Empty;
impl ShaderStage for Empty {
	fn shader_stage() -> ShaderStageFlags { ShaderStageFlags::empty() }
}

macro_rules! impl_shader_stage {
	($($name:ident, $flag:ident,)*) => {
		$(
			#[derive(Copy, Clone)]
			pub struct $name<S>(pub S) where S: ShaderStage;
			impl<S> ShaderStage for $name<S> where S: ShaderStage {
				fn shader_stage() -> ShaderStageFlags {
					ShaderStageFlags::$flag | S::shader_stage()
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

