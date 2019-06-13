pub mod usage;
pub mod extent;
pub mod format;
pub mod sample_count;
pub mod aspect;

use super::*;

pub use usage::ImageUsage;
pub use extent::Extent;
pub use format::{ Format, DepthFormat, StencilFormat };
pub use sample_count::SampleCount;

pub struct VkImage<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperties
{
	memory: Arc<VkMemory<A, P>>,
	handle: vk::Image,
	extent: E,
	mip_levels: u32,
	array_layers: u32,
	_usage: PhantomData<U>,
	_format: PhantomData<F>,
	_sample_count: PhantomData<S>,
}

pub struct VkImageView<A, E, F, S, U, MA, P>
	where A: Aspect,
		  E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  MA: Allocator,
		  P: MemoryProperties
{
	image: Arc<VkImage<E, F, S, U, A, P>>,
	handle: vk::ImageView,
	aspect: PhantomData<A>,
}

impl<E, F, S, U, A, P> VkImage<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperties
{
	pub fn new(
		memory: VkMemory<A, P>,
		extent: E,
		mip_levels: u32,
		array_layers: u32,
		_usage: U,
		_format: F,
		_sample_count: S,
		initial_layout: vk::ImageLayout,
	) -> Arc<Self> {
		let queue_family_index = memory.device.physical_device_index as u32;

		let info = vk::ImageCreateInfo {
			s_type: StructureType::IMAGE_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::ImageCreateFlags::empty(),
			usage: U::flags(),
			format: F::format(),
			image_type: E::image_type(),
			extent: extent.extent(),
			samples: S::flags(),
			mip_levels,
			array_layers,
			initial_layout,
			sharing_mode: vk::SharingMode::EXCLUSIVE,
			tiling: vk::ImageTiling::OPTIMAL,
			queue_family_index_count: 1,
			p_queue_family_indices: &queue_family_index as *const _,

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
}

impl<E, F, S, U, A, P> Drop for VkImage<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperties
{
	fn drop(&mut self) { unsafe { self.memory.device.handle.destroy_image(self.handle, None); } }
}

impl<E, F, S, U, A, P> VkImageView<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperties
{
	pub fn new(
		image: Arc<VkImage<E, F, S, U, A, P>>,
		aspect: vk::ImageAspectFlags,
		mip_level_range: ops::Range<u32>,
		array_layer_range: ops::Range<u32>,
	) -> Arc<Self> {
		// TODO: decide view_type by array_layer_range and F.
		// ref: https://vulkan.lunarg.com/doc/view/1.0.26.0/linux/vkspec.chunked/ch11s05.html

		// TODO: consider component mapping.

		let info = vk::ImageViewCreateInfo {
			s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
			p_next: ptr::null(),
			flags: vk::ImageViewCreateFlags::empty(),
			image: image.handle,
			view_type: unimplemented!(),
			format: F::format(),
			components: vk::ComponentMapping {
				r: vk::ComponentSwizzle::IDENTITY,
				g: vk::ComponentSwizzle::IDENTITY,
				b: vk::ComponentSwizzle::IDENTITY,
				a: vk::ComponentSwizzle::IDENTITY,
			},
			subresource_range: vk::ImageSubresourceRange {
				aspect_mask: aspect,
				base_mip_level: mip_level_range.start,
				level_count: mip_level_range.end - mip_level_range.start,
				base_array_layer: array_layer_range.start,
				layer_count: array_layer_range.end - array_layer_range.start,
			},
		};

		let handle = unsafe {
			image.memory.device.handle.create_image_view(&info, None).unwrap()
		};

		Arc::new(Self { image, handle })
	}
}

impl<E, F, S, U, A, P> Drop for VkImageView<E, F, S, U, A, P>
	where E: Extent,
		  F: Format,
		  S: SampleCount,
		  U: ImageUsage,
		  A: Allocator,
		  P: MemoryProperties
{
	fn drop(&mut self) {
		unsafe { self.image.memory.device.handle.destroy_image_view(self.handle, None); }
	}
}