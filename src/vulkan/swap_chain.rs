use super::*;
use super::mem_kai::image::*;

pub struct VkSwapchainKHR<U, F> {
	device: Arc<VkDevice>,
	surface: Arc<VkSurfaceKHR>,
	loader: khr::Swapchain,
	handle: vk::SwapchainKHR,
	color_space: vk::ColorSpaceKHR,
	min_image_count: u32,
	present_mode: vk::PresentModeKHR,
	_usage: PhatomData<U>,
	_format: PhantomData<F>,
}

impl<U, F> VkSwapchainKHR<U, F> where U: ImageUsage, F: Format {
	pub fn new<U, F>(
		device: Arc<VkDevice>,
		surface: Arc<VkSurfaceKHR>,
		present_mode: vk::PresentModeKHR,
		min_image_count: u32,
		_usage: U,
		_format: F,
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
			.find(|f| f.format == F::format())
			.unwrap();

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
			image_usage: U::flags(),
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

		Arc::new(Self {
			device,
			surface,
			loader,
			handle,
			color_space: surface_format.color_space,
			min_image_count,
			present_mode,
			_usage: PhantomData,
			_format: PhantomData,
		})
	}

	unsafe fn recreate(self: Arc<Self>) -> Arc<Self> {
		let surface_capabilities = unsafe {
			self.surface.loader
				.get_physical_device_surface_capabilities(
					self.device.instance.physical_devices[self.device.physical_device_index].handle,
					self.surface.handle,
				)
				.unwrap()
		};

		let info = vk::SwapchainCreateInfoKHR {
			s_type: StructureType::SWAPCHAIN_CREATE_INFO_KHR,
			p_next: ptr::null(),
			flags: vk::SwapchainCreateFlagsKHR::empty(),
			surface: self.surface.handle,
			min_image_count: self.min_image_count,
			image_format: F::format(),
			image_color_space: self.color_space,
			image_extent: surface_capabilities.current_extent,
			image_array_layers: 1,
			image_usage: U::flags(),
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

		let new_one = VkSwapchainKHR {
			device: self.device.clone(),
			surface: self.surface.clone(),
			loader: self.loader,
			handle: new_handle,
			min_image_count: self.min_image_count,
			color_space: self.color_space,
			present_mode: self.present_mode,
			_usage: PhantomData,
			_format: PhantomData,
		};

		unimplemented!()
	}
}

impl<U, F> Drop for VkSwapchainKHR<U, F> {
	fn drop(&mut self) { unsafe { self.loader.destroy_swapchain(self.handle, None); } }
}