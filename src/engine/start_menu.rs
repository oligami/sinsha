use ash::vk;
use winit::*;

use crate::vulkan::*;
use crate::linear_algebra::*;

use std::mem;
use std::ops;
use std::path::*;
use std::io::Write;
use std::error::Error;
use std::time::SystemTime;

const GUI_SCALE: f32 = 100_f32;

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


pub fn run(
	vk_core: &VkCore,
	vk_graphic: &VkGraphic,
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

	for i in 0..3 {
		let mut recorder = command_buffers
			.recorder(i, vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
			.unwrap();

		let push_constant = gui::PushConstant::new(RGBA::default(), XY::zero());
		recorder
			.into_graphic(vk_graphic, [[0.0, 0.0, 0.0, 1.0]])
			.bind_gui_pipeline()
			.draw(
				&vertex_buffer,
				0,
				&texture[0],
				&push_constant,
			)
			.draw(
				&vertex_buffer,
				4,
				&texture[1],
				&push_constant,
			)
			.end()
			.end()
			.unwrap();
	}

	let image_acquired = VkSemaphore::new(vk_core).unwrap();
	let render_finish = VkSemaphore::new(vk_core).unwrap();

	let mut fps_parser = 0_u64;
	let mut timer = SystemTime::now();

	loop {
		let image_index = vk_graphic.next_image(Some(&image_acquired), None).unwrap();

		command_buffers
			.queue_submit(
				image_index,
				vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
				&[image_acquired],
				&[render_finish],
				None,
			)
			.unwrap();

		vk_graphic
			.present(
				image_index as _,
				&[render_finish],
			)
			.unwrap();

		let mut end = false;
		events_loop.poll_events(|event| {
			match event {
				Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => end = true,
				_ => (),
			}
		});

		if end { break; }

		if fps_parser % 1000 == 0 {
			let time = timer.elapsed().unwrap().as_micros() as f32 / 1_000_000_f32;
			timer = SystemTime::now();
			eprint!("\rFPS: {}", 1000_f32 / time);
		}
		fps_parser += 1;
	}

	vk_core.queue_wait_idle().unwrap();
	image_acquired.drop(vk_core);
	render_finish.drop(vk_core);
}


