use crate::entities::enemy::Enemy;
use crate::entities::loot::{LootDrop, LootKind, PICKUP_RANGE};
use crate::input::InputState;
use crate::items::{ThrusterItem, WeaponItem};
use crate::missions::{self, MenuTab, MissionLog, PlanetMenu};
use crate::mobile::MobileOverlay;
use crate::player::Player;
use crate::projectile::{Owner, Projectile};
use crate::world::World;
use macroquad::prelude::*;

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

pub(crate) struct Game {
    player: Player,
    player_health: f32,
    asteroid_iframes: f32,

    projectiles: Vec<Projectile>,
    enemies: Vec<Enemy>,
    loot_drops: Vec<LootDrop>,
    credits: u32,

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
                self.planet_menu = Some(PlanetMenu::new(name.to_string(), available));
            }
        }

        // ── Planet menu input & pause ────────────────────────────────────────
        if let Some(ref mut menu) = self.planet_menu {
            // Tab switching: 1 = Missions, 2 = Active
            if is_key_pressed(KeyCode::Key1) {
                menu.tab = MenuTab::Missions;
                menu.selected = 0;
            }
            if is_key_pressed(KeyCode::Key2) {
                menu.tab = MenuTab::Active;
                menu.selected = 0;
            }

            // Selection movement
            let list_len = match menu.tab {
                MenuTab::Missions => menu.available.len(),
                MenuTab::Active => self.mission_log.active.len(),
            };
            if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                if menu.selected > 0 {
                    menu.selected -= 1;
                }
            }
            if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                if menu.selected + 1 < list_len {
                    menu.selected += 1;
                }
            }

            // Accept mission / Claim reward with Space
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
                }
            }

            let aspect = screen_width() / screen_height();
            self.camera.target = self.player.pos;
            self.camera.zoom = vec2(1.0 / (360.0 * aspect), 1.0 / 360.0);
            return;
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
                        crate::items::WeaponSlot::Main => "MAIN",
                        crate::items::WeaponSlot::Aux => "AUX",
                    };
                    if let Some((name, rarity)) = self.player.equip_weapon(w) {
                        let msg = format!("EQUIPPED [{}] {} {}", slot_label, rarity.label(), name);
                        self.pickup_notice = Some((msg, rarity.color(), 2.5));
                    }
                }
                LootKind::Thruster(t) => {
                    if let Some(rarity) = self.player.equip_thruster(t) {
                        let msg = format!("EQUIPPED [THR] {} THRUSTER", rarity.label());
                        self.pickup_notice = Some((msg, rarity.color(), 2.5));
                    }
                }
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

    fn respawn(&mut self) {
        self.dead = false;
        self.player_health = PLAYER_MAX_HEALTH;
        self.damage_flash = 0.0;
        self.asteroid_iframes = 1.0; // brief invincibility on respawn
        self.projectiles.clear();
        self.enemies.clear();
        // Keep credits and loadout — just reset position and health
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

        // Credits
        draw_text(
            &format!("CR {}", self.credits),
            screen_width() - 90.0,
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
            "W/↑ Thrust  S/↓ Brake  A/D Rotate  C Stabilize  Space Main  Ctrl/Z Aux  E Interact",
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

    fn draw_planet_menu(&self) {
        let menu = match &self.planet_menu {
            Some(m) => m,
            None => return,
        };

        let sw = screen_width();
        let sh = screen_height();

        // Full-screen dimmer
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.08, 0.80));

        // Panel — taller to fit missions
        let pw = 440.0_f32;
        let ph = 400.0_f32;
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
        let missions_color = if menu.tab == MenuTab::Missions {
            active_tab_color
        } else {
            inactive_tab_color
        };
        let active_color = if menu.tab == MenuTab::Active {
            active_tab_color
        } else {
            inactive_tab_color
        };
        draw_text("[1] AVAILABLE", px + 36.0, tab_y, 14.0, missions_color);
        let active_label = format!("[2] ACTIVE ({})", self.mission_log.active.len());
        draw_text(&active_label, px + 180.0, tab_y, 14.0, active_color);

        // Underline active tab
        let ul_y = tab_y + 4.0;
        if menu.tab == MenuTab::Missions {
            draw_line(px + 36.0, ul_y, px + 155.0, ul_y, 1.0, SKYBLUE);
        } else {
            draw_line(px + 180.0, ul_y, px + 330.0, ul_y, 1.0, SKYBLUE);
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
        }

        // Footer
        let footer = "[E]  Depart       [W/S] Navigate";
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
