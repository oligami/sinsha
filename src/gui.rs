use crate::graphics::Graphics;

use std::ops::Range;

pub struct Button {
    hit_box: HitBox,
}

pub struct HitBox {
    range_of_x: Range<f32>,
    range_of_y: Range<f32>,
}

impl Button {
    pub fn hit_and_then<F: FnMut()>(input: (), mut behavior: F) {
        behavior();
        unimplemented!()
    }
}