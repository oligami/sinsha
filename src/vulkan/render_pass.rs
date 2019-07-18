use super::*;
use super::mem::image::*;


pub struct RenderPass<A, S> {
	device: Arc<Device>,
	handle: vk::RenderPass,
	attachments: PhantomData<A>,
	subpasses: PhantomData<S>,
	subpass_count: u32,
}

pub struct Attachment<F, S> where F: Format, S: SampleCount {
	_format: PhantomData<F>,
	_sample_count: PhantomData<S>,
}

pub struct Subpass {
	input: Vec<vk::AttachmentReference>,
	color: Vec<vk::AttachmentReference>,
	resolve: Vec<vk::AttachmentReference>,
	depth_stencil: Option<Box<vk::AttachmentReference>>,
	preserve: Vec<u32>,
}

pub struct Subpasss {
	color_attachments: usize,
}

pub struct VkRenderPassBuilderAttachments<A> {
	attachments: Vec<vk::AttachmentDescription>,
	_attachments: PhantomData<A>,
}

pub struct VkRenderPassBuilderSubpasses<A, S> {
	attachments: Vec<vk::AttachmentDescription>,
	_attachments: PhantomData<A>,
	descriptions: Vec<vk::SubpassDescription>,
	subpasses: Vec<Subpass>,
	_subpasses: PhantomData<S>,
}

pub struct VkRenderPassBuilderDependencies<A, S> {
	attachments: Vec<vk::AttachmentDescription>,
	_attachments: PhantomData<A>,
	description: Vec<vk::SubpassDescription>,
	subpasses: Vec<Subpass>,
	_subpasses: PhantomData<S>,
	dependencies: Vec<vk::SubpassDependency>,
}

impl RenderPass<(), ()> {
	pub fn builder() -> VkRenderPassBuilderAttachments<()> {
		VkRenderPassBuilderAttachments {
			attachments: Vec::new(),
			_attachments: PhantomData,
		}
	}
}
impl<A, S> RenderPass<A, S> {
	#[inline]
	pub fn device(&self) -> &Arc<Device> { &self.device }

	#[inline]
	pub fn handle(&self) -> vk::RenderPass { self.handle }

	#[inline]
	pub fn subpass_count(&self) -> u32 { self.subpass_count }
}

impl<A, S> Destroy for RenderPass<A, S> {
	type Ok = ();
	type Error = Infallible;
	unsafe fn destroy(self) -> Result<Self::Ok, Self::Error> {
		unsafe { self.device.handle.destroy_render_pass(self.handle, None); }
		Ok(())
	}
}

impl<A> VkRenderPassBuilderAttachments<A> {
	pub fn color_attachment<F, S>(
		mut self,
		_format: F,
		_sample_count: S,
		load_op: vk::AttachmentLoadOp,
		store_op: vk::AttachmentStoreOp,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
	) -> VkRenderPassBuilderAttachments<(A, Attachment<F, S>)>
		where F: Format,
			  S: SampleCount
	{
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format: F::format(),
			samples: S::sample_count(),
			load_op,
			store_op,
			stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
			stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
			initial_layout,
			final_layout,
		};

		self.attachments.push(description);

		VkRenderPassBuilderAttachments {
			attachments: self.attachments,
			_attachments: PhantomData,
		}
	}

	pub fn depth_stencil_attachment<F, S>(
		mut self,
		_format: F,
		_sample_count: S,
		load_op: vk::AttachmentLoadOp,
		store_op: vk::AttachmentStoreOp,
		stencil_load_op: vk::AttachmentLoadOp,
		stencil_store_op: vk::AttachmentStoreOp,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
	) -> VkRenderPassBuilderAttachments<(A, Attachment<F, S>)>
		where F: DepthFormat + StencilFormat,
			  S: SampleCount,
	{
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format: F::format(),
			samples: S::sample_count(),
			load_op,
			store_op,
			stencil_load_op,
			stencil_store_op,
			initial_layout,
			final_layout,
		};

		self.attachments.push(description);

		VkRenderPassBuilderAttachments {
			attachments: self.attachments,
			_attachments: PhantomData,
		}
	}

	pub fn stencil_attachment<F, S>(
		mut self,
		_format: F,
		_sample_count: S,
		load_op: vk::AttachmentLoadOp,
		store_op: vk::AttachmentStoreOp,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
	) -> VkRenderPassBuilderAttachments<(A, Attachment<F, S>)>
		where F: StencilFormat,
			  S: SampleCount,
	{
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format: F::format(),
			samples: S::sample_count(),
			load_op: vk::AttachmentLoadOp::DONT_CARE,
			store_op: vk::AttachmentStoreOp::DONT_CARE,
			stencil_load_op: load_op,
			stencil_store_op: store_op,
			initial_layout,
			final_layout,
		};

		self.attachments.push(description);

		VkRenderPassBuilderAttachments {
			attachments: self.attachments,
			_attachments: PhantomData,
		}
	}

	pub fn subpasses(self) -> VkRenderPassBuilderSubpasses<A, ()> {
		VkRenderPassBuilderSubpasses {
			attachments: self.attachments,
			_attachments: PhantomData,
			subpasses: Vec::new(),
			descriptions: Vec::new(),
			_subpasses: PhantomData,
		}
	}
}

