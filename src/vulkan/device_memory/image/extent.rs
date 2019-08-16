use ash::vk;

use std::ops::Range;

/// Enumerations of ArrayLayers is created by referencing this.
/// Table 7 in https://vulkan.lunarg.com/doc/view/latest/windows/apispec.html#_vkimageviewcreateinfo3
pub trait Extent {
    type ArrayLayers: ArrayLayers;
    fn image_type() -> vk::ImageType;
    fn to_vk_extent_3d(&self) -> vk::Extent3D;
}

pub trait ArrayLayers {
    fn view_type(&self) -> vk::ImageViewType;
    fn base_layer_and_count(&self) -> (u32, u32);
    fn layer_range(&self) -> Range<u32>;
}

pub struct Extent1D {
    pub width: u32,
}

pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

pub enum ArrayLayers1D {
    Normal { base: u32 },
    Array { base: u32, layer_count: u32 },
}

pub enum ArrayLayers2D {
    Normal { base: u32 },
    Array { base: u32, layer_count: u32 },
    Cube { base: u32 },
    CubeArray { base: u32, cube_count: u32 },
}

pub enum ArrayLayers3D {}

impl Extent1D {
    pub fn new(width: u32) -> Self { Self { width } }
}

impl Extent2D {
    pub fn new(width: u32, height: u32) -> Self { Self { width, height } }
}

impl Extent3D {
    pub fn new(width: u32, height: u32, depth: u32) -> Self { Self { width, height, depth } }
}

impl Extent for Extent1D {
    type ArrayLayers = ArrayLayers1D;
    fn image_type() -> vk::ImageType { vk::ImageType::TYPE_1D }
    fn to_vk_extent_3d(&self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.width,
            height: 1,
            depth: 1,
        }
    }
}
impl Extent for Extent2D {
    type ArrayLayers = ArrayLayers2D;
    fn image_type() -> vk::ImageType { vk::ImageType::TYPE_2D }
    fn to_vk_extent_3d(&self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.width,
            height: self.height,
            depth: 1,
        }
    }
}
impl Extent for Extent3D {
    type ArrayLayers = ArrayLayers3D;
    fn image_type() -> vk::ImageType { vk::ImageType::TYPE_3D }
    fn to_vk_extent_3d(&self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.width,
            height: self.height,
            depth: self.depth,
        }
    }
}

impl ArrayLayers for ArrayLayers1D {
    fn view_type(&self) -> vk::ImageViewType {
        match self {
            ArrayLayers1D::Normal { .. } => vk::ImageViewType::TYPE_1D,
            ArrayLayers1D::Array { .. } => vk::ImageViewType::TYPE_1D_ARRAY,
        }
    }

    fn base_layer_and_count(&self) -> (u32, u32) {
        match self {
            ArrayLayers1D::Normal { base } => (*base, 1),
            ArrayLayers1D::Array { base, layer_count } => (*base, *layer_count),
        }
    }

    fn layer_range(&self) -> Range<u32> {
        match self {
            ArrayLayers1D::Normal { base } => *base .. *base + 1,
            ArrayLayers1D::Array { base, layer_count } => *base .. *base + *layer_count,
        }
    }
}
impl ArrayLayers for ArrayLayers2D {
    fn view_type(&self) -> vk::ImageViewType {
        match self {
            ArrayLayers2D::Normal { .. } => vk::ImageViewType::TYPE_2D,
            ArrayLayers2D::Array { .. } => vk::ImageViewType::TYPE_2D_ARRAY,
            ArrayLayers2D::Cube { .. } => vk::ImageViewType::TYPE_2D,
            ArrayLayers2D::CubeArray { .. } => vk::ImageViewType::TYPE_2D_ARRAY,
        }
    }

    fn base_layer_and_count(&self) -> (u32, u32) {
        match self {
            ArrayLayers2D::Normal { base } => (*base, 1),
            ArrayLayers2D::Array { base, layer_count } => (*base, *layer_count),
            ArrayLayers2D::Cube { base } => (*base, 6),
            ArrayLayers2D::CubeArray { base, cube_count } => (*base, *cube_count * 6),
        }
    }

    fn layer_range(&self) -> Range<u32> {
        match self {
            ArrayLayers2D::Normal { base } => *base .. *base + 1,
            ArrayLayers2D::Array { base, layer_count } => *base .. *base + *layer_count,
            ArrayLayers2D::Cube { base } => *base .. *base + 6,
            ArrayLayers2D::CubeArray { base, cube_count } => *base .. * base + *cube_count * 6,
        }
    }
}
impl ArrayLayers for ArrayLayers3D {
    fn view_type(&self) -> vk::ImageViewType { vk::ImageViewType::TYPE_3D }
    fn base_layer_and_count(&self) -> (u32, u32) { (0, 1) }
    fn layer_range(&self) -> Range<u32> { 0..1 }
}