use ash::vk;
use super::utility::TypeIterEnd;

pub trait DescriptorType {
	fn descriptor_type() -> vk::DescriptorType;
}

pub trait Buffer {}
pub trait Image {}
pub trait BufferView {}
pub trait PNext {}

pub struct Empty;
impl TypeIterEnd for Empty {}

macro_rules! impl_descriptor_type {
	($($name: ident, $flag: ident, $resource: ident,)*) => {
		$(
			pub struct $name;
			impl DescriptorType for $name {
				fn descriptor_type() -> vk::DescriptorType { vk::DescriptorType::$flag }
			}
			impl $resource for $name {}
		)*
	};
}

impl_descriptor_type!(
	Sampler, SAMPLER, Image,
	CombinedImageSampler, COMBINED_IMAGE_SAMPLER, Image,
	SampledImage, SAMPLED_IMAGE, Image,
	StorageImage, STORAGE_IMAGE, Image,
	UniformTexelBuffer, UNIFORM_TEXEL_BUFFER, BufferView,
	StorageTexelBuffer, STORAGE_TEXEL_BUFFER, BufferView,
	UniformBuffer, UNIFORM_BUFFER, Buffer,
	StorageBuffer, STORAGE_BUFFER, Buffer,
	UniformBufferDynamic, UNIFORM_BUFFER_DYNAMIC, Buffer,
	StorageBufferDynamic, STORAGE_BUFFER_DYNAMIC, Buffer,
	InputAttachment, INPUT_ATTACHMENT, Image,
	InlineUniformBlockExt, INLINE_UNIFORM_BLOCK_EXT, PNext,
	AccelerationStructureNv, ACCELERATION_STRUCTURE_NV, PNext,
);