use winit::*;

use crate::linear_algebra::*;

use std::collections::HashMap;

#[derive(Debug)]
pub enum KeyState {
    Pressed,
    Released,
    JustPressed,
    JustReleased,
}

#[derive(Clone, Debug)]
pub struct Key {
    previous: ElementState,
    current: ElementState,
}

#[derive(Debug)]
pub enum WheelState {
    ScrollUp,
    ScrollDown,
    None,
}

#[derive(Debug)]
pub struct Mouse {
    pub left: KeyState,
    pub right: KeyState,
    pub wheel: WheelState,
    /// -1.0 <= x, y <= 1.0
    pub position: XY<f32>,
    pub raw_move: XY<f32>,
}

#[derive(Debug)]
pub struct KeyBoard {
    hash_map: HashMap<VirtualKeyCode, KeyState>,
}

#[derive(Debug)]
pub struct InputDevices {
    pub mouse: Mouse,
    pub keyboard: KeyBoard,
    logical_size: XY<f32>,
}

impl Key {
    // TODO: (?_?)
    fn update(&mut self, next: ElementState) {
        self.previous = self.current;
        self.current = next;
    }
}

impl KeyState {
    fn update_by_event(&mut self, next_key_element_state: &ElementState) {
        match next_key_element_state {
            ElementState::Pressed => {
                match &self {
                    KeyState::Pressed | KeyState::JustPressed => *self = KeyState::Pressed,
                    KeyState::Released | KeyState::JustReleased => *self = KeyState::JustPressed,
                }
            },
            ElementState::Released => {
                match &self {
                    KeyState::Pressed | KeyState::JustPressed => *self = KeyState::JustReleased,
                    KeyState::Released | KeyState::JustReleased => *self = KeyState::Released,
                }
            },
        }
    }

    fn clear(&mut self) {
        match &self {
            KeyState::JustPressed => *self = KeyState::Pressed,
            KeyState::JustReleased => *self = KeyState::Released,
            _ => (),
        }
    }
}



impl Mouse {
    fn new() -> Self {
        Self {
            left: KeyState::Released,
            right: KeyState::Released,
            wheel: WheelState::None,
            position: XY::default(),
            raw_move: XY::default(),
        }
    }

    fn clear(&mut self) {
        self.left.clear();
        self.right.clear();
    }
}

impl KeyBoard {
    fn new() -> Self {
        Self {
            hash_map: HashMap::new(),
        }
    }

    pub fn get(&self, key_code: &VirtualKeyCode) -> &KeyState {
        self.hash_map
            .get(key_code)
            .unwrap_or(&KeyState::Released)
    }

    fn clear(&mut self) {
        self.hash_map
            .values_mut()
            .for_each(|key_state| {
                key_state.clear();
            });
    }
}

impl InputDevices {
    pub fn new(window: &Window) -> Self {
        let logical_size = window.get_inner_size().unwrap();
        Self {
            mouse: Mouse::new(),
            keyboard: KeyBoard::new(),
            logical_size: XY::new(logical_size.width as f32, logical_size.height as f32),
        }
    }

    pub fn event_update(&mut self, event: &Event) {
        match event {
            // Logical size update
            Event::WindowEvent { event: WindowEvent::Resized(logical_size), .. } => {
                self.logical_size = XY::new(logical_size.width as f32, logical_size.height as f32);
            }

            // Mouse update
            Event::WindowEvent { event: WindowEvent::MouseInput { button, state, ..}, ..} => {
                match button {
                    MouseButton::Left => self.mouse.left.update_by_event(state),
                    MouseButton::Right => self.mouse.right.update_by_event(state),
                    _ => (),
                }
            }
            Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                let position = XY::new(position.x as f32, position.y as f32);
                self.mouse.position = position.normalize(&self.logical_size);
            },
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta: (x, y) }, ..} => {
                self.mouse.raw_move = XY::new(*x as f32, *y as f32);
            },

            // Keyboard update
            Event::WindowEvent { event: WindowEvent::KeyboardInput { input: KeyboardInput {
                state,
                virtual_keycode: Some(keycode), ..
            }, .. }, .. } => {
                if let Some(key_state) = self.keyboard.hash_map.get_mut(keycode) {
                    key_state.update_by_event(state);
                } else {
                    let key_state = match state {
                        ElementState::Pressed => KeyState::JustPressed,
                        ElementState::Released => KeyState::JustReleased,
                    };
                    self.keyboard.hash_map.insert(*keycode, key_state);
                }
            }
            _ => (),
        }
    }

    pub fn clear(&mut self) {
        self.mouse.clear();
        self.keyboard.clear();
    }
}


