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
    /// On success: returns Ok((name, rarity, displaced_weapon)).
    /// On failure: returns Err(weapon) so it can be stashed.
    #[must_use]
    pub fn try_equip_weapon(
        &mut self,
        w: WeaponItem,
    ) -> Result<(String, Rarity, Option<WeaponItem>), WeaponItem> {
        let slot = match w.slot {
            WeaponSlot::Main => &mut self.main,
            WeaponSlot::Aux => &mut self.aux,
        };
        let should_equip = slot.as_ref().map(|e| w.rarity >= e.rarity).unwrap_or(true);
        if should_equip {
            let info = (w.name.clone(), w.rarity);
            let old = slot.take();
            *slot = Some(w);
            Ok((info.0, info.1, old))
        } else {
            Err(w)
        }
    }

    /// Equip thruster if slot is empty or new item has equal/higher rarity.
    /// On success: returns Ok((rarity, displaced_thruster)).
    /// On failure: returns Err(thruster) so it can be stashed.
    #[must_use]
    pub fn try_equip_thruster(
        &mut self,
        t: ThrusterItem,
    ) -> Result<(Rarity, Option<ThrusterItem>), ThrusterItem> {
        let should_equip = self
            .thruster
            .as_ref()
            .map(|e| t.rarity >= e.rarity)
            .unwrap_or(true);
        if should_equip {
            let r = t.rarity;
            let old = self.thruster.take();
            self.thruster = Some(t);
            Ok((r, old))
        } else {
            Err(t)
        }
    }

    /// Force-equip a weapon, returning the displaced item (if any).
    pub fn force_equip_weapon(&mut self, w: WeaponItem) -> Option<WeaponItem> {
        let slot = match w.slot {
            WeaponSlot::Main => &mut self.main,
            WeaponSlot::Aux => &mut self.aux,
        };
        let old = slot.take();
        *slot = Some(w);
        old
    }

    /// Force-equip a thruster, returning the displaced item (if any).
    pub fn force_equip_thruster(&mut self, t: ThrusterItem) -> Option<ThrusterItem> {
        let old = self.thruster.take();
        self.thruster = Some(t);
        old
    }

    /// Remove equipped weapon from a slot, returning it.
    pub fn unequip_weapon(&mut self, slot: WeaponSlot) -> Option<WeaponItem> {
        match slot {
            WeaponSlot::Main => self.main.take(),
            WeaponSlot::Aux => self.aux.take(),
        }
    }

    /// Remove equipped thruster, returning it.
    pub fn unequip_thruster(&mut self) -> Option<ThrusterItem> {
        self.thruster.take()
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
        assert!(result.is_ok());
        let (_, _, old) = result.unwrap();
        assert!(old.is_none());
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
        assert!(result.is_ok());
        let (_, _, old) = result.unwrap();
        assert!(old.is_some());
        assert_eq!(old.unwrap().rarity, Rarity::Common);
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
        assert!(result.is_err());
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
        assert!(result.is_ok());
    }

    #[test]
    fn equip_thruster_into_empty_slot() {
        let mut l = Loadout {
            main: None,
            aux: None,
            thruster: None,
        };
        let result = l.try_equip_thruster(make_thruster(Rarity::Common));
        assert!(result.is_ok());
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
        assert!(result.is_err());
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

    #[test]
    fn force_equip_returns_old_item() {
        let mut l = Loadout {
            main: Some(make_weapon(WeaponSlot::Main, Rarity::Epic)),
            aux: None,
            thruster: None,
        };
        let old = l.force_equip_weapon(make_weapon(WeaponSlot::Main, Rarity::Common));
        assert!(old.is_some());
        assert_eq!(old.unwrap().rarity, Rarity::Epic);
        assert_eq!(l.main.as_ref().unwrap().rarity, Rarity::Common);
    }

    #[test]
    fn unequip_weapon_returns_item() {
        let mut l = Loadout::starter();
        let w = l.unequip_weapon(WeaponSlot::Main);
        assert!(w.is_some());
        assert!(l.main.is_none());
    }
}
