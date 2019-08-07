//mod start_menu;

use crate::vulkan::*;
use device_memory::DeviceMemoryAbs;

use ash::vk;

use winit::window::*;
use winit::event_loop::EventLoop;
use winit::dpi::PhysicalSize;

pub struct Engine;

impl Engine {
    pub fn run() {
        let (window, event_loop) = Engine::create_window();
        let instance = Instance::new();
        let debugger = if cfg!(debug_assertions) { Some(DebugEXT::new(&instance)) } else { None };
        let surface = SurfaceKHR::new(&instance, window);
        let queue_flag = queue_flag::builder().graphics().build_with_surface_support();
        let mut valid_queue = None;
        let mut valid_physical_device = !0;
        for i in 0..instance.physical_device_num() {
            let info = Queue::get_queue_info_with_surface_support(&instance, i, &surface, queue_flag);
            if let Some(info) = info {
                valid_queue = Some(info);
                valid_physical_device = i;
                break;
            }
        }
        let queue_info = valid_queue.unwrap();
        let device = Device::new(&instance, valid_physical_device, &[(&queue_info, &[1.0])]);
        let queue = unsafe { Queue::from_device(&device, &queue_info, 0).convert_flag(queue_flag) };

        let memory = device_memory::DeviceMemory::with_allocator(
            &device,
            device_memory::alloc::BuddyAllocator::new_with_ref_cell(16, 1024),
            device_memory::MemoryProperties::empty().device_local(),
        ).unwrap();
        println!("device memory size: {}", memory.size());

        let allocator = device_memory::alloc::BuddyAllocator::new_with_ref_cell(4, 64);
        let usage = device_memory::buffer::usage::builder().transfer_src().uniform_buffer().build();
        let buffer = unsafe {
            device_memory::Buffer::new(&memory, &[queue.family_index()], allocator, usage)
                .unwrap()
        };

        let image = device_memory::image::Image::new(
            &memory,
            &[queue.family_index()],
            device_memory::image::Extent2D { width: 1280, height: 720 },
            vk::Format::R8G8B8A8_UNORM,
            vk::SampleCountFlags::TYPE_1,
            device_memory::image::usage::ImageUsages::empty().color_attachment(),
            1,
            1,
            vk::ImageLayout::UNDEFINED,
        );

        let view = device_memory::image::ImageView::new(
            &image,
            vk::ImageAspectFlags::COLOR,
            0..1,
            device_memory::image::ArrayLayers2D::Normal { base: 0 },
        );

        let data = unsafe { device_memory::Data::new(&buffer, &0_u32).unwrap() };

        let render_pass = render_pass::RenderPass::builder()
            .color_attachment(
                vk::Format::R8G8B8A8_UNORM,
                vk::SampleCountFlags::TYPE_1,
                vk::AttachmentLoadOp::CLEAR,
                vk::AttachmentStoreOp::DONT_CARE,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            )
            .subpasses()
            .subpass(
                vk::PipelineBindPoint::GRAPHICS,
                vec![vk::AttachmentReference { attachment: 0, layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL}],
                vec![],
                vec![],
                None,
                vec![],
            )
            .dependencies()
            .dependency(
                vk::SUBPASS_EXTERNAL,
                0,
                vk::AccessFlags::empty(),
                vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::DependencyFlags::BY_REGION,
            )
            .build(&device);

        let framebuffer = framebuffer::FrameBuffer::builder(&render_pass, 1280, 720, 1)
            .attach_image_view(&view)
            .build();

        let swapchain = swapchain::SwapchainKHR::new(
            &surface,
            &device,
            device_memory::image::ImageUsages::empty().color_attachment(),
            vk::Format::B8G8R8A8_UNORM,
            vk::PresentModeKHR::MAILBOX,
            2,
        );

        {
            let swapchain_views = swapchain::SwapchainKHR::views(&swapchain);
        }
        let swapchain = swapchain.recreate();

        let descriptor_set_layout = descriptor::DescriptorSetLayout::new(
            &device,
            &[
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .build(),
            ],
        );

        let descriptor_pool = descriptor::DescriptorPool::from_layouts(
            &[&descriptor_set_layout],
            10,
        );

        let descriptor_sets = descriptor::DescriptorSet::new(
            &[&descriptor_set_layout],
            &descriptor_pool
        ).into_iter()
            .map(|set| {
                set.updater()
                    .write_data(0, 0, descriptor::DataInfos::new().add_data(&data))
                    .update()
            })
            .collect::<Vec<_>>();


    }
}

impl Engine {
    fn create_window() -> (Window, EventLoop<()>) {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_inner_size(
                PhysicalSize::new(1280_f64, 720_f64)
                    .to_logical(event_loop.primary_monitor().hidpi_factor())
            )
            .with_title("sinsha")
            .build(&event_loop)
            .unwrap();

        (window, event_loop)
    }
}