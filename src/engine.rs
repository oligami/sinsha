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
		let mut command_buffers = CommandBuffers::new(
			vk_core,
			vk::CommandPoolCreateFlags::TRANSIENT,
			vk::CommandBufferLevel::PRIMARY,
			1,
		).unwrap();

		let mut command_recorder = command_buffers
			.recorder(0, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).unwrap();
		let fence = VkFence::new(vk_core, false).unwrap();
		let [staging_buffer, textures, vertex_buffer] =
			start_menu::load_gui(vk_core, &mut command_recorder).unwrap();
		command_recorder.end().unwrap();
		command_buffers
			.queue_submit(0, vk::PipelineStageFlags::empty(), &[], &[], Some(&fence))
			.unwrap();
		fence.wait(None).unwrap();
		drop(command_buffers);

		let sampler = VkSampler::new(
			vk_core,
			(vk::Filter::LINEAR, vk::Filter::NEAREST),
			vk::SamplerAddressMode::REPEAT,
			vk::SamplerAddressMode::REPEAT,
			vk::SamplerAddressMode::REPEAT,
			vk::BorderColor::FLOAT_OPAQUE_BLACK,
			vk::SamplerMipmapMode::NEAREST,
			0.0,
			0.0..10.0,
			None,
			None,
			vk::FALSE,
		).unwrap();

		let descriptor_pool = gui::DescriptorPool::new(
			vk_graphic,
			&[(textures.image_ref(0), &sampler)],
		).unwrap();



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