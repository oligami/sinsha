use super::*;
use super::mem_kai::image::*;


pub struct VkRenderPass<A, S> {
	device: Arc<VkDevice>,
	handle: vk::RenderPass,
	// A: (VkAttachments<C, D>, .. ); maybe array
	attachments: PhantomData<A>,
	subpasses: PhantomData<S>,
}

pub struct VkAttachment<F, S> where F: Format, S: SampleCount {
	_format: PhantomData<F>,
	_sample_count: PhantomData<S>,
}

pub struct VkSubpass {
	input: Vec<vk::AttachmentReference>,
	color: Vec<vk::AttachmentReference>,
	resolve: Vec<vk::AttachmentReference>,
	depth_stencil: Option<Box<vk::AttachmentReference>>,
	preserve: Vec<u32>,
}

use subpass::SubpassPipeline;
pub mod subpass {
	use ash::vk::PipelineBindPoint;
	pub struct Input;

	pub trait SubpassPipeline {
		fn bind_point() -> PipelineBindPoint;
	}
	pub struct Graphics;
	impl SubpassPipeline for Graphics {
		fn bind_point() -> PipelineBindPoint { PipelineBindPoint::GRAPHICS }
	}
}

pub struct VkRenderPassBuilderAttachments<A> {
	attachments: Vec<vk::AttachmentDescription>,
	_attachments: PhantomData<A>,
}

pub struct VkRenderPassBuilderSubpasses<A, S> {
	attachments: Vec<vk::AttachmentDescription>,
	_attachments: PhantomData<A>,
	descriptions: Vec<vk::SubpassDescription>,
	subpasses: Vec<VkSubpass>,
	_subpasses: PhantomData<S>,
}

pub struct VkRenderPassBuilderDependencies<A, S> {
	attachments: Vec<vk::AttachmentDescription>,
	_attachments: PhantomData<A>,
	description: Vec<vk::SubpassDescription>,
	subpasses: Vec<VkSubpass>,
	_subpasses: PhantomData<S>,
	dependencies: Vec<vk::SubpassDependency>,
}

impl VkRenderPass<(), ()> {
	pub fn builder() -> VkRenderPassBuilderAttachments<()> {
		VkRenderPassBuilderAttachments {
			attachments: Vec::new(),
			_attachments: PhantomData,
		}
	}
}

impl<A, S> Drop for VkRenderPass<A, S> {
	fn drop(&mut self) {
		unsafe { self.device.handle.destroy_render_pass(self.handle, None); }
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
	) -> VkRenderPassBuilderAttachments<(A, VkAttachment<F, S>)>
		where F: Format,
			  S: SampleCount
	{
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format: F::format(),
			samples: S::flags(),
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
	) -> VkRenderPassBuilderAttachments<(A, VkAttachment<F, S>)>
		where F: DepthFormat + StencilFormat,
			  S: SampleCount,
	{
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format: F::format(),
			samples: S::flags(),
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
	) -> VkRenderPassBuilderAttachments<(A, VkAttachment<F, S>)>
		where F: StencilFormat,
			  S: SampleCount,
	{
		let description = vk::AttachmentDescription {
			flags: vk::AttachmentDescriptionFlags::empty(),
			format: F::format(),
			samples: S::flags(),
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
		depth_stencil: Option<Box<vk::AttachmentReference>>,
		preserve: Vec<u32>,
	) -> VkRenderPassBuilderSubpasses<A, (S, P)> where P: SubpassPipeline {
		let (depth_stencil, depth_stencil_pointer) = match depth_stencil {
			Some(reference) => {
				debug_assert!((reference.attachment as usize) < self.attachments.len());

				let pointer = Box::into_raw(reference);
				let depth_stencil_pointer = pointer as *const _;

				let depth_stencil = unsafe { Box::from_raw(pointer) };

				(Some(depth_stencil), depth_stencil_pointer)
			},
			None => (None, ptr::null()),
		};

		color
			.iter()
			.for_each(|reference| {
				debug_assert!((reference.attachment as usize) < self.attachments.len());
			});

		resolve
			.iter()
			.for_each(|reference| {
				debug_assert!((reference.attachment as usize) < self.attachments.len());
			});

		preserve
			.iter()
			.for_each(|index| debug_assert!((*index as usize) < self.attachments.len()));

		let input = Vec::new();

		let description = vk::SubpassDescription {
			flags: vk::SubpassDescriptionFlags::empty(),
			pipeline_bind_point: P::bind_point(),
			color_attachment_count: color.len() as u32,
			p_color_attachments: color.as_ptr(),
			p_resolve_attachments: if resolve.len() != 0 { resolve.as_ptr() } else { ptr::null() },
			input_attachment_count: 0,
			p_input_attachments: input.as_ptr(),
			p_depth_stencil_attachment: depth_stencil_pointer,
			preserve_attachment_count: preserve.len() as u32,
			p_preserve_attachments: preserve.as_ptr(),
		};

		self.descriptions.push(description);
		self.subpasses.push(VkSubpass { input, color, resolve, depth_stencil, preserve });

		VkRenderPassBuilderSubpasses {
			attachments: self.attachments,
			_attachments: PhantomData,
			descriptions: self.descriptions,
			subpasses: self.subpasses,
			_subpasses: PhantomData,
		}
	}
}
impl<A, S, I> VkRenderPassBuilderSubpasses<A, (S, I)> {
	// TODO: subpass::Input should be taken place by what is descriptor::InputAttachment.
	pub fn input(
		mut self,
		input: vk::AttachmentReference,
	) -> VkRenderPassBuilderSubpasses<A, (S, (I, subpass::Input))> {
		self.subpasses.last_mut().unwrap().input.push(input);
		let description = self.descriptions.last_mut().unwrap();
		description.input_attachment_count += 1;

		VkRenderPassBuilderSubpasses {
			attachments: self.attachments,
			_attachments: PhantomData,
			descriptions: self.descriptions,
			subpasses: self.subpasses,
			_subpasses: PhantomData,
		}
	}

	// NOTE: Implementing here enforce that this render pass has at least one subpass.
	pub fn dependencies(self) -> VkRenderPassBuilderDependencies<A, (S, I)> {
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
		if cfg!(debug_assertions) {

		}


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

	pub fn build(self, device: Arc<VkDevice>) -> Arc<VkRenderPass<A, S>> {
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

		Arc::new(VkRenderPass { device, handle, attachments: PhantomData, subpasses: PhantomData })
	}
}

