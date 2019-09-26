use super::*;

use std::ops::Index;
use std::path::Path;

pub struct VkRender<D: Borrow<device::Device>> {
    device: D,
    surface: SurfaceKHR,
    swapchain: SwapchainKHR,
    render_pass: vk::RenderPass,
    framebuffers: Framebuffers,
    descriptor_layouts: Vec<vk::DescriptorSetLayout>,
    pipeline_layouts: Vec<vk::PipelineLayout>,
    pipelines: Vec<vk::Pipeline>,
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

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct DescriptorSetLayout(usize);
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct PipelineLayout(usize);
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Pipeline(usize);

impl<D: Borrow<device::Device>> VkRender<D> {
    pub fn new(device: D) -> Self {
        let surface = unsafe { Self::create_surface(device.borrow(), unimplemented!()) };
        let swapchain = unsafe { Self::create_swapchain(device.borrow(), &surface) };
        let render_pass = Self::create_render_pass(&device.borrow().device);
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

        let first_subpass_depth_attachment = vk::AttachmentReference::builder()
            .attachment(Self::DEPTH_ATTACHMENT_INDEX)
            .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
            .build();

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
                .depth_stencil_attachment(&first_subpass_depth_attachment)
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

    fn create_framebuffers(
        device: &VkDevice,
        physical_device: &PhysicalDevice,
        swapchain: &SwapchainKHR,
        render_pass: vk::RenderPass,
    ) -> Framebuffers {
        // Create images. --
        let g_buffer_position_info = vk::ImageCreateInfo::builder()
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
            .queue_family_indices(&[])
            .build();

        let g_buffer_normal_info = g_buffer_position_info;

        let g_buffer_color_info = {
            let mut temp = g_buffer_position_info;
            temp.format = vk::Format::R8G8B8A8_UNORM;
            temp
        };

        let depth_info = {
            let mut temp = g_buffer_position_info.clone();
            temp.usage = vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
            temp.format = vk::Format::D32_SFLOAT;
            temp
        };

        // Infos of vk::Image for one framebuffer.
        let infos_for_one_framebuffer = [
            depth_info,
            g_buffer_position_info,
            g_buffer_normal_info,
            g_buffer_color_info,
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
        unimplemented!("size maybe not enough because of alignment.");
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
                let device_local = property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL);
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
        let g_buffer_position_info = vk::ImageViewCreateInfo::builder()
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
            )
            .build();
        let g_buffer_normal_info = g_buffer_position_info;
        let g_buffer_color_info = {
            let mut temp = g_buffer_position_info;
            temp.format = vk::Format::R8G8B8A8_UNORM;
            temp
        };
        let depth_image_view_info = {
            let mut tmp = g_buffer_position_info;
            tmp.format = vk::Format::D32_SFLOAT;
            tmp.subresource_range.aspect_mask = vk::ImageAspectFlags::DEPTH;
            tmp
        };
        let swapchain_image_view_info = {
            let mut tmp = g_buffer_position_info;
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
                    g_buffer_position_info,
                    g_buffer_normal_info,
                    g_buffer_color_info,
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

                unsafe { device.create_framebuffer(&framebuffer_info, None).unwrap() }
            })
            .collect::<Vec<_>>();

        Framebuffers { handles, memory, images, views }
    }

    fn descriptor_layouts(device: &VkDevice) -> Vec<vk::DescriptorSetLayout> {
        let mut descriptor_set_layouts = Vec::new();

        // Create vk::DescriptorSetLayout for a camera.
        let bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .descriptor_count(1)
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .build(),
        ];

        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .flags(vk::DescriptorSetLayoutCreateFlags::empty())
            .bindings(&bindings[..]);

        let handle = unsafe { device.create_descriptor_set_layout(&info, None).unwrap() };
        descriptor_set_layouts.push(handle);

