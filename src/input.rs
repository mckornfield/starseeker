use macroquad::prelude::*;

#[derive(Default, Clone)]
pub(crate) struct InputState {
    pub rotate_left: bool,
    pub rotate_right: bool,
    pub thrust: bool,
    pub brake: bool,
    pub stabilize: bool,
    pub interact: bool,
    pub fire_main: bool,
    pub fire_aux: bool,
    /// Edge-triggered: true only on the frame the button is first tapped.
    pub toggle_map: bool,
    pub toggle_inventory: bool,
    pub toggle_quests: bool,
}

impl InputState {
    pub fn from_keyboard() -> Self {
        Self {
            rotate_left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
            rotate_right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            thrust: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
            brake: is_key_down(KeyCode::Down) || is_key_down(KeyCode::S),
            stabilize: is_key_down(KeyCode::C),
            interact: is_key_pressed(KeyCode::E),
            fire_main: is_key_down(KeyCode::Space),
            fire_aux: is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::Z),
            toggle_map: is_key_pressed(KeyCode::M),
            toggle_inventory: is_key_pressed(KeyCode::I) || is_key_pressed(KeyCode::Tab),
            toggle_quests: is_key_pressed(KeyCode::Q),
        }
    }

    /// Merge two InputStates with OR — lets keyboard and touch coexist.
    pub fn merge(&self, other: &InputState) -> InputState {
        InputState {
            rotate_left: self.rotate_left || other.rotate_left,
            rotate_right: self.rotate_right || other.rotate_right,
            thrust: self.thrust || other.thrust,
            brake: self.brake || other.brake,
            stabilize: self.stabilize || other.stabilize,
            interact: self.interact || other.interact,
            fire_main: self.fire_main || other.fire_main,
            fire_aux: self.fire_aux || other.fire_aux,
            toggle_map: self.toggle_map || other.toggle_map,
            toggle_inventory: self.toggle_inventory || other.toggle_inventory,
            toggle_quests: self.toggle_quests || other.toggle_quests,
        }
    }
}
