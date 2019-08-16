//mod start_menu;

use crate::linear_algebra::*;
use crate::vulkan::*;
use crate::window;
use crate::window::CustomEvent;

use ash::vk;
use ash::version::DeviceV1_0;

use winit::event::{ Event, WindowEvent };

use std::ptr;
use std::time::Duration;

pub struct Engine;

impl Engine {
    pub fn run() {
        let (window, proxy, event_receiver) = window::create_window();
        let instance = Instance::new();
        let debugger = if cfg!(debug_assertions) { Some(DebugEXT::new(&instance)) } else { None };
        let surface = SurfaceKHR::new(&instance, window);

        let (device, queue_info) = {
            let queue_flag = vk::QueueFlags::GRAPHICS;
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

            (Device::new(&instance, valid_physical_device, &[(&queue_info, &[1.0])]), queue_info)
        };
        let mut queue = Queue::from_device(&device, &queue_info, 0);

        let render_pass = render_pass::RenderPass::new(
            &device,
            &[
                *vk::AttachmentDescription::builder()
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                    .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .samples(vk::SampleCountFlags::TYPE_1)
            ],
            &[
                *vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[
                        *vk::AttachmentReference::builder()
                            .attachment(0)
                            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    ])
            ],
            &[
                *vk::SubpassDependency::builder()
                    .src_subpass(vk::SUBPASS_EXTERNAL)
                    .dst_subpass(0)
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                    .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                    .dependency_flags(vk::DependencyFlags::BY_REGION)
            ],
        );

        let swapchain = swapchain::SwapchainKHR::new(
            &surface,
            &device,
            image::ImageUsage::builder().color_attachment().build(),
            vk::Format::B8G8R8A8_UNORM,
            vk::PresentModeKHR::MAILBOX,
            3,
        );

        let image_views = swapchain::SwapchainKHR::views(&swapchain);
        let framebuffers = image_views.iter()
            .map(|view| {
                framebuffer::FrameBuffer::new(
                    &render_pass,
                    1280,
                    720,
                    1,
                    framebuffer::Attachments::new().add_swapchain(view),
                )
            })
            .collect::<Vec<_>>();


        let pipeline_layout = pipeline::PipelineLayout::<_, _, &descriptor::DescriptorSetLayout<_, _>>::new(&device, vec![], vec![]);

        let shader_stages = pipeline::ShaderStages::new(&device)
            .shader_stage("shaders/test/vert.spv", "main", vk::ShaderStageFlags::VERTEX, None)
            .shader_stage("shaders/test/frag.spv", "main", vk::ShaderStageFlags::FRAGMENT, None);

        let pipeline = unsafe {
            pipeline::GraphicsPipeline::new(
                *vk::GraphicsPipelineCreateInfo::builder()
                    .vertex_input_state(
                        &vk::PipelineVertexInputStateCreateInfo::builder()
                            .vertex_attribute_descriptions(&vertex::Vertex::ATTRIBUTES)
                            .vertex_binding_descriptions(&vertex::Vertex::BINDINGS)
                    )
                    .input_assembly_state(
                        &vk::PipelineInputAssemblyStateCreateInfo::builder()
                            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                    )
                    .viewport_state(
                        &vk::PipelineViewportStateCreateInfo::builder()
                            .scissors(&[swapchain.whole_area()])
                            .viewports(&[
                                *vk::Viewport::builder()
                                    .x(0.0)
                                    .y(0.0)
                                    .width(swapchain.vk_extent_2d().width as f32)
                                    .height(swapchain.vk_extent_2d().height as f32)
                                    .min_depth(0.0)
                                    .max_depth(1.0)
                            ])
                    )
                    .rasterization_state(
                        &vk::PipelineRasterizationStateCreateInfo::builder()
                            .polygon_mode(vk::PolygonMode::FILL)
                            .cull_mode(vk::CullModeFlags::BACK)
                            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                            .line_width(1.0)
                    )
                    .multisample_state(
                        &vk::PipelineMultisampleStateCreateInfo::builder()
                            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                    )
                    .color_blend_state(
                        &vk::PipelineColorBlendStateCreateInfo::builder()
                            .attachments(&[
                                *vk::PipelineColorBlendAttachmentState::builder()
                                    .color_write_mask(
                                        vk::ColorComponentFlags::R
                                            | vk::ColorComponentFlags::G
                                            | vk::ColorComponentFlags::B
                                            | vk::ColorComponentFlags::A
                                    )
                                    .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                                    .color_blend_op(vk::BlendOp::ADD)
                                    .src_alpha_blend_factor(vk::BlendFactor::ONE)
                                    .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                                    .alpha_blend_op(vk::BlendOp::ADD)
                            ])
                    ),
                shader_stages,
                &render_pass,
                0,
                &pipeline_layout,
                None,
            )
        };

        let device_local_memory = device_memory::DeviceMemory::with_allocator(
            &device,
            device_memory::alloc::BuddyAllocator::new_with_ref_cell(16, 2048),
            device_memory::MemoryProperty::builder().device_local().build(),
        ).unwrap();

