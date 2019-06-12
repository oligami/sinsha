use ash::vk::ImageUsageFlags;
pub trait ImageUsage {
	fn flags() -> ImageUsageFlags;
}