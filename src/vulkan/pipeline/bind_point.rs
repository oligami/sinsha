use ash::vk::PipelineBindPoint as VkPipelineBindPoint;
pub struct Input;

pub trait PipelineBindPoint {
	fn bind_point() -> VkPipelineBindPoint;
}
pub struct Graphics;
pub struct Compute;
impl PipelineBindPoint for Graphics {
	fn bind_point() -> VkPipelineBindPoint { VkPipelineBindPoint::GRAPHICS }
}
impl PipelineBindPoint for Compute {
	fn bind_point() -> VkPipelineBindPoint { VkPipelineBindPoint::COMPUTE }
}