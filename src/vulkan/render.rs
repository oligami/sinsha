use super::*;

pub struct VkRender<D: Borrow<device::Device>> {
    device: D,
    surface: SurfaceKHR,
    swapchain: SwapchainKHR,
    render_pass: vk::RenderPass,
    descriptor_layouts: Vec<vk::DescriptorSetLayout>,
    pipeline_layouts: Vec<vk::PipelineLayout>,
    pipelines: Vec<vk::Pipeline>,
    framebuffers: Framebuffers,
}

struct SurfaceKHR {
    window: Window,
    loader: khr::Surface,
    handle: vk::SurfaceKHR,
}

struct SwapchainKHR {
    loader: khr::Swapchain,
    handle: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    min_image_count: u32,
    format: vk::Format,
    color_space: vk::ColorSpaceKHR,
    extent: vk::Extent2D,
    present_mode: vk::PresentModeKHR,
}

struct Framebuffers {
    handles: Vec<vk::Framebuffer>,
    memory: vk::DeviceMemory,
    images: Vec<[vk::Image; 4]>,
    views: Vec<[vk::ImageView; 5]>,
}

impl<D: Borrow<device::Device>> VkRender<D> {
    pub fn new(device: D) -> Self {
        let surface = unsafe { Self::create_surface(device.borrow(), unimplemented!()) };
        let swapchain = unsafe { Self::create_swapchain(device.borrow(), &surface) };
        let render_pass = Self::create_render_pass(device.borrow());
        let framebuffers = Self::create_framebuffers(
            &device.borrow().device,
            &device.borrow().physical_device,
            &swapchain,
            render_pass,
        );

        unimplemented!()
    }


    /// # Safety
    /// Ensure the device has surface extension.
    unsafe fn create_surface(device: &device::Device, window: Window) -> SurfaceKHR {
        let loader = khr::Surface::new(&device.entry, &device.instance);
        let handle = unsafe { SurfaceKHR::handle(&device.entry, &device.instance, &window) };

        SurfaceKHR { window, loader, handle }
    }

    /// # Safety
    /// Ensure the device has swapchain extension.
    unsafe fn create_swapchain(device: &device::Device, surface: &SurfaceKHR) -> SwapchainKHR {
        let loader = khr::Swapchain::new(&device.instance, &device.device);

        // evaluate minimum image count.
        let capabilities = surface.loader
            .get_physical_device_surface_capabilities(
                device.physical_device.handle,
                surface.handle,
            )
            .unwrap();
        let min_image_count = if capabilities.min_image_count == capabilities.max_image_count {
            capabilities.min_image_count
        } else {
            capabilities.min_image_count + 1
        };


        // select format and color space.
        let supported_surface_format = surface.loader
            .get_physical_device_surface_formats(device.physical_device.handle, surface.handle)
            .unwrap();
        let &vk::SurfaceFormatKHR { format, color_space } = supported_surface_format.iter()
            .find(|format| format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .unwrap_or(supported_surface_format.iter().next().unwrap());

        // evaluate extent.
        let extent = capabilities.current_extent;

        // evaluate present mode.
        let supported_present_modes = surface.loader
            .get_physical_device_surface_present_modes(
                device.physical_device.handle,
                surface.handle)
            .unwrap();
        let &present_mode = supported_present_modes.iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .or_else(|| {
                supported_present_modes.iter()
                    .find(|mode| **mode == vk::PresentModeKHR::FIFO)
            })
            .unwrap_or(supported_present_modes.iter().next().unwrap());

        // create vk::SwapchainKHR.
        let info = vk::SwapchainCreateInfoKHR::builder()
            .flags(vk::SwapchainCreateFlagsKHR::empty())
            .surface(surface.handle)
            .min_image_count(min_image_count)
            .image_format(format)
            .image_color_space(color_space)
            .image_extent(unimplemented!())
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&[])
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(unimplemented!())
            .clipped(true);

        let handle = loader.create_swapchain(&info, None).unwrap();

        let images = loader.get_swapchain_images(handle).unwrap();

        SwapchainKHR {
            loader,
            handle,
            images,
            min_image_count,
            format,
            color_space,
            extent,
            present_mode,
        }
    }


