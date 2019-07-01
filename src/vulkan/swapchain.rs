use super::*;
use super::mem::*;
use super::mem::image::*;

use std::ops::Range;

pub struct VkSwapchainKhr<F, U> where F: Format, U: ImageUsage {
	device: Arc<Device>,
	surface: Arc<SurfaceKhr>,
	loader: khr::Swapchain,
	handle: vk::SwapchainKHR,
	images: Vec<vk::Image>,
	extent: extent::Extent2D,
	color_space: vk::ColorSpaceKHR,
	min_image_count: u32,
	present_mode: vk::PresentModeKHR,
	_usage: PhantomData<U>,
	_format: PhantomData<F>,
}

pub struct VkSwapchainImageView<F, U> where F: Format, U: ImageUsage {
	swapchain: Arc<VkSwapchainKhr<F, U>>,
	handle: vk::ImageView,
}

impl<F, U> VkSwapchainKhr<F, U> where U: ImageUsage, F: Format {
	pub fn new(
		device: Arc<Device>,
		surface: Arc<SurfaceKhr>,
		_usage: U,
		_format: F,
		present_mode: vk::PresentModeKHR,
		min_image_count: u32,
	) -> Arc<Self> {
		let surface_capabilities = unsafe {
			surface.loader
				.get_physical_device_surface_capabilities(
					surface.instance.physical_devices[device.physical_device_index].handle,
					surface.handle,
				)
				.unwrap()
		};

		let min_allowed = surface_capabilities.min_image_count <= min_image_count;
		let max_allowed = min_image_count <= surface_capabilities.max_image_count;
		let max_unlimited = surface_capabilities.max_image_count == 0;
		if !(min_allowed && (max_allowed || max_unlimited)) {
			panic!("min_image_count is invalid.");
		}

		let window_extent = surface_capabilities.current_extent;

		let surface_formats = unsafe {
			surface.loader
				.get_physical_device_surface_formats(
					surface.instance.physical_devices[device.physical_device_index].handle,
					surface.handle,
				)
				.unwrap()
		};

		let surface_format = surface_formats
			.iter()
			.inspect(|f| println!("format: {:?}", f))
			.find(|f| f.format == F::format())
			.unwrap();

		unsafe {
			surface.loader
				.get_physical_device_surface_present_modes(
					surface.instance.physical_devices[device.physical_device_index].handle,
					surface.handle,
				)
				.unwrap()
				.iter()
				.find(|mode| **mode == present_mode)
				.unwrap()
		};

		let info = vk::SwapchainCreateInfoKHR {
			s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::SwapchainCreateFlagsKHR::empty(),
			surface: surface.handle,
			min_image_count,
			image_format: surface_format.format,
			image_color_space: surface_format.color_space,
			image_extent: window_extent,
			image_array_layers: 1,
			image_usage: U::image_usage(),
			image_sharing_mode: vk::SharingMode::EXCLUSIVE,
			queue_family_index_count: 0,
			p_queue_family_indices: ptr::null(),
			pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
			composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
			present_mode,
			clipped: vk::TRUE,
			old_swapchain: vk::SwapchainKHR::null(),
		};

		let loader = unsafe { khr::Swapchain::new(&device.instance.handle, &device.handle) };

		let handle = unsafe {
			loader
				.create_swapchain(&info, None)
				.unwrap()
		};

		let images = unsafe { loader.get_swapchain_images(handle).unwrap() };

		Arc::new(Self {
			device,
			surface,
			loader,
			handle,
			images,
			extent: extent::Extent2D { width: window_extent.width, height: window_extent.height },
			color_space: surface_format.color_space,
			min_image_count,
			present_mode,
			_usage: PhantomData,
			_format: PhantomData,
		})
	}


	// NOTE: Multiple mip levels and array layers maybe need in the future.
	pub fn views(swapchain: &Arc<Self>) -> Vec<Arc<VkSwapchainImageView<F, U>>> {
		swapchain.images.iter()
			.map(|image| {
				let info = vk::ImageViewCreateInfo {
					s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
					p_next: ptr::null(),
					flags: vk::ImageViewCreateFlags::empty(),
					image: *image,
					view_type: vk::ImageViewType::TYPE_2D,
					format: F::format(),
					components: vk::ComponentMapping {
						r: vk::ComponentSwizzle::IDENTITY,
						g: vk::ComponentSwizzle::IDENTITY,
						b: vk::ComponentSwizzle::IDENTITY,
						a: vk::ComponentSwizzle::IDENTITY,
					},
					subresource_range: vk::ImageSubresourceRange {
						aspect_mask: vk::ImageAspectFlags::COLOR,
						base_mip_level: 0,
						level_count: 1,
						base_array_layer: 0,
						layer_count: 1,
					},
				};

				let handle = unsafe {
					swapchain.device.handle.create_image_view(&info, None).unwrap()
				};

				Arc::new(VkSwapchainImageView { swapchain: swapchain.clone(), handle })
			})
			.collect()
	}

	pub unsafe fn recreate(self: Arc<Self>) -> Arc<Self> {
		let extent = unsafe {
			self.surface.loader
				.get_physical_device_surface_capabilities(
					self.device.instance.physical_devices[self.device.physical_device_index].handle,
					self.surface.handle,
				)
				.unwrap()
				.current_extent
		};

		let info = vk::SwapchainCreateInfoKHR {
			s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::SwapchainCreateFlagsKHR::empty(),
			surface: self.surface.handle,
			min_image_count: self.min_image_count,
			image_format: F::format(),
			image_color_space: self.color_space,
			image_extent: extent,
			image_array_layers: 1,
			image_usage: U::image_usage(),
			image_sharing_mode: vk::SharingMode::EXCLUSIVE,
			queue_family_index_count: 0,
			p_queue_family_indices: ptr::null(),
			pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
			composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
			present_mode: self.present_mode,
			clipped: vk::TRUE,
			old_swapchain: self.handle,
		};

		unsafe { self.loader.destroy_swapchain(self.handle, None); }

		let new_handle = unsafe { self.loader.create_swapchain(&info, None).unwrap() };

		let new_images = unsafe { self.loader.get_swapchain_images(new_handle).unwrap() };

		let new_one = VkSwapchainKhr {
			device: self.device.clone(),
			surface: self.surface.clone(),
			loader: self.loader.clone(),
			handle: new_handle,
			images: new_images,
			extent: extent::Extent2D { width: extent.width, height: extent.height },
			min_image_count: self.min_image_count,
			color_space: self.color_space,
			present_mode: self.present_mode,
			_usage: PhantomData,
			_format: PhantomData,
		};


		// TODO: consider old swapchains remained by Arc.
		unimplemented!();

		Arc::new(new_one)
	}

	pub fn extent(&self) -> &extent::Extent2D { &self.extent }
}

impl<F, U> Drop for VkSwapchainKhr<F, U> where F: Format, U: image::ImageUsage {
	fn drop(&mut self) { unsafe { self.loader.destroy_swapchain(self.handle, None); } }
}

impl<F, U> VkSwapchainImageView<F, U> where F: Format, U: image::ImageUsage {
	#[inline]
	pub fn handle(&self) -> vk::ImageView { self.handle }
	#[inline]
	pub fn extent(&self) -> &extent::Extent2D { &self.swapchain.extent }
}

impl<F, U> Drop for VkSwapchainImageView<F, U> where F: Format, U: image::ImageUsage {
	fn drop(&mut self) {
		unsafe { self.swapchain.device.handle.destroy_image_view(self.handle, None); }
	}
}