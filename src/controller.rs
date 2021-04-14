use cgmath::{Vector2, Zero};
use winit::event::VirtualKeyCode;

pub struct Controller {
    pan_speed: f32,
    zoom_speed: f32,

    pan_up: bool,
    pan_down: bool,
    pan_left: bool,
    pan_right: bool,
    zoom_in: bool,
    zoom_out: bool,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            pan_speed: 300.0,
            zoom_speed: 2.0,

            pan_up: false,
            pan_down: false,
            pan_left: false,
            pan_right: false,
            zoom_in: false,
            zoom_out: false,
        }
    }

    pub fn handle_keyboard_input(&mut self, keycode: VirtualKeyCode, pressed: bool) {
        match keycode {
            VirtualKeyCode::Up => self.pan_up = pressed,
            VirtualKeyCode::Down => self.pan_down = pressed,
            VirtualKeyCode::Left => self.pan_left = pressed,
            VirtualKeyCode::Right => self.pan_right = pressed,
            VirtualKeyCode::PageUp => self.zoom_in = pressed,
            VirtualKeyCode::PageDown => self.zoom_out = pressed,
            _ => {}
        };
    }

    pub fn camera_pan(&self) -> Vector2<f32> {
        let mut acc = Vector2::zero();
        if self.pan_up {
            acc += Vector2::unit_y();
        }
        if self.pan_down {
            acc -= Vector2::unit_y();
        }
        if self.pan_left {
            acc -= Vector2::unit_x();
        }
        if self.pan_right {
            acc += Vector2::unit_x();
        }
        acc * self.pan_speed
    }

    pub fn camera_zoom(&self) -> f32 {
        let mut acc = 1.0;
        if self.zoom_in {
            acc *= self.zoom_speed;
        }
        if self.zoom_out {
            acc /= self.zoom_speed;
        }
        acc
    }
}
