mod start_menu;

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

		if cfg!(debug_assertions) {
			let memory_properties = vk_core.memory_properties();
			memory_properties.memory_types
				.iter()
				.zip(0..memory_properties.memory_type_count)
				.for_each(|(ty, i)| eprintln!(
					"[memory (type {:2})] properties: {:?}, heap_index: {}",
					i, ty.property_flags, ty.heap_index,
				));
			eprintln!();

			memory_properties.memory_heaps
				.iter()
				.zip(0..memory_properties.memory_heap_count)
				.for_each(|(heap, i)| eprintln!(
					"[heap{}] size: {}, flags: {:?}",
					i, heap.size, heap.flags,
				));
			eprintln!();
		}

		start_menu::run(&vk_core, &mut vk_graphic, &window, &mut events_loop);
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
}