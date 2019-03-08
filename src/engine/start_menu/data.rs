use crate::vulkan::gui::*;
use crate::linear_algebra::*;

pub const START_BUTTON: [Vertex; 4] = [
	Vertex::new(RGBA::new(1.0, 1.0, 1.0, 1.0), XY::new(-1.0 / 16.0, -1.0 / 16.0), XY::zero()),
	Vertex::new(RGBA::new(1.0, 1.0, 1.0, 1.0), XY::new(-1.0 / 16.0,  1.0 / 16.0), XY::zero()),
	Vertex::new(RGBA::new(1.0, 1.0, 1.0, 1.0), XY::new( 1.0 / 16.0,  1.0 / 16.0), XY::zero()),
	Vertex::new(RGBA::new(1.0, 1.0, 1.0, 1.0), XY::new( 1.0 / 16.0, -1.0 / 16.0), XY::zero()),
];