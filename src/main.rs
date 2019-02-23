mod linear_algebra;
mod vulkan_api;
mod engine;
mod gui;
mod interaction;

use engine::Engine;

fn main() {
	let mut engine = Engine::new();
	engine.run();
}
