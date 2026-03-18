use crate::input::InputState;
use macroquad::prelude::*;

struct Button {
    center: Vec2,
    radius: f32,
    label: &'static str,
    pressed: bool,
}

impl Button {
    fn new(label: &'static str, radius: f32) -> Self {
        Self {
            center: Vec2::ZERO,
            radius,
            label,
            pressed: false,
        }
    }

    fn hit_test(&self, pos: Vec2) -> bool {
        self.center.distance(pos) < self.radius * 1.3
    }

    fn draw(&self) {
        let alpha = if self.pressed { 0.65 } else { 0.22 };
        let fill = if self.pressed {
            Color::new(0.35, 0.65, 1.0, alpha)
        } else {
            Color::new(0.4, 0.4, 0.5, alpha)
        };
        draw_circle(self.center.x, self.center.y, self.radius, fill);
        draw_circle_lines(
            self.center.x,
            self.center.y,
            self.radius,
            1.5,
            Color::new(1.0, 1.0, 1.0, 0.35),
        );
        let fs = if self.label.len() > 2 {
            13.0_f32
        } else {
            18.0_f32
        };
        let tw = measure_text(self.label, None, fs as u16, 1.0).width;
        draw_text(
            self.label,
            self.center.x - tw * 0.5,
            self.center.y + fs * 0.38,
            fs,
            Color::new(1.0, 1.0, 1.0, 0.85),
        );
    }
}

/// On-screen touch overlay.  Buttons are re-laid-out every frame so they
/// adapt to screen rotation / resize.
pub(crate) struct MobileOverlay {
    // 0=thrust 1=brake 2=rot_left 3=rot_right 4=fire_main 5=fire_aux
    buttons: [Button; 6],
}

impl MobileOverlay {
    pub fn new() -> Self {
        Self {
            buttons: [
                Button::new("UP", 36.0),
                Button::new("DN", 36.0),
                Button::new("<", 36.0),
                Button::new(">", 36.0),
                Button::new("FIRE", 46.0),
                Button::new("AUX", 32.0),
            ],
        }
    }

    fn layout(&mut self) {
        let sw = screen_width();
        let sh = screen_height();
        let gap = 84.0_f32;
        let pad = 60.0_f32;

        // Left d-pad cluster
        let lx = pad + gap * 0.5;
        let ly = sh - pad - gap * 0.5;
        self.buttons[0].center = vec2(lx, ly - gap); // thrust ▲
        self.buttons[1].center = vec2(lx, ly + gap * 0.65); // brake  ▼
        self.buttons[2].center = vec2(lx - gap, ly); // left   ◄
        self.buttons[3].center = vec2(lx + gap, ly); // right  ►

        // Right weapon cluster
        let rx = sw - pad - gap * 0.3;
        let ry = sh - pad - gap * 0.5;
        self.buttons[4].center = vec2(rx, ry); // FIRE (main)
        self.buttons[5].center = vec2(rx - gap * 1.1, ry - gap * 0.6); // AUX
    }

    /// Collect touches, update pressed states, return merged touch InputState.
    pub fn update(&mut self) -> InputState {
        self.layout();

        for b in &mut self.buttons {
            b.pressed = false;
        }

        for touch in touches() {
            if matches!(
                touch.phase,
                TouchPhase::Started | TouchPhase::Stationary | TouchPhase::Moved
            ) {
                let pos = touch.position;
                for b in &mut self.buttons {
                    if b.hit_test(pos) {
                        b.pressed = true;
                    }
                }
            }
        }

        InputState {
            thrust: self.buttons[0].pressed,
            brake: self.buttons[1].pressed,
            rotate_left: self.buttons[2].pressed,
            rotate_right: self.buttons[3].pressed,
            fire_main: self.buttons[4].pressed,
            fire_aux: self.buttons[5].pressed,
        }
    }

    pub fn draw(&self) {
        for b in &self.buttons {
            b.draw();
        }
    }
}
