pub mod ty;

use super::*;

pub use ty::DescriptorType;
use stage::ShaderStages;


pub struct DescriptorSetLayout<T> {
	device: Arc<Device>,
	handle: vk::DescriptorSetLayout,
	types: PhantomData<T>,
}

pub struct DescriptorSetLayoutBuilder<T> {
	binding_index: u32,
	bindings: Vec<vk::DescriptorSetLayoutBinding>,
	types: PhantomData<T>,
}

pub struct DescriptorPool {
	device: Arc<Device>,
	handle: vk::DescriptorPool,
}

pub struct DescriptorPoolBuilder<T> {
	pool_sizes: Vec<vk::DescriptorPoolSize>,
	types: PhantomData<T>,
}

pub struct DescriptorSet<L> {
	layout: Arc<DescriptorSetLayout<L>>,
	pool: Arc<DescriptorPool>,
	handle: vk::DescriptorSet,
}


impl DescriptorSetLayout<()> {
	pub fn builder() -> DescriptorSetLayoutBuilder<()> {
		DescriptorSetLayoutBuilder {
			binding_index: 0,
			bindings: Vec::new(),
			types: PhantomData,
		}
	}
}

impl<L> DescriptorSetLayout<L> {
	#[inline]
	pub fn handle(&self) -> vk::DescriptorSetLayout { self.handle }
}

impl<L> Drop for DescriptorSetLayout<L> {
	fn drop(&mut self) {
		unsafe { self.device.handle.destroy_descriptor_set_layout(self.handle, None); }
	}
}

impl<T> DescriptorSetLayoutBuilder<T> {
	pub fn binding<A, D, S>(
		mut self,
		descriptor_array: A,
		shader_stages: S,
		// reserve for future use.
		_immutable_samplers: (),
	) -> DescriptorSetLayoutBuilder<(T, A)>
		where A: Array<D>, D: DescriptorType, S: ShaderStages
	{
		let binding = vk::DescriptorSetLayoutBinding {
			binding: self.binding_index,
			descriptor_type: D::descriptor_type(),
			descriptor_count: A::len() as u32,
			stage_flags: S::shader_stages(),
			p_immutable_samplers: ptr::null(),
		};

		self.bindings.push(binding);
		DescriptorSetLayoutBuilder {
			binding_index: self.binding_index + 1,
			bindings: self.bindings,
			types: PhantomData,
		}
	}

	pub fn skip_this_binding_index(self) -> DescriptorSetLayoutBuilder<(T, ())> {
		DescriptorSetLayoutBuilder {
			binding_index: self.binding_index + 1,
			bindings: self.bindings,
			types: PhantomData,
		}
	}
}

impl<T1, T2> DescriptorSetLayoutBuilder<(T1, T2)> {
	pub fn build(self, device: Arc<Device>) -> Arc<DescriptorSetLayout<(T1, T2)>> {
		let info = vk::DescriptorSetLayoutCreateInfo {
			s_type: StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DescriptorSetLayoutCreateFlags::empty(),
			binding_count: self.bindings.len() as u32,
			p_bindings: self.bindings.as_ptr(),
		};

		let handle = unsafe { device.handle.create_descriptor_set_layout(&info, None).unwrap() };

		Arc::new(DescriptorSetLayout { device, handle, types: self.types })
	}
}

impl DescriptorPool {
	pub fn builder() -> DescriptorPoolBuilder<()> {
		DescriptorPoolBuilder {
			pool_sizes: Vec::new(),
			types: PhantomData,
		}
	}
}

impl Drop for DescriptorPool {
	fn drop(&mut self) { unsafe { self.device.handle.destroy_descriptor_pool(self.handle, None); } }
}

impl DescriptorPoolBuilder<()> {
	/// Register new descriptor layout.
	pub fn layout<T>(
		self,
		layout: Arc<DescriptorSetLayout<T>>,
	) -> DescriptorPoolBuilder<T> {
		DescriptorPoolBuilder {
			pool_sizes: self.pool_sizes,
			types: PhantomData,
		}
	}

	pub fn build(self, max_sets: u32, device: Arc<Device>) -> Arc<DescriptorPool> {
		let info = vk::DescriptorPoolCreateInfo {
			s_type: StructureType::DESCRIPTOR_POOL_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::DescriptorPoolCreateFlags::empty(),
			max_sets,
			pool_size_count: self.pool_sizes.len() as u32,
			p_pool_sizes: self.pool_sizes.as_ptr(),
		};

		let handle = unsafe { device.handle.create_descriptor_pool(&info, None).unwrap() };

		Arc::new(DescriptorPool { device, handle })
	}
}

impl<L, A> DescriptorPoolBuilder<(L, A)> {
	pub fn pool_size<T>(mut self) -> DescriptorPoolBuilder<L> where A: Array<T>, T: DescriptorType {
		let pool_size = vk::DescriptorPoolSize {
			ty: T::descriptor_type(),
			descriptor_count: A::len() as u32,
		};

		self.pool_sizes.push(pool_size);

		DescriptorPoolBuilder {
			pool_sizes: self.pool_sizes,
			types: PhantomData,
		}
	}
}

impl<L> DescriptorSet<L> {
	pub fn new(
		layouts: &[Arc<DescriptorSetLayout<L>>],
		pool: Arc<DescriptorPool>,
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

	pub fn write(set: Self, ) {
		let write_info = vk::WriteDescriptorSet {
			s_type: StructureType::WRITE_DESCRIPTOR_SET,
			p_next: ptr::null(),
			dst_set: vk::DescriptorSet::null(),
			dst_binding: 0,
			dst_array_element: 0,
			descriptor_count: 1,
			descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
			p_buffer_info: ptr::null(),
			p_image_info: ptr::null(),
			p_texel_buffer_view: ptr::null(),
		};

	}
}