use std::ops;
use std::fmt;
use std::alloc::Layout;

pub trait VkAlloc {
	fn alloc<T>() -> ops::Range<u64>;
	fn alloc_unsized<T>(_t: &T) -> ops::Range<u64> where T: ?Sized;
	fn dealloc<T>(t: T);
}

pub trait AllocationManager {
	type Identifier;
	fn alloc(&mut self, layout: Layout) -> Result<(ops::Range<u64>, Self::Identifier), AllocErr>;
	fn dealloc(&mut self, id: Self::Identifier);
}

#[derive(Debug)]
pub struct BuddyAllocManager {
	order: u32,
	block_size: u64,
	used: Vec<Vec<u32>>,
	unused: Vec<Vec<u32>>,
}

pub struct BuddyAllocIdentifier {
	order: u32,
	index: u32,
}

#[derive(Debug)]
pub enum AllocErr {
	ExcessSizeOfHeap,
	OutOfHeap,
}

impl BuddyAllocManager {
	pub fn new(order: u32, block_size: u64) -> Self {
		let used = (0..order)
			.map(|order| Vec::with_capacity(2_usize.pow(order)))
			.rev()
			.collect::<Vec<_>>();

		let mut unused = used.clone();
		unused[(order - 1) as usize].push(0);

		Self { order, block_size, used, unused }
	}

	pub fn size(&self) -> u64 { 2_u64.pow(self.order) * self.block_size }
}

impl AllocationManager for BuddyAllocManager {
	type Identifier = BuddyAllocIdentifier;

	/// TODO: check alignment.
	fn alloc(&mut self, layout: Layout) -> Result<(ops::Range<u64>, Self::Identifier), AllocErr> {
		let required_order = (0..self.order)
			.try_for_each(|order| {
				if layout.size() <= self.block_size as usize * 2_usize.pow(order) {
					Err(Some(order))
				} else {
					Ok(())
				}
			})
			.err()
			.ok_or(AllocErr::ExcessSizeOfHeap)?
			.unwrap();

		let order_index = required_order as usize;
		let alloc_size = self.block_size * 2_u64.pow(required_order);
		match self.unused[order_index].pop() {
			Some(index) => {
				let range_start = index as u64 * alloc_size;
				let range = range_start .. range_start + alloc_size;
				self.used[order_index].push(index);
				Ok((range, Self::Identifier::new(required_order, index)))
			},
			None => {
				for big_buddy_order in order_index + 1..self.order as usize {
					if self.unused[big_buddy_order].last().is_some() {
						let big_buddy_index = self.unused[big_buddy_order].pop().unwrap();
						let range_start = self.block_size * 2_u64.pow(big_buddy_order as u32);
						let range = range_start .. range_start + alloc_size;

						let (_, index) = (0 .. big_buddy_order - order_index)
							.fold((big_buddy_order - 1, big_buddy_index << 1), |(order, index), _| {
								self.unused[order].push(index + 1);
								(order.overflowing_sub(1).0, index << 1)
							});
						self.used[order_index].push(index >> 1);

						return Ok((range, Self::Identifier::new(order_index as u32, index >> 1)));
					}
				}

				Err(AllocErr::OutOfHeap)
			},
		}
	}

	fn dealloc(&mut self, id: Self::Identifier) {
		let order_index = id.order as usize;
		let dealloc_position = self.used[order_index]
			.iter()
			.position(|block| block == &id.index)
			.unwrap();

		self.used[order_index].swap_remove(dealloc_position);

		// search a buddy of the dealloc block
		let mut order = order_index;
		let mut self_index = id.index;
		let mut buddy_index = if id.index & 1 == 0 { id.index + 1 } else { id.index - 1 };
		while let Some(buddy_position) = self.unused[order]
			.iter()
			.position(|index| *index == buddy_index)
		{

			self.unused[order].swap_remove(buddy_position);
			order += 1;
			self_index /= 2;
			buddy_index = if self_index & 1 == 0 { self_index + 1 } else { self_index - 1 };
		}
		self.unused[order].push(self_index);
	}
}

impl fmt::Display for BuddyAllocManager {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		(0..self.order as usize)
			.try_for_each(|order| {
				write!(f, "Order[{}] Size: {}\n", order, self.block_size * 2_u64.pow(order as u32))
					.and(write!(f, "\tUsed: "))
					.and(
						self.used[order]
							.iter()
							.try_for_each(|index| write!(f, "{}, ", index))
					)
					.and(writeln!(f))
					.and(write!(f, "\tUnused: "))
					.and(
						self.unused[order]
							.iter()
							.try_for_each(|index| write!(f, "{}, ", index))
					)
					.and(writeln!(f))
			})
	}
}

impl BuddyAllocIdentifier {
	fn new(order: u32, index: u32) -> Self { Self { order, index, }}
}