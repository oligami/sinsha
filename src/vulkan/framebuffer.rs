use super::*;
use super::mem_kai::*;
use super::mem_kai::image::*;
use super::render_pass::{ VkRenderPass, VkAttachment };
use super::swap_chain::VkSwapchainImageView;

pub struct VkFrameBuffer<V, A, S> {
	handle: vk::Framebuffer,
	render_pass: Arc<VkRenderPass<A, S>>,
	image_views: V,
}

pub struct VkFrameBufferBuilder<V, H, A> {
	info: vk::FramebufferCreateInfo,
	image_views: V,
	handles: H,
	_attachments: PhantomData<A>,
}

impl VkFrameBuffer<(), (), ()> {
	pub fn builder(width: u32, height: u32, layers: u32) -> VkFrameBufferBuilder<(), (), ()> {
		VkFrameBufferBuilder {
			info: vk::FramebufferCreateInfo {
				s_type: StructureType::FRAMEBUFFER_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::FramebufferCreateFlags::empty(),
				render_pass: vk::RenderPass::null(),
				attachment_count: 0,
				p_attachments: ptr::null(),
				width,
				height,
				layers,
			},
			image_views: (),
			handles: (),
			_attachments: PhantomData,
		}
	}
}

impl<V, A, S> Drop for VkFrameBuffer<V, A, S> {
	fn drop(&mut self) {
		unsafe { self.render_pass.device().handle.destroy_framebuffer(self.handle, None); }
	}
}

impl<V, H, A> VkFrameBufferBuilder<V, H, A> {
	pub fn attach_image_view<F, S, U, MA, P>(
		mut self,
		image_view: Arc<image::VkImageView<extent::Extent2D, F, S, U, MA, P>>,
	) -> VkFrameBufferBuilder<
		(V, Arc<image::VkImageView<extent::Extent2D, F, S, U, MA, P>>),
		(H, vk::ImageView),
		(A, VkAttachment<F, S>)
	>
		where F: Format,
			  S: SampleCount,
			  U: image::ImageUsage,
			  MA: Allocator,
			  P: MemoryProperties,
	{
		self.info.attachment_count += 1;

		debug_assert_eq!(self.info.width, image_view.image().extent().width);
		debug_assert_eq!(self.info.height, image_view.image().extent().height);

		let layers = {
			let range = image_view.layer_range();
			range.end - range.start
		};
		debug_assert_eq!(self.info.layers, layers);

		let handle = image_view.handle();
		VkFrameBufferBuilder {
			info: self.info,
			image_views: (self.image_views, image_view),
			handles: (self.handles, handle),
			_attachments: PhantomData,
		}
	}

	pub fn attach_swapchain_image_view<F, U>(
		mut self,
		image_view: Arc<VkSwapchainImageView<F, U>>,
	) -> VkFrameBufferBuilder<
		(V, Arc<VkSwapchainImageView<F, U>>),
		(H, vk::ImageView),
		(A, VkAttachment<F, sample_count::Type1>)
	> where F: Format, U: image::ImageUsage
	{
		self.info.attachment_count += 1;

		debug_assert_eq!(self.info.width, image_view.extent().width);
		debug_assert_eq!(self.info.height, image_view.extent().height);
		debug_assert_eq!(self.info.layers, 1);

		let handle = image_view.handle();

		VkFrameBufferBuilder {
			info: self.info,
			image_views: (self.image_views, image_view),
			handles: (self.handles, handle),
			_attachments: PhantomData,
		}
	}

	// this builder must have more than one attachment because A is same as A of VkRenderPass.
	pub fn build<S>(mut self, render_pass: Arc<VkRenderPass<A, S>>) -> Arc<VkFrameBuffer<V, A, S>> {
		self.info.p_attachments = &self.handles as *const _ as *const _;
		self.info.render_pass = render_pass.handle();

		let handle = unsafe {
			render_pass.device().handle.create_framebuffer(&self.info, None).unwrap()
		};

		Arc::new(VkFrameBuffer { render_pass, handle, image_views: self.image_views })
	}
}