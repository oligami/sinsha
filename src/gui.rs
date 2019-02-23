use winit::VirtualKeyCode;

use crate::vulkan_api::gui::*;
use crate::interaction::*;
use crate::linear_algebra::*;

use std::ops::FnMut;


pub struct Button {
	tag: &'static str,
	key_binding: VirtualKeyCode,
}

impl Button {
	pub fn new(tag: &'static str, key_binding: VirtualKeyCode) -> Self {
		Self { tag, key_binding }
	}

	pub fn behavior<F>(
		&self,
		interaction_devices: &InteractionDevices,
		rect2ds: &mut Rect2Ds,
		mut subroutine: F,
	) where F: FnMut() {
		match interaction_devices.keyboard.get(&self.key_binding) {
			KeyState::JustReleased => {
				subroutine();
				rect2ds.update_color_weight(self.tag, RGBA::default());
			},
			KeyState::JustPressed => {
				rect2ds.update_color_weight(self.tag, RGBA::new(0.7, 0.7, 0.7, 1.0))
			},
			_ => (),
		}

		if rect2ds.extent(self.tag).contain(&interaction_devices.mouse.position) {
			match interaction_devices.mouse.left {
				KeyState::JustReleased => {
					subroutine();
					rect2ds.update_color_weight(self.tag, RGBA::default());
				},
				KeyState::JustPressed => {
					rect2ds.update_color_weight(self.tag, RGBA::new(0.7, 0.7, 0.7, 1.0));
				},
				_ => (),
			}
		} else {
			match interaction_devices.mouse.left {
				KeyState::JustReleased => {
					rect2ds.update_color_weight(self.tag, RGBA::default());
				},
				_ => (),
			}
		}
	}
}
