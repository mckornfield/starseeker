use crate::entities::enemy::Enemy;
use crate::entities::loot::{LootDrop, LootKind, PICKUP_RANGE};
use crate::input::InputState;
use crate::items::{Item, Rarity, ThrusterItem, WeaponItem, WeaponSlot};
use crate::missions::{self, MenuTab, MissionLog, PlanetMenu};
use crate::mobile::MobileOverlay;
use crate::player::Player;
use crate::projectile::{Owner, Projectile};
use crate::world::World;
use macroquad::prelude::*;

/// Slot label, optional (rarity, name, stat summary) for loadout display
type LoadoutSlotDisplay<'a> = (&'a str, Option<(&'a Rarity, String, String)>);

const PLAYER_RADIUS: f32 = 12.0;
const PLAYER_MAX_HEALTH: f32 = 100.0;
/// Invincibility window after an asteroid hit so one collision doesn't insta-kill
const ASTEROID_HIT_IFRAMES: f32 = 0.5;
/// Cull enemies that wander farther than this from the player
const ENEMY_CULL_DIST: f32 = 7000.0;
/// Duration of the red damage-flash overlay
const DAMAGE_FLASH_DURATION: f32 = 0.25;
/// Duration of the death screen before respawn is available
const DEATH_SCREEN_DELAY: f32 = 1.5;
/// Maximum cargo hold capacity
const MAX_CARGO: usize = 40;

pub(crate) struct Game {
    player: Player,
    player_health: f32,
    asteroid_iframes: f32,

    projectiles: Vec<Projectile>,
    enemies: Vec<Enemy>,
    loot_drops: Vec<LootDrop>,
    credits: u32,
    cargo: Vec<Item>,

    /// Brief pickup notification: (message, rarity color, seconds remaining)
    pickup_notice: Option<(String, Color, f32)>,

    /// Red flash timer when taking damage (counts down from DAMAGE_FLASH_DURATION)
    damage_flash: f32,
    /// true when player is dead; timer counts down before respawn is allowed
    dead: bool,
    death_timer: f32,

    mission_log: MissionLog,

    camera: Camera2D,
    world: World,
    mobile: MobileOverlay,

    planet_menu: Option<PlanetMenu>,
    show_map: bool,
    show_inventory: bool,
    inv_cursor: usize,
    prev_interact: bool,
}

impl Game {
    pub fn new() -> Self {
        Self {
            player: Player::new(Vec2::ZERO),
            player_health: PLAYER_MAX_HEALTH,
            asteroid_iframes: 0.0,

            projectiles: Vec::new(),
            enemies: Vec::new(),
            loot_drops: Vec::new(),
            credits: 0,
            cargo: Vec::new(),

            pickup_notice: None,

            damage_flash: 0.0,
            dead: false,
            death_timer: 0.0,

            mission_log: MissionLog::new(),

            camera: Camera2D {
                zoom: vec2(1.0 / 640.0, 1.0 / 360.0),
                ..Default::default()
            },
            world: World::new(),
            mobile: MobileOverlay::new(),

            planet_menu: None,
            show_map: false,
            show_inventory: false,
            inv_cursor: 0,
            prev_interact: false,
        }
    }

    pub fn update(&mut self) {
        let dt = get_frame_time();

        // ── Damage flash countdown ───────────────────────────────────────────
        self.damage_flash = (self.damage_flash - dt).max(0.0);

        // ── Death screen ─────────────────────────────────────────────────────
        if self.dead {
            self.death_timer = (self.death_timer - dt).max(0.0);
            let kb = InputState::from_keyboard();
            let touch = self.mobile.update(false);
            let input = kb.merge(&touch);
            if self.death_timer <= 0.0 && (input.interact || input.fire_main) {
                self.respawn();
            }
            let aspect = screen_width() / screen_height();
            self.camera.target = self.player.pos;
            self.camera.zoom = vec2(1.0 / (360.0 * aspect), 1.0 / 360.0);
            return;
        }

        // ── Input ─────────────────────────────────────────────────────────────
        let near_planet = self.world.nearby_planet_name(self.player.pos).is_some()
            || self.planet_menu.is_some();
        let kb = InputState::from_keyboard();
        let touch = self.mobile.update(near_planet);
        let input = kb.merge(&touch);

        // ── Inventory toggle (I or Tab) ─────────────────────────────────────
        if (is_key_pressed(KeyCode::I) || is_key_pressed(KeyCode::Tab))
            && self.planet_menu.is_none() {
                self.show_inventory = !self.show_inventory;
                if self.show_inventory {
                    self.inv_cursor = 0;
                }
            }

        // ── Inventory input & pause ─────────────────────────────────────────
        if self.show_inventory {
            self.update_inventory();
            let aspect = screen_width() / screen_height();
            self.camera.target = self.player.pos;
            self.camera.zoom = vec2(1.0 / (360.0 * aspect), 1.0 / 360.0);
            return;
        }

        // ── Planet menu toggle (edge-detected) ────────────────────────────────
        let interact_just = input.interact && !self.prev_interact;
        self.prev_interact = input.interact;
        if interact_just {
            if self.planet_menu.is_some() {
                self.planet_menu = None;
            } else if let Some(name) = self.world.nearby_planet_name(self.player.pos) {
                // Notify visit-mission tracking
                if let Some(msg) = self.mission_log.notify_visit(name) {
                    self.pickup_notice = Some((msg, GOLD, 3.0));
                }
                let nearby = self.world.known_planet_names();
                let active_titles: Vec<String> =
                    self.mission_log.active.iter().map(|m| m.title.clone()).collect();
                let available = missions::gen_planet_missions(
                    name,
                    self.mission_log.completed_count,
                    &nearby,
                    &active_titles,
                );
                let shop_stock = missions::gen_shop_stock(name);
                self.planet_menu = Some(PlanetMenu::new(name.to_string(), available, shop_stock));
            }
        }

        // ── Planet menu input & pause ────────────────────────────────────────
        if let Some(ref mut menu) = self.planet_menu {
            // Tab switching: 1 = Missions, 2 = Active, 3 = Shop
            if is_key_pressed(KeyCode::Key1) {
                menu.tab = MenuTab::Missions;
                menu.selected = 0;
            }
            if is_key_pressed(KeyCode::Key2) {
                menu.tab = MenuTab::Active;
                menu.selected = 0;
            }
            if is_key_pressed(KeyCode::Key3) {
                menu.tab = MenuTab::Shop;
                menu.selected = 0;
            }

            // Selection movement
            let list_len = match menu.tab {
                MenuTab::Missions => menu.available.len(),
                MenuTab::Active => self.mission_log.active.len(),
                MenuTab::Shop => menu.shop_stock.len() + self.cargo.len(),
            };
            if (is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W))
                && menu.selected > 0 {
                    menu.selected -= 1;
                }
            if (is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S))
                && menu.selected + 1 < list_len {
                    menu.selected += 1;
                }

            // Accept mission / Claim reward / Buy / Sell with Space
            if is_key_pressed(KeyCode::Space) {
                match menu.tab {
                    MenuTab::Missions => {
                        if menu.selected < menu.available.len() && self.mission_log.can_accept() {
                            let mission = menu.available.remove(menu.selected);
                            let msg = format!("ACCEPTED: {}", mission.title);
                            self.mission_log.accept(mission);
                            self.pickup_notice = Some((msg, SKYBLUE, 2.5));
                            if menu.selected > 0 && menu.selected >= menu.available.len() {
                                menu.selected -= 1;
                            }
                        }
                    }
                    MenuTab::Active => {
                        // Claim all completed missions
                        let reward = self.mission_log.claim_completed();
                        if reward > 0 {
                            self.credits += reward;
                            self.pickup_notice =
                                Some((format!("REWARD: +{} CR", reward), GOLD, 3.0));
                        }
                    }
                    MenuTab::Shop => {
                        let shop_len = menu.shop_stock.len();
                        if menu.selected < shop_len {
                            // Buy from shop
                            let price = menu.shop_stock[menu.selected].buy_price();
                            if self.credits >= price && self.cargo.len() < MAX_CARGO {
                                let item = menu.shop_stock.remove(menu.selected);
                                let msg = format!(
                                    "BOUGHT {} {} (-{} CR)",
                                    item.rarity().label(),
                                    item.name(),
                                    price
                                );
                                self.credits -= price;
                                self.cargo.push(item);
                                self.pickup_notice = Some((msg, SKYBLUE, 2.5));
                                if menu.selected > 0 && menu.selected >= menu.shop_stock.len() {
                                    menu.selected -= 1;
                                }
                            }
                        } else {
                            // Sell from cargo
                            let cargo_idx = menu.selected - shop_len;
                            if cargo_idx < self.cargo.len() {
                                let item = self.cargo.remove(cargo_idx);
                                let price = item.sell_price();
                                let msg = format!(
                                    "SOLD {} {} (+{} CR)",
                                    item.rarity().label(),
                                    item.name(),
                                    price
                                );
                                self.credits += price;
                                self.pickup_notice = Some((msg, GOLD, 2.5));
                                let new_total = menu.shop_stock.len() + self.cargo.len();
                                if menu.selected > 0 && menu.selected >= new_total {
                                    menu.selected -= 1;
                                }
                            }
                        }
                    }
                }
            }

            let aspect = screen_width() / screen_height();
            self.camera.target = self.player.pos;
            self.camera.zoom = vec2(1.0 / (360.0 * aspect), 1.0 / 360.0);
            return;
        }

        // ── Map toggle ───────────────────────────────────────────────────────
        if is_key_pressed(KeyCode::M) {
            self.show_map = !self.show_map;
        }

        // ── Player ────────────────────────────────────────────────────────────
        self.player.update(dt, &input, &mut self.projectiles);

        // ── World ─────────────────────────────────────────────────────────────
        self.world.update(self.player.pos, dt);

        // Drain newly spawned enemies from freshly loaded chunks
        for (pos, archetype) in self.world.spawn_queue.drain(..) {
            self.enemies.push(Enemy::new(pos, archetype));
        }

        // ── Projectiles ───────────────────────────────────────────────────────
        self.projectiles.retain_mut(|p| p.update(dt));

        // ── Enemy AI & Physics ────────────────────────────────────────────────
        let player_pos = self.player.pos;
        for enemy in &mut self.enemies {
            enemy.update(dt, player_pos, &mut self.projectiles);
        }

        // ── Loot spin ─────────────────────────────────────────────────────────
        for loot in &mut self.loot_drops {
            loot.update(dt);
        }

        // ── Pickup notice timer ───────────────────────────────────────────────
        if let Some((_, _, ref mut t)) = self.pickup_notice {
            *t -= dt;
            if *t <= 0.0 {
                self.pickup_notice = None;
            }
        }

        // ── Collision: player bullets → asteroids + enemies (single pass) ────
        // Iterate backwards so swap_remove doesn't invalidate unvisited indices.
        let mut i = self.projectiles.len();
        while i > 0 {
            i -= 1;
            if self.projectiles[i].owner != Owner::Player {
                continue;
            }
            let pos = self.projectiles[i].pos;
            let damage = self.projectiles[i].damage;

            // Check asteroid hit first (fragments are spawned inside remove_asteroid_hit)
            if self.world.remove_asteroid_hit(pos, 3.0).is_some() {
                self.projectiles.swap_remove(i);
                continue;
            }

            // Check enemy hit
            let mut hit = false;
            for enemy in &mut self.enemies {
                if pos.distance(enemy.pos) < enemy.archetype.hit_radius() {
                    enemy.take_damage(damage);
                    hit = true;
                    break;
                }
            }
            if hit {
                self.projectiles.swap_remove(i);
            }
        }

        // ── Enemy death → loot drops + mission tracking ─────────────────────
        let mut drops: Vec<LootDrop> = Vec::new();
        let mut kill_count = 0u32;
        self.enemies.retain(|e| {
            if e.is_dead() {
                kill_count += 1;
                drops.push(LootDrop::new(
                    e.pos,
                    LootKind::Credits(10 + (e.max_health as u32 / 5)),
                ));
                // 40% chance: weapon drop (60% main / 40% aux)
                if quad_rand::gen_range(0.0_f32, 1.0) < 0.40 {
                    let weapon = if quad_rand::gen_range(0.0_f32, 1.0) < 0.6 {
                        WeaponItem::gen_main()
                    } else {
                        WeaponItem::gen_aux()
                    };
                    drops.push(LootDrop::new(
                        e.pos + Vec2::new(25.0, 0.0),
                        LootKind::Weapon(weapon),
                    ));
                }
                // 20% chance: thruster drop
                if quad_rand::gen_range(0.0_f32, 1.0) < 0.20 {
                    drops.push(LootDrop::new(
                        e.pos + Vec2::new(-25.0, 0.0),
                        LootKind::Thruster(ThrusterItem::gen()),
                    ));
                }
                false
            } else {
                true
            }
        });
        self.loot_drops.extend(drops);
        for _ in 0..kill_count {
            if let Some(msg) = self.mission_log.notify_kill() {
                self.pickup_notice = Some((msg, GOLD, 3.0));
            }
        }

        // ── Collision: enemy bullets → player ────────────────────────────────
        let player_pos = self.player.pos;
        let mut player_damage = 0.0_f32;
        self.projectiles.retain(|p| {
            if p.owner == Owner::Enemy && p.pos.distance(player_pos) < PLAYER_RADIUS {
                player_damage += p.damage;
                false
            } else {
                true
            }
        });
        if player_damage > 0.0 {
            self.player_health -= player_damage;
            self.damage_flash = DAMAGE_FLASH_DURATION;
        }

        // ── Collision: player → asteroids ────────────────────────────────────
        self.asteroid_iframes = (self.asteroid_iframes - dt).max(0.0);
        if self.asteroid_iframes == 0.0 && self.world.overlaps_asteroid(player_pos, PLAYER_RADIUS) {
            self.player_health -= 10.0;
            self.asteroid_iframes = ASTEROID_HIT_IFRAMES;
            self.damage_flash = DAMAGE_FLASH_DURATION;
        }

        // ── Death check ─────────────────────────────────────────────────────
        if self.player_health <= 0.0 {
            self.player_health = 0.0;
            self.dead = true;
            self.death_timer = DEATH_SCREEN_DELAY;
            return;
        }

        // ── Loot pickup ───────────────────────────────────────────────────────
        let player_pos = self.player.pos;
        let mut i = self.loot_drops.len();
        while i > 0 {
            i -= 1;
            if self.loot_drops[i].pos.distance(player_pos) >= PICKUP_RANGE {
                continue;
            }
            let loot = self.loot_drops.swap_remove(i);
            match loot.kind {
                LootKind::Credits(amt) => {
                    self.credits += amt;
                    if let Some(msg) = self.mission_log.notify_credits(amt) {
                        self.pickup_notice = Some((msg, GOLD, 3.0));
                    }
                }
                LootKind::Weapon(w) => {
                    let slot_label = match w.slot {
                        WeaponSlot::Main => "MAIN",
                        WeaponSlot::Aux => "AUX",
                    };
                    match self.player.equip_weapon(w) {
                        Ok((name, rarity, old)) => {
                            let msg =
                                format!("EQUIPPED [{}] {} {}", slot_label, rarity.label(), name);
                            self.pickup_notice = Some((msg, rarity.color(), 2.5));
                            // Stash displaced item in cargo
                            if let Some(old_w) = old {
                                if self.cargo.len() < MAX_CARGO {
                                    self.cargo.push(Item::Weapon(old_w));
                                } else {
                                    self.pickup_notice = Some((
                                        format!("CARGO FULL — {} lost", old_w.name),
                                        Color::new(0.9, 0.4, 0.2, 1.0),
                                        2.5,
                                    ));
                                }
                            }
                        }
                        Err(w) => {
                            // Didn't auto-equip — stash in cargo
                            if self.cargo.len() < MAX_CARGO {
                                let msg = format!(
                                    "STASHED [{}] {} {}",
                                    slot_label,
                                    w.rarity.label(),
                                    w.name
                                );
                                self.pickup_notice = Some((msg, w.rarity.color(), 2.0));
                                self.cargo.push(Item::Weapon(w));
                            } else {
                                self.pickup_notice = Some((
                                    format!("CARGO FULL — {} lost", w.name),
                                    Color::new(0.9, 0.4, 0.2, 1.0),
                                    2.5,
                                ));
                            }
                        }
                    }
                }
                LootKind::Thruster(t) => match self.player.equip_thruster(t) {
                    Ok((rarity, old)) => {
                        let msg = format!("EQUIPPED [THR] {} THRUSTER", rarity.label());
                        self.pickup_notice = Some((msg, rarity.color(), 2.5));
                        if let Some(old_t) = old {
                            if self.cargo.len() < MAX_CARGO {
                                self.cargo.push(Item::Thruster(old_t));
                            } else {
                                self.pickup_notice = Some((
                                    format!("CARGO FULL — {} THRUSTER lost", old_t.rarity.label()),
                                    Color::new(0.9, 0.4, 0.2, 1.0),
                                    2.5,
                                ));
                            }
                        }
                    }
                    Err(t) => {
                        if self.cargo.len() < MAX_CARGO {
                            let msg =
                                format!("STASHED [THR] {} THRUSTER", t.rarity.label());
                            self.pickup_notice = Some((msg, t.rarity.color(), 2.0));
                            self.cargo.push(Item::Thruster(t));
                        } else {
                            self.pickup_notice = Some((
                                format!("CARGO FULL — {} THRUSTER lost", t.rarity.label()),
                                Color::new(0.9, 0.4, 0.2, 1.0),
                                2.5,
                            ));
                        }
                    }
                },
            }
        }

        // ── Cull far enemies ──────────────────────────────────────────────────
        let player_pos = self.player.pos;
        self.enemies
            .retain(|e| e.pos.distance(player_pos) < ENEMY_CULL_DIST);

        // ── Camera ────────────────────────────────────────────────────────────
        let aspect = screen_width() / screen_height();
        self.camera.target = self.player.pos;
        self.camera.zoom = vec2(1.0 / (360.0 * aspect), 1.0 / 360.0);
    }

    fn update_inventory(&mut self) {
        // Total list: 3 equipment slots + cargo items
        let total = 3 + self.cargo.len();

        if (is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W))
            && self.inv_cursor > 0 {
                self.inv_cursor -= 1;
            }
        if (is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S))
            && self.inv_cursor + 1 < total {
                self.inv_cursor += 1;
            }

        // Space: equip/unequip/swap
        if is_key_pressed(KeyCode::Space) {
            match self.inv_cursor {
                0 => {
                    // Unequip main weapon → cargo
                    if self.player.loadout.main.is_some() && self.cargo.len() < MAX_CARGO {
                        if let Some(w) = self.player.loadout.unequip_weapon(WeaponSlot::Main) {
                            self.cargo.push(Item::Weapon(w));
                        }
                    }
                }
                1 => {
                    // Unequip aux weapon → cargo
                    if self.player.loadout.aux.is_some() && self.cargo.len() < MAX_CARGO {
                        if let Some(w) = self.player.loadout.unequip_weapon(WeaponSlot::Aux) {
                            self.cargo.push(Item::Weapon(w));
                        }
                    }
                }
                2 => {
                    // Unequip thruster → cargo
                    if self.player.loadout.thruster.is_some() && self.cargo.len() < MAX_CARGO {
                        if let Some(t) = self.player.loadout.unequip_thruster() {
                            self.cargo.push(Item::Thruster(t));
                        }
                    }
                }
                idx => {
                    // Equip from cargo (force-equip, displaced item goes back to cargo)
                    let cargo_idx = idx - 3;
                    if cargo_idx < self.cargo.len() {
                        let item = self.cargo.remove(cargo_idx);
                        match item {
                            Item::Weapon(w) => {
                                if let Some(old) = self.player.loadout.force_equip_weapon(w) {
                                    self.cargo.insert(cargo_idx, Item::Weapon(old));
                                }
                            }
                            Item::Thruster(t) => {
                                if let Some(old) = self.player.loadout.force_equip_thruster(t) {
                                    self.cargo.insert(cargo_idx, Item::Thruster(old));
                                }
                            }
                        }
                    }
                }
            }
        }

        // X or Delete: discard cargo item
        if (is_key_pressed(KeyCode::X) || is_key_pressed(KeyCode::Delete))
            && self.inv_cursor >= 3 {
                let cargo_idx = self.inv_cursor - 3;
                if cargo_idx < self.cargo.len() {
                    let item = self.cargo.remove(cargo_idx);
                    let msg = format!("DISCARDED {} {}", item.rarity().label(), item.name());
                    self.pickup_notice = Some((msg, Color::new(0.5, 0.5, 0.5, 1.0), 2.0));
                    // Clamp cursor
                    let total = 3 + self.cargo.len();
                    if self.inv_cursor >= total && self.inv_cursor > 0 {
                        self.inv_cursor -= 1;
                    }
                }
            }

        // Close inventory
        if is_key_pressed(KeyCode::Escape) {
            self.show_inventory = false;
        }
    }

    fn respawn(&mut self) {
        self.dead = false;
        self.player_health = PLAYER_MAX_HEALTH;
        self.damage_flash = 0.0;
        self.asteroid_iframes = 1.0; // brief invincibility on respawn
        self.projectiles.clear();
        self.enemies.clear();
        // Keep credits, loadout, and cargo — just reset position and health
        self.player.pos = Vec2::ZERO;
        self.player.vel = Vec2::ZERO;
        self.world = World::new();
        self.loot_drops.clear();
    }

    pub fn draw(&self) {
        clear_background(Color::new(0.02, 0.02, 0.06, 1.0));

        set_camera(&self.camera);

        self.world.draw();

        for loot in &self.loot_drops {
            loot.draw();
        }
        for enemy in &self.enemies {
            enemy.draw();
        }
        for p in &self.projectiles {
            p.draw();
        }
        self.player.draw();

        set_default_camera();

        // ── Damage flash overlay ────────────────────────────────────────────
        if self.damage_flash > 0.0 {
            let alpha = (self.damage_flash / DAMAGE_FLASH_DURATION).min(1.0) * 0.35;
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(1.0, 0.0, 0.0, alpha),
            );
        }

        self.draw_hud();
        if self.show_map {
            self.draw_map();
        }
        if self.show_inventory {
            self.draw_inventory();
        }
        if self.planet_menu.is_some() {
            self.draw_planet_menu();
        }

        // ── Death overlay ───────────────────────────────────────────────────
        if self.dead {
            self.draw_death_screen();
        }

        let near_planet = self.world.nearby_planet_name(self.player.pos).is_some()
            || self.planet_menu.is_some();
        self.mobile.draw(near_planet);
    }

    fn draw_hud(&self) {
        let pad = 12.0;
        let fs = 18.0;

        draw_text("STARSEEKER", pad, pad + fs, fs, SKYBLUE);
        draw_text(
            &format!("FPS: {}", get_fps()),
            pad,
            pad + fs * 2.4,
            14.0,
            GRAY,
        );

        // Health bar
        let bar_w = 160.0;
        let bar_h = 12.0;
        let bx = pad;
        let by = pad + fs * 3.8;
        draw_rectangle(bx, by, bar_w, bar_h, Color::new(0.3, 0.0, 0.0, 0.85));
        let fill = (self.player_health / PLAYER_MAX_HEALTH).max(0.0);
        let hp_color = if fill > 0.5 {
            Color::new(0.1, 0.85, 0.2, 0.95)
        } else if fill > 0.25 {
            ORANGE
        } else {
            RED
        };
        draw_rectangle(bx, by, bar_w * fill, bar_h, hp_color);
        draw_text(
            &format!("HP  {:.0}", self.player_health.max(0.0)),
            bx + bar_w + 6.0,
            by + bar_h - 1.0,
            12.0,
            LIGHTGRAY,
        );

        // ── Quest tracker (below HP bar) ────────────────────────────────────
        if !self.mission_log.active.is_empty() {
            let qt_x = pad;
            let mut qt_y = by + bar_h + 16.0;
            draw_text(
                "MISSIONS",
                qt_x,
                qt_y,
                11.0,
                Color::new(0.5, 0.6, 0.8, 0.7),
            );
            qt_y += 4.0;

            for m in &self.mission_log.active {
                qt_y += 14.0;
                let complete = m.objective.is_complete();
                let title_color = if complete {
                    Color::new(0.2, 0.9, 0.3, 0.9)
                } else {
                    Color::new(0.75, 0.75, 0.8, 0.85)
                };
                draw_text(&m.title, qt_x, qt_y, 12.0, title_color);

                // Mini progress bar
                qt_y += 10.0;
                let pbar_w = 140.0;
                let pbar_h = 3.0;
                draw_rectangle(
                    qt_x,
                    qt_y,
                    pbar_w,
                    pbar_h,
                    Color::new(0.2, 0.2, 0.3, 0.5),
                );
                let frac = m.objective.progress_frac().min(1.0);
                let pbar_color = if complete {
                    Color::new(0.2, 0.9, 0.3, 0.8)
                } else {
                    Color::new(0.3, 0.5, 1.0, 0.7)
                };
                draw_rectangle(qt_x, qt_y, pbar_w * frac, pbar_h, pbar_color);

                // Short progress text to the right of the bar
                let status = if complete {
                    "DONE".to_string()
                } else {
                    m.objective.progress_text()
                };
                draw_text(
                    &status,
                    qt_x + pbar_w + 6.0,
                    qt_y + 3.0,
                    10.0,
                    Color::new(0.5, 0.55, 0.65, 0.75),
                );
                qt_y += 4.0;
            }
        }

        // Credits + Cargo count
        let cr_text = format!("CR {}  CARGO {}/{}", self.credits, self.cargo.len(), MAX_CARGO);
        let cr_w = measure_text(&cr_text, None, fs as u16, 1.0).width;
        draw_text(
            &cr_text,
            screen_width() - cr_w - pad,
            pad + fs,
            fs,
            GOLD,
        );

        // ── Loadout panel (bottom-right) ──────────────────────────────────────
        self.draw_loadout_panel();

        // ── Pickup notice (center) ────────────────────────────────────────────
        if let Some((msg, color, t)) = &self.pickup_notice {
            let alpha = (*t / 2.5).min(1.0);
            let c = Color::new(color.r, color.g, color.b, alpha);
            let tw = measure_text(msg, None, 16, 1.0).width;
            draw_text(
                msg,
                screen_width() * 0.5 - tw * 0.5,
                screen_height() * 0.5 - 50.0,
                16.0,
                c,
            );
        }

        // Planet approach prompt
        if let Some(name) = self.world.nearby_planet_name(self.player.pos) {
            let msg = format!("[E] Land on {}", name);
            let tw = measure_text(&msg, None, 18, 1.0).width;
            draw_text(
                &msg,
                screen_width() * 0.5 - tw * 0.5,
                screen_height() * 0.5 + 60.0,
                18.0,
                YELLOW,
            );
        }

        draw_text(
            "W/↑ Thrust  S/↓ Brake  A/D Rotate  C Stabilize  Space Main  Ctrl/Z Aux  E Interact  M Map  I Cargo",
            pad,
            screen_height() - pad,
            13.0,
            DARKGRAY,
        );
    }

    fn draw_loadout_panel(&self) {
        let sw = screen_width();
        let pad = 10.0;
        let row_h = 20.0;
        let panel_w = 270.0;
        let panel_x = sw - panel_w - pad;
        // Anchor below the title/credits row so it never overlaps bottom controls
        let panel_y = pad + 52.0;

        let loadout = &self.player.loadout;

        // Background
        draw_rectangle(
            panel_x - 4.0,
            panel_y - 14.0,
            panel_w + 8.0,
            row_h * 3.5 + 4.0,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );

        self.draw_loadout_slot(
            panel_x,
            panel_y,
            "MAIN",
            loadout.main.as_ref().map(|w| {
                (
                    w.rarity.color(),
                    w.rarity.label(),
                    w.name.as_str(),
                    w.stat_summary(),
                )
            }),
        );
        self.draw_loadout_slot(
            panel_x,
            panel_y + row_h * 1.2,
            "AUX ",
            loadout.aux.as_ref().map(|w| {
                (
                    w.rarity.color(),
                    w.rarity.label(),
                    w.name.as_str(),
                    w.stat_summary(),
                )
            }),
        );
        self.draw_loadout_slot(
            panel_x,
            panel_y + row_h * 2.4,
            "THR ",
            loadout.thruster.as_ref().map(|t| {
                (
                    t.rarity.color(),
                    t.rarity.label(),
                    "THRUSTER",
                    t.stat_summary(),
                )
            }),
        );
    }

    fn draw_inventory(&self) {
        let sw = screen_width();
        let sh = screen_height();

        // Full-screen dimmer
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.05, 0.80));

        // Panel
        let pw = 500.0_f32;
        let ph = 500.0_f32;
        let px = sw * 0.5 - pw * 0.5;
        let py = sh * 0.5 - ph * 0.5;
        draw_rectangle(px, py, pw, ph, Color::new(0.04, 0.06, 0.18, 0.96));
        draw_rectangle_lines(px, py, pw, ph, 1.5, Color::new(0.3, 0.5, 1.0, 0.55));

        // Title
        let title = format!("CARGO HOLD ({}/{})", self.cargo.len(), MAX_CARGO);
        let fs_title = 24.0_f32;
        let tw = measure_text(&title, None, fs_title as u16, 1.0).width;
        draw_text(&title, sw * 0.5 - tw * 0.5, py + 32.0, fs_title, SKYBLUE);

        // Divider
        draw_line(
            px + 20.0,
            py + 44.0,
            px + pw - 20.0,
            py + 44.0,
            0.7,
            Color::new(0.3, 0.4, 0.6, 0.45),
        );

        let content_x = px + 24.0;
        let mut y = py + 62.0;
        let row_h = 22.0;

        // ── Equipped section ────────────────────────────────────────────────
        draw_text(
            "EQUIPPED",
            content_x,
            y,
            12.0,
            Color::new(0.5, 0.6, 0.8, 0.7),
        );
        y += 6.0;

        let equipped_slots: [LoadoutSlotDisplay<'_>; 3] = [
            (
                "MAIN",
                self.player.loadout.main.as_ref().map(|w| {
                    (&w.rarity, w.name.clone(), w.stat_summary())
                }),
            ),
            (
                "AUX ",
                self.player.loadout.aux.as_ref().map(|w| {
                    (&w.rarity, w.name.clone(), w.stat_summary())
                }),
            ),
            (
                "THR ",
                self.player.loadout.thruster.as_ref().map(|t| {
                    (&t.rarity, "THRUSTER".to_string(), t.stat_summary())
                }),
            ),
        ];

        for (i, (label, item_info)) in equipped_slots.iter().enumerate() {
            y += row_h;
            let selected = self.inv_cursor == i;
            if selected {
                draw_rectangle(
                    content_x - 4.0,
                    y - 14.0,
                    pw - 40.0,
                    row_h - 2.0,
                    Color::new(0.15, 0.2, 0.4, 0.6),
                );
            }
            let label_color = if selected { WHITE } else { DARKGRAY };
            draw_text(label, content_x, y, 14.0, label_color);
            if let Some((rarity, name, stats)) = item_info {
                let rc = rarity.color();
                let tag = format!("[{}]", rarity.label());
                draw_text(&tag, content_x + 42.0, y, 14.0, rc);
                let tag_w = measure_text(&tag, None, 14, 1.0).width;
                draw_text(
                    &format!(" {} {}", name, stats),
                    content_x + 42.0 + tag_w,
                    y,
                    12.0,
                    Color::new(0.7, 0.7, 0.7, 1.0),
                );
            } else {
                draw_text(
                    "--- empty ---",
                    content_x + 42.0,
                    y,
                    12.0,
                    Color::new(0.3, 0.3, 0.3, 1.0),
                );
            }
        }

        // ── Cargo section ───────────────────────────────────────────────────
        y += row_h + 8.0;
        draw_line(
            px + 20.0,
            y - 6.0,
            px + pw - 20.0,
            y - 6.0,
            0.5,
            Color::new(0.3, 0.4, 0.6, 0.3),
        );
        draw_text(
            "CARGO",
            content_x,
            y + 4.0,
            12.0,
            Color::new(0.5, 0.6, 0.8, 0.7),
        );
        y += 10.0;

        if self.cargo.is_empty() {
            y += row_h;
            draw_text(
                "Empty",
                content_x + 42.0,
                y,
                12.0,
                Color::new(0.3, 0.3, 0.3, 1.0),
            );
        } else {
            // Scrollable area: show items that fit in remaining space
            let max_visible = ((py + ph - 60.0 - y) / row_h) as usize;
            let cargo_cursor = self.inv_cursor.saturating_sub(3);
            let scroll_start = if cargo_cursor >= max_visible {
                cargo_cursor - max_visible + 1
            } else {
                0
            };

            for (ci, item) in self.cargo.iter().enumerate().skip(scroll_start) {
                y += row_h;
                if y > py + ph - 50.0 {
                    break;
                }
                let list_idx = ci + 3;
                let selected = self.inv_cursor == list_idx;
                if selected {
                    draw_rectangle(
                        content_x - 4.0,
                        y - 14.0,
                        pw - 40.0,
                        row_h - 2.0,
                        Color::new(0.15, 0.2, 0.4, 0.6),
                    );
                }
                let rc = item.rarity().color();
                let slot = item.slot_label();
                let tag = format!("[{}]", item.rarity().label());
                let name = item.name();
                let stats = item.stat_summary();
                let label_color = if selected { WHITE } else { LIGHTGRAY };
                draw_text(slot, content_x, y, 12.0, Color::new(0.4, 0.4, 0.5, 0.8));
                draw_text(&tag, content_x + 42.0, y, 14.0, rc);
                let tag_w = measure_text(&tag, None, 14, 1.0).width;
                draw_text(
                    &format!(" {} {}", name, stats),
                    content_x + 42.0 + tag_w,
                    y,
                    12.0,
                    label_color,
                );
            }
        }

        // Footer
        let footer = "[I/Tab] Close   [W/S] Navigate   [SPACE] Equip/Unequip   [X] Discard";
        let fw = measure_text(footer, None, 12, 1.0).width;
        draw_text(
            footer,
            sw * 0.5 - fw * 0.5,
            py + ph - 14.0,
            12.0,
            YELLOW,
        );
    }

    fn draw_map(&self) {
        let sw = screen_width();
        let sh = screen_height();

        // Semi-transparent background
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.05, 0.75));

        // Map viewport: centered on screen, showing loaded chunks
        let map_w = sw * 0.8;
        let map_h = sh * 0.8;
        let map_x = sw * 0.1;
        let map_y = sh * 0.1;

        // Border
        draw_rectangle_lines(map_x, map_y, map_w, map_h, 1.0, Color::new(0.3, 0.5, 1.0, 0.4));

        // Title
        let title = "SECTOR MAP";
        let tw = measure_text(title, None, 20, 1.0).width;
        draw_text(title, sw * 0.5 - tw * 0.5, map_y - 6.0, 20.0, SKYBLUE);

        // Determine world-space bounds from loaded chunks
        let chunks = self.world.map_chunks();
        let planets = self.world.map_planets();

        if chunks.is_empty() {
            return;
        }

        // Find bounding box of all loaded chunks
        let chunk_size = 3200.0_f32;
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for &(origin, _) in &chunks {
            min_x = min_x.min(origin.x);
            min_y = min_y.min(origin.y);
            max_x = max_x.max(origin.x + chunk_size);
            max_y = max_y.max(origin.y + chunk_size);
        }

        let world_w = max_x - min_x;
        let world_h = max_y - min_y;
        // Scale to fit map viewport with some padding
        let padding = 20.0;
        let usable_w = map_w - padding * 2.0;
        let usable_h = map_h - padding * 2.0;
        let scale = (usable_w / world_w).min(usable_h / world_h);

        // Offset to center the map content
        let center_x = map_x + map_w * 0.5;
        let center_y = map_y + map_h * 0.5;
        let world_cx = (min_x + max_x) * 0.5;
        let world_cy = (min_y + max_y) * 0.5;

        let to_screen = |world_pos: Vec2| -> Vec2 {
            Vec2::new(
                center_x + (world_pos.x - world_cx) * scale,
                center_y + (world_pos.y - world_cy) * scale,
            )
        };

        // Draw chunk grid with zone colors
        for &(origin, color) in &chunks {
            let tl = to_screen(origin);
            let sz = chunk_size * scale;
            draw_rectangle(tl.x, tl.y, sz, sz, color);
            draw_rectangle_lines(tl.x, tl.y, sz, sz, 0.5, Color::new(0.3, 0.3, 0.4, 0.2));
        }

        // Draw planets
        for &(pos, radius, color, name) in &planets {
            let sp = to_screen(pos);
            let sr = (radius * scale).max(4.0); // minimum visible size
            draw_circle(sp.x, sp.y, sr, Color::new(color.r, color.g, color.b, 0.8));
            draw_circle_lines(
                sp.x,
                sp.y,
                sr,
                1.0,
                Color::new(
                    (color.r + 0.3).min(1.0),
                    (color.g + 0.3).min(1.0),
                    (color.b + 0.3).min(1.0),
                    0.7,
                ),
            );
            // Planet name label
            let fs = 11.0_f32;
            let ntw = measure_text(name, None, fs as u16, 1.0).width;
            draw_text(
                name,
                sp.x - ntw * 0.5,
                sp.y + sr + 12.0,
                fs,
                Color::new(1.0, 1.0, 1.0, 0.75),
            );
        }

        // Draw player position
        let player_sp = to_screen(self.player.pos);
        // Direction indicator
        let forward = Vec2::new(self.player.rotation.sin(), -self.player.rotation.cos());
        let arrow_len = 8.0;
        draw_line(
            player_sp.x,
            player_sp.y,
            player_sp.x + forward.x * arrow_len,
            player_sp.y + forward.y * arrow_len,
            1.5,
            WHITE,
        );
        draw_circle(player_sp.x, player_sp.y, 3.0, WHITE);
        draw_text("YOU", player_sp.x + 6.0, player_sp.y - 6.0, 10.0, WHITE);

        // Draw mission targets
        for m in &self.mission_log.active {
            if let crate::missions::Objective::VisitPlanet {
                ref planet_name,
                visited,
            } = m.objective
            {
                if visited {
                    continue;
                }
                // Find the target planet in loaded data
                for &(pos, radius, _, name) in &planets {
                    if name == planet_name {
                        let sp = to_screen(pos);
                        let sr = (radius * scale).max(4.0) + 6.0;
                        // Pulsing ring around mission target
                        let pulse = ((get_time() * 2.0).sin() * 0.3 + 0.7) as f32;
                        draw_circle_lines(
                            sp.x,
                            sp.y,
                            sr,
                            1.5,
                            Color::new(1.0, 0.8, 0.0, pulse),
                        );
                    }
                }
            }
        }

        // Footer
        let hint = "[M] Close Map";
        let hw = measure_text(hint, None, 13, 1.0).width;
        draw_text(
            hint,
            sw * 0.5 - hw * 0.5,
            map_y + map_h + 18.0,
            13.0,
            YELLOW,
        );
    }

    fn draw_planet_menu(&self) {
        let menu = match &self.planet_menu {
            Some(m) => m,
            None => return,
        };

        let sw = screen_width();
        let sh = screen_height();

        // Full-screen dimmer
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.08, 0.80));

        // Panel — taller to fit missions + shop
        let pw = 480.0_f32;
        let ph = 440.0_f32;
        let px = sw * 0.5 - pw * 0.5;
        let py = sh * 0.5 - ph * 0.5;
        draw_rectangle(px, py, pw, ph, Color::new(0.04, 0.06, 0.18, 0.96));
        draw_rectangle_lines(px, py, pw, ph, 1.5, Color::new(0.3, 0.5, 1.0, 0.55));

        // Planet name
        let title = menu.name.to_uppercase();
        let fs_title = 28.0_f32;
        let tw = measure_text(&title, None, fs_title as u16, 1.0).width;
        draw_text(&title, sw * 0.5 - tw * 0.5, py + 40.0, fs_title, SKYBLUE);

        // Credits
        let cr = format!("Credits: {}", self.credits);
        let crw = measure_text(&cr, None, 13, 1.0).width;
        draw_text(&cr, sw * 0.5 - crw * 0.5, py + 58.0, 13.0, GOLD);

        // Divider
        draw_line(
            px + 20.0,
            py + 68.0,
            px + pw - 20.0,
            py + 68.0,
            0.7,
            Color::new(0.3, 0.4, 0.6, 0.45),
        );

        // Tab bar
        let tab_y = py + 88.0;
        let active_tab_color = WHITE;
        let inactive_tab_color = Color::new(0.4, 0.4, 0.5, 0.65);

        let tab_color = |t: MenuTab| {
            if menu.tab == t {
                active_tab_color
            } else {
                inactive_tab_color
            }
        };
        draw_text("[1] MISSIONS", px + 24.0, tab_y, 13.0, tab_color(MenuTab::Missions));
        let active_label = format!("[2] ACTIVE ({})", self.mission_log.active.len());
        draw_text(&active_label, px + 145.0, tab_y, 13.0, tab_color(MenuTab::Active));
        draw_text("[3] SHOP", px + 300.0, tab_y, 13.0, tab_color(MenuTab::Shop));

        // Underline active tab
        let ul_y = tab_y + 4.0;
        match menu.tab {
            MenuTab::Missions => draw_line(px + 24.0, ul_y, px + 130.0, ul_y, 1.0, SKYBLUE),
            MenuTab::Active => draw_line(px + 145.0, ul_y, px + 285.0, ul_y, 1.0, SKYBLUE),
            MenuTab::Shop => draw_line(px + 300.0, ul_y, px + 380.0, ul_y, 1.0, SKYBLUE),
        }

        // Content area
        let content_x = px + 36.0;
        let content_y = tab_y + 26.0;
        let row_h = 52.0;

        match menu.tab {
            MenuTab::Missions => {
                if menu.available.is_empty() {
                    draw_text(
                        "No missions available.",
                        content_x,
                        content_y + 16.0,
                        14.0,
                        Color::new(0.4, 0.4, 0.5, 0.7),
                    );
                } else {
                    for (i, m) in menu.available.iter().enumerate() {
                        let y = content_y + i as f32 * row_h;
                        let selected = i == menu.selected;

                        // Selection highlight
                        if selected {
                            draw_rectangle(
                                content_x - 6.0,
                                y - 10.0,
                                pw - 60.0,
                                row_h - 4.0,
                                Color::new(0.15, 0.2, 0.4, 0.5),
                            );
                        }

                        // Title + reward
                        let title_color = if selected { WHITE } else { LIGHTGRAY };
                        draw_text(&m.title, content_x, y + 4.0, 15.0, title_color);
                        let reward_str = format!("+{} CR", m.reward_credits);
                        let rw = measure_text(&reward_str, None, 13, 1.0).width;
                        draw_text(
                            &reward_str,
                            px + pw - 36.0 - rw,
                            y + 4.0,
                            13.0,
                            GOLD,
                        );

                        // Briefing
                        draw_text(
                            &m.briefing,
                            content_x,
                            y + 20.0,
                            11.0,
                            Color::new(0.5, 0.55, 0.65, 0.85),
                        );

                        // Objective hint
                        draw_text(
                            &m.objective.progress_text(),
                            content_x,
                            y + 34.0,
                            11.0,
                            Color::new(0.4, 0.6, 0.8, 0.8),
                        );
                    }

                    // Accept prompt
                    if self.mission_log.can_accept() {
                        let prompt = "[SPACE] Accept Mission";
                        let pw2 = measure_text(prompt, None, 13, 1.0).width;
                        draw_text(
                            prompt,
                            sw * 0.5 - pw2 * 0.5,
                            py + ph - 38.0,
                            13.0,
                            Color::new(0.6, 0.8, 1.0, 0.85),
                        );
                    } else {
                        let full = "Mission log full (max 3)";
                        let fw = measure_text(full, None, 13, 1.0).width;
                        draw_text(
                            full,
                            sw * 0.5 - fw * 0.5,
                            py + ph - 38.0,
                            13.0,
                            Color::new(0.7, 0.3, 0.3, 0.85),
                        );
                    }
                }
            }
            MenuTab::Active => {
                if self.mission_log.active.is_empty() {
                    draw_text(
                        "No active missions.",
                        content_x,
                        content_y + 16.0,
                        14.0,
                        Color::new(0.4, 0.4, 0.5, 0.7),
                    );
                } else {
                    let has_completed = self.mission_log.active.iter().any(|m| m.objective.is_complete());
                    for (i, m) in self.mission_log.active.iter().enumerate() {
                        let y = content_y + i as f32 * row_h;
                        let complete = m.objective.is_complete();

                        // Title
                        let title_color = if complete { GOLD } else { LIGHTGRAY };
                        draw_text(&m.title, content_x, y + 4.0, 15.0, title_color);

                        // Reward
                        let reward_str = format!("+{} CR", m.reward_credits);
                        let rw = measure_text(&reward_str, None, 13, 1.0).width;
                        draw_text(
                            &reward_str,
                            px + pw - 36.0 - rw,
                            y + 4.0,
                            13.0,
                            GOLD,
                        );

                        // Progress bar
                        let bar_w = pw - 72.0;
                        let bar_h = 6.0;
                        let bar_y = y + 16.0;
                        draw_rectangle(content_x, bar_y, bar_w, bar_h, Color::new(0.2, 0.2, 0.3, 0.6));
                        let frac = m.objective.progress_frac().min(1.0);
                        let bar_color = if complete {
                            Color::new(0.2, 0.9, 0.3, 0.9)
                        } else {
                            Color::new(0.3, 0.5, 1.0, 0.8)
                        };
                        draw_rectangle(content_x, bar_y, bar_w * frac, bar_h, bar_color);

                        // Progress text
                        let status = if complete {
                            "COMPLETE".to_string()
                        } else {
                            m.objective.progress_text()
                        };
                        draw_text(
                            &status,
                            content_x,
                            y + 34.0,
                            11.0,
                            if complete {
                                Color::new(0.2, 0.9, 0.3, 0.9)
                            } else {
                                Color::new(0.5, 0.55, 0.65, 0.85)
                            },
                        );
                    }

                    // Claim prompt
                    if has_completed {
                        let prompt = "[SPACE] Claim Rewards";
                        let pw2 = measure_text(prompt, None, 13, 1.0).width;
                        draw_text(
                            prompt,
                            sw * 0.5 - pw2 * 0.5,
                            py + ph - 38.0,
                            13.0,
                            GOLD,
                        );
                    }
                }
            }
            MenuTab::Shop => {
                self.draw_shop_tab(menu, content_x, content_y, px, py, pw, ph);
            }
        }

        // Footer
        let footer = "[E] Depart    [W/S] Navigate    [1/2/3] Tabs";
        let fs_foot = 13.0_f32;
        let ftw = measure_text(footer, None, fs_foot as u16, 1.0).width;
        draw_text(
            footer,
            sw * 0.5 - ftw * 0.5,
            py + ph - 14.0,
            fs_foot,
            YELLOW,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_shop_tab(
        &self,
        menu: &PlanetMenu,
        content_x: f32,
        content_y: f32,
        px: f32,
        py: f32,
        pw: f32,
        ph: f32,
    ) {
        let sw = screen_width();
        let row_h = 26.0;
        let shop_len = menu.shop_stock.len();
        let mut y = content_y;

        // ── FOR SALE section ────────────────────────────────────────────────
        draw_text(
            "FOR SALE",
            content_x,
            y,
            12.0,
            Color::new(0.5, 0.6, 0.8, 0.7),
        );
        y += 4.0;

        if menu.shop_stock.is_empty() {
            y += row_h;
            draw_text(
                "Sold out.",
                content_x,
                y,
                13.0,
                Color::new(0.4, 0.4, 0.5, 0.7),
            );
            y += 6.0;
        } else {
            for (i, item) in menu.shop_stock.iter().enumerate() {
                y += row_h;
                let selected = menu.selected == i;
                if selected {
                    draw_rectangle(
                        content_x - 6.0,
                        y - 14.0,
                        pw - 60.0,
                        row_h - 2.0,
                        Color::new(0.15, 0.2, 0.4, 0.5),
                    );
                }
                let rc = item.rarity().color();
                let slot = item.slot_label();
                let tag = format!("[{}]", item.rarity().label());
                let name = item.name();
                let stats = item.stat_summary();
                let label_color = if selected { WHITE } else { LIGHTGRAY };

                draw_text(slot, content_x, y, 11.0, Color::new(0.4, 0.4, 0.5, 0.8));
                draw_text(&tag, content_x + 40.0, y, 13.0, rc);
                let tag_w = measure_text(&tag, None, 13, 1.0).width;
                draw_text(
                    &format!(" {} {}", name, stats),
                    content_x + 40.0 + tag_w,
                    y,
                    11.0,
                    label_color,
                );

                // Price (right-aligned)
                let price_str = format!("{} CR", item.buy_price());
                let price_w = measure_text(&price_str, None, 12, 1.0).width;
                let can_afford = self.credits >= item.buy_price();
                let price_color = if can_afford {
                    GOLD
                } else {
                    Color::new(0.7, 0.3, 0.3, 0.85)
                };
                draw_text(
                    &price_str,
                    px + pw - 36.0 - price_w,
                    y,
                    12.0,
                    price_color,
                );
            }
        }

        // ── YOUR CARGO section ──────────────────────────────────────────────
        y += row_h + 4.0;
        draw_line(
            px + 20.0,
            y - 10.0,
            px + pw - 20.0,
            y - 10.0,
            0.5,
            Color::new(0.3, 0.4, 0.6, 0.3),
        );
        let cargo_header = format!("YOUR CARGO ({}/{})", self.cargo.len(), MAX_CARGO);
        draw_text(
            &cargo_header,
            content_x,
            y,
            12.0,
            Color::new(0.5, 0.6, 0.8, 0.7),
        );
        y += 4.0;

        if self.cargo.is_empty() {
            y += row_h;
            draw_text(
                "Empty",
                content_x,
                y,
                13.0,
                Color::new(0.4, 0.4, 0.5, 0.7),
            );
        } else {
            for (ci, item) in self.cargo.iter().enumerate() {
                y += row_h;
                if y > py + ph - 56.0 {
                    break;
                }
                let list_idx = shop_len + ci;
                let selected = menu.selected == list_idx;
                if selected {
                    draw_rectangle(
                        content_x - 6.0,
                        y - 14.0,
                        pw - 60.0,
                        row_h - 2.0,
                        Color::new(0.2, 0.15, 0.1, 0.5),
                    );
                }
                let rc = item.rarity().color();
                let slot = item.slot_label();
                let tag = format!("[{}]", item.rarity().label());
                let name = item.name();
                let stats = item.stat_summary();
                let label_color = if selected { WHITE } else { LIGHTGRAY };

                draw_text(slot, content_x, y, 11.0, Color::new(0.4, 0.4, 0.5, 0.8));
                draw_text(&tag, content_x + 40.0, y, 13.0, rc);
                let tag_w = measure_text(&tag, None, 13, 1.0).width;
                draw_text(
                    &format!(" {} {}", name, stats),
                    content_x + 40.0 + tag_w,
                    y,
                    11.0,
                    label_color,
                );

                // Sell price (right-aligned)
                let sell_str = format!("+{} CR", item.sell_price());
                let sell_w = measure_text(&sell_str, None, 12, 1.0).width;
                draw_text(
                    &sell_str,
                    px + pw - 36.0 - sell_w,
                    y,
                    12.0,
                    Color::new(0.2, 0.8, 0.3, 0.9),
                );
            }
        }

        // Action prompt
        let prompt = if menu.selected < shop_len {
            if self.cargo.len() >= MAX_CARGO {
                "[SPACE] BUY (cargo full!)".to_string()
            } else {
                "[SPACE] BUY".to_string()
            }
        } else {
            "[SPACE] SELL".to_string()
        };
        let prompt_w = measure_text(&prompt, None, 13, 1.0).width;
        draw_text(
            &prompt,
            sw * 0.5 - prompt_w * 0.5,
            py + ph - 38.0,
            13.0,
            Color::new(0.6, 0.8, 1.0, 0.85),
        );
    }

    fn draw_loadout_slot(
        &self,
        x: f32,
        y: f32,
        label: &str,
        item: Option<(Color, &str, &str, String)>,
    ) {
        draw_text(label, x, y, 14.0, DARKGRAY);
        if let Some((color, tier, name, stats)) = item {
            let tx = x + 40.0;
            draw_text(&format!("[{}]", tier), tx, y, 14.0, color);
            let tw = measure_text(&format!("[{}]", tier), None, 14, 1.0).width;
            draw_text(
                &format!(" {} {}", name, stats),
                tx + tw,
                y,
                12.0,
                Color::new(0.7, 0.7, 0.7, 1.0),
            );
        } else {
            draw_text(
                "--- empty ---",
                x + 40.0,
                y,
                12.0,
                Color::new(0.3, 0.3, 0.3, 1.0),
            );
        }
    }

    fn draw_death_screen(&self) {
        let sw = screen_width();
        let sh = screen_height();

        // Full-screen dark red overlay
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.15, 0.0, 0.0, 0.85));

        let title = "DESTROYED";
        let fs = 36.0_f32;
        let tw = measure_text(title, None, fs as u16, 1.0).width;
        draw_text(title, sw * 0.5 - tw * 0.5, sh * 0.4, fs, RED);

        if self.death_timer <= 0.0 {
            let prompt = "[E] or [SPACE] to respawn";
            let pfs = 16.0_f32;
            let pw = measure_text(prompt, None, pfs as u16, 1.0).width;
            // Blink effect
            let alpha = ((get_time() * 3.0).sin() * 0.5 + 0.5) as f32;
            draw_text(
                prompt,
                sw * 0.5 - pw * 0.5,
                sh * 0.55,
                pfs,
                Color::new(1.0, 1.0, 1.0, alpha),
            );
        }

        let credits_msg = format!("Credits: {}", self.credits);
        let cfs = 14.0_f32;
        let cw = measure_text(&credits_msg, None, cfs as u16, 1.0).width;
        draw_text(
            &credits_msg,
            sw * 0.5 - cw * 0.5,
            sh * 0.65,
            cfs,
            GOLD,
        );
    }
}
