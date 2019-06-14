use ash::vk::ImageUsageFlags;
pub trait ImageUsage {
	fn flags() -> ImageUsageFlags;
}

pub struct ColorAttachment;
impl ImageUsage for ColorAttachment {
	fn flags() -> ImageUsageFlags { ImageUsageFlags::COLOR_ATTACHMENT }
}