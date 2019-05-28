use std::ops;

pub trait VkAlloc {
	fn alloc<T>() -> ops::Range<u64>;
	fn alloc_unsized<T>(_t: &T) -> ops::Range<u64> where T: ?Sized;
	fn dealloc<T>(t: T);
}

pub trait BuddyAlloc {
	fn order() -> u64;
	fn block_size() -> u64;


	fn size() -> u64 { 2_u64.pow(Self::order() as u32) * Self::block_size() }

	fn alloc(size: u64) -> ops::Range<u64> {
		unimplemented!()
	}

	fn dealloc();
}