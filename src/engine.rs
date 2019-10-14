use winit::*;

use crate::vulkan::Vulkan;

pub fn run() {
    let (mut window, mut events_loop) = crate::window::create_window();

    let mut loop_end = false;

    let event_handler = |event| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => loop_end = true,
            _ => ()
        };
    };
    while !loop_end {
        events_loop.poll_events(event_handler);
    }
}