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

pub use inner::{
    ImageUsages,
    ImageUsage,
    TransferSrc,
    TransferDst,
    Sampled,
    Storage,
    ColorAttachment,
    DepthStencilAttachment,
    TransientAttachment,
    InputAttachment,
    ShadingRateImageNv,
    FragmentDensityMapExt,
};

mod inner {
    use ash::vk;
    use std::marker::PhantomData;

    pub struct ImageUsages<U>(PhantomData<U>);

    pub trait ImageUsage {
        fn image_usage() -> vk::ImageUsageFlags;
    }

    pub trait ImageUsageFlag {
        fn flag() -> vk::ImageUsageFlags;
    }

    pub struct Not;
    pub struct TransferSrcFlag;
    pub struct TransferDstFlag;
    pub struct SampledFlag;
    pub struct StorageFlag;
    pub struct ColorAttachmentFlag;
    pub struct DepthStencilAttachmentFlag;
    pub struct TransientAttachmentFlag;
    pub struct InputAttachmentFlag;
    pub struct ShadingRateImageNvFlag;
    pub struct FragmentDensityMapExtFlag;
    pub trait TransferSrc {}
    pub trait TransferDst {}
    pub trait Sampled {}
    pub trait Storage {}
    pub trait ColorAttachment {}
    pub trait DepthStencilAttachment {}
    pub trait TransientAttachment {}
    pub trait InputAttachment {}
    pub trait ShadingRateImageNv {}
    pub trait FragmentDensityMapExt {}

    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9, U10> ImageUsage
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9, U10)> where
            U1: ImageUsageFlag,
            U2: ImageUsageFlag,
            U3: ImageUsageFlag,
            U4: ImageUsageFlag,
            U5: ImageUsageFlag,
            U6: ImageUsageFlag,
            U7: ImageUsageFlag,
            U8: ImageUsageFlag,
            U9: ImageUsageFlag,
            U10: ImageUsageFlag,
    {
        fn image_usage() -> vk::ImageUsageFlags {
            U1::flag() | U2::flag() | U3::flag() | U4::flag() | U5::flag()
                | U6::flag() | U7::flag() | U8::flag() | U9::flag() | U10::flag()
        }
    }

    impl ImageUsageFlag for Not {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::empty() }
    }
    impl ImageUsageFlag for TransferSrcFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::TRANSFER_SRC }
    }
    impl ImageUsageFlag for TransferDstFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::TRANSFER_DST }
    }
    impl ImageUsageFlag for SampledFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::SAMPLED }
    }
    impl ImageUsageFlag for StorageFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags:: STORAGE }
    }
    impl ImageUsageFlag for ColorAttachmentFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::COLOR_ATTACHMENT }
    }
    impl ImageUsageFlag for DepthStencilAttachmentFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT }
    }
    impl ImageUsageFlag for TransientAttachmentFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::TRANSIENT_ATTACHMENT }
    }
    impl ImageUsageFlag for InputAttachmentFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::INPUT_ATTACHMENT }
    }
    impl ImageUsageFlag for ShadingRateImageNvFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::SHADING_RATE_IMAGE_NV }
    }
    impl ImageUsageFlag for FragmentDensityMapExtFlag {
        fn flag() -> vk::ImageUsageFlags { vk::ImageUsageFlags::FRAGMENT_DENSITY_MAP_EXT }
    }

    impl ImageUsages<(Not, Not, Not, Not, Not, Not, Not, Not, Not, Not)> {
        pub fn empty() -> Self { ImageUsages(PhantomData) }
    }

    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> TransferSrc
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(Not, U1, U2, U3, U4, U5, U6, U7, U8, U9)> {
        pub fn transfer_src(self) -> ImageUsages<(TransferSrcFlag, U1, U2, U3, U4, U5, U6, U7, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> TransferDst
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, Not, U2, U3, U4, U5, U6, U7, U8, U9)> {
        pub fn transfer_dst(self) -> ImageUsages<(U1, TransferDstFlag, U2, U3, U4, U5, U6, U7, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> Sampled
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, Not, U3, U4, U5, U6, U7, U8, U9)> {
        pub fn sampled(self) -> ImageUsages<(U1, U2, SampledFlag, U3, U4, U5, U6, U7, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> Storage
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, U3, Not, U4, U5, U6, U7, U8, U9)> {
        pub fn storage(self) -> ImageUsages<(U1, U2, U3, StorageFlag, U4, U5, U6, U7, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ColorAttachment
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, U3, U4, Not, U5, U6, U7, U8, U9)> {
        pub fn color_attachment(self) -> ImageUsages<(U1, U2, U3, U4, ColorAttachmentFlag, U5, U6, U7, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> DepthStencilAttachment
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, U3, U4, U5, Not, U6, U7, U8, U9)> {
        pub fn depth_stencil_attachment(self) -> ImageUsages<(U1, U2, U3, U4, U5, DepthStencilAttachmentFlag, U6, U7, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> TransientAttachment
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, U3, U4, U5, U6, Not, U7, U8, U9)> {
        pub fn transient_attachment(self) -> ImageUsages<(U1, U2, U3, U4, U5, U6, TransientAttachmentFlag, U7, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> InputAttachment
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, U3, U4, U5, U6, U7, Not, U8, U9)> {
        pub fn input_attachment(self) -> ImageUsages<(U1, U2, U3, U4, U5, U6, U7, InputAttachmentFlag, U8, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ShadingRateImageNv
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, Not, U9)> {
        pub fn shading_rate_image_nv(self) -> ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, ShadingRateImageNvFlag, U9)> {
            ImageUsages(PhantomData)
        }
    }
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> FragmentDensityMapExt
        for ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9)> {}
    impl<U1, U2, U3, U4, U5, U6, U7, U8, U9> ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9, Not)> {
        pub fn fragment_density_map_ext(self) -> ImageUsages<(U1, U2, U3, U4, U5, U6, U7, U8, U9, FragmentDensityMapExtFlag)> {
            ImageUsages(PhantomData)
        }
    }
}

