use ash::vk;
use winit::*;

use crate::vulkan::*;
use crate::linear_algebra::*;
use crate::interaction::*;
use crate::vulkan::mem_kai;

use std::mem;
use std::ops;
use std::path::*;
use std::error::Error;
use std::time::SystemTime;
use std::sync::Arc;

pub fn run_kai(
	surface: Arc<SurfaceKHR>,
	device: Arc<Device>,
	queue: Arc<Queue<Graphics>>,
	_events_loop: &mut EventsLoop,
) {
	let alloc = mem_kai::alloc::BuddyAllocator::new(5, 0x100);
	let memory = mem_kai::VkMemory::with_allocator(
		device.clone(),
		alloc,
		mem_kai::memory_property::HostVisibleFlag(mem_kai::memory_property::Empty),
	).unwrap();

	let buffer = mem_kai::buffer::VkBuffer::new(
		memory.clone(),
		queue.clone(),
		mem_kai::alloc::BuddyAllocator::new(4, 0x10),
		mem_kai::buffer::usage::VertexBufferFlag(mem_kai::buffer::usage::Empty),
	).unwrap();

	let data = mem_kai::buffer::VkData::new(buffer.clone(), &31_u32).unwrap();
	let mut access = data.access();
	let uninit = access.as_ref().clone();
	*access.as_mut() = 32;
	let read = access.as_ref().clone();
	drop(access);
	println!("uninit: {}, init: {}", uninit, read);

	let data = Arc::new(data);
	let data2 = Arc::new(mem_kai::buffer::VkData::new(buffer.clone(), &(1_u32, 0_u32)).unwrap());
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

	use mem_kai::image::*;
	let render_pass = render_pass::VkRenderPass::builder()
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

	let swapchain = swapchain::VkSwapchainKHR::new(
		device.clone(),
		surface.clone(),
		usage::ColorAttachmentFlag(usage::Empty),
		format::B8G8R8A8_UNORM,
		vk::PresentModeKHR::MAILBOX,
		2,
	);

	let extent = swapchain.extent();
	let swapchain_image_views = swapchain::VkSwapchainKHR::views(&swapchain);

	let framebuffers: Vec<_> = swapchain_image_views.iter()
		.map(|view| {
			framebuffer::VkFrameBuffer::builder(extent.width, extent.height, 1)
				.attach_swapchain_image_view(view.clone())
				.build(render_pass.clone())
		})
		.collect();

	let descriptor_set_layout = shader::descriptor::VkDescriptorSetLayout::builder()
		.binding(
			shader::descriptor::ty::CombinedImageSampler,
			1,
			shader::stage::Fragment(shader::stage::Empty),
		)
		.build(device.clone());

	let descriptor_pool = shader::descriptor::VkDescriptorPool::builder()
		.layout(descriptor_set_layout.clone())
		.pool_size()
		.build(3, device.clone());

	let descriptor_sets = shader::descriptor::VkDescriptorSet::new(
		&[descriptor_set_layout.clone()],
		descriptor_pool.clone(),
	);

	let pipeline_layout = shader::pipeline::VkPipelineLayout::builder()
		.push_constant::<RGBA, _>(shader::stage::Vertex(shader::stage::Empty))
		.descriptor_set_layout()
		.set_layout(descriptor_set_layout.clone())
		.build(device.clone());
}


