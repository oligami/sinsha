use super::*;

pub struct RenderPass<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
     device: D,
    handle: vk::RenderPass,
}


impl<I, D> RenderPass<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    pub fn new(
        device: D,
        attachments: &[vk::AttachmentDescription],
        subpasses: &[vk::SubpassDescription],
        dependencies: &[vk::SubpassDependency],
    ) -> Self {
        let info = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(subpasses)
            .dependencies(dependencies);
        let handle = unsafe { device.handle.create_render_pass(&info, None).unwrap() };

        RenderPass { _marker: PhantomData, device, handle }
    }

    #[inline]
    pub fn device(&self) -> &Device<I> { &self.device }
    #[inline]
    pub fn handle(&self) -> vk::RenderPass { self.handle }
}

impl<I, D> Drop for RenderPass<I, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_render_pass(self.handle, None);
        }
    }
}