    // These are for render pass
    const SAMPLE_COUNT: vk::SampleCountFlags = vk::SampleCountFlags::TYPE_1;
    const SWAPCHAIN_ATTACHMENT_INDEX: u32 = 0;
    const DEPTH_ATTACHMENT_INDEX: u32 = 1;
    const POSITION_G_BUFFER_ATTACHMENT_INDEX: u32 = 2;
    const NORMAL_G_BUFFER_ATTACHMENT_INDEX: u32 = 3;
    const COLOR_G_BUFFER_ATTACHMENT_INDEX: u32 = 4;

    fn create_render_pass(device: &VkDevice) -> vk::RenderPass {
        let attachments = [
            // To present on surface.
            vk::AttachmentDescription::builder()
                .format(unimplemented!("This must be the swapchain format."))
                .samples(vk::SampleCountFlags::TYPE_1)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .build(),

            // Depth image.
            vk::AttachmentDescription::builder()
                .format(vk::Format::D32_SFLOAT)
                .samples(Self::SAMPLE_COUNT)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::UNDEFINED)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .build(),

            // This is what is called G-Buffer. Storing positions of vertex.
            vk::AttachmentDescription::builder()
                .format(vk::Format::R32G32B32_SFLOAT)
                .samples(Self::SAMPLE_COUNT)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) // TODO: Is this valid?
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .build(),

