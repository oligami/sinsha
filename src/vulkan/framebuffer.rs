use super::*;
use super::mem_kai::*;
use super::mem_kai::image::*;
use super::render_pass::VkRenderPass;

pub struct VkFrameBuffer<V, A, S> {
	handle: vk::Framebuffer,
	render_pass: Arc<VkRenderPass<A, S>>,
	image_views: V,
}

pub struct VkFrameBufferBuilder<V, A> {
	info: vk::FramebufferCreateInfo,
	image_views: V,
	_attachments: PhantomData<A>,
}

impl VkFrameBuffer<(), (), ()> {
	fn builder() -> VkFrameBufferBuilder<(), ()> {
		VkFrameBufferBuilder {
			info: vk::FramebufferCreateInfo {
				s_type: StructureType::FRAMEBUFFER_CREATE_INFO,
				p_next: ptr::null(),
				flags: vk::FramebufferCreateFlags::empty(),
				render_pass: vk::RenderPass::null(),
				attachment_count: 0,
				p_attachments: ptr::null(),
				width: 0,
				height: 0,
				layers: 1,
			},
			image_views: (),
			_attachments: PhantomData,
		}
	}
}

impl<V, A> VkFrameBufferBuilder<V, A> {
	fn attach_image_view<E, F, S, U, MA, P>(
		mut self,
		image_view: image::VkImageView<E, F, S, U, MA, P>,
	) -> VkFrameBufferBuilder<(V, image::VkImageView<E, F, S, U, MA, P>), (A, (F, S))>
		where E: Extent,
			  F: Format,
			  S: SampleCount,
			  U: image::ImageUsage,
			  MA: Allocator,
			  P: MemoryProperties,
	{
		self.info.attachment_count += 1;

		// TODO: Check width, height, layers.

		VkFrameBufferBuilder {
			info: self.info,
			image_views: (self.image_views, image_view),
			_attachments: PhantomData,
		}
	}

	fn build<S>(mut self, render_pass: Arc<VkRenderPass<A, S>>) -> Arc<VkFrameBuffer<V, A, S>> {
		// TODO: Check whether this is valid alignment or not.
		self.info.p_attachments = &self.image_views as *const _ as *const _;

		let handle = unsafe {
			render_pass.device().handle.create_framebuffer(&self.info, None).unwrap()
		};

		Arc::new(VkFrameBuffer { render_pass, handle, image_views: self.image_views })
	}
}