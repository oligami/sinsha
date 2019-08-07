use super::*;

pub struct RenderPass<I, D>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
	_instance: PhantomData<I>,
	device: D,
	handle: vk::RenderPass,
	attachments: Vec<Attachment>,
	subpasses: Vec<Subpass>,
}

pub trait RenderPassAbs {
	type Instance: Borrow<Instance> + Deref<Target = Instance>;
	type Device: Borrow<Device<Self::Instance>> + Deref<Target = Device<Self::Instance>>;

	fn instance(&self) -> &Instance;
	fn device(&self) -> &Device<Self::Instance>;
	fn handle(&self) -> vk::RenderPass;
	fn attachments(&self) -> &[Attachment];
	fn subpasses(&self) -> &[Subpass];
}

pub struct Attachment {
	format: vk::Format,
	samples: vk::SampleCountFlags,
}

pub struct Subpass {
	pipeline_bind_point: vk::PipelineBindPoint,
	input: Vec<vk::AttachmentReference>,
	color: Vec<vk::AttachmentReference>,
	resolve: Vec<vk::AttachmentReference>,
	depth_stencil: Option<Box<vk::AttachmentReference>>,
	preserve: Vec<u32>,
}

pub struct Subpasss {
	color_attachments: usize,
}

pub struct AttachmentState;
pub struct SubpassState;
pub struct DependencyState;
pub struct ReadyToBuild;
pub struct RenderPassBuilder<S> {
	attachments: Vec<Attachment>,
	attachment_descriptions: Vec<vk::AttachmentDescription>,
	subpasses: Vec<Subpass>,
	subpass_descriptions: Vec<vk::SubpassDescription>,
	subpass_dependencies: Vec<vk::SubpassDependency>,
	_state: PhantomData<S>,
}

impl RenderPass<&'static Instance, &'static Device<&'static Instance>> {
	pub fn builder() -> RenderPassBuilder<AttachmentState> {
		RenderPassBuilder {
			attachments: Vec::new(),
			attachment_descriptions: Vec::new(),
			subpasses: Vec::new(),
			subpass_descriptions: Vec::new(),
			subpass_dependencies: Vec::new(),
			_state: PhantomData,
		}
	}
}

impl<I, D> RenderPassAbs for RenderPass<I, D>
    where I: Borrow<Instance> + Deref<Target = Instance>,
          D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
	type Instance = I;
	type Device = D;

    #[inline]
	fn instance(&self) -> &Instance { &self.device.instance }
    #[inline]
	fn device(&self) -> &Device<Self::Instance> { &self.device }
    #[inline]
	fn handle(&self) -> vk::RenderPass { self.handle }
    #[inline]
	fn attachments(&self) -> &[Attachment] { &self.attachments[..] }
    #[inline]
	fn subpasses(&self) -> &[Subpass] { &self.subpasses[..] }
}

impl<I, D> Destroy for RenderPass<I, D> where I: Borrow<Instance> + Deref<Target = Instance>,
                                              D: Borrow<Device<I>> + Deref<Target = Device<I>>, {
	type Ok = ();
	type Error = Infallible;
	unsafe fn destroy(self) -> Result<Self::Ok, Self::Error> {
		unsafe { self.device.handle.destroy_render_pass(self.handle, None); }
		Ok(())
	}
}

impl<I, D> Drop for RenderPass<I, D> where I: Borrow<Instance> + Deref<Target = Instance>,
                                           D: Borrow<Device<I>> + Deref<Target = Device<I>>, {
	fn drop(&mut self) {
		unsafe {
			self.device.handle.destroy_render_pass(self.handle, None);
		}
	}
}

impl Attachment {
	pub fn format(&self) -> vk::Format { self.format }
	pub fn samples(&self) -> vk::SampleCountFlags { self.samples }
}

impl<S1> RenderPassBuilder<S1> {
	fn into<S2>(self) -> RenderPassBuilder<S2> {
		RenderPassBuilder {
			attachments: self.attachments,
			attachment_descriptions: self.attachment_descriptions,
			subpasses: self.subpasses,
			subpass_descriptions: self.subpass_descriptions,
			subpass_dependencies: self.subpass_dependencies,
			_state: PhantomData,
		}
	}
}

impl RenderPassBuilder<AttachmentState> {
	pub fn color_attachment(
		mut self,
		format: vk::Format,
		samples: vk::SampleCountFlags,
		load_op: vk::AttachmentLoadOp,
		store_op: vk::AttachmentStoreOp,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
	) -> Self {
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format,
			samples,
			load_op,
			store_op,
			stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
			stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
			initial_layout,
			final_layout,
		};

		self.attachments.push(Attachment { format, samples });
		self.attachment_descriptions.push(description);

