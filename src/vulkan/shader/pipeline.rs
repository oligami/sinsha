pub mod stage;

use super::*;
use super::stage::ShaderStages;
use super::descriptor::VkDescriptorSetLayout;

pub struct VkPipelineLayout<P, L> {
	device: Arc<VkDevice>,
	handle: vk::PipelineLayout,
	push_constants: P,
	set_layouts: L,
}

pub struct VkPipelineLayoutBuilderPushConstants<R, P> {
	push_constant_count: u32,
	push_constant_offset: u32,
	push_constant_ranges: R,
	push_constants: P,
}

pub struct VkPipelineLayoutBuilderSetLayout<R, P, H, L> {
	push_constant_count: u32,
	push_constant_ranges: R,
	push_constants: P,
	set_layout_count: u32,
	set_layout_handles: H,
	set_layouts: L,
}

impl VkPipelineLayout<(), ()> {
	pub fn builder() -> VkPipelineLayoutBuilderPushConstants<(), ()> {
		VkPipelineLayoutBuilderPushConstants {
			push_constant_count: 0,
			push_constant_ranges: (),
			push_constant_offset: 0,
			push_constants: (),
		}
	}
}

impl<P, L> Drop for VkPipelineLayout<P, L> {
	fn drop(&mut self) { unsafe { self.device.handle.destroy_pipeline_layout(self.handle, None) } }
}

impl<R, P> VkPipelineLayoutBuilderPushConstants<R, P> {
	pub fn push_constant<T, S>(
		self,
		stage: S,
	) -> VkPipelineLayoutBuilderPushConstants<
		(R, vk::PushConstantRange),
		(P, (PhantomData<fn() -> T>, S))
	> where S: ShaderStages {
		let push_constant = vk::PushConstantRange {
			stage_flags: S::shader_stages(),
			offset: self.push_constant_offset,
			size: std::mem::size_of::<T>() as u32,
		};

		println!("size: {}", std::mem::size_of::<T>());

		VkPipelineLayoutBuilderPushConstants {
			push_constant_count: self.push_constant_count + 1,
			push_constant_ranges: (self.push_constant_ranges, push_constant),
			push_constant_offset: self.push_constant_offset + std::mem::size_of::<T>() as u32,
			push_constants: (self.push_constants, (PhantomData, stage)),
		}
	}

	pub fn descriptor_set_layout(self) -> VkPipelineLayoutBuilderSetLayout<R, P, (), ()> {
		VkPipelineLayoutBuilderSetLayout {
			push_constant_count: self.push_constant_count,
			push_constant_ranges: self.push_constant_ranges,
			push_constants: self.push_constants,
			set_layout_count: 0,
			set_layout_handles: (),
			set_layouts: (),
		}
	}
}

impl<R, P, H, L> VkPipelineLayoutBuilderSetLayout<R, P, H, L> {
	pub fn set_layout<L1>(
		self,
		set_layout: Arc<VkDescriptorSetLayout<L1>>,
	) -> VkPipelineLayoutBuilderSetLayout<R, P, (H, vk::DescriptorSetLayout), (L, L1)>
		where L1: Copy
	{
		VkPipelineLayoutBuilderSetLayout {
			push_constant_count: self.push_constant_count,
			push_constant_ranges: self.push_constant_ranges,
			push_constants: self.push_constants,
			set_layout_count: self.set_layout_count + 1,
			set_layout_handles: (self.set_layout_handles, set_layout.handle()),
			set_layouts: (self.set_layouts, set_layout.layout()),
		}
	}

	pub fn build(self, device: Arc<VkDevice>) -> Arc<VkPipelineLayout<P, L>> {
		let info = vk::PipelineLayoutCreateInfo {
			s_type: StructureType::PIPELINE_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::PipelineLayoutCreateFlags::empty(),
			set_layout_count: self.set_layout_count,
			p_set_layouts: &self.set_layout_handles as *const _ as *const _,
			push_constant_range_count: self.push_constant_count,
			p_push_constant_ranges: &self.push_constant_ranges as *const _ as *const _,
		};

		let handle = unsafe { device.handle.create_pipeline_layout(&info, None).unwrap() };

		Arc::new(VkPipelineLayout {
			device,
			handle,
			set_layouts: self.set_layouts,
			push_constants: self.push_constants,
		})
	}
}