            // This is what is called G-Buffer. Storing normal vectors of vertex.
            vk::AttachmentDescription::builder()
                .format(vk::Format::R32G32B32_SFLOAT)
                .samples(Self::SAMPLE_COUNT)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) // TODO: Is this valid?
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .build(),

            // This is what is called G-Buffer. Storing color of vertex.
            vk::AttachmentDescription::builder()
                .format(vk::Format::R32G32B32_SFLOAT)
                .samples(Self::SAMPLE_COUNT)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) // TODO: Is this valid?
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .build(),

            // TODO: Need Texture coordinates?
        ];

        // Render objects in G-Buffers.
        let first_subpass_color_attachments = [
            vk::AttachmentReference::builder()
                .attachment(Self::POSITION_G_BUFFER_ATTACHMENT_INDEX)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build(),
            vk::AttachmentReference::builder()
                .attachment(Self::NORMAL_G_BUFFER_ATTACHMENT_INDEX)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build(),
            vk::AttachmentReference::builder()
                .attachment(Self::COLOR_G_BUFFER_ATTACHMENT_INDEX)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build(),
        ];

        // Swapchain images.
        let second_subpass_color_attachments = [
            vk::AttachmentReference::builder()
                .attachment(Self::SWAPCHAIN_ATTACHMENT_INDEX)
                .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build(),
        ];

        // G-Buffers.
        let second_subpass_input_attachments = [
            vk::AttachmentReference::builder()
                .attachment(Self::POSITION_G_BUFFER_ATTACHMENT_INDEX)
                .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .build(),
            vk::AttachmentReference::builder()
                .attachment(Self::NORMAL_G_BUFFER_ATTACHMENT_INDEX)
                .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .build(),
            vk::AttachmentReference::builder()
                .attachment(Self::COLOR_G_BUFFER_ATTACHMENT_INDEX)
                .layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .build(),
        ];

        let subpasses = [
            // Render objects in G-Buffers.
            vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&first_subpass_color_attachments[..])
                .build(),

            // Do lighting and write to swapchain images.
            vk::SubpassDescription::builder()
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .color_attachments(&second_subpass_color_attachments[..])
                .input_attachments(&second_subpass_input_attachments[..])
                .build(),
        ];

        // Sync 2 subpasses and swapchain image layout transition
        // (ColorAttachmentOptimal -> PresentSrcKHR) happened in second subpass.
        let subass_dependencies = [
            vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(unimplemented!())
                .dst_stage_mask(unimplemented!())
                .src_access_mask(unimplemented!())
                .dst_access_mask(unimplemented!())
                .dependency_flags(vk::DependencyFlags::BY_REGION)
                .build(),
            vk::SubpassDependency::builder()
                .src_subpass(0)
                .dst_subpass(1)
                .src_stage_mask(unimplemented!())
                .dst_stage_mask(unimplemented!())
                .src_access_mask(unimplemented!())
                .dst_access_mask(unimplemented!())
                .dependency_flags(vk::DependencyFlags::BY_REGION)
                .build(),
            vk::SubpassDependency::builder()
                .src_subpass(1)
                .dst_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(unimplemented!())
                .dst_stage_mask(unimplemented!())
                .src_access_mask(unimplemented!())
                .dst_access_mask(unimplemented!())
                .dependency_flags(vk::DependencyFlags::BY_REGION)
                .build(),
        ];

        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments[..])
            .subpasses(&subpasses[..])
            .dependencies(&subass_dependencies[..]);

        let render_pass = unsafe { device.create_render_pass(&render_pass_info, None).unwrap() };

        render_pass
    }

    fn descriptor_layouts() -> Vec<vk::DescriptorSetLayout> {
        unimplemented!()
    }

    fn pipeline_layouts() -> Vec<vk::PipelineLayout> {
        unimplemented!()
    }

    fn pipelines() -> Vec<vk::Pipeline> {
        unimplemented!()
    }

    fn create_framebuffers(
        device: &VkDevice,
        physical_device: &PhysicalDevice,
        swapchain: &SwapchainKHR,
        render_pass: vk::RenderPass,
    ) -> Framebuffers {
        // Create images. --
        let g_buffer_info = vk::ImageCreateInfo::builder()
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::INPUT_ATTACHMENT)
            .format(vk::Format::R32G32B32_SFLOAT)
            .image_type(vk::ImageType::TYPE_2D)
            .extent(
                vk::Extent3D::builder()
                    .width(swapchain.extent.width)
                    .height(swapchain.extent.height)
                    .depth(1)
                    .build()
            )
            .samples(Self::SAMPLE_COUNT)
            .mip_levels(1)
            .array_layers(1)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .tiling(vk::ImageTiling::OPTIMAL)
            .queue_family_indices(&[]);

        let depth_info = {
            let mut temp = g_buffer_info.clone();
            temp.usage = vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
            temp.format = vk::Format::D32_SFLOAT;
            temp
        };

        // Infos of vk::Image for one framebuffer.
        let infos_for_one_framebuffer = [
            depth_info,
            // These are "Deref" magic.
            g_buffer_info.clone(),
            g_buffer_info.clone(),
            g_buffer_info.clone(),
        ];

        // Create vk::Images for all framebuffers.
        let images = (0..swapchain.images.len())
            .map(|_| {
                let mut images_for_one_framebuffer = [vk::Image::default(); 4];
                images_for_one_framebuffer
                    .iter_mut()
                    .zip(infos_for_one_framebuffer.iter())
                    .for_each(|(image, info)| {
                        *image = unsafe { device.create_image(info, None).unwrap() };
                    });

                images_for_one_framebuffer
            })
            .collect::<Vec<_>>();

        // Allocate vk::DeviceMemory. --
        // Select memory type.
        let mut requirements = [vk::MemoryRequirements::default(); 4];
        let (supported_memory_types, size) = images[0]
            .iter()
            .zip(requirements.iter_mut())
            .fold((!0, 0), |(bit_flags, size), (image, requirements)| {
                *requirements = unsafe { device.get_image_memory_requirements(*image) };
                let new_bit_flags = bit_flags & requirements.memory_type_bits;
                let new_size = size + requirements.size;
                (new_bit_flags, new_size)
            });
        let requirements = requirements;

        let memory_type_index = physical_device.memory_types
            .iter()
            .enumerate()
            .position(|(i, vk::MemoryType { property_flags, .. })| {
                let device_local = property_flags.contain(vk::MemoryPropertyFlags::DEVICE_LOCAL);
                let bit_of_this_index = 1 << i as u32;
                let supported = supported_memory_types & bit_of_this_index != 0;
                device_local && supported
            })
            .unwrap() as u32;

        // Allocate memory.
        let memory_info = vk::MemoryAllocateInfo::builder()
            .memory_type_index(memory_type_index)
            .allocation_size(size);
        let memory = unsafe { device.allocate_memory(&memory_info, None).unwrap() };

        // Bind vk::Images to vk::DeviceMemory, and then, create vk::ImageViews. --
        // Infos of vk::ImageViews for one framebuffer. (but, only image field is invalid)
        let g_buffer_info = vk::ImageViewCreateInfo::builder()
            .flags(vk::ImageViewCreateFlags::empty())
            .format(vk::Format::R32G32B32_SFLOAT)
            .view_type(vk::ImageViewType::TYPE_2D)
            .components(
                vk::ComponentMapping::builder()
                    .r(vk::ComponentSwizzle::IDENTITY)
                    .g(vk::ComponentSwizzle::IDENTITY)
                    .b(vk::ComponentSwizzle::IDENTITY)
                    .a(vk::ComponentSwizzle::IDENTITY)
                    .build()
            )
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build()
            );
        let depth_image_view_info = {
            let mut tmp = g_buffer_info.clone();
            tmp.format = vk::Format::D32_SFLOAT;
            tmp.subresource_range.aspect_mask = vk::ImageAspectFlags::DEPTH;
            tmp
        };
        let swapchain_image_view_info = {
            let mut tmp = g_buffer_info.clone();
            tmp.format = swapchain.format;
            tmp
        };

        // Uninitialized vk::ImageViews.
        let mut views = vec![[vk::ImageView::default(); 5]; swapchain.images.len()];

        // Create vk::ImageViews.
        swapchain.images
            .iter()
            .zip(images.iter())
            .zip(views.iter_mut())
            .fold(0, |offset, ((swapchain_image, images), image_views_uninit)| {
                // Create vk::ImageViews of swapchain.
                // For now, the image field of the info is null, so make it a valid handle.
                let mut info = swapchain_image_view_info.clone();
                info.image = *swapchain_image;
                image_views_uninit[0] = unsafe { device.create_image_view(&info, None).unwrap() };

                let infos = [
                    depth_image_view_info,
                    g_buffer_info.clone(),
                    g_buffer_info.clone(),
                    g_buffer_info.clone(),
                ];

                images
                    .iter()
                    .zip(image_views_uninit[1..].iter_mut())
                    .zip(requirements.iter().zip(infos.iter()))
                    .fold(offset, |offset, ((image, image_view), (requirements, info))| {
                        // Adjust alignment.
                        let offset = if offset % requirements.alignment != 0 {
                            (offset / requirements.alignment + 1) * requirements.alignment
                        } else {
                            offset
                        };
                        unsafe { device.bind_image_memory(*image, memory, offset).unwrap() };

                        // Create vk::ImageView.
                        // For now, the image field of the info is null, so make it a valid handle.
                        let mut info = info.clone();
                        info.image = *image;
                        *image_view = unsafe { device.create_image_view(&info, None).unwrap() };

                        offset + requirements.size
                    })
            });

        // Create vk::Framebuffers. --
        let handles = views
            .iter()
            .map(|views| {
                let framebuffer_info = vk::FramebufferCreateInfo::builder()
                    .flags(vk::FramebufferCreateFlags::empty())
                    .render_pass(unimplemented!())
                    .width(swapchain.extent.width)
                    .height(swapchain.extent.height)
                    .layers(1)
                    .attachments(views);

                unsafe { device.create_framebuffer(&info, None).unwrap() }
            })
            .collect::<Vec<_>>();

        Framebuffers { handles, memory, images, views }
    }
}

impl SurfaceKHR {
    #[cfg(target_os = "windows")]
    unsafe fn handle(
        entry: &Entry,
        instance: &VkInstance,
        window: &Window
    ) -> vk::SurfaceKHR {
        use winapi::um::libloaderapi::GetModuleHandleW;
        use winit::platform::windows::WindowExtWindows;

        let info = vk::Win32SurfaceCreateInfoKHR::builder()
            .hwnd(window.hwnd())
            .hinstance(GetModuleHandleW(ptr::null()) as _);

        khr::Win32Surface::new(entry, instance)
            .create_win32_surface(&*info, None)
            .unwrap()
    }
}