impl<A, S> VkRenderPassBuilderSubpasses<A, S> {
	pub fn subpass<P>(
		mut self,
		_pipeline_bind_point: P,
		color: Vec<vk::AttachmentReference>,
		resolve: Vec<vk::AttachmentReference>,
		input: Vec<vk::AttachmentReference>,
		depth_stencil: Option<Box<vk::AttachmentReference>>,
		preserve: Vec<u32>,
	) -> VkRenderPassBuilderSubpasses<A, (S, P)> where P: pipeline::PipelineBindPoint {
		let (depth_stencil, depth_stencil_pointer) = match depth_stencil {
			Some(reference) => {
				debug_assert!((reference.attachment as usize) < self.attachments.len());

				let depth_stencil_pointer = &*reference as *const _;
				(Some(reference), depth_stencil_pointer)
			},
			None => (None, ptr::null()),
		};

		color.iter()
			.for_each(|reference| {
				debug_assert!((reference.attachment as usize) < self.attachments.len());
			});

		resolve.iter()
			.for_each(|reference| {
				debug_assert!((reference.attachment as usize) < self.attachments.len());
			});

		input.iter()
			.for_each(|reference| {
				debug_assert!((reference.attachment as usize) < self.attachments.len());
			});

		preserve.iter()
			.for_each(|index| debug_assert!((*index as usize) < self.attachments.len()));

		let description = vk::SubpassDescription {
			flags: vk::SubpassDescriptionFlags::empty(),
			pipeline_bind_point: P::bind_point(),
			color_attachment_count: color.len() as u32,
			p_color_attachments: color.as_ptr(),
			p_resolve_attachments: if resolve.len() != 0 { resolve.as_ptr() } else { ptr::null() },
			input_attachment_count: input.len() as u32,
			p_input_attachments: input.as_ptr(),
			p_depth_stencil_attachment: depth_stencil_pointer,
			preserve_attachment_count: preserve.len() as u32,
			p_preserve_attachments: preserve.as_ptr(),
		};

		self.descriptions.push(description);
		self.subpasses.push(Subpass { input, color, resolve, depth_stencil, preserve });

		VkRenderPassBuilderSubpasses {
			attachments: self.attachments,
			_attachments: PhantomData,
			descriptions: self.descriptions,
			subpasses: self.subpasses,
			_subpasses: PhantomData,
		}
	}
}
impl<A, S1, S2> VkRenderPassBuilderSubpasses<A, (S1, S2)> {
	// NOTE: Implementing here enforce that this render pass has at least one subpass.
	pub fn dependencies(self) -> VkRenderPassBuilderDependencies<A, (S1, S2)> {
		VkRenderPassBuilderDependencies {
			attachments: self.attachments,
			_attachments: PhantomData,
			description: self.descriptions,
			subpasses: self.subpasses,
			_subpasses: PhantomData,
			dependencies: Vec::new(),
		}
	}
}

impl<A, S> VkRenderPassBuilderDependencies<A, S> {
	pub fn dependency(
		mut self,
		src_subpass: u32,
		dst_subpass: u32,
		src_access_mask: vk::AccessFlags,
		dst_access_mask: vk::AccessFlags,
		src_stage_mask: vk::PipelineStageFlags,
		dst_stage_mask: vk::PipelineStageFlags,
		dependency_flags: vk::DependencyFlags,
	) -> Self {
		debug_assert!(src_subpass < self.subpasses.len() as u32 || src_subpass == !0);
		debug_assert!(dst_subpass < self.subpasses.len() as u32 || dst_subpass == !0);

		self.dependencies.push(
			vk::SubpassDependency {
				src_subpass,
				dst_subpass,
				src_access_mask,
				dst_access_mask,
				src_stage_mask,
				dst_stage_mask,
				dependency_flags,
			}
		);

		self
	}

	pub fn build(self, device: Arc<Device>) -> Arc<RenderPass<A, S>> {
		let info = vk::RenderPassCreateInfo {
			s_type: StructureType::RENDER_PASS_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::RenderPassCreateFlags::empty(),
			attachment_count: self.attachments.len() as u32,
			p_attachments: self.attachments.as_ptr(),
			subpass_count: self.description.len() as u32,
			p_subpasses: self.description.as_ptr(),
			dependency_count: self.dependencies.len() as u32,
			p_dependencies: self.dependencies.as_ptr(),
		};

		let handle = unsafe { device.handle.create_render_pass(&info, None).unwrap() };

		Arc::new(RenderPass {
			device,
			handle,
			attachments: PhantomData,
			subpasses: PhantomData,
			subpass_count: self.description.len() as u32,
		})
	}
}

