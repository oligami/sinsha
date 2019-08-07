//! Implemented traits are below.
//! TransferSrcFlag
//! TransferDstFlag
//! UniformTexelBufferFlag
//! StorageTexelBufferFlag
//! UniformBufferFlag
//! StorageBufferFlag
//! IndexBufferFlag
//! VertexBufferFlag
//! IndirectBufferFlag
//! TransformFeedbackBufferExtFlag
//! TransformFeedbackCounterBufferExtFlag
//! ConditionalRenderingExtFlag
//! RayTracingNvFlag
//! ShaderDeviceAddressExtFlag
//!
//! Each trait represents each flag below.
//! TRANSFER_SRC
//! TRANSFER_DST
//! UNIFORM_TEXEL_BUFFER
//! STORAGE_TEXEL_BUFFER
//! UNIFORM_BUFFER
//! STORAGE_BUFFER
//! INDEX_BUFFER
//! VERTEX_BUFFER
//! INDIRECT_BUFFER
//! TRANSFORM_FEEDBACK_BUFFER_EXT
//! TRANSFORM_FEEDBACK_COUNTER_BUFFER_EXT
//! CONDITIONAL_RENDERING_EXT
//! RAY_TRACING_NV
//! SHADER_DEVICE_ADDRESS_EXT

#![allow(unused)]

use ash::vk::BufferUsageFlags;

use super::utility::TypeIterEnd;


pub trait BufferUsage {
	fn buffer_usage() -> BufferUsageFlags;
}

pub struct BufferUsageBuilder<T>(T) where T: BufferUsage;
pub fn builder() -> BufferUsageBuilder<Empty> { BufferUsageBuilder(Empty) }
impl<T> BufferUsageBuilder<T> where T: BufferUsage {
	pub fn build(self) -> T { self.0 }
}


pub struct Empty;
impl TypeIterEnd for Empty {}
impl BufferUsage for Empty {
	fn buffer_usage() -> BufferUsageFlags { BufferUsageFlags::empty() }
}


macro_rules! impl_buffer_usage {
	($($usage_flag:ident, $flag:ident,)*) => {
		$(
			pub struct $usage_flag;

			impl<U> BufferUsage for (U, $usage_flag) where U: BufferUsage {
				fn buffer_usage() -> BufferUsageFlags {
					BufferUsageFlags::$flag | U::buffer_usage()
				}
			}
		)*
	};
}

impl_buffer_usage!(
	TransferSrcFlag, TRANSFER_SRC,
	TransferDstFlag, TRANSFER_DST,
	UniformTexelBufferFlag, UNIFORM_TEXEL_BUFFER,
	StorageTexelBufferFlag, STORAGE_TEXEL_BUFFER,
	UniformBufferFlag, UNIFORM_BUFFER,
	StorageBufferFlag, STORAGE_BUFFER,
	IndexBufferFlag, INDEX_BUFFER,
	VertexBufferFlag, VERTEX_BUFFER,
	IndirectBufferFlag, INDIRECT_BUFFER,
	TransformFeedbackBufferExtFlag, TRANSFORM_FEEDBACK_BUFFER_EXT,
	TransformFeedbackCounterBufferExtFlag, TRANSFORM_FEEDBACK_COUNTER_BUFFER_EXT,
	ConditionalRenderingExtFlag, CONDITIONAL_RENDERING_EXT,
	RayTracingNvFlag, RAY_TRACING_NV,
	ShaderDeviceAddressExtFlag, SHADER_DEVICE_ADDRESS_EXT,
);

macro_rules! impl_usage_trait {
	($usage_flag:ident, $usage_fn:ident, $usage_trait:ident, $not_trait:ident, $($other_usage_flag:ident,)*) => {
		pub trait $usage_trait: BufferUsage {}
		pub trait $not_trait: BufferUsage {}

		impl<U> $usage_trait for (U, $usage_flag) where U: $not_trait {}
		$(impl<U> $usage_trait for (U, $other_usage_flag) where U: $usage_trait {})*

		impl $not_trait for Empty {}
		$(impl<U> $not_trait for (U, $other_usage_flag) where U: $not_trait {})*

		impl<T> BufferUsageBuilder<T> where T: $not_trait {
			pub fn $usage_fn(self) -> BufferUsageBuilder<(T, $usage_flag)> {
				BufferUsageBuilder((self.0, $usage_flag))
			}
		}
	};
}


impl_usage_trait!(
	TransferSrcFlag,
	transfer_src,
	TransferSrc,
	NotTransferSrc,
		// TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	TransferDstFlag,
	transfer_dst,
	TransferDst,
	NotTransferDst,
		TransferSrcFlag,
		// TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	UniformTexelBufferFlag,
	uniform_texel_buffer,
	UniformTexelBuffer,
	NotUniformTexelBuffer,
		TransferSrcFlag,
		TransferDstFlag,
		// UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	StorageTexelBufferFlag,
	storage_texel_buffer,
	StorageTexelBuffer,
	NotStorageTexelBuffer,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		// StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	UniformBufferFlag,
	uniform_buffer,
	UniformBuffer,
	NotUniformBuffer,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		// UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	StorageBufferFlag,
	storage_buffer,
	StorageBuffer,
	NotStorageBuffer,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		// StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	IndexBufferFlag,
	index_buffer,
	IndexBuffer,
	NotIndexBuffer,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		// IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	VertexBufferFlag,
	vertex_buffer,
	VertexBuffer,
	NotVertexBuffer,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		// VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	IndirectBufferFlag,
	indirect_buffer,
	IndirectBuffer,
	NotIndirectBuffer,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		// IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	TransformFeedbackBufferExtFlag,
	transform_feedback_buffer_ext,
	TransformFeedbackBufferExt,
	NotTransformFeedbackBufferExt,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		// TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	TransformFeedbackCounterBufferExtFlag,
	transform_feedback_counter_buffer_ext,
	TransformFeedbackCounterBufferExt,
	NotTransformFeedbackCounterBufferExt,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		// TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	ConditionalRenderingExtFlag,
	conditional_rendering_ext,
	ConditionalRenderingExt,
	NotConditionalRenderingExt,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		// ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	RayTracingNvFlag,
	ray_tracing_nv,
	RayTracingNv,
	NotRayTracingNv,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		// RayTracingNvFlag,
		ShaderDeviceAddressExtFlag,
);

impl_usage_trait!(
	ShaderDeviceAddressExtFlag,
	shader_device_address_ext,
	ShaderDeviceAddressExt,
	NotShaderDeviceAddressExt,
		TransferSrcFlag,
		TransferDstFlag,
		UniformTexelBufferFlag,
		StorageTexelBufferFlag,
		UniformBufferFlag,
		StorageBufferFlag,
		IndexBufferFlag,
		VertexBufferFlag,
		IndirectBufferFlag,
		TransformFeedbackBufferExtFlag,
		TransformFeedbackCounterBufferExtFlag,
		ConditionalRenderingExtFlag,
		RayTracingNvFlag,
		// ShaderDeviceAddressExtFlag,
);


