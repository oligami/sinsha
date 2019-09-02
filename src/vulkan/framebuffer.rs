use super::*;
use render_pass::RenderPass;
use device_memory::{ DeviceMemory, alloc::Allocator };
use image::{ Image, ImageView, Extent };
use swapchain::{ SwapchainKHR, SwapchainImageView };

pub struct FrameBuffer<I, D, R, IVs> where
    D: Borrow<Device<I>>,
    R: Borrow<RenderPass<I, D>>,
{
    _marker: PhantomData<(I, D)>,
    render_pass: R,
    handle: vk::Framebuffer,
    attachments: IVs,
}

pub struct Attachments<IVs> {
    image_views: Vec<vk::ImageView>,
    actual_attachments: IVs,
}

impl<I, D, R, IVs> FrameBuffer<I, D, R, IVs> where
    D: Borrow<Device<I>>,
    R: Borrow<RenderPass<I, D>>,
{
    pub fn new(
        render_pass: R,
        width: u32,
        height: u32,
        layers: u32,
        attachments: Attachments<IVs>,
    ) -> Self {
        let info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass.borrow().handle())
            .width(width)
            .height(height)
            .layers(layers)
            .attachments(&attachments.image_views[..]);
        let handle = unsafe { render_pass.borrow().device().handle.create_framebuffer(&info, None).unwrap() };

        FrameBuffer {
            _marker: PhantomData,
            render_pass,
            handle,
            attachments: attachments.actual_attachments,
        }
    }

    #[inline]
    pub fn handle(&self) -> vk::Framebuffer { self.handle }
}

impl<I, D, R, IVs> Drop for FrameBuffer<I, D, R, IVs> where
    D: Borrow<Device<I>>,
    R: Borrow<RenderPass<I, D>>,
{
    fn drop(&mut self) {
        unsafe { self.render_pass.borrow().device().handle.destroy_framebuffer(self.handle, None); }
    }
}

impl Attachments<()> {
    pub fn new() -> Self {
        Self { image_views: Vec::new(), actual_attachments: () }
    }
}
impl<IVs> Attachments<IVs> {
    pub fn add_attachment<I, D, M, Im, IV, A, E>(mut self, image_view: IV) -> Attachments<(IVs, IV)> where
        D: Borrow<Device<I>>,
        M: Borrow<DeviceMemory<I, D, A>>,
        Im: Borrow<Image<I, D, M, A, E>>,
        IV: Borrow<ImageView<I, D, M, Im, A, E>>,
        A: Allocator,
    {
        self.image_views.push(image_view.borrow().handle());
        Attachments {
            image_views: self.image_views,
            actual_attachments: (self.actual_attachments, image_view)
        }
    }

    pub fn add_swapchain<I, S, D, Sw, IV>(mut self, image_view: IV) -> Attachments<(IVs, IV)> where
        S: Borrow<SurfaceKHR<I>>,
        D: Borrow<Device<I>>,
        Sw: Borrow<SwapchainKHR<I, S, D>>,
        IV: Borrow<SwapchainImageView<I, S, D, Sw>>,
    {
        self.image_views.push(image_view.borrow().handle());
        Attachments {
            image_views: self.image_views,
            actual_attachments: (self.actual_attachments, image_view)
        }
    }
}

