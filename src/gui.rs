use ash::vk;

use crate::vulkan::Vulkan;

use std::ops::Range;

pub struct StartMenu {}

impl StartMenu {
    pub fn commands(&self, command_pool: vk::CommandPool, vulkan: &Vulkan) -> vk::CommandBuffer {

    }
}

