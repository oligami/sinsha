use ash::vk;

use crate::vulkan::*;
use crate::interaction::*;
use crate::linear_algebra::*;

use winit::*;
use winit::dpi::PhysicalSize;

use std::error::Error;
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
		let mut alloc = MemoryBlock::allocator(
			vk_core,
			vk::MemoryPropertyFlags::HOST_VISIBLE,
		);
		alloc.bind_buffer(unsafe {
			Buffer::uninitialized(
				vk_core,
				1025,
				vk::BufferUsageFlags::VERTEX_BUFFER,
				vk::SharingMode::EXCLUSIVE,
			).unwrap()
		});
		let memory = unsafe { alloc.allocate().unwrap() };
		let mut memory_accessor = memory.accessor().unwrap();
		memory_accessor.write(&[1; 32]).unwrap();

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
	}
}