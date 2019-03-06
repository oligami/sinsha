use ash::vk;

use crate::vulkan::*;
use crate::interaction::*;
use crate::linear_algebra::*;

use winit::*;
use winit::dpi::PhysicalSize;

use std::time::*;
use std::io::*;

pub struct Engine;

impl Engine {
	pub fn run() {
		let (window, mut events_loop) = Engine::create_window();
		let vk_core = VkCore::new(&window);
		let mut vk_graphic = VkGraphic::new(&vk_core);

		Engine::start_menu(&vk_core, &mut vk_graphic, &window, &mut events_loop);
	}
}

impl Engine {
	fn create_window() -> (Window, EventsLoop) {
		let events_loop = EventsLoop::new();
		let window = WindowBuilder::new()
			.with_dimensions(
				PhysicalSize::new(1280_f64, 720_f64)
					.to_logical(events_loop.get_primary_monitor().get_hidpi_factor())
			)
			.with_title("sinsha")
			.build(&events_loop)
			.unwrap();

		(window, events_loop)
	}

	fn start_menu(
		vk_core: &VkCore,
		vk_graphic: &mut VkGraphic,
		window: &Window,
		events_loop: &mut EventsLoop,
	) {
		let mut interaction_devices = InteractionDevices::new(window);

		let _font_path = "assets/font/friz_quadrata.png";
		let info_box_path = "assets/textures/info_box.png";
		let (image, extent) = LogicalImage::load_image_file(info_box_path).unwrap();
		let logical_image = LogicalImage::new(
			vk_core,
			vk::ImageType::TYPE_2D,
			extent,
			vk::Format::R8G8B8_UNORM,
			vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_SRC,
			vk::SharingMode::EXCLUSIVE,
			vk::ImageLayout::UNDEFINED,
			vk::SampleCountFlags::TYPE_1,
			vk::ImageAspectFlags::COLOR,
			1,
			1,
		).unwrap();

		let image_memory = MemoryBlock::new(
			vk_core,
			vec![],
			vec![(logical_image, vk::ImageViewType::TYPE_2D, vk::ComponentMapping::default())],
			vk::MemoryPropertyFlags::HOST_VISIBLE,
		);

		let logical_buffer = LogicalBuffer::new(
			vk_core,
			gui::Vertex::size(4),
			vk::BufferUsageFlags::VERTEX_BUFFER,
			vk::SharingMode::EXCLUSIVE,
		).unwrap();

		let memory = MemoryBlock::new(
			vk_core,
			vec![logical_buffer],
			vec![],
			vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
		).unwrap();



		let semaphore = VkSemaphore::new(vk_core).unwrap();

		loop {
			let mut close_requested = false;
			events_loop.poll_events(|event| {
				interaction_devices.event_update(&event);
				match event {
					Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
						close_requested = true;
					}
					_ => (),
				}
			});
			if close_requested { break; }

			interaction_devices.clear();
		}

		semaphore.drop(vk_core);
	}
}