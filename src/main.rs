mod utility;
mod linear_algebra;
mod vulkan;
mod window;
//mod engine;
mod objects;
//mod gui;
//mod interaction;

//use engine::Engine;

fn main() {
	println!("Hello world!");
	dbg!(std::mem::size_of::<ash::Device>());
//	Engine::run();
}