		self
	}

	pub fn depth_stencil_attachment(
		mut self,
		format: vk::Format,
		samples: vk::SampleCountFlags,
		load_op: vk::AttachmentLoadOp,
		store_op: vk::AttachmentStoreOp,
		stencil_load_op: vk::AttachmentLoadOp,
		stencil_store_op: vk::AttachmentStoreOp,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
	) -> Self {
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format,
			samples,
			load_op,
			store_op,
			stencil_load_op,
			stencil_store_op,
			initial_layout,
			final_layout,
		};

		self.attachments.push(Attachment { format, samples });
		self.attachment_descriptions.push(description);

		self
	}

	pub fn stencil_attachment(
		mut self,
		format: vk::Format,
		samples: vk::SampleCountFlags,
		load_op: vk::AttachmentLoadOp,
		store_op: vk::AttachmentStoreOp,
		initial_layout: vk::ImageLayout,
		final_layout: vk::ImageLayout,
	) -> Self {
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format,
			samples,
			load_op: vk::AttachmentLoadOp::DONT_CARE,
			store_op: vk::AttachmentStoreOp::DONT_CARE,
			stencil_load_op: load_op,
			stencil_store_op: store_op,
			initial_layout,
			final_layout,
		};

		self.attachments.push(Attachment { format, samples });
		self.attachment_descriptions.push(description);

		self
	}

	pub fn subpasses(self) -> RenderPassBuilder<SubpassState> { self.into() }
}

impl RenderPassBuilder<SubpassState> {
	pub fn subpass(
		mut self,
		pipeline_bind_point: vk::PipelineBindPoint,
		color: Vec<vk::AttachmentReference>,
		resolve: Vec<vk::AttachmentReference>,
		input: Vec<vk::AttachmentReference>,
		depth_stencil: Option<Box<vk::AttachmentReference>>,
		preserve: Vec<u32>,
	) -> Self {
		let depth_stencil_pointer = depth_stencil.as_ref()
			.map(|reference| {
				assert!((reference.attachment as usize) < self.attachments.len());
				&**reference as *const _
			})
			.unwrap_or(ptr::null());

		color.iter()
			.for_each(|reference| {
				assert!((reference.attachment as usize) < self.attachments.len());
			});

		resolve.iter()
			.for_each(|reference| {
				assert!((reference.attachment as usize) < self.attachments.len());
			});

		input.iter()
			.for_each(|reference| {
				assert!((reference.attachment as usize) < self.attachments.len());
			});

		preserve.iter()
			.for_each(|index| assert!((*index as usize) < self.attachments.len()));

		let description = vk::SubpassDescription {
			flags: vk::SubpassDescriptionFlags::empty(),
			pipeline_bind_point,
			color_attachment_count: color.len() as u32,
			p_color_attachments: color.as_ptr(),
			p_resolve_attachments: if resolve.len() != 0 { resolve.as_ptr() } else { ptr::null() },
			input_attachment_count: input.len() as u32,
			p_input_attachments: input.as_ptr(),
			p_depth_stencil_attachment: depth_stencil_pointer,
			preserve_attachment_count: preserve.len() as u32,
			p_preserve_attachments: preserve.as_ptr(),
		};

		self.subpass_descriptions.push(description);
		self.subpasses.push(Subpass {
			pipeline_bind_point,
			input,
			color,
			resolve,
			depth_stencil,
			preserve
		});

		self
	}

	pub fn dependencies(self) -> RenderPassBuilder<DependencyState> {
		self.into()
	}
}

impl RenderPassBuilder<DependencyState> {
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
		debug_assert!(src_subpass < self.subpasses.len() as u32 || src_subpass == vk::SUBPASS_EXTERNAL);
		debug_assert!(dst_subpass < self.subpasses.len() as u32 || dst_subpass == vk::SUBPASS_EXTERNAL);

		self.subpass_dependencies.push(
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

	pub fn build<I, D>(self, device: D) -> RenderPass<I, D>
		where I: Borrow<Instance> + Deref<Target = Instance>,
              D: Borrow<Device<I>> + Deref<Target = Device<I>>,
	{
		let info = vk::RenderPassCreateInfo {
			s_type: StructureType::RENDER_PASS_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::RenderPassCreateFlags::empty(),
			attachment_count: self.attachments.len() as u32,
			p_attachments: self.attachment_descriptions.as_ptr(),
			subpass_count: self.subpass_descriptions.len() as u32,
			p_subpasses: self.subpass_descriptions.as_ptr(),
			dependency_count: self.subpass_dependencies.len() as u32,
			p_dependencies: self.subpass_dependencies.as_ptr(),
		};

		let handle = unsafe { device.handle.create_render_pass(&info, None).unwrap() };

		RenderPass {
			_instance: PhantomData,
			device,
			handle,
			attachments: self.attachments,
			subpasses: self.subpasses,
		}
	}
}

