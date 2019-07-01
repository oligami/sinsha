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

pub trait BufferUsage {
	fn buffer_usage() -> BufferUsageFlags;
}

pub struct Empty;
impl BufferUsage for Empty {
	fn buffer_usage() -> BufferUsageFlags { BufferUsageFlags::empty() }
}

macro_rules! impl_buffer_usage {
	($($usage_flag:ident, $flag:ident,)*) => {
		$(
			pub struct $usage_flag<U>(pub U) where U: BufferUsage;

			impl<U> BufferUsage for $usage_flag<U> where U: BufferUsage {
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
	($usage_flag:ident, $usage_trait:ident, $not_trait:ident, $($other_flag:ident,)*) => {
		pub trait $usage_trait: BufferUsage {}
		pub trait $not_trait: BufferUsage {}

		impl<U> $usage_trait for $usage_flag<U> where U: $not_trait {}
		$(impl<U> $usage_trait for $other_flag<U> where U: $usage_trait {})*

		impl $not_trait for Empty {}
		$(impl<U> $not_trait for $other_flag<U> where U: $not_trait {})*
	};
}


impl_usage_trait!(
	TransferSrcFlag,
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


