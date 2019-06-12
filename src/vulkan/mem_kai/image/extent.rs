use ash::vk;

pub trait Extent {
	fn image_type() -> vk::ImageType;
	fn view_type(layer_count: u32) -> vk::ImageViewType;
	fn extent(self) -> vk::Extent3D;
}

pub struct Extent1D {
	width: u32,
}

pub struct Extent2D {
	width: u32,
	height: u32,
}

pub struct Extent3D {
	width: u32,
	height: u32,
	depth: u32,
}

impl Extent for Extent1D {
	fn image_type() -> vk::ImageType { vk::ImageType::TYPE_1D }
	fn view_type(layer_count: u32) -> vk::ImageViewType {
		if layer_count == 1 {
			vk::ImageViewType::TYPE_1D
		} else {
			vk::ImageViewType::TYPE_1D_ARRAY
		}
	}
	fn extent(self) -> vk::Extent3D {
		vk::Extent3D {
			width: self.width,
			height: 1,
			depth: 1,
		}
	}
}
impl Extent for Extent2D {
	fn image_type() -> vk::ImageType { vk::ImageType::TYPE_2D }
	fn view_type(layer_count: u32) -> vk::ImageViewType {
		if layer_count == 1 {
			vk::ImageViewType::TYPE_2D
		} else {
			vk::ImageViewType::TYPE_2D_ARRAY
		}
	}
	fn extent(self) -> vk::Extent3D {
		vk::Extent3D {
			width: self.width,
			height: self.height,
			depth: 1,
		}
	}
}
impl Extent for Extent3D {
	fn image_type() -> vk::ImageType { vk::ImageType::TYPE_3D }
	fn view_type(layer_count: u32) -> vk::ImageViewType {
		if layer_count == 1 {
			vk::ImageViewType::TYPE_3D
		} else {
			vk::ImageViewType::TYPE_3D_ARRAY
		}
	}
	fn extent(self) -> vk::Extent3D {
		vk::Extent3D {
			width: self.width,
			height: self.height,
			depth: self.depth,
		}
	}
}