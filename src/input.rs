use macroquad::prelude::*;

#[derive(Default, Clone)]
pub struct InputState {
    pub rotate_left: bool,
    pub rotate_right: bool,
    pub thrust: bool,
    pub brake: bool,
    pub fire_main: bool,
    pub fire_aux: bool,
}

impl InputState {
    pub fn from_keyboard() -> Self {
        Self {
            rotate_left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
            rotate_right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            thrust: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
            brake: is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
            fire_main: is_key_down(KeyCode::Space),
            fire_aux: is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::Z),
        }
    }

    /// Merge two InputStates with OR — lets keyboard and touch coexist.
    pub fn merge(&self, other: &InputState) -> InputState {
        InputState {
            rotate_left: self.rotate_left || other.rotate_left,
            rotate_right: self.rotate_right || other.rotate_right,
            thrust: self.thrust || other.thrust,
            brake: self.brake || other.brake,
            fire_main: self.fire_main || other.fire_main,
            fire_aux: self.fire_aux || other.fire_aux,
        }
    }
}
