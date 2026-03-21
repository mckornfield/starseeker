use crate::items::{Item, ThrusterItem, WeaponItem};
use macroquad::prelude::*;

/// Maximum number of missions a player can have active at once.
const MAX_ACTIVE: usize = 3;

// ── Mission objective types ─────────────────────────────────────────────────

#[derive(Clone)]
pub(crate) enum Objective {
    /// Kill N enemies anywhere in space.
    KillEnemies { target: u32, killed: u32 },
    /// Accumulate N credits from pickups (not mission rewards).
    CollectCredits { target: u32, collected: u32 },
    /// Fly to a specific planet and land.
    VisitPlanet { planet_name: String, visited: bool },
}

impl Objective {
    pub fn progress_text(&self) -> String {
        match self {
            Objective::KillEnemies { target, killed } => {
                format!("Destroy hostiles: {}/{}", killed, target)
            }
            Objective::CollectCredits { target, collected } => {
                format!("Collect credits: {}/{}", collected, target)
            }
            Objective::VisitPlanet {
                planet_name,
                visited,
            } => {
                if *visited {
                    format!("Land on {}: DONE", planet_name)
                } else {
                    format!("Land on {}", planet_name)
                }
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        match self {
            Objective::KillEnemies { target, killed } => killed >= target,
            Objective::CollectCredits { target, collected } => collected >= target,
            Objective::VisitPlanet { visited, .. } => *visited,
        }
    }

    pub fn progress_frac(&self) -> f32 {
        match self {
            Objective::KillEnemies { target, killed } => *killed as f32 / *target as f32,
            Objective::CollectCredits { target, collected } => {
                *collected as f32 / *target as f32
            }
            Objective::VisitPlanet { visited, .. } => {
                if *visited {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

// ── Mission ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub(crate) struct Mission {
    pub title: String,
    pub briefing: String,
    pub objective: Objective,
    pub reward_credits: u32,
}

// ── Mission log (lives on Game) ─────────────────────────────────────────────

pub(crate) struct MissionLog {
    pub active: Vec<Mission>,
    pub completed_count: u32,
}

impl MissionLog {
    pub fn new() -> Self {
        Self {
            active: Vec::new(),
            completed_count: 0,
        }
    }

    pub fn can_accept(&self) -> bool {
        self.active.len() < MAX_ACTIVE
    }

    pub fn accept(&mut self, mission: Mission) -> bool {
        if self.active.len() >= MAX_ACTIVE {
            return false;
        }
        self.active.push(mission);
        true
    }

    /// Notify that one enemy was killed. Returns all missions that just completed.
    pub fn notify_kill(&mut self) -> Vec<String> {
        let mut msgs = Vec::new();
        for m in &mut self.active {
            if let Objective::KillEnemies {
                ref target,
                ref mut killed,
            } = m.objective
            {
                if *killed < *target {
                    *killed += 1;
                    if *killed >= *target {
                        msgs.push(format!("MISSION COMPLETE: {}", m.title));
                    }
                }
            }
        }
        msgs
    }

    /// Notify that credits were picked up. Returns all missions that just completed.
    pub fn notify_credits(&mut self, amount: u32) -> Vec<String> {
        let mut msgs = Vec::new();
        for m in &mut self.active {
            if let Objective::CollectCredits {
                ref target,
                ref mut collected,
            } = m.objective
            {
                if *collected < *target {
                    *collected += amount;
                    if *collected >= *target {
                        msgs.push(format!("MISSION COMPLETE: {}", m.title));
                    }
                }
            }
        }
        msgs
    }

    /// Notify that a planet was visited. Returns completion message if any mission finished.
    pub fn notify_visit(&mut self, planet_name: &str) -> Option<String> {
        let mut completed_msg = None;
        for m in &mut self.active {
            if let Objective::VisitPlanet {
                planet_name: ref target_name,
                ref mut visited,
            } = m.objective
            {
                if !*visited && target_name == planet_name {
                    *visited = true;
                    completed_msg = Some(format!("MISSION COMPLETE: {}", m.title));
                }
            }
        }
        completed_msg
    }

    /// Claim all completed missions. Returns total reward credits.
    pub fn claim_completed(&mut self) -> u32 {
        let mut total = 0;
        let mut claimed = 0u32;
        self.active.retain(|m| {
            if m.objective.is_complete() {
                total += m.reward_credits;
                claimed += 1;
                false
            } else {
                true
            }
        });
        self.completed_count += claimed;
        total
    }
}

// ── Planet menu state ───────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum MenuTab {
    Missions,
    Active,
    Shop,
}

pub(crate) struct PlanetMenu {
    pub name: String,
    pub available: Vec<Mission>,
    pub shop_stock: Vec<Item>,
    pub selected: usize,
    pub tab: MenuTab,
}

impl PlanetMenu {
    pub fn new(name: String, available: Vec<Mission>, shop_stock: Vec<Item>) -> Self {
        Self {
            name,
            available,
            shop_stock,
            selected: 0,
            tab: MenuTab::Missions,
        }
    }
}

// ── Mission generation ──────────────────────────────────────────────────────

const KILL_TITLES: &[&str] = &[
    "Bounty Hunt",
    "Clear the Sector",
    "Hostile Sweep",
    "Pirate Purge",
    "Zone Cleanup",
];

const KILL_BRIEFS: &[&str] = &[
    "Hostiles have been raiding supply lines. Take them out.",
    "A sector has been overrun. Eliminate the threat.",
    "Pirates are disrupting trade routes. Handle it.",
    "Wanted criminals spotted nearby. Bring them down.",
    "Too many hostiles in the area. Clear them out.",
];

const COLLECT_TITLES: &[&str] = &[
    "Salvage Run",
    "Scrap Collection",
    "Resource Acquisition",
    "Field Recovery",
    "Debris Harvest",
];

const COLLECT_BRIEFS: &[&str] = &[
    "We need materials. Collect credits from the field.",
    "Salvageable wreckage is scattered nearby. Recover what you can.",
    "The station needs resources. Gather credits from drops.",
    "There's valuable scrap out there. Go pick it up.",
    "We'll pay you to collect what's floating around.",
];

const VISIT_TITLES: &[&str] = &[
    "Courier Run",
    "Delivery Contract",
    "Diplomatic Envoy",
    "Supply Drop",
    "Recon Flyby",
];

const VISIT_BRIEFS: &[&str] = &[
    "Deliver a data package to the station at {}.",
    "A shipment needs to reach {}. Fly it there.",
    "We need eyes on {}. Fly there and report back.",
    "Drop off supplies at {}. Standard courier pay.",
    "Make contact with the station at {}.",
];

/// Generate 2-3 available missions for a given planet.
/// `nearby_planets` provides valid targets for visit missions.
pub(crate) fn gen_planet_missions(
    planet_name: &str,
    completed_count: u32,
    nearby_planets: &[String],
    active_titles: &[String],
) -> Vec<Mission> {
    // Simple seed from planet name + completion count so offerings feel stable
    // but refresh after completing missions
    let seed: u64 = planet_name
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
        .wrapping_add(completed_count as u64 * 0x9e3779b9);
    quad_rand::srand(seed);

    let reward_scale = 1.0 + completed_count as f32 * 0.1;
    let mut missions = Vec::new();

    // Always offer a kill mission
    let ki = quad_rand::gen_range(0_u32, KILL_TITLES.len() as u32) as usize;
    let kill_target = quad_rand::gen_range(3_u32, 10);
    let kill_reward = ((kill_target as f32 * 12.0) * reward_scale) as u32;
    let kill_mission = Mission {
        title: KILL_TITLES[ki].to_string(),
        briefing: KILL_BRIEFS[ki].to_string(),
        objective: Objective::KillEnemies {
            target: kill_target,
            killed: 0,
        },
        reward_credits: kill_reward,
    };
    if !active_titles.contains(&kill_mission.title) {
        missions.push(kill_mission);
    }

    // Always offer a collect mission
    let ci = quad_rand::gen_range(0_u32, COLLECT_TITLES.len() as u32) as usize;
    let collect_target = quad_rand::gen_range(50_u32, 250);
    let collect_reward = ((collect_target as f32 * 0.6) * reward_scale) as u32;
    let collect_mission = Mission {
        title: COLLECT_TITLES[ci].to_string(),
        briefing: COLLECT_BRIEFS[ci].to_string(),
        objective: Objective::CollectCredits {
            target: collect_target,
            collected: 0,
        },
        reward_credits: collect_reward,
    };
    if !active_titles.contains(&collect_mission.title) {
        missions.push(collect_mission);
    }

    // Offer a visit mission if there are nearby planets to visit
    let other_planets: Vec<&String> = nearby_planets
        .iter()
        .filter(|n| *n != planet_name)
        .collect();
    if !other_planets.is_empty() {
        let vi = quad_rand::gen_range(0_u32, VISIT_TITLES.len() as u32) as usize;
        let pi = quad_rand::gen_range(0_u32, other_planets.len() as u32) as usize;
        let dest = other_planets[pi].clone();
        let visit_reward = (80.0 * reward_scale) as u32;
        let briefing = VISIT_BRIEFS[vi].replace("{}", &dest);
        let visit_mission = Mission {
            title: format!("{}: {}", VISIT_TITLES[vi], dest),
            briefing,
            objective: Objective::VisitPlanet {
                planet_name: dest,
                visited: false,
            },
            reward_credits: visit_reward,
        };
        if !active_titles.contains(&visit_mission.title) {
            missions.push(visit_mission);
        }
    }

    missions
}

/// Generate procedural shop stock for a planet, seeded by planet name.
pub(crate) fn gen_shop_stock(planet_name: &str) -> Vec<Item> {
    // Use a distinct salt so shop seed differs from mission seed
    let seed: u64 = planet_name
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(37).wrapping_add(b as u64))
        .wrapping_add(0xdeadbeef_1337cafe);
    quad_rand::srand(seed);

    let count = quad_rand::gen_range(4_u32, 7);
    let mut items = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let roll = quad_rand::gen_range(0.0_f32, 1.0);
        if roll < 0.25 {
            items.push(Item::Thruster(ThrusterItem::gen()));
        } else if roll < 0.65 {
            items.push(Item::Weapon(WeaponItem::gen_main()));
        } else {
            items.push(Item::Weapon(WeaponItem::gen_aux()));
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_log_accept_and_limit() {
        let mut log = MissionLog::new();
        for i in 0..4 {
            let m = Mission {
                title: format!("Mission {}", i),
                briefing: String::new(),
                objective: Objective::KillEnemies {
                    target: 5,
                    killed: 0,
                },
                reward_credits: 50,
            };
            if i < MAX_ACTIVE {
                assert!(log.accept(m));
            } else {
                assert!(!log.accept(m));
            }
        }
        assert_eq!(log.active.len(), MAX_ACTIVE);
    }

    #[test]
    fn kill_tracking() {
        let mut log = MissionLog::new();
        log.accept(Mission {
            title: "Test".into(),
            briefing: String::new(),
            objective: Objective::KillEnemies {
                target: 2,
                killed: 0,
            },
            reward_credits: 50,
        });
        assert!(log.notify_kill().is_empty());
        assert!(!log.notify_kill().is_empty());
    }

    #[test]
    fn credit_tracking() {
        let mut log = MissionLog::new();
        log.accept(Mission {
            title: "Test".into(),
            briefing: String::new(),
            objective: Objective::CollectCredits {
                target: 100,
                collected: 0,
            },
            reward_credits: 60,
        });
        assert!(log.notify_credits(50).is_empty());
        assert!(!log.notify_credits(50).is_empty());
    }

    #[test]
    fn visit_tracking() {
        let mut log = MissionLog::new();
        log.accept(Mission {
            title: "Test".into(),
            briefing: String::new(),
            objective: Objective::VisitPlanet {
                planet_name: "Veltar".into(),
                visited: false,
            },
            reward_credits: 80,
        });
        assert!(log.notify_visit("Other").is_none());
        assert!(log.notify_visit("Veltar").is_some());
    }

    #[test]
    fn claim_completed() {
        let mut log = MissionLog::new();
        log.accept(Mission {
            title: "Done".into(),
            briefing: String::new(),
            objective: Objective::KillEnemies {
                target: 1,
                killed: 1,
            },
            reward_credits: 100,
        });
        log.accept(Mission {
            title: "Not Done".into(),
            briefing: String::new(),
            objective: Objective::KillEnemies {
                target: 5,
                killed: 0,
            },
            reward_credits: 50,
        });
        let reward = log.claim_completed();
        assert_eq!(reward, 100);
        assert_eq!(log.active.len(), 1);
    }

    #[test]
    fn gen_produces_missions() {
        let missions = gen_planet_missions("TestPlanet", 0, &["OtherPlanet".into()], &[]);
        assert!(missions.len() >= 2);
        assert!(missions.len() <= 3);
    }

    #[test]
    fn gen_shop_stock_produces_items() {
        let stock = gen_shop_stock("TestPlanet");
        assert!(stock.len() >= 4);
        assert!(stock.len() <= 6);
    }

}
