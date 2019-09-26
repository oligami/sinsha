use winit::event_loop::EventLoop;
use winit::window::Window;
use winit::window::WindowBuilder;
use winit::dpi::LogicalSize;

pub enum CustomEvent {}

pub fn event_loop() -> EventLoop<CustomEvent> {
    EventLoop::new_user_event()
}

pub fn create_window(event_loop: &EventLoop<CustomEvent>) -> Window {
    let dpi = event_loop.primary_monitor().hidpi_factor();
    WindowBuilder::new()
        .with_title("sinsha")
        .with_inner_size(LogicalSize::from_physical((1280.0, 720.0), dpi))
        .build(&event_loop)
        .unwrap()
}