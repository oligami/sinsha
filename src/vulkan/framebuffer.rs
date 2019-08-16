use super::*;
use render_pass::RenderPass;
use device_memory::{ DeviceMemory, alloc::Allocator };
use image::{ Image, ImageView, Extent };
use swapchain::{ SwapchainKHR, SwapchainImageView };

// TODO: Add lifetime limitations on vkImageView. Owning vkImage and vkImageView is one of the solutions.
pub struct FrameBuffer<I, D, R, A>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
{
    _marker: PhantomData<(I, D, A)>,
    render_pass: R,
    handle: vk::Framebuffer,
}

pub struct Attachments<IVs> {
    image_views: Vec<vk::ImageView>,
    _marker: PhantomData<IVs>,
}

impl<I, D, R, IVs> FrameBuffer<I, D, R, IVs>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
{
    pub fn new(
        render_pass: R,
        width: u32,
        height: u32,
        layers: u32,
        attachments: Attachments<IVs>,
    ) -> Self {
        let info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass.handle())
            .width(width)
            .height(height)
            .layers(layers)
            .attachments(&attachments.image_views[..]);
        let handle = unsafe { render_pass.device().handle.create_framebuffer(&info, None).unwrap() };

        FrameBuffer {
            _marker: PhantomData,
            render_pass,
            handle,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Framebuffer { self.handle }
}

impl<I, D, R, IVs> Drop for FrameBuffer<I, D, R, IVs>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
{
    fn drop(&mut self) {
        unsafe { self.render_pass.device().handle.destroy_framebuffer(self.handle, None); }
    }
}

impl Attachments<()> {
    pub fn new() -> Self {
        Self { image_views: Vec::new(), _marker: PhantomData }
    }
}
impl<IVs> Attachments<IVs> {
    pub fn add_attachment<I, D, M, Im, IV, A, E>(mut self, image_view: IV) -> Attachments<(IVs, IV)> where
        I: Borrow<Instance> + Deref<Target = Instance>,
        D: Borrow<Device<I>> + Deref<Target = Device<I>>,
        M: Borrow<DeviceMemory<I, D, A>> + Deref<Target = DeviceMemory<I, D, A>>,
        Im: Borrow<Image<I, D, M, A, E>> + Deref<Target = Image<I, D, M, A, E>>,
        IV: Borrow<ImageView<I, D, M, Im, A, E>> + Deref<Target = ImageView<I, D, M, Im, A, E>>,
        A: Allocator,
        E: Extent,
    {
        self.image_views.push(image_view.handle());
        Attachments {
            image_views: self.image_views,
            _marker: PhantomData,
        }
    }

    pub fn add_swapchain<I, S, D, Sw, IV>(mut self, image_view: IV) -> Attachments<(IVs, IV)> where
        I: Borrow<Instance> + Deref<Target = Instance>,
        S: Borrow<SurfaceKHR<I>> + Deref<Target = SurfaceKHR<I>>,
        D: Borrow<Device<I>> + Deref<Target = Device<I>>,
        Sw: Borrow<SwapchainKHR<I, S, D>> + Deref<Target = SwapchainKHR<I, S, D>>,
        IV: Borrow<SwapchainImageView<I, S, D, Sw>> + Deref<Target = SwapchainImageView<I, S, D, Sw>>,
    {
        self.image_views.push(image_view.handle());
        Attachments {
            image_views: self.image_views,
            _marker: PhantomData,
        }
    }
}

