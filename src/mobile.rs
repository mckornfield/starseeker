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

    fn draw(&self, dimmed: bool) {
        let alpha = if dimmed {
            0.10
        } else if self.pressed {
            0.65
        } else {
            0.22
        };
        let fill = if self.pressed && !dimmed {
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
            Color::new(1.0, 1.0, 1.0, if dimmed { 0.12 } else { 0.35 }),
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
            Color::new(1.0, 1.0, 1.0, if dimmed { 0.25 } else { 0.85 }),
        );
    }
}

/// On-screen touch overlay.  Buttons are re-laid-out every frame so they
/// adapt to screen rotation / resize.
pub(crate) struct MobileOverlay {
    // 0=thrust 1=brake 2=rot_left 3=rot_right 4=fire_main 5=fire_aux 6=stabilize 7=interact
    buttons: [Button; 8],
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
                Button::new("STAB", 30.0),
                Button::new("E", 30.0),
            ],
        }
    }

    fn layout(&mut self) {
        let sw = screen_width();
        let sh = screen_height();
        let gap = 84.0_f32;
        let pad = 110.0_f32;

        // Left d-pad cluster
        let lx = pad + gap * 0.5;
        let ly = sh - pad - gap * 0.5;
        self.buttons[0].center = vec2(lx, ly - gap);           // thrust ▲
        self.buttons[1].center = vec2(lx, ly + gap * 0.65);    // brake  ▼
        self.buttons[2].center = vec2(lx - gap, ly);           // left   ◄
        self.buttons[3].center = vec2(lx + gap, ly);           // right  ►
        self.buttons[6].center = vec2(lx, ly - gap * 0.25);    // STAB   slightly above d-pad center

        // Right weapon cluster
        let rx = sw - pad - gap * 0.3;
        let ry = sh - pad - gap * 0.5;
        self.buttons[4].center = vec2(rx, ry);                        // FIRE (main)
        self.buttons[5].center = vec2(rx - gap * 1.1, ry - gap * 0.6); // AUX
        self.buttons[7].center = vec2(rx, ry - gap * 1.1);            // E (interact)
    }

    /// Collect touches, update pressed states, return merged touch InputState.
    /// `near_planet` gates the interact button so it only registers when relevant.
    pub fn update(&mut self, near_planet: bool) -> InputState {
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
                for (i, b) in self.buttons.iter_mut().enumerate() {
                    // Only register the interact button when near a planet
                    if i == 7 && !near_planet {
                        continue;
                    }
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
            stabilize: self.buttons[6].pressed,
            interact: self.buttons[7].pressed,
        }
    }

    pub fn draw(&self, near_planet: bool) {
        for (i, b) in self.buttons.iter().enumerate() {
            let dimmed = i == 7 && !near_planet;
            b.draw(dimmed);
        }
    }
}
