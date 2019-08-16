use super::*;
use image::Extent2D;

use std::ops::Range;
use std::time::Duration;

pub struct SwapchainKHR<I, S, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    S: Borrow<SurfaceKHR<I>> + Deref<Target = SurfaceKHR<I>>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    _marker: PhantomData<I>,
    surface: S,
    device: D,
    loader: khr::Swapchain,
    handle: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    usage: vk::ImageUsageFlags,
    extent: Extent2D,
    format: vk::Format,
    color_space: vk::ColorSpaceKHR,
    min_image_count: u32,
    present_mode: vk::PresentModeKHR,
}

pub struct SwapchainImageView<I, S, D, Sw> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    S: Borrow<SurfaceKHR<I>> + Deref<Target = SurfaceKHR<I>>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    Sw: Borrow<SwapchainKHR<I, S, D>> + Deref<Target = SwapchainKHR<I, S, D>>,
{
    _marker: PhantomData<(I, S, D)>,
    swapchain: Sw,
    handle: vk::ImageView,
}

impl<I, S, D> SwapchainKHR<I, S, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    S: Borrow<SurfaceKHR<I>> + Deref<Target = SurfaceKHR<I>>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    pub fn new(
        surface: S,
        device: D,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        present_mode: vk::PresentModeKHR,
        min_image_count: u32,
    ) -> Self {
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
            panic!("min_image_count: {} is invalid.\ncapabilities: {:?}", min_image_count, surface_capabilities);
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
//            .inspect(|f| println!("format: {:?}", f))
            .find(|f| f.format == format)
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
            image_usage: usage,
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

        let handle = unsafe { loader.create_swapchain(&info, None).unwrap() };

        let images = unsafe { loader.get_swapchain_images(handle).unwrap() };

        Self {
            _marker: PhantomData,
            device,
            surface,
            loader,
            handle,
            images,
            usage,
            extent: Extent2D { width: window_extent.width, height: window_extent.height },
            format,
            color_space: surface_format.color_space,
            min_image_count,
            present_mode,
        }
    }


    // NOTE: Multiple mip levels and array layers maybe need in the future.
    pub fn views<Sw>(swapchain: Sw) -> Vec<SwapchainImageView<I, S, D, Sw>>
        where Sw: Borrow<Self> + Deref<Target = Self> + Clone,
    {
        swapchain.images.iter()
            .map(|image| {
                let info = vk::ImageViewCreateInfo {
                    s_type: StructureType::IMAGE_VIEW_CREATE_INFO,
                    p_next: ptr::null(),
                    flags: vk::ImageViewCreateFlags::empty(),
                    image: *image,
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: swapchain.format,
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

                SwapchainImageView { _marker: PhantomData, swapchain: swapchain.clone(), handle }
            })
            .collect()
    }

    pub fn recreate(mut self) -> Self {
        let new_extent = unsafe {
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
            image_format: self.format,
            image_color_space: self.color_space,
            image_extent: new_extent,
            image_array_layers: 1,
            image_usage: self.usage,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            pre_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode: self.present_mode,
            clipped: vk::TRUE,
            old_swapchain: self.handle,
        };

        let new_handle = unsafe { self.loader.create_swapchain(&info, None).unwrap() };
        let new_images = unsafe { self.loader.get_swapchain_images(new_handle).unwrap() };

        unsafe { self.loader.destroy_swapchain(self.handle, None); }

        self.extent = Extent2D { width: new_extent.width, height: new_extent.height };
        self.handle = new_handle;
        self.images = new_images;

        self
    }

    pub fn acquire_next_image(
        &self,
        timeout: Duration,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> u32 {
        unsafe {
            self.loader
                .acquire_next_image(self.handle, timeout.as_nanos() as u64, semaphore, fence)
                .unwrap()
                .0
        }
    }

    pub fn queue_present(
        &self,
        queue: vk::Queue,
        image_index: u32,
        wait_semaphores: &[vk::Semaphore]
    ) {
        unsafe {
            self.loader
                .queue_present(
                    queue,
                    &vk::PresentInfoKHR::builder()
                        .swapchains(&[self.handle])
                        .image_indices(&[image_index])
                        .wait_semaphores(wait_semaphores)
                )
                .unwrap();
        }
    }

    #[inline]
    pub fn whole_area(&self) -> vk::Rect2D {
        vk::Rect2D::builder()
            .offset(vk::Offset2D::builder().x(0).y(0).build())
            .extent(self.vk_extent_2d())
            .build()
    }
    #[inline]
    pub fn extent(&self) -> &Extent2D { &self.extent }
    #[inline]
    pub fn vk_extent_2d(&self) -> vk::Extent2D {
        vk::Extent2D { width: self.extent.width, height: self.extent.height }
    }
    #[inline]
    pub fn image_count(&self) -> usize { self.images.len() }
}

impl<I, S, D> Drop for SwapchainKHR<I, S, D> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    S: Borrow<SurfaceKHR<I>> + Deref<Target = SurfaceKHR<I>>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
{
    fn drop(&mut self) { unsafe { self.loader.destroy_swapchain(self.handle, None); } }
}

impl<I, S, D, Sw> SwapchainImageView<I, S, D, Sw> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    S: Borrow<SurfaceKHR<I>> + Deref<Target = SurfaceKHR<I>>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    Sw: Borrow<SwapchainKHR<I, S, D>> + Deref<Target = SwapchainKHR<I, S, D>>,
{
    #[inline]
    pub fn handle(&self) -> vk::ImageView { self.handle }
    #[inline]
    pub fn extent(&self) -> &Extent2D { &self.swapchain.extent }
}

impl<I, S, D, Sw> Drop for SwapchainImageView<I, S, D, Sw> where
    I: Borrow<Instance> + Deref<Target = Instance>,
    S: Borrow<SurfaceKHR<I>> + Deref<Target = SurfaceKHR<I>>,
    D: Borrow<Device<I>> + Deref<Target = Device<I>>,
    Sw: Borrow<SwapchainKHR<I, S, D>> + Deref<Target = SwapchainKHR<I, S, D>>,
{
    fn drop(&mut self) {
        unsafe { self.swapchain.device.handle.destroy_image_view(self.handle, None); }
    }
}