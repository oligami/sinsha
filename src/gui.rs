use std::ops::Range;

pub struct Texture;


pub struct Button {
    hit_box: HitBox,
    texture_index: usize,
}

pub struct HitBox {
    range_of_x: Range<f32>,
    range_of_y: Range<f32>,
}

impl Button {
    pub fn builder() {}

    pub fn hit_and_then<F: FnMut()>(input: (), mut behavior: F) {
        let hit = unimplemented!();
        if hit { behavior() }
    }
}

