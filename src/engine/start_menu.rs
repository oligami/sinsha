use ash::vk;
use winit::*;

use crate::vulkan::*;
use crate::linear_algebra::*;
use crate::interaction::*;
use crate::vulkan::mem_kai;

use std::mem;
use std::ops;
use std::path::*;
use std::io::Write;
use std::error::Error;
use std::time::SystemTime;
use std::sync::Arc;

pub fn run_kai(
	surface: Arc<VkSurfaceKHR>,
	device: Arc<VkDevice>,
	queue: Arc<VkQueue<Graphics>>,
	_events_loop: &mut EventsLoop,
) {
	let alloc = mem_kai::alloc::BuddyAllocator::new(5, 0x100);
	let memory = mem_kai::VkMemory::with_allocator(device.clone(), alloc, mem_kai::HostVisibleFlag)
		.unwrap();
	let buffer = mem_kai::buffer::VkBuffer::new(
		memory.clone(),
		queue.clone(),
		mem_kai::alloc::BuddyAllocator::new(4, 0x10),
		mem_kai::Vertex,
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
			format::R8G8B8A8_UNORM,
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

	swap_chain::VkSwapchainKHR::new(
		device.clone(),
		surface.clone(),
		usage::ColorAttachment,
		format::B8G8R8_UNORM,
		vk::PresentModeKHR::MAILBOX,
		2,
	);
}


const GUI_SCALE: f32 = 200_f32;

pub struct Rect {
	origin: XY,
	top: f32,
	bottom: f32,
	left: f32,
	right: f32,
	u: ops::Range<f32>,
	v: ops::Range<f32>,
}

impl Rect {
	fn to_vertices(&self) -> [gui::Vertex; 4] {
		[
			gui::Vertex::new(
				XY::new(self.left, self.top) / GUI_SCALE,
				XY::new(self.u.start, self.v.start),
			),
			gui::Vertex::new(
				XY::new(self.left, self.bottom) / GUI_SCALE,
				XY::new(self.u.start, self.v.end),
			),
			gui::Vertex::new(
				XY::new(self.right, self.bottom) / GUI_SCALE,
				XY::new(self.u.end, self.v.end),
			),
			gui::Vertex::new(
				XY::new(self.right, self.top) / GUI_SCALE,
				XY::new(self.u.end, self.v.start),
			),
		]
	}

	fn to_push_constants(&self) -> gui::PushConstants {
		gui::PushConstants::new(RGBA::default(), self.origin)
	}

	fn to_area(&self) -> Area2D {
		Area2D {
			x: self.left / GUI_SCALE + self.origin.x .. self.right / GUI_SCALE + self.origin.x,
			y: self.top / GUI_SCALE + self.origin.y .. self.bottom / GUI_SCALE + self.origin.y,
		}
	}
}

struct StartMenu;

impl StartMenu {
	const FILE: &'static str = "assets/gui/start_menu.png";

	const START_BUTTON: Rect = Rect {
		origin: XY::new(0.0, 0.0),
		top: -10.0,
		bottom: 10.0,
		left: -20.0,
		right: 20.0,
		u: 0.0..0.5,
		v: 0.0..0.25,
	};

	const QUIT_BUTTON: Rect = Rect {
		origin: XY::new(0.0, 0.0),
		top: 25.0,
		bottom: 45.0,
		left: -20.0,
		right: 20.0,
		u: 0.0..0.5,
		v: 0.25..0.5,
	};
}

struct Button<'a, 'b, 'c> {
	obj: gui::Obj<'a, 'b, 'c>,
	area: Area2D,
}

#[derive(Debug)]
struct Area2D {
	x: ops::Range<f32>,
	y: ops::Range<f32>,
}

impl<'a, 'b, 'c> Button<'a, 'b, 'c> {
	fn new(obj: gui::Obj<'a, 'b, 'c>, area: Area2D) -> Self {
		Self { obj, area }
	}

	fn on_mouse(&mut self, interaction: &InteractionDevices, rgba: RGBA) {
		if self.area.contain(interaction.mouse.position) {
			self.obj.set_rgba(rgba);
		} else {
			self.obj.set_rgba(RGBA::default());
		}
	}
}

impl Area2D {
	fn contain(&self, XY { x, y } : XY) -> bool {
		self.x.start <= x && x <= self.x.end && self.y.start <= y && y <= self.y.end
	}
}

