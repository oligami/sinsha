use crate::linear_algebra::*;

use std::default::Default;


pub trait DrawCommand {
    fn draw(&self, command: &mut ());
}

pub struct Graphics {
    vertices: Vec<f32>,
    texture: Vec<u8>,
}

impl DrawCommand for Graphics {
    fn draw(&self, command: &mut ()) {
        unimplemented!()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    pub position: XYZ<f32>,
    pub normal: XYZ<f32>,
}

impl Vertex {
    fn new(position: XYZ<f32>, normal: XYZ<f32>) -> Self {
        Self { position, normal }
    }
}

pub fn regular_icosahedron() -> ([Vertex; 12], [u16; 60]) {
    const PHI: f32 = 1.6180339887498948482;

    let norm = 1.0 + PHI * PHI;
    let regular_positions = [
        XYZ::new( 1.0,  PHI,  0.0) / norm,
        XYZ::new( 1.0, -PHI,  0.0) / norm,
        XYZ::new(-1.0,  PHI,  0.0) / norm,
        XYZ::new(-1.0, -PHI,  0.0) / norm,
        XYZ::new( 0.0,  1.0,  PHI) / norm,
        XYZ::new( 0.0,  1.0, -PHI) / norm,
        XYZ::new( 0.0, -1.0,  PHI) / norm,
        XYZ::new( 0.0, -1.0, -PHI) / norm,
        XYZ::new( PHI,  0.0,  1.0) / norm,
        XYZ::new(-PHI,  0.0,  1.0) / norm,
        XYZ::new( PHI,  0.0, -1.0) / norm,
        XYZ::new(-PHI,  0.0, -1.0) / norm,
    ];

    let mut vertices = [Vertex::default(); 12];

    vertices.iter_mut()
        .zip(regular_positions.iter())
        .for_each(|(Vertex { position, normal }, xyz)| {
            *position = *xyz;
            *normal = *xyz;
        });

    let indices = unimplemented!();

    (vertices, indices)
}