        // For G-buffers.
        let bindings = [
            vk::DescriptorSetLayoutBinding::builder()
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .descriptor_count(1)
                .binding(0)
                .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .descriptor_count(1)
                .binding(1)
                .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .descriptor_count(1)
                .binding(2)
                .descriptor_type(vk::DescriptorType::INPUT_ATTACHMENT)
                .build(),
        ];

        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .flags(vk::DescriptorSetLayoutCreateFlags::empty())
            .bindings(&bindings[..]);

        let handle = unsafe { device.create_descriptor_set_layout(&info, None).unwrap() };
        descriptor_set_layouts.push(handle);


        descriptor_set_layouts
    }

    unsafe fn pipeline_layouts(&mut self) {
        let device = &self.device.borrow().device;

        let layouts = [self[DescriptorSetLayout::CAMERA]];
        let info = vk::PipelineLayoutCreateInfo::builder()
            .flags(vk::PipelineLayoutCreateFlags::empty())
            .set_layouts(&layouts[..])
            .push_constant_ranges(&[]);

        let handle = unsafe { device.create_pipeline_layout(&info, None).unwrap() };
        self.pipeline_layouts.push(handle);

        let set_layouts = [self[DescriptorSetLayout::G_BUFFER]];
        let info = vk::PipelineLayoutCreateInfo::builder()
            .flags(vk::PipelineLayoutCreateFlags::empty())
            .set_layouts(&set_layouts[..])
            .push_constant_ranges(&[]);

        let handle = unsafe { device.create_pipeline_layout(&info, None).unwrap() };
        self.pipeline_layouts.push(handle);

        unimplemented!("impl layout for lighting.");
    }

    /// # Safety
    /// Ensure to call this function
    /// after descriptor set layouts and pipeline layouts are initialized.
    unsafe fn pipelines(&mut self) {
        // G-Buffer rendering.
        unimplemented!("shader files must be updated.");
        let vert = self.shader_module(&include_bytes!("render/dim3/vert.spv")[..]);
        let frag = self.shader_module(&include_bytes!("render/dim3/frag.spv")[..]);
        let fn_name = CString::new("main").unwrap();
        let stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .flags(vk::PipelineShaderStageCreateFlags::empty())
                .module(vert)
                .stage(vk::ShaderStageFlags::VERTEX)
                .name(&fn_name)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .flags(vk::PipelineShaderStageCreateFlags::empty())
                .module(frag)
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .name(&fn_name)
                .build(),
        ];

        let assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .flags(vk::PipelineInputAssemblyStateCreateFlags::empty())
            .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
            .primitive_restart_enable(false)
            .build();

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::builder()
            .flags(vk::PipelineVertexInputStateCreateFlags::empty())
            .vertex_binding_descriptions(unimplemented!())
            .vertex_attribute_descriptions(unimplemented!())
            .build();

        let viewports = [
            vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(self.swapchain.extent.width as f32)
                .height(self.swapchain.extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0)
                .build(),
        ];
        let scissors = [
            vk::Rect2D::builder()
                .offset(vk::Offset2D::builder().x(0).y(0).build())
                .extent(self.swapchain.extent)
                .build(),
        ];
        let viewport = vk::PipelineViewportStateCreateInfo::builder()
            .flags(vk::PipelineViewportStateCreateFlags::empty())
            .viewports(&viewports[..])
            .scissors(&scissors[..])
            .build();

        let rasterization = vk::PipelineRasterizationStateCreateInfo::builder()
            .flags(vk::PipelineRasterizationStateCreateFlags::empty())
            .rasterizer_discard_enable(false)
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::BACK)
            .polygon_mode(vk::PolygonMode::FILL)
            .depth_bias_enable(false)
            .depth_clamp_enable(false)
            .build();

        let multisample = vk::PipelineMultisampleStateCreateInfo::builder()
            .flags(vk::PipelineMultisampleStateCreateFlags::empty())
            .rasterization_samples(Self::SAMPLE_COUNT)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .sample_shading_enable(false)
            .build();

        let color_blend_attachments = [
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(false)
                .build(),
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(false)
                .build(),
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(vk::ColorComponentFlags::all())
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .build(),
        ];

        let color_blend = vk::PipelineColorBlendStateCreateInfo::builder()
            .flags(vk::PipelineColorBlendStateCreateFlags::empty())
            .logic_op_enable(false)
            .attachments(&color_blend_attachments[..])
            .build();


        // G-Buffer rendering.
        let info = vk::GraphicsPipelineCreateInfo::builder()
            .flags(vk::PipelineCreateFlags::empty())
            .render_pass(self.render_pass)
            .subpass(0)
            .layout(self[PipelineLayout::G_BUFFER])
            .stages(&stages[..])
            .input_assembly_state(&assembly)
            .vertex_input_state(unimplemented!())
            .viewport_state(&viewport)
            .multisample_state(&multisample)
            .rasterization_state(&rasterization)
            .color_blend_state(&color_blend)
            .build();

        // Lighting and write into swapchain framebuffer. --
        let stages = unimplemented!();
        let assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .primitive_restart_enable(false)
            .topology(vk::PrimitiveTopology::TRIANGLE_STRIP)
            .build();

        let rasterization = vk::PipelineRasterizationStateCreateInfo::builder()
            .flags(vk::PipelineRasterizationStateCreateFlags::empty())
            .rasterizer_discard_enable(false)
            .line_width(1.0)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::empty())
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_clamp_enable(false)
            .depth_bias_enable(false)
            .build();


        let color_blend_attachments = [
            vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::all())
                .build()
        ];
        let color_blend = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments[..])
            .build();

        let info2 = vk::GraphicsPipelineCreateInfo::builder()
            .flags(vk::PipelineCreateFlags::empty())
            .render_pass(self.render_pass)
            .subpass(1)
            .layout(self[PipelineLayout::LIGHTING])
            .stages(unimplemented!())
            .input_assembly_state(&assembly)
            .viewport_state(&viewport)
            .multisample_state(&multisample)
            .rasterization_state(&rasterization)
            .color_blend_state(&color_blend)
            .build();

        let handles = unsafe {
            self.device.borrow().device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[info, info2],
                    None,
                )
                .unwrap()
        };

        self.pipelines = handles;
    }

    unsafe fn shader_module(&self, bytes: &[u8]) -> vk::ShaderModule {
        debug_assert_eq!(bytes.len() % 4, 0);
        let code = std::slice::from_raw_parts(bytes[0] as *const u8 as *const u32, bytes.len() / 4);

        let info = vk::ShaderModuleCreateInfo::builder()
            .flags(vk::ShaderModuleCreateFlags::empty())
            .code(code);

        unsafe { self.device.borrow().device.create_shader_module(&info, None).unwrap() }
    }
}

