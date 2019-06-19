use ash::vk;

pub trait DescriptorType {
	fn descriptor_type() -> vk::DescriptorType;
}

#[derive(Copy, Clone)]
pub struct Empty;

macro_rules! impl_descriptor_type {
	($($name:ident, $flag:ident,)*) => {
		$(
			#[derive(Copy, Clone)]
			pub struct $name;
			impl DescriptorType for $name {
				fn descriptor_type() -> vk::DescriptorType { vk::DescriptorType::$flag }
			}
		)*
	};
}

impl_descriptor_type!(
	Sampler, SAMPLER,
	CombinedImageSampler, COMBINED_IMAGE_SAMPLER,
	SampledImage, SAMPLED_IMAGE,
	StorageImage, STORAGE_IMAGE,
	UniformTexelBuffer, UNIFORM_TEXEL_BUFFER,
	StorageTexelBuffer, STORAGE_TEXEL_BUFFER,
	UniformBuffer, UNIFORM_BUFFER,
	StorageBuffer, STORAGE_BUFFER,
	UniformBufferDynamic, UNIFORM_BUFFER_DYNAMIC,
	StorageBufferDynamic, STORAGE_BUFFER_DYNAMIC,
	InputAttachment, INPUT_ATTACHMENT,
	InlineUniformBlockExt, INLINE_UNIFORM_BLOCK_EXT,
	AccelerationStructureNv, ACCELERATION_STRUCTURE_NV,
);