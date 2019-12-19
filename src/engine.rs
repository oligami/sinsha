use winit::*;

use crate::vulkan::Vulkan;

pub fn run() {
    let (mut window, mut events_loop) = crate::window::create_window();
    let vulkan = Vulkan::new(window);

    let mut loop_end = false;
    while !loop_end {
        let event_handler = |event| {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => loop_end = true,
                _ => ()
            };
        };

        events_loop.poll_events(event_handler);
    }
}