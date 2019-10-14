use winit::EventsLoop;
use winit::Window;
use winit::WindowBuilder;
use winit::dpi::LogicalSize;

pub fn create_window() -> (Window, EventsLoop) {
    let events_loop = EventsLoop::new();
    let dpi = events_loop.get_primary_monitor().get_hidpi_factor();
    let window = WindowBuilder::new()
        .with_title("sinsha")
        .with_dimensions(LogicalSize::from_physical((1280.0, 720.0), dpi))
        .build(&events_loop)
        .unwrap();

    (window, events_loop)
}