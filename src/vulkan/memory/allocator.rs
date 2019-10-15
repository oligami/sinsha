use std::alloc::Layout;

pub unsafe trait Allocator {
    unsafe fn alloc(&mut self, layout: Layout) -> u64;
    unsafe fn dealloc(&mut self, offset: u64, layout: Layout);
}