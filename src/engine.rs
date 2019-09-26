use winit::event::*;
use winit::event_loop::ControlFlow;

use crate::vulkan::Vulkan;

pub fn run() {
    let mut event_loop = crate::window::event_loop();
    let window = crate::window::create_window(&event_loop);
    let vulkan = Vulkan::new(window);

    event_loop.run(|event, _, ctrl_flow| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *ctrl_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}