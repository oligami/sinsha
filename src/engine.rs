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
			.zip([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10].iter())
			.for_each(|(ty, i)| eprintln!("type{}: {:?}", i, ty));
		mem_prop.memory_heaps
			.iter()
			.zip([0, 1].iter())
			.for_each(|(heap, i)| eprintln!("heap{}: {:?}", i, heap));

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

		let mut vk_alloc = mem::VkAlloc::new(vk_core);
		let buffer_handle = vk_alloc
			.push_buffer(
				0x1000_0000,
				vk::BufferUsageFlags::TRANSFER_DST,
				vk::SharingMode::EXCLUSIVE,
				vk::MemoryPropertyFlags::DEVICE_LOCAL,
			)
			.unwrap();

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