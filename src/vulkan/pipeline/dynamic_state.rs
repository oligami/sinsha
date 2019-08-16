use ash::vk;
use ash::vk::DynamicState as VkDynamicState;
use ash::version::DeviceV1_0;

pub trait DynamicState {
	type Type: ?Sized;
	fn dynamic_state() -> VkDynamicState;
	unsafe fn set_state(device: &ash::Device, command_buffer: vk::CommandBuffer, state: &Self::Type);
}

pub struct Viewport;
impl DynamicState for Viewport {
	type Type = [vk::Viewport];
	fn dynamic_state() -> VkDynamicState {
		VkDynamicState::VIEWPORT
	}

	unsafe fn set_state(device: &ash::Device, command_buffer: vk::CommandBuffer, state: &Self::Type) {
		device.cmd_set_viewport(command_buffer, 0, state);
	}
}

pub struct Scissor;
impl DynamicState for Scissor {
	type Type = [vk::Rect2D];
	fn dynamic_state() -> VkDynamicState {
		VkDynamicState::SCISSOR
	}

	unsafe fn set_state(device: &ash::Device, command_buffer: vk::CommandBuffer, state: &Self::Type) {
		device.cmd_set_scissor(command_buffer, 0, state);
	}
}

pub struct LineWidth;
impl DynamicState for LineWidth {
	type Type = f32;
	fn dynamic_state() -> VkDynamicState {
		VkDynamicState::LINE_WIDTH
	}

	unsafe fn set_state(device: &ash::Device, command_buffer: vk::CommandBuffer, state: &Self::Type) {
		device.cmd_set_line_width(command_buffer, *state);
	}
}

pub struct DepthBias;
impl DynamicState for DepthBias {
	type Type = (f32, f32, f32);
	fn dynamic_state() -> VkDynamicState {
		VkDynamicState::DEPTH_BIAS
	}

	unsafe fn set_state(device: &ash::Device, command_buffer: vk::CommandBuffer, state: &Self::Type) {
		device.cmd_set_depth_bias(command_buffer, state.0, state.1, state.2);
	}
}

// DepthBias, DEPTH_BIAS, (f32, f32, f32),
// BlendConstants, BLEND_CONSTANTS, [f32; 4],
// DepthBounds, DEPTH_BOUNDS, std::ops::Range<f32>,
// StencilCompareMask, STENCIL_COMPARE_MASK, (ash::vk::StencilFaceFlags, u32),
// StencilWriteMask, STENCIL_WRITE_MASK, (ash::vk::StencilFaceFlags, u32),
// StencilReference, STENCIL_REFERENCE, (ash::vk::StencilFaceFlags, u32),
// ViewportWScalingNv, VIEWPORT_W_SCALING_NV, [ash::vk::ViewportWScalingNV],
// DiscardRectangleExt, DISCARD_RECTANGLE_EXT, [ash::vk::Rect2D],
// SampleLocationsExt, SAMPLE_LOCATIONS_EXT, ash::vk::SampleLocationsInfoEXT,
// ViewportShadingRatePaletteNv, VIEWPORT_SHADING_RATE_PALETTE_NV, [ash::vk::ShadingRatePaletteNV],
// ViewportCoarseSampleOrderNv, VIEWPORT_COARSE_SAMPLE_ORDER_NV, (ash::vk::CoarseSampleOrderTypeNV, [ash::vk::CoarseSampleOrderCustomNV]),
// ExclusiveScissorNv, EXCLUSIVE_SCISSOR_NV, [ash::vk::Rect2D],
