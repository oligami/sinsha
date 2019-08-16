use super::*;

/// Host access to CommandPool must be externally synchronized.
/// Synchronization take some cost, so this struct is not Send.
pub struct CommandPool<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<(I, *const ())>,
    device: D,
    handle: vk::CommandPool,
}

pub struct CommandBuffer<I, D, P> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    P: Borrow<CommandPool<I, D>> + Deref<Target = CommandPool<I, D>>,
{
    _marker: PhantomData<(I, D)>,
    pool: P,
    handle: vk::CommandBuffer,
}

impl<I, D> CommandPool<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    pub fn new(
        device: D,
        flags: vk::CommandPoolCreateFlags,
        queue: &Queue<I, D>,
    ) -> Self {
        let info = vk::CommandPoolCreateInfo {
            s_type: StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags,
            queue_family_index: queue.family_index,
        };

        let handle = unsafe { device.handle.create_command_pool(&info, None).unwrap() };

        Self { _marker: PhantomData, device, handle }
    }

    pub unsafe fn reset(&self) {
        self.device.handle
            .reset_command_pool(self.handle, vk::CommandPoolResetFlags::empty())
            .unwrap()
    }
}

impl<I, D, P> CommandBuffer<I, D, P> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    P: Borrow<CommandPool<I, D>> + Deref<Target = CommandPool<I, D>>,
{
    pub fn begin_primary(
        pool: P,
        usage: &[vk::CommandBufferUsageFlags],
    ) -> Vec<Self> where P: Clone {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool.handle)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(usage.len() as u32);

        let handles = unsafe {
            pool.device.handle.allocate_command_buffers(&info).unwrap()
        };

        let command_buffers = handles.into_iter()
            .map(|handle| CommandBuffer { _marker: PhantomData, pool: pool.clone(), handle })
            .collect::<Vec<_>>();

        command_buffers.iter()
            .zip(usage.iter())
            .for_each(|(command_buffer, usage)| {
                let info = vk::CommandBufferBeginInfo::builder()
                    .flags(*usage);

                unsafe {
                    pool.device.handle.begin_command_buffer(command_buffer.handle, &info).unwrap();
                }
            });

        command_buffers
    }

    pub fn begin_secondary(
        pool: P,
        usage: &[vk::CommandBufferUsageFlags],
        inheritances: &[vk::CommandBufferInheritanceInfo],
    ) -> Vec<Self> where P: Clone {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(pool.handle)
            .level(vk::CommandBufferLevel::SECONDARY)
            .command_buffer_count(usage.len() as u32);

        let handles = unsafe {
            pool.device.handle.allocate_command_buffers(&info).unwrap()
        };

        let command_buffers = handles.into_iter()
            .map(|handle| CommandBuffer { _marker: PhantomData, pool: pool.clone(), handle })
            .collect::<Vec<_>>();

        assert_eq!(usage.len(), inheritances.len());
        command_buffers.iter()
            .zip(usage.iter())
            .zip(inheritances.iter())
            .for_each(|((command_buffer, usage), inheritance)| {
                let info = vk::CommandBufferBeginInfo::builder()
                    .inheritance_info(inheritance)
                    .flags(*usage);

                unsafe {
                    pool.device.handle.begin_command_buffer(command_buffer.handle, &info).unwrap();
                }
            });

        command_buffers
    }

    pub fn free(self) {
        unsafe { self.pool.device.handle.free_command_buffers(self.pool.handle, &[self.handle]); }
    }

    #[inline]
    pub fn handle(&self) -> vk::CommandBuffer { self.handle }

    #[inline]
    fn device_handle(&self) -> &VkDevice { &self.pool.device.handle }
}


impl<I, D> Drop for CommandPool<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) {
        unsafe { self.device.handle.destroy_command_pool(self.handle, None); }
    }
}