impl<D: Borrow<device::Device>> Drop for VkRender<D> {
    fn drop(&mut self) {
        unimplemented!()
    }
}

impl DescriptorSetLayout {
    pub const CAMERA: Self = DescriptorSetLayout(0);
    pub const G_BUFFER: Self = DescriptorSetLayout(1);
}
impl PipelineLayout {
    pub const DIM3: Self = PipelineLayout(0);
    pub const G_BUFFER: Self = PipelineLayout(1);
    pub const LIGHTING: Self = PipelineLayout(2);
}
impl Pipeline {
    pub const DIM3: Self = Pipeline(0);
    pub const G_BUFFER: Self = Pipeline(1);
}

impl<D: Borrow<device::Device>> Index<DescriptorSetLayout> for VkRender<D> {
    type Output = vk::DescriptorSetLayout;
    fn index(&self, DescriptorSetLayout(index): DescriptorSetLayout) -> &Self::Output {
        &self.descriptor_layouts[index]
    }
}
impl<D: Borrow<device::Device>> Index<PipelineLayout> for VkRender<D> {
    type Output = vk::PipelineLayout;
    fn index(&self, PipelineLayout(index): PipelineLayout) -> &Self::Output {
        &self.pipeline_layouts[index]
    }
}
impl<D: Borrow<device::Device>> Index<Pipeline> for VkRender<D> {
    type Output = vk::Pipeline;
    fn index(&self, Pipeline(index): Pipeline) -> &Self::Output {
        &self.pipelines[index]
    }
}
