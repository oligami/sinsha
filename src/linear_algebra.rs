use std::slice;
use std::mem;
use std::default::Default;
use std::ops::{Add, Sub, Mul, Div};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RGBA {
	pub r: f32,
	pub g: f32,
	pub b: f32,
	pub a: f32,
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct XY {
	pub x: f32,
	pub y: f32,
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct XYZ {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct XYZW {
	pub x: f32,
	pub y: f32,
	pub z: f32,
	pub w: f32,
}


#[repr(C)]
#[derive(Clone, Debug)]
pub struct Matrix4 {
	c0: XYZW,
	c1: XYZW,
	c2: XYZW,
	c3: XYZW,
}


pub struct Camera {
	pos: XYZ,
	up: XYZ,
	dir: XYZ,
}


impl Default for RGBA {
	fn default() -> Self {
		Self {
			r: 1.0,
			g: 1.0,
			b: 1.0,
			a: 1.0,
		}
	}
}

impl RGBA {
	pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
		Self { r, g, b, a }
	}

	pub fn to_ref_u8_slice(&self) -> &[u8] {
		unsafe {
			let ptr = self as *const _ as *const u8;
			slice::from_raw_parts(ptr, mem::size_of::<Self>())
		}
	}
}


impl XY {
	pub const fn zero() -> Self {
		Self {
			x: 0.0,
			y: 0.0,
		}
	}

	pub const fn new(x: f32, y: f32) -> Self {
		Self { x, y }
	}

	/// 0 < x, y < A (A > 0) into -1.0 < x, y < 1.0
	pub fn normalize(&self, frame: &Self) -> Self {
		let x = ((self.x / frame.x) - 0.5) * 2.0;
		let y = ((self.y / frame.y) - 0.5) * 2.0;
		Self::new(x, y)
	}
}

impl Add for XY {
	type Output = Self;
	fn add(self, rhs: Self) -> Self {
		Self {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
		}
	}
}

impl Sub for XY {
	type Output = Self;
	fn sub(self, rhs: Self) -> Self {
		Self {
			x: self.x - rhs.x,
			y: self.y - rhs.y,
		}
	}
}

impl Mul for XY {
	type Output = Self;
	fn mul(self, rhs: Self) -> Self {
		Self {
			x: self.x * rhs.x,
			y: self.y * rhs.y,
		}
	}
}

impl Div for XY {
	type Output = Self;
	fn div(self, rhs: Self) -> Self {
		Self {
			x: self.x / rhs.x,
			y: self.y / rhs.y,
		}
	}
}