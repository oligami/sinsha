//! Implemented traits are below.
//! TransferSrc
//! TransferDst
//! Sampled
//! Storage
//! ColorAttachment
//! DepthStencilAttachment
//! TransientAttachment
//! InputAttachment
//! ShadingRateImageNv
//! FragmentDensityMapExt
//!
//! Each trait represents each flag below.
//! TRANSFER_SRC
//! TRANSFER_DST
//! SAMPLED
//! STORAGE
//! COLOR_ATTACHMENT
//! DEPTH_STENCIL_ATTACHMENT
//! TRANSIENT_ATTACHMENT
//! INPUT_ATTACHMENT
//! SHADING_RATE_IMAGE_NV
//! FRAGMENT_DENSITY_MAP_EXT

#![allow(unused)]

use ash::vk::ImageUsageFlags;
pub trait ImageUsage {
	fn image_usage() -> ImageUsageFlags;
}

pub struct Empty;
impl ImageUsage for Empty {
	fn image_usage() -> ImageUsageFlags { ImageUsageFlags::empty() }
}

macro_rules! impl_image_usage {
	($($usage_flag:ident, $flag:ident,)*) => {
		$(
			pub struct $usage_flag<U>(pub U) where U: ImageUsage;

			impl<U> ImageUsage for $usage_flag<U> where U: ImageUsage {
				fn image_usage() -> ImageUsageFlags { ImageUsageFlags::$flag | U::image_usage() }
			}
		)*
	};
}

impl_image_usage!(
	TransferSrcFlag, TRANSFER_SRC,
	TransferDstFlag, TRANSFER_DST,
	SampledFlag, SAMPLED,
	StorageFlag, STORAGE,
	ColorAttachmentFlag, COLOR_ATTACHMENT,
	DepthStencilAttachmentFlag, DEPTH_STENCIL_ATTACHMENT,
	TransientAttachmentFlag, TRANSIENT_ATTACHMENT,
	InputAttachmentFlag, INPUT_ATTACHMENT,
	ShadingRateImageNvFlag, SHADING_RATE_IMAGE_NV,
	FragmentDensityMapExtFlag, FRAGMENT_DENSITY_MAP_EXT,
);


macro_rules! impl_usage_trait {
	($usage_flag:ident, $usage_trait:ident, $not_trait:ident, $($other_flag:ident,)*) => {
		pub trait $usage_trait: ImageUsage {}
		pub trait $not_trait: ImageUsage {}

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
		SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	TransferDstFlag,
	TransferDst,
	NotTransferDst,
		TransferSrcFlag,
		// TransferDstFlag,
		SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	SampledFlag,
	Sampled,
	NotSampled,
		TransferSrcFlag,
		TransferDstFlag,
		// SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	StorageFlag,
	Storage,
	NotStorage,
		TransferSrcFlag,
		TransferDstFlag,
		SampledFlag,
		// StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	ColorAttachmentFlag,
	ColorAttachment,
	NotColorAttachment,
		TransferSrcFlag,
		TransferDstFlag,
		SampledFlag,
		StorageFlag,
		// ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	DepthStencilAttachmentFlag,
	DepthStencilAttachment,
	NotDepthStencilAttachment,
		TransferSrcFlag,
		TransferDstFlag,
		SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		// DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	TransientAttachmentFlag,
	TransientAttachment,
	NotTransientAttachment,
		TransferSrcFlag,
		TransferDstFlag,
		SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		// TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	InputAttachmentFlag,
	InputAttachment,
	NotInputAttachment,
		TransferSrcFlag,
		TransferDstFlag,
		SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		// InputAttachmentFlag,
		ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	ShadingRateImageNvFlag,
	ShadingRateImageNv,
	NotShadingRateImageNv,
		TransferSrcFlag,
		TransferDstFlag,
		SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		// ShadingRateImageNvFlag,
		FragmentDensityMapExtFlag,
);

impl_usage_trait!(
	FragmentDensityMapExtFlag,
	FragmentDensityMapExt,
	NotFragmentDensityMapExt,
		TransferSrcFlag,
		TransferDstFlag,
		SampledFlag,
		StorageFlag,
		ColorAttachmentFlag,
		DepthStencilAttachmentFlag,
		TransientAttachmentFlag,
		InputAttachmentFlag,
		ShadingRateImageNvFlag,
		// FragmentDensityMapExtFlag,
);

