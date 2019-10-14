use std::default::Default;
use std::ops::{ Neg, Add, Sub, Mul, Div };

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Ord, PartialOrd, Eq, PartialEq)]
pub struct XY<T> {
    pub x: T,
    pub y: T,
}


#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct XYZ<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct XYZW<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

pub struct Camera {
    pos: XYZ<f32>,
    up: XYZ<f32>,
    dir: XYZ<f32>,
}

impl<T> XY<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T> XYZ<T> {
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T> Div<T> for XYZ<T> where T: Div<T, Output = T>, T: Copy {
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}