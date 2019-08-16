use winit::window::Window;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::event_loop::EventLoopProxy;
use winit::event_loop::ControlFlow;
use winit::event::Event;
use winit::dpi::PhysicalSize;

use std::thread;
use std::sync::mpsc;

#[derive(Copy, Clone, Debug)]
pub enum CustomEvent {
    Exit,
}


pub fn create_window() -> (Window, EventLoopProxy<CustomEvent>, mpsc::Receiver<Event<CustomEvent>>) {
    let (window_sender, window_receiver) = mpsc::channel();
    let (proxy_sender, proxy_receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();

    // TODO: join this thread before main thread close
    let handle = std::thread::spawn(move || {
        let event_loop = EventLoop::new_user_event();
        let proxy = event_loop.create_proxy();
        let window = WindowBuilder::new()
            .with_inner_size(
                PhysicalSize::new(1280_f64, 720_f64)
                    .to_logical(event_loop.primary_monitor().hidpi_factor())
            )
            .with_title("sinsha")
            .build(&event_loop)
            .unwrap();

        window_sender.send(window).unwrap();
        proxy_sender.send(proxy).unwrap();

        event_loop.run(move |event, _, ctrl| {
            *ctrl = match event {
                // TODO: Exit when all GPU resources has dropped.
                Event::UserEvent(CustomEvent::Exit) => ControlFlow::Exit,
                _ => ControlFlow::Wait,
            };
            event_sender.send(event).unwrap();
        });
    });

    let window = window_receiver.recv().unwrap();
    let proxy = proxy_receiver.recv().unwrap();

    (window, proxy, event_receiver)
}