use ash::vk;
use crate::linear_algebra::*;
use std::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct Vertex {
    xyz: XYZ<f32>,
    rgb: XYZ<f32>,
}

impl Vertex {
    pub const BINDINGS: [vk::VertexInputBindingDescription; 1] = [
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        },
    ];

    pub const ATTRIBUTES: [vk::VertexInputAttributeDescription; 2] = [
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 0,
            offset: 0,
            format: vk::Format::R32G32B32_SFLOAT,
        },
        vk::VertexInputAttributeDescription {
            binding: 0,
            location: 1,
            offset: size_of::<XYZ<f32>>() as u32,
            format: vk::Format::R32G32B32_SFLOAT,
        },
    ];

    pub fn new(xyz: XYZ<f32>, rgb: XYZ<f32>) -> Self {
        Self { xyz, rgb }
    }
}