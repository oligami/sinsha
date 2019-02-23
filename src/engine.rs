use ash::vk::Result as VkResult;

use crate::vulkan_api::*;
use crate::vulkan_api::gui::*;
use crate::linear_algebra::*;
use crate::gui::*;
use crate::interaction::*;

use winit::*;
use winit::dpi::PhysicalSize;

use std::error::Error;
use std::time::*;

pub struct Engine {
	vulkan: Vulkan,
	window: Window,
	events_loop: EventsLoop,
}

impl Engine {
	pub fn new() -> Self {
		let (window, events_loop) = Self::create_window();
		let vulkan = Self::init_vulkan(&window);

		dbg!("engine started.");
		Self {
			vulkan,
			window,
			events_loop,
		}
	}

	pub fn run(&mut self) {
		self.start_menu();
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

	fn init_vulkan(window: &Window) -> Vulkan {
		Vulkan::new(window)
	}

	fn start_menu(&mut self) {
		let mut interaction_devices = InteractionDevices::new(&self.window);
		let sampler = self.vulkan.default_sampler();
		let mut rect2ds = Rect2Ds::start_builder()
			.next("quit", "assets/textures/info_box.png", sampler)
			.extent(Extent2D {
				top: 300,
				bottom: 364,
				left: 300,
				right: 364,
			})
			.origin(TOP_LEFT)
			.color([
				RGBA::new(1.0, 1.0, 0.5, 1.0),
				RGBA::new(1.0, 0.5, 1.0, 1.0),
				RGBA::default(),
				RGBA::new(0.5, 1.0, 1.0, 1.0)
			])
			.next_same_texture("test")
			.extent(Extent2D {
				top: -128,
				bottom: 0,
				left: -128,
				right: 0,
			})
			.origin(BOTTOM_RIGHT)
			.build(&self.vulkan);

		let button = Button::new("quit", VirtualKeyCode::Escape);

		let mut system_time = SystemTime::now();
		let mut counter = 0_u64;
		loop {
			let command_recorder = match self.vulkan.begin_frame() {
				Ok(command_recorder) => command_recorder,
				Err(VkResult::ERROR_OUT_OF_DATE_KHR) | Err(VkResult::SUBOPTIMAL_KHR) => {
					self.vulkan.deal_with_window_resize();
					rect2ds.deal_with_window_resize(&self.vulkan);
					continue;
				},
				Err(err) => panic!("{}", err.description()),
			};

			let command_recorder = command_recorder
				.begin_recording()
				.begin_render_pass(&self.vulkan.default_clear_value())
				.enter_gui_pipeline()
				.draw(&rect2ds)
				.quit_gui_pipeline()
				.end_render_pass()
				.end_recording();

			match self.vulkan.end_frame(command_recorder) {
				Ok(()) => (),
				Err(VkResult::ERROR_OUT_OF_DATE_KHR) | Err(VkResult::SUBOPTIMAL_KHR) => {
					self.vulkan.deal_with_window_resize();
					rect2ds.deal_with_window_resize(&self.vulkan);
					continue;
				},
				Err(err) => panic!("{}", err.description()),
			}

			let mut close_requested = false;
			self.events_loop.poll_events(|event| {
				interaction_devices.event_update(&event);
				match event {
					Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
						close_requested = true;
					},
					_ => (),
				}
			});

//			dbg!(&interaction_devices);
			button.behavior(&interaction_devices, &mut rect2ds, || { close_requested = true });

			if close_requested {
				break;
			}

			interaction_devices.clear();
			if counter % 1_000 == 0 {
				eprintln!("{}", 1_000_000_000_f32 / system_time.elapsed().unwrap().subsec_micros() as f32);
				system_time = SystemTime::now();
			}
			counter += 1;
		}

		self.vulkan.destroy(rect2ds);
		self.vulkan.destroy(sampler);
	}
}