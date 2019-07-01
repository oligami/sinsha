use ash::vk::DynamicState as VkDynamicState;
pub trait DynamicState {
	type Type: ?Sized;
	fn dynamic_state() -> VkDynamicState;
}

macro_rules! impl_dynamic_state {
	($($name: ident, $value: ident, $ty: ty,)*) => {$(
		pub struct $name;
		impl DynamicState for $name {
			type Type = $ty;
			fn dynamic_state() -> VkDynamicState {
				VkDynamicState::$value
			}
		}
	)*};
}

impl_dynamic_state!(
	Viewport, VIEWPORT, [ash::vk::Viewport],
	Scissor, SCISSOR, [ash::vk::Rect2D],
	LineWidth, LINE_WIDTH, f32,
	DepthBias, DEPTH_BIAS, (f32, f32, f32),
	BlendConstants, BLEND_CONSTANTS, [f32; 4],
	DepthBounds, DEPTH_BOUNDS, std::ops::Range<f32>,
	StencilCompareMask, STENCIL_COMPARE_MASK, (ash::vk::StencilFaceFlags, u32),
	StencilWriteMask, STENCIL_WRITE_MASK, (ash::vk::StencilFaceFlags, u32),
	StencilReference, STENCIL_REFERENCE, (ash::vk::StencilFaceFlags, u32),
	ViewportWScalingNv, VIEWPORT_W_SCALING_NV, [ash::vk::ViewportWScalingNV],
	DiscardRectangleExt, DISCARD_RECTANGLE_EXT, [ash::vk::Rect2D],
	SampleLocationsExt, SAMPLE_LOCATIONS_EXT, ash::vk::SampleLocationsInfoEXT,
	ViewportShadingRatePaletteNv, VIEWPORT_SHADING_RATE_PALETTE_NV, [ash::vk::ShadingRatePaletteNV],
	ViewportCoarseSampleOrderNv, VIEWPORT_COARSE_SAMPLE_ORDER_NV, (ash::vk::CoarseSampleOrderTypeNV, [ash::vk::CoarseSampleOrderCustomNV]),
	ExclusiveScissorNv, EXCLUSIVE_SCISSOR_NV, [ash::vk::Rect2D],
);