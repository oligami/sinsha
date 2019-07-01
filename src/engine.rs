mod start_menu;

use crate::vulkan::*;

use winit::*;
use winit::dpi::PhysicalSize;

pub struct Engine;

impl Engine {
	pub fn run() {
		let (window, mut events_loop) = Engine::create_window();
		let instance = Instance::new();
		let surface = SurfaceKhr::new(instance.clone(), window);
		let (device, queue) = Device::new_with_a_graphics_queue(instance.clone(), surface.clone(), 1.0);

		start_menu::run_kai(surface.clone(), device.clone(), queue.clone(), &mut events_loop);

		drop(queue);
		unsafe {
			device.try_destroy().unwrap();
			surface.try_destroy().unwrap();
			instance.try_destroy().unwrap();
		}
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