        let image = image::Image::new(
            &device_local_memory,
            &[queue.family_index()],
            image::Extent2D { width: 1280, height: 720 },
            vk::Format::R8G8B8A8_UNORM,
            vk::SampleCountFlags::TYPE_1,
            image::ImageUsage::builder().transfer_dst().input_attachment().build(),
            1,
            1,
            vk::ImageLayout::UNDEFINED,
        );

        let view = image::ImageView::new(
            &image,
            vk::ImageAspectFlags::COLOR,
            0..1,
            image::ArrayLayers2D::Normal { base: 0 },
        );

        let host_visible_memory = device_memory::DeviceMemory::with_allocator(
            &device,
            device_memory::alloc::BuddyAllocator::new_with_ref_cell(16, 1024),
            device_memory::MemoryProperty::builder().host_visible().host_coherent().build(),
        ).unwrap();

        let host_visible_buffer = buffer::Buffer::new(
            &host_visible_memory,
            &[queue.family_index()],
            device_memory::alloc::BuddyAllocator::new_with_ref_cell(4, 256),
            buffer::BufferUsage::builder()
                .transfer_src()
                .vertex_buffer()
                .uniform_buffer()
                .build(),
        ).unwrap();

        let staging = buffer::Data::new_slice::<vertex::Vertex>(&host_visible_buffer, 3).unwrap();

        let vertex_data = [
            vertex::Vertex::new(
                XYZ::new(0.7, 0.5, 0.0),
                XYZ::new(1.0, 1.0, 0.0),
            ),
            vertex::Vertex::new(
                XYZ::new(0.5, -0.9, 0.0),
                XYZ::new(0.0, 1.0, 1.0),
            ),
            vertex::Vertex::new(
                XYZ::new(-0.5, 0.5, 0.0),
                XYZ::new(1.0, 0.0, 1.0),
            ),
        ];

        let count_up = 1000000;
        unsafe {
            let map_memory = device_memory::DeviceMemoryMapper::map(
                &host_visible_memory,
                staging.offset_by_memory() .. staging.offset_by_memory() + staging.size()
            );
            ptr::copy(vertex_data.as_ptr(), map_memory.as_mut_ptr(), 3);
        }

        let command_pool = command::CommandPool::new(
            &device,
            vk::CommandPoolCreateFlags::empty(),
            &queue,
        );

        unsafe {
            command_pool.reset();

            let command_buffers = command::CommandBuffer::begin_primary(
                &command_pool,
                &vec![vk::CommandBufferUsageFlags::SIMULTANEOUS_USE; swapchain.image_count() ],
            );

            command_buffers.iter()
                .enumerate()
                .for_each(|(i, command_buffer)| {
                    let clear_values = [
                        vk::ClearValue {
                            color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] }
                        }
                    ];
                    device.fp()
                        .cmd_begin_render_pass(
                            command_buffer.handle(),
                            &vk::RenderPassBeginInfo::builder()
                                .render_pass(render_pass.handle())
                                .framebuffer(framebuffers[i].handle())
                                .clear_values(&clear_values[..])
                                .render_area(swapchain.whole_area())
                                .build(),
                            vk::SubpassContents::INLINE,
                        );

                    device.fp()
                        .cmd_bind_pipeline(
                            command_buffer.handle(),
                            vk::PipelineBindPoint::GRAPHICS,
                            pipeline.handle(),
                        );

                    device.fp()
                        .cmd_bind_vertex_buffers(
                            command_buffer.handle(),
                            0,
                            &[staging.buffer().handle()],
                            &[staging.offset_by_buffer()],
                        );

                    device.fp().cmd_draw(command_buffer.handle(), staging.len() as u32, 1, 0, 0);
                    device.fp().cmd_end_render_pass(command_buffer.handle());
                    device.fp().end_command_buffer(command_buffer.handle()).unwrap();
                });

            let image_acquired = (0..swapchain.image_count())
                .fold(Vec::with_capacity(swapchain.image_count()), |mut v, _| {
                    v.push(sync::Semaphore::new(&device)); v
                });
            let render_finished = (0..swapchain.image_count())
                .fold(Vec::with_capacity(swapchain.image_count()), |mut v, _| {
                    v.push(sync::Semaphore::new(&device)); v
                });

            let mut idx = 0;
            'draw: loop {
                let image_index = swapchain.acquire_next_image(
                    Duration::new(0, 1_000_000),
                    image_acquired[idx].handle(),
                    vk::Fence::null(),
                );

                let command_buffers = [command_buffers[idx].handle()];
                let wait_semaphores = [image_acquired[idx].handle()];
                let signal_semaphores = [render_finished[idx].handle()];
                queue.submit(
                    &[
                        vk::SubmitInfo::builder()
                            .command_buffers(&command_buffers[..])
                            .wait_semaphores(&wait_semaphores[..])
                            .wait_dst_stage_mask(&[
                                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                            ])
                            .signal_semaphores(&signal_semaphores[..])
                            .build()
                    ],
                    vk::Fence::null(),
                );

                swapchain.queue_present(queue.handle(), image_index, &signal_semaphores[..]);

                idx = (idx + 1) % swapchain.image_count();

                while let Ok(event) = event_receiver.try_recv() {
                    match event {
                        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                            break 'draw;
                        },
                        _ => (),
                    }
                }
            }

            println!("hey");
            queue.wait_idle();
        }
    }
}
