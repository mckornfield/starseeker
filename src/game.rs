use macroquad::prelude::*;
use crate::entities::enemy::Enemy;
use crate::entities::loot::{LootDrop, LootKind, PICKUP_RANGE};
use crate::input::InputState;
use crate::items::{ThrusterItem, WeaponItem};
use crate::mobile::MobileOverlay;
use crate::player::Player;
use crate::projectile::{Owner, Projectile};
use crate::world::World;

const PLAYER_RADIUS: f32 = 12.0;
const PLAYER_MAX_HEALTH: f32 = 100.0;
/// Invincibility window after an asteroid hit so one collision doesn't insta-kill
const ASTEROID_HIT_IFRAMES: f32 = 0.5;
/// Cull enemies that wander farther than this from the player
const ENEMY_CULL_DIST: f32 = 7000.0;

pub struct Game {
    player: Player,
    player_health: f32,
    asteroid_iframes: f32,

    projectiles: Vec<Projectile>,
    enemies: Vec<Enemy>,
    loot_drops: Vec<LootDrop>,
    credits: u32,

    /// Brief pickup notification: (message, rarity color, seconds remaining)
    pickup_notice: Option<(String, Color, f32)>,

    camera: Camera2D,
    world: World,
    mobile: MobileOverlay,
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

            camera: Camera2D {
                zoom: vec2(1.0 / 640.0, 1.0 / 360.0),
                ..Default::default()
            },
            world: World::new(),
            mobile: MobileOverlay::new(),
        }
    }

    pub fn update(&mut self) {
        let dt = get_frame_time();

        // ── Input ─────────────────────────────────────────────────────────────
        let kb = InputState::from_keyboard();
        let touch = self.mobile.update();
        let input = kb.merge(&touch);

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
            if *t <= 0.0 { self.pickup_notice = None; }
        }

        // ── Collision: player bullets → asteroids ─────────────────────────────
        let player_proj_positions: Vec<(usize, Vec2)> = self
            .projectiles
            .iter()
            .enumerate()
            .filter(|(_, p)| p.owner == Owner::Player)
            .map(|(i, p)| (i, p.pos))
            .collect();

        let mut remove_projs: Vec<usize> = Vec::new();
        for (i, pos) in &player_proj_positions {
            if self.world.remove_asteroid_hit(*pos, 3.0) {
                remove_projs.push(*i);
            }
        }
        for i in remove_projs.into_iter().rev() {
            self.projectiles.swap_remove(i);
        }

        // ── Collision: player bullets → enemies ───────────────────────────────
        let player_proj_data: Vec<(usize, Vec2, f32)> = self
            .projectiles
            .iter()
            .enumerate()
            .filter(|(_, p)| p.owner == Owner::Player)
            .map(|(i, p)| (i, p.pos, p.damage))
            .collect();

        let mut remove_projs: Vec<usize> = Vec::new();
        for (i, pos, damage) in &player_proj_data {
            for enemy in &mut self.enemies {
                let hit_r = match enemy.archetype {
                    crate::entities::enemy::EnemyArchetype::Tank   => 18.0,
                    crate::entities::enemy::EnemyArchetype::Agile  => 10.0,
                    crate::entities::enemy::EnemyArchetype::Ranged => 13.0,
                };
                if pos.distance(enemy.pos) < hit_r {
                    enemy.take_damage(*damage);
                    remove_projs.push(*i);
                    break;
                }
            }
        }
        for i in remove_projs.into_iter().rev() {
            self.projectiles.swap_remove(i);
        }

        // ── Enemy death → loot drops ──────────────────────────────────────────
        let mut drops: Vec<LootDrop> = Vec::new();
        self.enemies.retain(|e| {
            if e.is_dead() {
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
        self.player_health -= player_damage;

        // ── Collision: player → asteroids ────────────────────────────────────
        self.asteroid_iframes = (self.asteroid_iframes - dt).max(0.0);
        if self.asteroid_iframes == 0.0 && self.world.overlaps_asteroid(player_pos, PLAYER_RADIUS) {
            self.player_health -= 10.0;
            self.asteroid_iframes = ASTEROID_HIT_IFRAMES;
        }

        // ── Loot pickup ───────────────────────────────────────────────────────
        let player_pos = self.player.pos;
        let mut picked: Vec<LootDrop> = Vec::new();
        let mut remaining: Vec<LootDrop> = Vec::new();
        for loot in self.loot_drops.drain(..) {
            if loot.pos.distance(player_pos) < PICKUP_RANGE {
                picked.push(loot);
            } else {
                remaining.push(loot);
            }
        }
        self.loot_drops = remaining;

        for loot in picked {
            match loot.kind {
                LootKind::Credits(amt) => {
                    self.credits += amt;
                }
                LootKind::Weapon(w) => {
                    let slot_label = match w.slot {
                        crate::items::WeaponSlot::Main => "MAIN",
                        crate::items::WeaponSlot::Aux  => "AUX",
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

        self.draw_hud();
        self.mobile.draw();
    }

    fn draw_hud(&self) {
        let pad = 12.0;
        let fs  = 18.0;

        draw_text("STARSEEKER", pad, pad + fs, fs, SKYBLUE);
        draw_text(&format!("FPS: {}", get_fps()), pad, pad + fs * 2.4, 14.0, GRAY);

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
            &format!("¢ {}", self.credits),
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
            "W/↑ Thrust  S/↓ Brake  A/D Rotate  Space Main  Ctrl/Z Aux",
            pad,
            screen_height() - pad,
            13.0,
            DARKGRAY,
        );
    }

    fn draw_loadout_panel(&self) {
        let sw = screen_width();
        let sh = screen_height();
        let pad = 10.0;
        let row_h = 18.0;
        let panel_w = 220.0;
        let panel_x = sw - panel_w - pad;
        let panel_y = sh - pad - row_h * 3.5;

        let loadout = &self.player.loadout;

        // Background
        draw_rectangle(
            panel_x - 4.0, panel_y - 14.0,
            panel_w + 8.0, row_h * 3.5 + 4.0,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );

        self.draw_loadout_slot(
            panel_x, panel_y,
            "MAIN",
            loadout.main.as_ref().map(|w| (w.rarity.color(), w.rarity.label(), w.name.as_str(), w.stat_summary())),
        );
        self.draw_loadout_slot(
            panel_x, panel_y + row_h * 1.2,
            "AUX ",
            loadout.aux.as_ref().map(|w| (w.rarity.color(), w.rarity.label(), w.name.as_str(), w.stat_summary())),
        );
        self.draw_loadout_slot(
            panel_x, panel_y + row_h * 2.4,
            "THR ",
            loadout.thruster.as_ref().map(|t| (t.rarity.color(), t.rarity.label(), "THRUSTER", t.stat_summary())),
        );
    }

    fn draw_loadout_slot(
        &self,
        x: f32, y: f32,
        label: &str,
        item: Option<(Color, &str, &str, String)>,
    ) {
        draw_text(label, x, y, 13.0, DARKGRAY);
        if let Some((color, tier, name, stats)) = item {
            let tx = x + 36.0;
            draw_text(&format!("[{}]", tier), tx, y, 13.0, color);
            let tw = measure_text(&format!("[{}]", tier), None, 13, 1.0).width;
            draw_text(
                &format!(" {} {}", name, stats),
                tx + tw,
                y,
                11.0,
                Color::new(0.7, 0.7, 0.7, 1.0),
            );
        } else {
            draw_text("--- empty ---", x + 36.0, y, 11.0, Color::new(0.3, 0.3, 0.3, 1.0));
        }
    }
}
