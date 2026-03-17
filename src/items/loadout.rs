use super::{Rarity, ThrusterItem, WeaponItem, WeaponSlot};

pub struct Loadout {
    pub main:     Option<WeaponItem>,
    pub aux:      Option<WeaponItem>,
    pub thruster: Option<ThrusterItem>,
}

impl Loadout {
    pub fn starter() -> Self {
        Self {
            main:     Some(WeaponItem::default_main()),
            aux:      Some(WeaponItem::default_aux()),
            thruster: None,
        }
    }

    /// Equip weapon if slot is empty or new item has equal/higher rarity.
    /// Returns (name, rarity) of the equipped item if it was accepted.
    pub fn try_equip_weapon(&mut self, w: WeaponItem) -> Option<(String, Rarity)> {
        let slot = match w.slot {
            WeaponSlot::Main => &mut self.main,
            WeaponSlot::Aux  => &mut self.aux,
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
    pub fn try_equip_thruster(&mut self, t: ThrusterItem) -> Option<Rarity> {
        let should_equip = self.thruster.as_ref().map(|e| t.rarity >= e.rarity).unwrap_or(true);
        if should_equip {
            let r = t.rarity;
            self.thruster = Some(t);
            Some(r)
        } else {
            None
        }
    }
}
