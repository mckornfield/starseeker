pub(crate) mod loadout;
pub(crate) mod rarity;
pub(crate) mod thruster;
pub(crate) mod weapon;

pub(crate) use loadout::Loadout;
pub(crate) use rarity::Rarity;
pub(crate) use thruster::ThrusterItem;
pub(crate) use weapon::{WeaponItem, WeaponSlot};

#[derive(Clone)]
pub(crate) enum Item {
    Weapon(WeaponItem),
    Thruster(ThrusterItem),
}

impl Item {
    pub fn rarity(&self) -> Rarity {
        match self {
            Item::Weapon(w) => w.rarity,
            Item::Thruster(t) => t.rarity,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Item::Weapon(w) => w.name.clone(),
            Item::Thruster(_) => "THRUSTER".to_string(),
        }
    }

    pub fn slot_label(&self) -> &'static str {
        match self {
            Item::Weapon(w) => match w.slot {
                WeaponSlot::Main => "MAIN",
                WeaponSlot::Aux => "AUX",
            },
            Item::Thruster(_) => "THR",
        }
    }

    pub fn stat_summary(&self) -> String {
        match self {
            Item::Weapon(w) => w.stat_summary(),
            Item::Thruster(t) => t.stat_summary(),
        }
    }

    pub fn buy_price(&self) -> u32 {
        self.rarity().base_price()
    }

    pub fn sell_price(&self) -> u32 {
        self.buy_price() / 2
    }
}
