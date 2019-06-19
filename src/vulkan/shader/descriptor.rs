pub mod ty;

use super::*;

pub use ty::DescriptorType;

use stage::ShaderStage;

pub struct VkDescriptorSetLayout<L> {
	device: Arc<VkDevice>,
	handle: vk::DescriptorSetLayout,
	layout: L,
}

pub struct VkDescriptorSetLayoutBuilder<B, L> {
	binding_index: u32,
	binging_count: u32,
	bindings: B,
	layout: L,
}

pub struct VkDescriptorPool {
	device: Arc<VkDevice>,
	handle: vk::DescriptorPool,
}

pub struct VkDescriptorPoolBuilder<P, L> {
	pool_size_count: u32,
	pool_sizes: P,
	layout: L,
}

pub struct VkDescriptorSet<L> {
	layout: Arc<VkDescriptorSetLayout<L>>,
	pool: Arc<VkDescriptorPool>,
	handle: vk::DescriptorSet,
}


impl VkDescriptorSetLayout<()> {
	pub fn builder() -> VkDescriptorSetLayoutBuilder<(), ()> {
		VkDescriptorSetLayoutBuilder {
			binding_index: 0,
			binging_count: 0,
			bindings: (),
			layout: (),
		}
	}
}

impl<L> Drop for VkDescriptorSetLayout<L> {
	fn drop(&mut self) {
		unsafe { self.device.handle.destroy_descriptor_set_layout(self.handle, None); }
	}
}

impl<B, L> VkDescriptorSetLayoutBuilder<B, L> {
	pub fn binding<D, S>(
		self,
		descriptor_type: D,
		descriptor_count: u32,
		shader_stages: S
	) -> VkDescriptorSetLayoutBuilder<(B, vk::DescriptorSetLayoutBinding), (L, (D, u32, S))>
		where D: DescriptorType, S: ShaderStage
	{
		let binding = vk::DescriptorSetLayoutBinding {
			binding: self.binding_index,
			descriptor_type: D::descriptor_type(),
			descriptor_count,
			stage_flags: S::shader_stage(),
			p_immutable_samplers: ptr::null(),
		};

		VkDescriptorSetLayoutBuilder {
			binding_index: self.binding_index + 1,
			binging_count: self.binging_count + 1,
			bindings: (self.bindings, binding),
			layout: (self.layout, (descriptor_type, descriptor_count, shader_stages)),
		}
	}

	pub fn skip(self) -> VkDescriptorSetLayoutBuilder<
		B,
		(L, (descriptor::ty::Empty, u32, shader::stage::Empty))
	> {
		VkDescriptorSetLayoutBuilder {
			binding_index: self.binding_index + 1,
			binging_count: self.binging_count,
			bindings: self.bindings,
			layout: (self.layout, (descriptor::ty::Empty, 0, shader::stage::Empty)),
		}
	}
}

impl<B1, B2, L> VkDescriptorSetLayoutBuilder<(B1, B2), L> {
	pub fn build(self, device: Arc<VkDevice>) -> Arc<VkDescriptorSetLayout<L>> {
		let info = vk::DescriptorSetLayoutCreateInfo {
			s_type: StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DescriptorSetLayoutCreateFlags::empty(),
			binding_count: self.binging_count,
			p_bindings: &self.bindings as *const _ as *const _,
		};

		let handle = unsafe { device.handle.create_descriptor_set_layout(&info, None).unwrap() };

		Arc::new(VkDescriptorSetLayout { device, handle, layout: self.layout })
	}
}

impl VkDescriptorPool {
	pub fn builder() -> VkDescriptorPoolBuilder<(), ()> {
		VkDescriptorPoolBuilder {
			pool_size_count: 0,
			pool_sizes: (),
			layout: (),
		}
	}
}

impl Drop for VkDescriptorPool {
	fn drop(&mut self) { unsafe { self.device.handle.destroy_descriptor_pool(self.handle, None); } }
}

impl<P> VkDescriptorPoolBuilder<P, ()> {
	pub fn layout<DL: Copy>(
		self,
		layout: Arc<VkDescriptorSetLayout<DL>>,
	) -> VkDescriptorPoolBuilder<P, DL> {
		VkDescriptorPoolBuilder {
			pool_size_count: self.pool_size_count,
			pool_sizes: self.pool_sizes,
			layout: layout.layout,
		}
	}
}

impl<P, L, D, S> VkDescriptorPoolBuilder<P, (L, (D, u32, S))>
	where D: DescriptorType, S: ShaderStage,
{
	pub fn pool_size(self) -> VkDescriptorPoolBuilder<(P, vk::DescriptorPoolSize), L> {
		let pool_size = vk::DescriptorPoolSize {
			ty: D::descriptor_type(),
			descriptor_count: (self.layout.1).1,
		};

		VkDescriptorPoolBuilder {
			pool_size_count: self.pool_size_count + 1,
			pool_sizes: (self.pool_sizes, pool_size),
			layout: self.layout.0,
		}
	}
}

impl<P1, P2> VkDescriptorPoolBuilder<(P1, P2), ()> {
	pub fn build(self, max_sets: u32, device: Arc<VkDevice>) -> Arc<VkDescriptorPool> {
		let info = vk::DescriptorPoolCreateInfo {
			s_type: StructureType::DESCRIPTOR_POOL_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DescriptorPoolCreateFlags::empty(),
			max_sets,
			pool_size_count: self.pool_size_count,
			p_pool_sizes: &self.pool_sizes as *const _ as *const _,
		};

		let handle = unsafe { device.handle.create_descriptor_pool(&info, None).unwrap() };

		Arc::new(VkDescriptorPool { device, handle })
	}
}

impl<L> VkDescriptorSet<L> {
	pub fn new(
		layouts: &[Arc<VkDescriptorSetLayout<L>>],
		pool: Arc<VkDescriptorPool>,
	) -> Vec<Self> {
		let handles: Vec<_> = layouts.iter()
			.map(|l| l.handle)
			.collect();

		let info = vk::DescriptorSetAllocateInfo {
			s_type: StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
			p_next: ptr::null(),
			descriptor_pool: pool.handle,
			descriptor_set_count: layouts.len() as u32,
			p_set_layouts: handles.as_ptr(),
		};

		let handles = unsafe { layouts[0].device.handle.allocate_descriptor_sets(&info).unwrap() };

		handles.into_iter()
			.zip(layouts.iter())
			.map(|(handle, layout)| Self { handle, layout: layout.clone(), pool: pool.clone() })
			.collect()
	}
}