pub fn run(
	vk_core: &VkCore,
	vk_graphic: &mut VkGraphic,
	window: &Window,
	events_loop: &mut EventsLoop,
) -> () {
	let start_button = StartMenu::START_BUTTON.to_vertices();
	let quit_button = StartMenu::QUIT_BUTTON.to_vertices();

	let texture = image_crate::open(StartMenu::FILE).unwrap().to_rgba();
	let (width, height) = texture.dimensions();
	let texture = texture.into_raw();

	// allocate local memory.
	let staging_buffer = VkBuffer::new(
		vk_core,
		0x1000_0000,
		vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::VERTEX_BUFFER,
		vk::SharingMode::EXCLUSIVE,
	).unwrap();

	let mut staging_memory = VkMemory::new_by_buffer_properties(
		0x1000_0000,
		vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
		&staging_buffer,
	).unwrap();

	staging_memory
		.binder(0)
		.bind_buffer(&staging_buffer).unwrap();

	let mut access = staging_memory.access().unwrap();
	let offset = access.write(0, &start_button[..]).unwrap();
	let offset = access.write(offset, &quit_button[..]).unwrap();
	access.write(offset, &texture[..]).unwrap();

	let vertex_buffer = VkBuffer::new(
		vk_core,
		0x100_0000,
		vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
		vk::SharingMode::EXCLUSIVE,
	).unwrap();

	let texture = VkImage::new(
		vk_core,
		vk::ImageType::TYPE_2D,
		vk::Extent3D { width, height, depth: 1 },
		vk::Format::R8G8B8A8_UNORM,
		vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
		vk::SharingMode::EXCLUSIVE,
		vk::ImageLayout::UNDEFINED,
		vk::SampleCountFlags::TYPE_1,
		vk::ImageAspectFlags::COLOR,
		1,
		1,
	).unwrap();

	let mut local_memory = VkMemory::new_by_buffer_properties(
		0x1000_0000,
		vk::MemoryPropertyFlags::DEVICE_LOCAL,
		&vertex_buffer,
	).unwrap();

	local_memory
		.binder(0)
		.bind_image(&texture).unwrap()
		.bind_buffer(&vertex_buffer).unwrap();

	let mut command_buffers = CommandBuffers::new(
		vk_core,
		vk::CommandPoolCreateFlags::TRANSIENT | vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
		vk::CommandBufferLevel::PRIMARY,
		3,
	).unwrap();

	let mut recorder = command_buffers
		.recorder(0, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
		.unwrap();

	recorder
		.buffer_to_buffer(
			&staging_buffer,
			&vertex_buffer,
			&[
				vk::BufferCopy {
					src_offset: 0,
					dst_offset: 0,
					size: 128,
				},
			],
		)
		.image_barrier(
			(vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER),
			(vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE),
			&texture,
			(vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL),
			0..1,
			0..1,
		)
		.buffer_to_image(
			&staging_buffer,
			offset,
			&texture,
			vk::ImageLayout::TRANSFER_DST_OPTIMAL,
			0,
		)
		.image_barrier(
			(vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER),
			(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::SHADER_READ),
			&texture,
			(vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
			0..1,
			0..1,
		);
	recorder.end().unwrap();

	command_buffers
		.queue_submit(0, vk::PipelineStageFlags::empty(), &[], &[], None)
		.unwrap();

	vk_core.queue_wait_idle().unwrap();
	drop(staging_buffer);

	// descriptor set.
	let texture_view = texture
		.view(
			vk::ImageViewType::TYPE_2D,
			vk::ComponentMapping {
				r: vk::ComponentSwizzle::IDENTITY,
				g: vk::ComponentSwizzle::IDENTITY,
				b: vk::ComponentSwizzle::IDENTITY,
				a: vk::ComponentSwizzle::IDENTITY,
			},
		)
		.unwrap();

	let sampler = VkSampler::new(
		vk_core,
		(vk::Filter::LINEAR, vk::Filter::NEAREST),
		vk::SamplerAddressMode::REPEAT,
		vk::SamplerAddressMode::REPEAT,
		vk::SamplerAddressMode::REPEAT,
		vk::BorderColor::FLOAT_OPAQUE_WHITE,
		vk::SamplerMipmapMode::LINEAR,
		0.0,
		0.0..0.0,
		None,
		None,
		vk::FALSE,
	).unwrap();

	let texture = gui::DescriptorPool::new(
		vk_graphic,
		&[(&texture_view, &sampler); 2],
	).unwrap();

	let start_button = gui::Obj::new(
		&vertex_buffer,
		0,
		&texture[0],
		StartMenu::START_BUTTON.to_push_constants(),
	);

	let mut start_button = Button::new(start_button, StartMenu::START_BUTTON.to_area());

	let quit_button = gui::Obj::new(
		&vertex_buffer,
		4,
		&texture[1],
		StartMenu::QUIT_BUTTON.to_push_constants(),
	);

	let mut command_buffers_unused = [
		VkFence::new(vk_core, true).unwrap(),
		VkFence::new(vk_core, true).unwrap(),
		VkFence::new(vk_core, true).unwrap(),
	];
	let image_acquire = VkSemaphore::new(vk_core).unwrap();
	let render_finish = VkSemaphore::new(vk_core).unwrap();

	let mut fps_parser = 0_u64;
	let mut timer = SystemTime::now();
	let mut interaction = InteractionDevices::new(window);

	loop {
		let image_index = vk_graphic.next_image(Some(&image_acquire), None).unwrap();

		command_buffers_unused[image_index].wait(None).unwrap();
		command_buffers_unused[image_index].reset().unwrap();
		command_buffers
			.recorder(image_index, vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
			.unwrap()
			.into_graphic(&vk_graphic, [[0.1, 0.1, 0.1, 1.0]])
			.bind_gui_pipeline()
			.draw(&start_button.obj)
			.draw(&quit_button)
			.end()
			.end()
			.unwrap();

		command_buffers
			.queue_submit(
				image_index,
				vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
				&[image_acquire],
				&[render_finish],
				Some(&command_buffers_unused[image_index]),
			)
			.unwrap();

		if let Err(err) = vk_graphic.present(image_index as _, &[render_finish]) {
			vk_core.queue_wait_idle().unwrap();
			match err {
				vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => {
					vk_graphic.recreate().unwrap();
				},
				_ => Err(err).unwrap(),
			}
			continue;
		}


		let mut end = false;
		events_loop.poll_events(|event| {
			interaction.event_update(&event);
			match event {
				Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => end = true,
				_ => (),
			}
		});

		if end { break; }

		start_button.on_mouse(&interaction, RGBA::new(0.8, 0.8, 0.8, 1.0));

		if fps_parser % 1000 == 0 {
			let time = timer.elapsed().unwrap().as_micros() as f32 / 1_000_000_f32;
			timer = SystemTime::now();
			eprint!("\rFPS: {}", 1000_f32 / time);
		}
		fps_parser += 1;

		interaction.clear();
	}

	vk_core.queue_wait_idle().unwrap();
	image_acquire.drop(vk_core);
	render_finish.drop(vk_core);
}


