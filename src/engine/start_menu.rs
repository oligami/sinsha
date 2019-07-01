use ash::vk;
use winit::*;

use crate::vulkan::*;
use crate::linear_algebra::*;
use crate::interaction::*;
use crate::vulkan::mem::*;

use std::sync::Arc;

pub fn run_kai(
	surface: Arc<SurfaceKhr>,
	device: Arc<Device>,
	queue: Arc<Queue<Graphics>>,
	_events_loop: &mut EventsLoop,
) {
	let alloc = alloc::BuddyAllocator::new(5, 0x100);
	let memory = DeviceMemory::with_allocator(
		device.clone(),
		alloc,
		memory_property::HostVisibleFlag(memory_property::Empty),
	).unwrap();

	let buffer = buffer::Buffer::new(
		memory.clone(),
		queue.clone(),
		alloc::BuddyAllocator::new(4, 0x10),
		buffer::usage::VertexBufferFlag(buffer::usage::Empty),
	).unwrap();

	let data = buffer::Data::new(buffer.clone(), &31_u32).unwrap();
	let mut access = data.access();
	let uninit = access.as_ref().clone();
	*access.as_mut() = 32;
	let read = access.as_ref().clone();
	drop(access);
	println!("uninit: {}, init: {}", uninit, read);

	let data = Arc::new(data);
	let data2 = Arc::new(buffer::Data::new(buffer.clone(), &(1_u32, 0_u32)).unwrap());
	let handle = {
		let data = data.clone();
		let data2 = data2.clone();
		std::thread::spawn(move || {
			let mut access = data.access();
			let mut access2 = data2.access();
			*access.as_mut() = 64;
			*access2.as_mut() = (2234, 111);
		})
	};

	handle.join().unwrap();

	let access = data.access();
	let access2 = data2.access();
	let read = access.as_ref().clone();
	let read2 = access2.as_ref().clone();
	drop(access);
	drop(access2);

	println!("changed by thread: {}, and 2: {:?}", read, read2);

	use image::*;
	let render_pass = render_pass::RenderPass::builder()
		.color_attachment(
			format::B8G8R8A8_UNORM,
			sample_count::Type1,
			vk::AttachmentLoadOp::CLEAR,
			vk::AttachmentStoreOp::STORE,
			vk::ImageLayout::UNDEFINED,
			vk::ImageLayout::PRESENT_SRC_KHR,
		)
		.subpasses()
		.subpass(
			render_pass::subpass::Graphics,
			vec![
				vk::AttachmentReference {
					attachment: 0,
					layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
				}
			],
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
		.build(device.clone());

	let swapchain = swapchain::VkSwapchainKhr::new(
		device.clone(),
		surface.clone(),
		usage::ColorAttachmentFlag(usage::Empty),
		format::B8G8R8A8_UNORM,
		vk::PresentModeKHR::MAILBOX,
		2,
	);

	let extent = swapchain.extent();
	let swapchain_image_views = swapchain::VkSwapchainKhr::views(&swapchain);

	let framebuffers: Vec<_> = swapchain_image_views.iter()
		.map(|view| {
			framebuffer::VkFrameBuffer::builder(extent.width, extent.height, 1)
				.attach_swapchain_image_view(view.clone())
				.build(render_pass.clone())
		})
		.collect();

	let descriptor_set_layout = shader::descriptor::DescriptorSetLayout::builder()
		.binding(
			[shader::descriptor::ty::CombinedImageSampler; 1],
			shader::stage::Fragment(shader::stage::Empty),
			(),
		)
		.build(device.clone());

	let descriptor_pool = shader::descriptor::DescriptorPool::builder()
		.layout(descriptor_set_layout.clone())
		.pool_size()
		.build(3, device.clone());

	let descriptor_sets = shader::descriptor::DescriptorSet::new(
		&[descriptor_set_layout.clone()],
		descriptor_pool.clone(),
	);

	let pipeline_layout = shader::pipeline::PipelineLayout::builder()
		.push_constant(
			shader::pipeline::PushConstant::new::<RGBA>(),
			shader::stage::Vertex(shader::stage::Empty)
		)
		.descriptor_set_layout()
		.set_layout(descriptor_set_layout.clone())
		.build(device.clone());


}


