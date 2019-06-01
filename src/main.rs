mod linear_algebra;
//mod vulkan_api;
mod vulkan;
mod engine;
//mod gui;
mod interaction;

use engine::Engine;
use vulkan::alloc;

fn main() {
	let mut alloc = alloc::BuddyAllocManager::new(5, 0x10);

	use std::alloc::Layout;
	use alloc::AllocationManager;

	let layout1 = Layout::from_size_align(24, 32).unwrap();
	let handle1 = alloc.alloc(layout1).unwrap();
	let handle2 = alloc.alloc(layout1).unwrap();
	let handle3 = alloc.alloc(layout1).unwrap();

	alloc.dealloc(handle1.1);
	alloc.dealloc(handle2.1);
	alloc.dealloc(handle3.1);
	println!("{}", alloc);

//	Engine::run();
}
