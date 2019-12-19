mod presets;

use std::alloc::Layout;

use std::rc::Rc;
use std::cell::RefCell;

use std::sync::{ Arc, Mutex, RwLock, TryLockError };

pub enum AllocErr {
    OutOfMemory,
    AllocatorBusy,
    AllocatorPoisoned,
}

pub unsafe trait InnerAllocator {
    fn capacity(&self) -> u64;
    unsafe fn alloc(&mut self, layout: Layout) -> Result<u64, AllocErr>;
    unsafe fn dealloc(&mut self, offset: u64, layout: Layout);
}

/// TODO: 参照カウンタで所有権を複製しても安全かどうか。
pub unsafe trait Allocator {
    fn capacity(&self) -> u64;
    unsafe fn alloc(&self, layout: Layout) -> Result<u64, AllocErr>;
    unsafe fn dealloc(&self, offset: u64, layout: Layout);
}


// Implementations //
impl<T> From<TryLockError<T>> for AllocErr {
    fn from(err: TryLockError<T>) -> AllocErr {
        match err {
            TryLockError::Poisoned(_) => AllocErr::AllocatorPoisoned,
            TryLockError::WouldBlock => AllocErr::AllocatorBusy,
        }
    }
}

unsafe impl<A: InnerAllocator> Allocator for Rc<RefCell<A>> {
    fn capacity(&self) -> u64 { self.borrow().capacity() }

    unsafe fn alloc(&self, layout: Layout) -> Result<u64, AllocErr> {
        self.borrow_mut().alloc(layout)
    }

    unsafe fn dealloc(&self, offset: u64, layout: Layout) {
        self.borrow_mut().dealloc(offset, layout)
    }
}

unsafe impl<A: InnerAllocator> Allocator for Arc<Mutex<A>> {
    fn capacity(&self) -> u64 { self.lock().unwrap().capacity() }

    unsafe fn alloc(&self, layout: Layout) -> Result<u64, AllocErr> {
        self.try_lock()?
            .alloc(layout)
    }

    unsafe fn dealloc(&self, offset: u64, layout: Layout) {
        self.lock().unwrap().dealloc(offset, layout)
    }
}

unsafe impl<A: InnerAllocator> Allocator for Arc<RwLock<A>> {
    fn capacity(&self) -> u64 { self.read().unwrap().capacity() }

    unsafe fn alloc(&self, layout: Layout) -> Result<u64, AllocErr> {
        self.try_write()?
            .alloc(layout)
    }

    unsafe fn dealloc(&self, offset: u64, layout: Layout) {
        self.write().unwrap().dealloc(offset, layout)
    }
}
