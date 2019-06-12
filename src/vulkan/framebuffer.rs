use super::*;
use super::render_pass::VkRenderPass;

pub struct VkFrameBuffer<A, S> {
	device: Arc<VkDevice>,
	handle: vk::Framebuffer,
	attachments: Arc<vk::ImageView>,
	render_pass: Arc<VkRenderPass<A, S>>
}