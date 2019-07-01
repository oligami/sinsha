pub mod ty;

use super::*;

pub use ty::DescriptorType;

use stage::ShaderStages;

/// Used for get length of a descriptor array.
pub trait DescriptorArray {
	type Type: DescriptorType;
	fn len() -> u32;
}

macro_rules! impl_array_length {
	($($len: expr),*) => {$(
		impl<T> DescriptorArray for [T; $len] where T: DescriptorType {
			type Type = T;
			fn len() -> u32 { $len as u32 }
		}
	)*};
}

impl_array_length!(
	1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
	17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
);

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
	pub fn binding<D, S>(
		mut self,
		descriptor_array: D,
		shader_stages: S,
		// reserve for future use.
		_immutable_samplers: (),
	) -> DescriptorSetLayoutBuilder<(T, D)>
		where D: DescriptorArray, S: ShaderStages
	{
		let binding = vk::DescriptorSetLayoutBinding {
			binding: self.binding_index,
			descriptor_type: D::Type::descriptor_type(),
			descriptor_count: D::len(),
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

impl<T1, T2> DescriptorPoolBuilder<(T1, T2)>
	where T2: DescriptorArray,
{
	pub fn pool_size(mut self) -> DescriptorPoolBuilder<T1> {
		let pool_size = vk::DescriptorPoolSize {
			ty: T2::Type::descriptor_type(),
			descriptor_count: T2::len(),
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
}