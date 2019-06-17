use ash::vk::BufferUsageFlags;

pub trait BufferUsage {
	fn flags() -> BufferUsageFlags;
}

pub struct None;
pub struct TransferSrcFlag<U>(pub U) where U: BufferUsage;
pub struct TransferDstFlag<U>(pub U) where U: BufferUsage;
pub struct UniformTexelBufferFlag<U>(pub U) where U: BufferUsage;
pub struct StorageTexelBufferFlag<U>(pub U) where U: BufferUsage;
pub struct UniformBufferFlag<U>(pub U) where U: BufferUsage;
pub struct StorageBufferFlag<U>(pub U) where U: BufferUsage;
pub struct IndexBufferFlag<U>(pub U) where U: BufferUsage;
pub struct VertexBufferFlag<U>(pub U) where U: BufferUsage;
pub struct IndirectBufferFlag<U>(pub U) where U: BufferUsage;
pub struct TransformFeedbackBufferExtFlag<U>(pub U) where U: BufferUsage;
pub struct TransformFeedbackCounterBufferExtFlag<U>(pub U) where U: BufferUsage;
pub struct ConditionalRenderingExtFlag<U>(pub U) where U: BufferUsage;
pub struct RayTracingNvFlag<U>(pub U) where U: BufferUsage;
pub struct ShaderDeviceAddressExtFlag<U>(pub U) where U: BufferUsage;

pub trait TransferSrc: BufferUsage {}
pub trait TransferDst: BufferUsage {}
pub trait UniformTexelBuffer: BufferUsage {}
pub trait StorageTexelBuffer: BufferUsage {}
pub trait UniformBuffer: BufferUsage {}
pub trait StorageBuffer: BufferUsage {}
pub trait IndexBuffer: BufferUsage {}
pub trait VertexBuffer: BufferUsage {}
pub trait IndirectBuffer: BufferUsage {}
pub trait TransformFeedbackBufferExt: BufferUsage {}
pub trait TransformFeedbackCounterBufferExt: BufferUsage {}
pub trait ConditionalRenderingExt: BufferUsage {}
pub trait RayTracingNv: BufferUsage {}
pub trait ShaderDeviceAddressExt: BufferUsage {}

pub trait NoTransferSrc: BufferUsage {}
pub trait NoTransferDst: BufferUsage {}
pub trait NoUniformTexelBuffer: BufferUsage {}
pub trait NoStorageTexelBuffer: BufferUsage {}
pub trait NoUniformBuffer: BufferUsage {}
pub trait NoStorageBuffer: BufferUsage {}
pub trait NoIndexBuffer: BufferUsage {}
pub trait NoVertexBuffer: BufferUsage {}
pub trait NoIndirectBuffer: BufferUsage {}
pub trait NoTransformFeedbackBufferExt: BufferUsage {}
pub trait NoTransformFeedbackCounterBufferExt: BufferUsage {}
pub trait NoConditionalRenderingExt: BufferUsage {}
pub trait NoRayTracingNv: BufferUsage {}
pub trait NoShaderDeviceAddressExt: BufferUsage {}

impl BufferUsage for None {
	fn flags() -> BufferUsageFlags { BufferUsageFlags::empty() }
}

macro_rules! impl_buffer_usage {
	($($flag_struct:ident, $flag:ident,)*) => {
		$(impl<U> BufferUsage for $flag_struct<U> where U: BufferUsage {
			fn flags() -> BufferUsageFlags { BufferUsageFlags::$flag | U::flags() }
		})*
	};
}

impl_buffer_usage!(
	TransferSrcFlag, TRANSFER_SRC,
	TransferDstFlag, TRANSFER_DST,
);


macro_rules! impl_no_trait_for_none {
	($($no_trait:ident,)*) => { $(impl $no_trait for None {})* };
}
impl_no_trait_for_none!(
	NoTransferSrc,
	NoTransferDst,
);

macro_rules! impl_no_trait {
	($no_trait:ident, $($other_flag:ident,)*) => {
		$(impl<U> $no_trait for $other_flag<U> where U: $no_trait {})*
	};
}
impl_no_trait!(NoTransferSrc, TransferDstFlag,);

macro_rules! impl_usage_trait {
	($usage_trait:ident, $flag:ident, $no_trait:ident, $($other_flag:ident,)*) => {
		impl<U> $usage_trait for $flag<U> where U: $no_trait {}
		$(impl<U> $usage_trait for $other_flag<U> where U: $usage_trait {})*
	};
}
impl_usage_trait!(TransferSrc, TransferSrcFlag, NoTransferSrc, TransferDstFlag,);

// TRANSFER_SRC
// TRANSFER_DST
// UNIFORM_TEXEL_BUFFER
// STORAGE_TEXEL_BUFFER
// UNIFORM_BUFFER
// STORAGE_BUFFER
// INDEX_BUFFER
// VERTEX_BUFFER
// INDIRECT_BUFFER
// TRANSFORM_FEEDBACK_BUFFER_EXT
// TRANSFORM_FEEDBACK_COUNTER_BUFFER_EXT
// CONDITIONAL_RENDERING_EXT
// RAY_TRACING_NV
// SHADER_DEVICE_ADDRESS_EXT
