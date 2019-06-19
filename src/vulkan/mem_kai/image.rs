pub mod usage;
pub mod extent;
pub mod format;
pub mod sample_count;

use super::*;

pub use usage::ImageUsage;
pub use extent::{ Extent, ArrayLayers };
pub use format::{ Format, DepthFormat, StencilFormat };
pub use sample_count::SampleCount;

pub struct VkImage<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperty
{
	memory: Arc<VkMemory<A, P>>,
	handle: vk::Image,
	extent: E,
	_format: PhantomData<F>,
	_sample_count: PhantomData<S>,
	_usage: PhantomData<U>,
	mip_levels: u32,
	array_layers: u32,
}

// TODO: VkImage need P but VkImageView doesn't need P. (I think.)
pub struct VkImageView<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperty
{
	image: Arc<VkImage<E, F, S, U, A, P>>,
	handle: vk::ImageView,
	mip_range: ops::Range<u32>,
	layer_range: ops::Range<u32>,
}

impl<E, F, S, U, A, P> VkImage<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperty
{
	pub fn new<T>(
		memory: Arc<VkMemory<A, P>>,
		queue: Arc<VkQueue<T>>,
		extent: E,
		_format: F,
		_sample_count: S,
		_usage: U,
		mip_levels: u32,
		array_layers: u32,
		initial_layout: vk::ImageLayout,
	) -> Arc<Self> {
		let info = vk::ImageCreateInfo {
			s_type: StructureType::IMAGE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::ImageCreateFlags::empty(),
			usage: U::image_usage(),
			format: F::format(),
			image_type: E::image_type(),
			extent: extent.extent(),
			samples: S::sample_count(),
			mip_levels,
			array_layers,
			initial_layout,
			sharing_mode: vk::SharingMode::EXCLUSIVE,
			tiling: vk::ImageTiling::OPTIMAL,
			queue_family_index_count: 1,
			p_queue_family_indices: &queue.family_index as *const _,

		};

		let handle = unsafe { memory.device.handle.create_image(&info, None).unwrap() };

		Arc::new(
			Self {
				memory,
				handle,
				extent,
				mip_levels,
				array_layers,
				_usage: PhantomData,
				_format: PhantomData,
				_sample_count: PhantomData,
			}
		)
	}

	#[inline]
	pub fn extent(&self) -> &E { &self.extent }
}

impl<E, F, S, U, A, P> Drop for VkImage<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperty
{
	fn drop(&mut self) { unsafe { self.memory.device.handle.destroy_image(self.handle, None); } }
}

impl<E, F, S, U, A, P> VkImageView<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperty
{
	pub fn new(
		image: Arc<VkImage<E, F, S, U, A, P>>,
		aspect: vk::ImageAspectFlags,
		mip_range: ops::Range<u32>,
		layer_range: E::ArrayLayers,
	) -> Arc<Self> {
		// TODO: consider component mapping.

		let (base_array_layer, layer_count) = layer_range.base_layer_and_count();
		let info = vk::ImageViewCreateInfo {
			s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::ImageViewCreateFlags::empty(),
			image: image.handle,
			view_type: layer_range.view_type(),
			format: F::format(),
			components: vk::ComponentMapping {
				r: vk::ComponentSwizzle::IDENTITY,
				g: vk::ComponentSwizzle::IDENTITY,
				b: vk::ComponentSwizzle::IDENTITY,
				a: vk::ComponentSwizzle::IDENTITY,
			},
			subresource_range: vk::ImageSubresourceRange {
				aspect_mask: aspect,
				base_mip_level: mip_range.start,
				level_count: mip_range.end - mip_range.start,
				base_array_layer,
				layer_count,
			},
		};

		let handle = unsafe {
			image.memory.device.handle.create_image_view(&info, None).unwrap()
		};

		Arc::new(Self { image, handle, mip_range, layer_range: layer_range.layer_range() })
	}

	#[inline]
	pub fn handle(&self) -> vk::ImageView { self.handle }

	#[inline]
	pub fn image(&self) -> &Arc<VkImage<E, F, S, U, A, P>> { &self.image }

	#[inline]
	pub fn layer_range(&self) -> &ops::Range<u32> { &self.layer_range }
}

impl<E, F, S, U, A, P> Drop for VkImageView<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperty
{
	fn drop(&mut self) {
		unsafe { self.image.memory.device.handle.destroy_image_view(self.handle, None); }
	}
}