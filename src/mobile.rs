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
    // Hold buttons  0=thrust 1=brake 2=rot_left 3=rot_right 4=fire_main 5=fire_aux 6=stabilize 7=interact
    // Tap buttons   8=toggle_map 9=toggle_inventory 10=toggle_quests
    buttons: [Button; 11],
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
                Button::new("MAP", 24.0),
                Button::new("INV", 24.0),
                Button::new("QST", 24.0),
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
        self.buttons[6].center = vec2(lx, ly - gap * 0.15);    // STAB (slightly above center)

        // Right weapon cluster
        let rx = sw - pad - gap * 0.3;
        let ry = sh - pad - gap * 0.5;
        self.buttons[4].center = vec2(rx, ry);                          // FIRE (main)
        self.buttons[5].center = vec2(rx - gap * 1.1, ry - gap * 0.6); // AUX
        self.buttons[7].center = vec2(rx, ry - gap * 1.1);             // E (interact)

        // Top-right tap buttons: MAP  INV  QST
        let tr_y = 40.0;
        let spacing = 58.0_f32;
        self.buttons[8].center  = vec2(sw - spacing * 3.0, tr_y); // MAP
        self.buttons[9].center  = vec2(sw - spacing * 2.0, tr_y); // INV
        self.buttons[10].center = vec2(sw - spacing * 1.0, tr_y); // QST
    }

    /// Collect touches, update pressed states, return merged touch InputState.
    /// `near_planet` gates the interact button so it only registers when relevant.
    pub fn update(&mut self, near_planet: bool) -> InputState {
        self.layout();

        for b in &mut self.buttons {
            b.pressed = false;
        }

        // Hold buttons (0-7): active while finger is down
        for touch in touches() {
            if matches!(
                touch.phase,
                TouchPhase::Started | TouchPhase::Stationary | TouchPhase::Moved
            ) {
                let pos = touch.position;
                for (i, b) in self.buttons[..8].iter_mut().enumerate() {
                    if i == 7 && !near_planet {
                        continue;
                    }
                    if b.hit_test(pos) {
                        b.pressed = true;
                    }
                }
            }
        }

        // Tap buttons (8-10): edge-triggered — only fire on TouchPhase::Started
        for touch in touches() {
            if touch.phase == TouchPhase::Started {
                let pos = touch.position;
                for b in &mut self.buttons[8..] {
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
            toggle_map: self.buttons[8].pressed,
            toggle_inventory: self.buttons[9].pressed,
            toggle_quests: self.buttons[10].pressed,
        }
    }

    pub fn draw(&self, near_planet: bool) {
        for (i, b) in self.buttons.iter().enumerate() {
            let dimmed = i == 7 && !near_planet;
            b.draw(dimmed);
        }
    }
}
