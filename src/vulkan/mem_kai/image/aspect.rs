use ash::vk::ImageAspectFlags;

pub trait Aspect {
	fn flags() -> ImageAspectFlags;
}

pub struct Color;
pub struct Depth;
pub struct Stencil;

impl Aspect for Color {
	fn flags() -> ImageAspectFlags { ImageAspectFlags::COLOR }
}

impl Aspect for Depth {
	fn flags() -> ImageAspectFlags { ImageAspectFlags::DEPTH }
}

impl Aspect for Stencil {
	fn flags() -> ImageAspectFlags { ImageAspectFlags::STENCIL }
}
