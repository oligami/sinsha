use super::*;

pub struct Device {
    pub(super) entry: Entry,
    pub(super) instance: VkInstance,
    pub(super) physical_device: PhysicalDevice,
    pub(super) device: VkDevice,
}

pub struct Queue<D: Borrow<Device>> {
    device: D,
    handle: vk::Queue,
}