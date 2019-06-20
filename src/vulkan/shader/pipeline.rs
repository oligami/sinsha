pub mod stage;

use super::*;

pub struct VkPipelineLayout<L> {
	device: Arc<VkDevice>,
	handle: vk::PipelineLayout,
	descriptor_layouts: L,
}

