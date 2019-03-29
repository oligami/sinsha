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

		let mem_prop = vk_core.memory_properties();
		eprintln!("types: {}, heaps: {}", mem_prop.memory_type_count, mem_prop.memory_heap_count);
		mem_prop.memory_types
			.iter()
			.zip(0..mem_prop.memory_type_count)
			.for_each(|(ty, i)| eprintln!("type{}: {:?}", i, ty));
		mem_prop.memory_heaps
			.iter()
			.zip(0..mem_prop.memory_heap_count)
			.for_each(|(heap, i)| eprintln!("heap{}: {:?}", i, heap));

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