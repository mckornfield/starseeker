use super::{Rarity, ThrusterItem, WeaponItem, WeaponSlot};

pub(crate) struct Loadout {
    pub main: Option<WeaponItem>,
    pub aux: Option<WeaponItem>,
    pub thruster: Option<ThrusterItem>,
}

impl Loadout {
    pub fn starter() -> Self {
        Self {
            main: Some(WeaponItem::default_main()),
            aux: Some(WeaponItem::default_aux()),
            thruster: None,
        }
    }

    /// Equip weapon if slot is empty or new item has equal/higher rarity.
    /// Returns (name, rarity) of the equipped item if it was accepted.
    #[must_use]
    pub fn try_equip_weapon(&mut self, w: WeaponItem) -> Option<(String, Rarity)> {
        let slot = match w.slot {
            WeaponSlot::Main => &mut self.main,
            WeaponSlot::Aux => &mut self.aux,
        };
        let should_equip = slot.as_ref().map(|e| w.rarity >= e.rarity).unwrap_or(true);
        if should_equip {
            let info = (w.name.clone(), w.rarity);
            *slot = Some(w);
            Some(info)
        } else {
            None
        }
    }

    /// Equip thruster if slot is empty or new item has equal/higher rarity.
    /// Returns the rarity if accepted.
    #[must_use]
    pub fn try_equip_thruster(&mut self, t: ThrusterItem) -> Option<Rarity> {
        let should_equip = self
            .thruster
            .as_ref()
            .map(|e| t.rarity >= e.rarity)
            .unwrap_or(true);
        if should_equip {
            let r = t.rarity;
            self.thruster = Some(t);
            Some(r)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::*;

    fn make_weapon(slot: WeaponSlot, rarity: Rarity) -> WeaponItem {
        WeaponItem {
            name: format!("Test {:?}", rarity),
            rarity,
            slot,
            damage: 10.0,
            fire_rate: 0.2,
            proj_speed: 500.0,
            proj_color: WHITE,
            spread: false,
        }
    }

    fn make_thruster(rarity: Rarity) -> ThrusterItem {
        ThrusterItem {
            rarity,
            speed_mult: 1.0,
            accel_mult: 1.0,
        }
    }

    #[test]
    fn starter_loadout_has_main_and_aux() {
        let l = Loadout::starter();
        assert!(l.main.is_some());
        assert!(l.aux.is_some());
        assert!(l.thruster.is_none());
    }

    #[test]
    fn equip_weapon_into_empty_slot() {
        let mut l = Loadout {
            main: None,
            aux: None,
            thruster: None,
        };
        let result = l.try_equip_weapon(make_weapon(WeaponSlot::Main, Rarity::Common));
        assert!(result.is_some());
        assert!(l.main.is_some());
    }

    #[test]
    fn equip_weapon_upgrades_on_higher_rarity() {
        let mut l = Loadout {
            main: Some(make_weapon(WeaponSlot::Main, Rarity::Common)),
            aux: None,
            thruster: None,
        };
        let result = l.try_equip_weapon(make_weapon(WeaponSlot::Main, Rarity::Rare));
        assert!(result.is_some());
        assert_eq!(l.main.as_ref().unwrap().rarity, Rarity::Rare);
    }

    #[test]
    fn equip_weapon_rejects_lower_rarity() {
        let mut l = Loadout {
            main: Some(make_weapon(WeaponSlot::Main, Rarity::Rare)),
            aux: None,
            thruster: None,
        };
        let result = l.try_equip_weapon(make_weapon(WeaponSlot::Main, Rarity::Common));
        assert!(result.is_none());
        assert_eq!(l.main.as_ref().unwrap().rarity, Rarity::Rare);
    }

    #[test]
    fn equip_weapon_accepts_equal_rarity() {
        let mut l = Loadout {
            main: Some(make_weapon(WeaponSlot::Main, Rarity::Uncommon)),
            aux: None,
            thruster: None,
        };
        let result = l.try_equip_weapon(make_weapon(WeaponSlot::Main, Rarity::Uncommon));
        assert!(result.is_some());
    }

    #[test]
    fn equip_thruster_into_empty_slot() {
        let mut l = Loadout {
            main: None,
            aux: None,
            thruster: None,
        };
        let result = l.try_equip_thruster(make_thruster(Rarity::Common));
        assert!(result.is_some());
        assert!(l.thruster.is_some());
    }

    #[test]
    fn equip_thruster_rejects_lower_rarity() {
        let mut l = Loadout {
            main: None,
            aux: None,
            thruster: Some(make_thruster(Rarity::Epic)),
        };
        let result = l.try_equip_thruster(make_thruster(Rarity::Common));
        assert!(result.is_none());
        assert_eq!(l.thruster.as_ref().unwrap().rarity, Rarity::Epic);
    }

    #[test]
    fn weapon_goes_to_correct_slot() {
        let mut l = Loadout {
            main: None,
            aux: None,
            thruster: None,
        };
        let _ = l.try_equip_weapon(make_weapon(WeaponSlot::Aux, Rarity::Rare));
        assert!(l.main.is_none());
        assert!(l.aux.is_some());
        assert_eq!(l.aux.as_ref().unwrap().rarity, Rarity::Rare);
    }
}
