use crate::graphics::Graphics;

use std::ops::Range;

pub struct Button<F> {
	graphics: Graphics,
	hit_area: (Range<f32>, Range<f32>),
	behavior: F,
}