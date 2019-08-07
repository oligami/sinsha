use super::*;
use device_memory::image::*;
use render_pass::{ RenderPass, RenderPassAbs };
//use swapchain::VkSwapchainImageView;

pub struct FrameBuffer<I, D, R, V>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
{
    _marker: PhantomData<(I, D)>,
    render_pass: R,
    handle: vk::Framebuffer,
    image_views: V,
}

pub struct FrameBufferBuilder<I, D, R, V> {
    _marker: PhantomData<(I, D)>,
    render_pass: R,
    w_h_l: (u32, u32, u32),
    view_handles: Vec<vk::ImageView>,
    image_views: V,
}

impl<I, D, R> FrameBuffer<I, D, R, ()>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
{
    pub fn builder(render_pass: R, width: u32, height: u32, layers: u32) -> FrameBufferBuilder<I, D, R, ()> {
        FrameBufferBuilder {
            _marker: PhantomData,
            render_pass,
            w_h_l: (width, height, layers),
            view_handles: Vec::new(),
            image_views: (),
        }
    }
}

impl<I, D, R, V> Drop for FrameBuffer<I, D, R, V>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
{
    fn drop(&mut self) {
        unsafe { self.render_pass.device().handle.destroy_framebuffer(self.handle, None); }
    }
}

impl<I, D, R, V> FrameBufferBuilder<I, D, R, V>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
          R: Borrow<RenderPass<I, D>> + Deref<Target = RenderPass<I, D>>,
{
    pub fn attach_image_view<BIV, IV>(
        mut self,
        image_view: BIV,
    ) -> FrameBufferBuilder<I, D, R, (V, BIV)> where BIV: Borrow<IV> + Deref<Target = IV>, IV: ImageViewAbs {
        assert!(self.view_handles.len() < self.render_pass.attachments().len());

        let extent = image_view.extent().extent();
        let layer_range = image_view.layer_range();
        let attachment = &self.render_pass.attachments()[self.view_handles.len()];

        assert_eq!(image_view.format(), attachment.format());
        assert_eq!(image_view.samples(), attachment.samples());
        assert_eq!(extent.width, self.w_h_l.0);
        assert_eq!(extent.height, self.w_h_l.1);
        assert_eq!(layer_range.end - layer_range.start, self.w_h_l.2);

        self.view_handles.push(image_view.handle());

        FrameBufferBuilder {
            _marker: PhantomData,
            render_pass: self.render_pass,
            w_h_l: self.w_h_l,
            view_handles: self.view_handles,
            image_views: (self.image_views, image_view),
        }
    }

    pub fn build(self) -> FrameBuffer<I, D, R, V> {
        assert_eq!(self.view_handles.len(), self.render_pass.attachments().len());

        let handle = unsafe {
            let info = vk::FramebufferCreateInfo::builder()
                .render_pass(self.render_pass.handle())
                .width(self.w_h_l.0)
                .height(self.w_h_l.1)
                .layers(self.w_h_l.2)
                .attachments(&self.view_handles[..]);

            self.render_pass.device().handle.create_framebuffer(&*info, None).unwrap()
        };

        FrameBuffer {
            _marker: PhantomData,
            render_pass: self.render_pass,
            handle,
            image_views: self.image_views,
        }
    }
}