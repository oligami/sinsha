use std::time;

pub struct Timer {
    start: time::SystemTime,
}

impl Timer {
    #[inline]
    pub fn new() -> Self { Self { start: time::SystemTime::now() } }
    #[inline]
    pub fn start(&mut self) { self.start = time::SystemTime::now(); }
    #[inline]
    pub fn lap(&self) -> time::Duration { self.start.elapsed().unwrap() }
}