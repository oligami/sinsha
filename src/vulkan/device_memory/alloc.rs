use std::fmt;
use std::alloc::Layout;
use std::cell::RefCell;
use std::sync::Mutex;


pub unsafe trait Allocator {
    type Identifier;
    fn size(&self) -> u64;
    fn alloc(&self, layout: Layout) -> Result<(u64, Self::Identifier), AllocErr>;
    fn dealloc(&self, id: &Self::Identifier);
}

pub struct BuddyAllocator<A>(A);

pub struct BuddyAllocatorInner {
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

impl BuddyAllocator<()> {
    pub fn new_with_ref_cell(order: u32, block_size: u64)
        -> BuddyAllocator<RefCell<BuddyAllocatorInner>>
    {
        let inner = BuddyAllocatorInner::new(order, block_size);
        BuddyAllocator(RefCell::new(inner))
    }

    pub fn new_with_mutex(order: u32, block_size: u64)
        -> BuddyAllocator<Mutex<BuddyAllocatorInner>>
    {
        let inner = BuddyAllocatorInner::new(order, block_size);
        BuddyAllocator(Mutex::new(inner))
    }
}

unsafe impl Allocator for BuddyAllocator<RefCell<BuddyAllocatorInner>> {
    type Identifier = BuddyAllocIdentifier;
    fn size(&self) -> u64 { self.0.borrow().size() }
    fn alloc(&self, layout: Layout) -> Result<(u64, Self::Identifier), AllocErr> {
        self.0.borrow_mut().alloc(layout)
    }
    fn dealloc(&self, id: &Self::Identifier) {
        self.0.borrow_mut().dealloc(id);
    }
}

unsafe impl Allocator for BuddyAllocator<Mutex<BuddyAllocatorInner>> {
    type Identifier = BuddyAllocIdentifier;
    fn size(&self) -> u64 { self.0.lock().unwrap().size() }
    fn alloc(&self, layout: Layout) -> Result<(u64, Self::Identifier), AllocErr> {
        self.0.lock().unwrap().alloc(layout)
    }
    fn dealloc(&self, id: &Self::Identifier) {
        self.0.lock().unwrap().dealloc(id);
    }
}

impl BuddyAllocatorInner {
    fn new(order: u32, block_size: u64) -> Self {
        let used = (0..order)
            .map(|order| Vec::with_capacity(2_usize.pow(order)))
            .rev()
            .collect::<Vec<_>>();

        let mut unused = used.clone();
        unused[(order - 1) as usize].push(0);

        Self { order, block_size, used, unused }
    }
}

impl BuddyAllocatorInner {
    fn size(&self) -> u64 { self.block_size * 2_u64.pow(self.order) }

    /// TODO: check alignment.
    fn alloc(&mut self, layout: Layout) -> Result<(u64, BuddyAllocIdentifier), AllocErr> {
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
                let address = index as u64 * alloc_size;
                self.used[order_index].push(index);
                Ok((address, BuddyAllocIdentifier::new(required_order, index)))
            },
            None => {
                for big_buddy_order in order_index + 1..self.order as usize {
                    if self.unused[big_buddy_order].last().is_some() {
                        let big_buddy_index = self.unused[big_buddy_order].pop().unwrap();

                        let (_, index) = (0 .. big_buddy_order - order_index)
                            .fold((big_buddy_order - 1, big_buddy_index << 1), |(order, index), _| {
                                self.unused[order].push(index + 1);
                                (order.overflowing_sub(1).0, index << 1)
                            });
                        self.used[order_index].push(index >> 1);

                        let address = self.block_size * 2_u64.pow(big_buddy_order as u32);
                        let ident = BuddyAllocIdentifier::new(order_index as u32, index >> 1);
                        return Ok((address, ident));
                    }
                }

                Err(AllocErr::OutOfHeap)
            },
        }
    }

    fn dealloc(&mut self, id: &BuddyAllocIdentifier) {
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

impl fmt::Display for BuddyAllocatorInner {
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