use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Default)]
pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    mouse_delta: (f32, f32)
}

impl InputState {
    pub fn key(&mut self, key: PhysicalKey, state: ElementState) {
        let pressed = state == ElementState::Pressed;
        let PhysicalKey::Code(key) = key else { return; };

        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => self.forward = pressed,
            KeyCode::KeyS | KeyCode::ArrowDown => self.backward = pressed,
            KeyCode::KeyA | KeyCode::ArrowLeft => self.left = pressed,
            KeyCode::KeyD | KeyCode::ArrowRight => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::ShiftLeft => self.down = pressed,
            _ => {},
        }
    }

    pub fn add_mouse_delta(&mut self, dx: f64, dy: f64) {
        self.mouse_delta.0 += dx as f32;
        self.mouse_delta.1 += dy as f32;
    }

    pub fn take_mouse_delta(&mut self) -> (f32, f32) {
        std::mem::take(&mut self.mouse_delta)
    }
}