pub trait Array<T> {
	fn len() -> usize;
}

macro_rules! impl_array_trait {
	($($len: expr,)*) => {$(
		impl<T> Array<T> for [T; $len] {
			fn len() -> usize { $len }
		}
	)*};
}

impl_array_trait!(
	 1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15, 16,
	17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
);

pub trait TypeIterEnd {}

pub trait TypeIterator {
	type This;
	type NextIter;
	fn size() -> usize;
}

impl<T> TypeIterator for T where T: TypeIterEnd {
	type This = T;
	type NextIter = T;
	fn size() -> usize { 0 }
}
impl<T, U> TypeIterator for (T, U) where T: TypeIterator {
	type This = U;
	type NextIter = T;
	fn size() -> usize { T::size() + 1 }
}