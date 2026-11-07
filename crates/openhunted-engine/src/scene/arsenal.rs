// SPDX-License-Identifier: GPL-3.0-or-later
//! Arsenal do Jake na cena (M7b): projéteis das armas 3–6 e o soco
//! (`docs/re/armas-2.md`, `furtividade.md`).

use super::Scene;
use crate::behaviour::{self, Blast, Event, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Rect, SUB};
use crate::sound::{Cue, PRIORITY_FRAME};

/// Linha `(1, 0, 0)` do `DAMAGE.TBL` (soco do Jake): 8 de dano, empurrão (2, 0), som 5062.
pub const PUNCH_DAMAGE: i32 = 8;
pub const PUNCH_PUSH: i32 = 2;
pub const SOUND_PUNCH: u16 = 5062;

/// Animações e BRDs dos projéteis das armas 3–6 (`docs/re/projeteis.md`, tabela de tipos).
pub const ANIMS: [&str; 7] = [
    "OBK.ANM",
    "ORO.ANM",
    "ONS.ANM",
    "OGR.ANM",
    "OSH00.ANM",
    "OKN.ANM",
    "OWB.ANM",
];
pub const BORDERS: [&str; 10] = [
    "OKN01.BRD",
    "OWB.BRD",
    "OBK00.BRD",
    "OBK01.BRD",
    "OBK02.BRD",
    "ORO00.BRD",
    "ORO01.BRD",
    "ONS2.BRD",
    "OGR2.BRD",
    "OSH.BRD",
];

/// Projétil das armas 3–6 em voo: (posição anterior, já parado no cenário).
fn flight(e: &Entity) -> Option<(Option<(i32, i32)>, bool)> {
    match &e.behaviour {
        Behaviour::Pellet(s) => Some((s.prev, s.spent)),
        Behaviour::Rocket(s) => Some((s.prev, s.spent)),
        Behaviour::Star(s) if s.in_flight() => Some((s.prev, false)),
        Behaviour::Grenade(s) => Some((s.prev, false)),
        _ => None,
    }
}

impl Scene {
    /// Entidade de um projétil das armas 3–6 (chumbo, foguete, estrela, granada, cartucho).
    pub(super) fn weapon_entity(&mut self, s: &crate::jake::Shot, layer: i32) -> Option<Entity> {
        use crate::behaviour::{grenade, pellet, rocket, star};
        use crate::jake::ShotKind;
        let (vx, vy) = (s.vx * SUB, s.vy * SUB);
        let (dir_label, mirror) = behaviour::bullet::label_and_mirror(s.vx, s.vy);
        let (mut x, mut y) = (s.x, s.y);
        let (kind, anim, label, mirror, brd, target, b) = match s.kind {
            ShotKind::Pellet => (
                crate::entity::PELLET,
                "OBK.ANM",
                dir_label,
                mirror,
                ["OBK00.BRD", "OBK00.BRD", "OBK01.BRD", "OBK02.BRD"][dir_label as usize],
                10,
                Behaviour::Pellet(pellet::PelletState::new(vx, vy)),
            ),
            ShotKind::Rocket => (
                crate::entity::ROCKET,
                "ORO.ANM",
                dir_label.min(2),
                mirror,
                if dir_label == 1 {
                    "ORO00.BRD"
                } else {
                    "ORO01.BRD"
                },
                25,
                Behaviour::Rocket(rocket::RocketState::new(vx, vy)),
            ),
            ShotKind::Star => {
                // 1º tique: avança 0–39/8 px na direção do voo
                let r = self.rng.below(star::FIRST_ADVANCE_RAND);
                let (dx, dy) = star::first_advance((vx, vy), r);
                (x, y) = (x + dx, y + dy);
                (
                    crate::entity::STAR,
                    "ONS.ANM",
                    star::LABEL_FLY,
                    0,
                    "ONS2.BRD",
                    10,
                    Behaviour::Star(star::StarState::new(vx, vy)),
                )
            }
            ShotKind::Grenade => {
                let fuse = grenade::FUSE_MIN + self.rng.below(grenade::FUSE_RAND);
                (
                    crate::entity::GRENADE,
                    "OGR.ANM",
                    grenade::LABEL_THROWN,
                    u8::from(s.vx < 0),
                    "OGR2.BRD",
                    8,
                    Behaviour::Grenade(grenade::GrenadeState::new(vx, vy, fuse)),
                )
            }
            ShotKind::Shell => (
                crate::entity::SHELL,
                "OSH00.ANM",
                1,
                0,
                "OSH.BRD",
                3,
                Behaviour::Casing(behaviour::casing::CasingState {
                    vx,
                    vy,
                    slow: 0,
                    clinked: false,
                }),
            ),
            ShotKind::Knife => (
                crate::entity::KNIFE,
                "OKN.ANM",
                star::KNIFE_LABEL_FLY,
                u8::from(s.vx < 0),
                "OKN01.BRD",
                3,
                Behaviour::Star(star::StarState::knife(vx, vy)),
            ),
            ShotKind::Whip => (
                crate::entity::WHIP_SHOT,
                "OWB.ANM",
                1,
                0,
                "OWB.BRD",
                10,
                Behaviour::Pellet(pellet::PelletState::whip(vx, vy)),
            ),
            ShotKind::Bullet | ShotKind::Casing => return None,
        };
        self.projectile_entity(
            super::Projectile {
                kind,
                anim,
                label,
                mirror,
                brd,
                target,
                x,
                y,
                layer,
            },
            b,
        )
    }

    /// Projéteis das armas 3–6 contra os objetos sólidos da camada e contra o cenário
    /// (`0x43d629` com a linha de cada tipo): dano direto em quem trata dano, explosões de
    /// área na lista de explosões do tique, faísca do chumbo.
    pub(super) fn weapon_hits(&mut self, out: &mut Out) {
        let n = self.entities.len();
        for i in 0..n {
            let e = &self.entities[i];
            if !e.alive {
                continue;
            }
            let Some((prev, spent)) = flight(e) else {
                continue;
            };
            let Some(sw) = behaviour::projectile::swept(e, prev) else {
                continue;
            };
            let (layer, kind) = (e.layer, e.kind);
            let victim = (0..n).find(|&k| {
                let v = &self.entities[k];
                k != i
                    && v.alive
                    && v.solid_shapes
                    && v.layer == layer
                    && v.flags & 0x80003 == 0x80001
                    && v.flags & 0x20 == 0
                    && flight(v).is_none()
                    && !matches!(
                        v.behaviour,
                        Behaviour::Bullet(_)
                            | Behaviour::Casing(_)
                            | Behaviour::Explosion(_)
                            | Behaviour::EnemyShot(_)
                    )
                    && !(kind == crate::entity::GRENADE && behaviour::grenade::ignores(v.kind))
                    && v.bounds().is_some_and(|b| b.overlaps(&sw))
            });
            if victim.is_none() && !spent {
                continue;
            }
            self.weapon_impact(i, victim, out);
        }
    }

    /// Impacto do projétil `i` (na vítima, ou no cenário com `None`).
    fn weapon_impact(&mut self, i: usize, victim: Option<usize>, out: &mut Out) {
        use crate::behaviour::{grenade, pellet, rocket, star};
        let e = &self.entities[i];
        let (c, layer) = (e.center(), e.layer);
        let dir = match &e.behaviour {
            Behaviour::Pellet(s) => s.vx.signum(),
            Behaviour::Rocket(s) => s.vx.signum(),
            Behaviour::Star(s) => s.vx.signum(),
            _ => 0,
        };
        let (direct, blast) = match &e.behaviour {
            Behaviour::Pellet(s) if s.whip => (
                Pending {
                    amount: pellet::WHIP_DAMAGE,
                    knockdown: true,
                    push: (dir * pellet::PUSH, 0),
                    ..Default::default()
                },
                None,
            ),
            Behaviour::Star(s) if s.knife => (
                Pending {
                    amount: star::KNIFE_DAMAGE,
                    shove: true,
                    push: (dir * 2, 0),
                    ..Default::default()
                },
                None,
            ),
            Behaviour::Pellet(_) => (
                Pending {
                    amount: pellet::DAMAGE,
                    explosion: true,
                    knockdown: true,
                    push: (dir * pellet::PUSH, 0),
                    ..Default::default()
                },
                None,
            ),
            Behaviour::Rocket(_) => (
                Pending {
                    amount: rocket::DAMAGE,
                    ..Default::default()
                },
                Some((rocket::RADIUS, rocket::PUSH, rocket::AREA)),
            ),
            Behaviour::Star(_) => (
                Pending {
                    amount: star::DAMAGE,
                    explosion: true,
                    shove: true,
                    ..Default::default()
                },
                Some((star::RADIUS, star::PUSH, star::AREA)),
            ),
            Behaviour::Grenade(_) => (
                Pending::default(),
                Some((grenade::RADIUS, grenade::PUSH, grenade::AREA)),
            ),
            _ => return,
        };
        self.entities[i].alive = false;
        if let Some(vi) = victim.filter(|_| !direct.is_empty()) {
            // as mortes contam pela cena (`enemy::kills`), qualquer que seja a causa
            let v = &mut self.entities[vi];
            let _ =
                behaviour::enemy::hurt(v, &direct, true) || behaviour::interact::hurt(v, &direct);
        }
        match blast {
            Some((radius, push, damage)) => out.blasts.push(Blast {
                center: c,
                layer,
                radius,
                push,
                damage,
            }),
            None => {
                // faísca `oex15` de raio 12 e o som do impacto
                out.effects.push((
                    Blast {
                        center: c,
                        layer,
                        radius: pellet::IMPACT_RADIUS,
                        push: 0,
                        damage: 0,
                    },
                    behaviour::bullet::IMPACT_LABEL,
                ));
                out.events.push(Event::Sound(Cue::at(
                    behaviour::bullet::SOUND_WALL,
                    PRIORITY_FRAME,
                    c,
                    layer,
                )));
            }
        }
    }

    /// Acerto do soco (`0x0043afe1`): cada objeto da camada (presente, `0x80001`, sem
    /// `0x8000`) cuja caixa cruza a do golpe leva a linha `(1, 0, 0)`; só quem trata dano
    /// (`enemy::hurt`, com crédito, e `interact::hurt`) reage.
    pub(super) fn apply_punch(&mut self, r: Rect) {
        let right = self.jake.as_ref().is_some_and(|j| j.facing_right());
        let p = self.jake.as_ref().and_then(|j| j.blow).unwrap_or(Pending {
            amount: PUNCH_DAMAGE,
            shove: true,
            push: (if right { PUNCH_PUSH } else { -PUNCH_PUSH }, 0),
            ..Default::default()
        });
        for e in self.entities.iter_mut() {
            if !e.alive
                || e.layer != self.jake_layer
                || e.flags & 0x80001 != 0x80001
                || e.flags & 0x8000 != 0
                || !e.bounds().is_some_and(|b| b.overlaps(&r))
            {
                continue;
            }
            if behaviour::enemy::hurt(e, &p, true) || behaviour::interact::hurt(e, &p) {
                self.events.push(Event::Sound(Cue::at(
                    SOUND_PUNCH,
                    PRIORITY_FRAME,
                    e.center(),
                    e.layer,
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::laser_eye::LaserEyeState;
    use crate::behaviour::test_support::logic;
    use crate::entity::LASER_EYE;

    fn eye(x: i32) -> Entity {
        let mut e = logic(LASER_EYE, x, 0, &[]);
        e.flags = 0x80809;
        e.solid_shapes = true;
        e.behaviour = Behaviour::LaserEye(LaserEyeState::new(&[0]));
        e
    }

    /// Cena de 2000 × 1000 com chão em y = 100, parede em x = 400 e o Jake parado em
    /// `x` com a arma `w` (munição 10), olhando para a direita.
    fn range(x: i32, w: usize) -> Scene {
        range_as(x, w, crate::character::Character::Jake)
    }

    fn range_as(x: i32, w: usize, ch: crate::character::Character) -> Scene {
        use std::rc::Rc;
        let mut s = Scene::empty(2000, 1000);
        let anm = Rc::new(
            openhunted_formats::Animation::parse(&openhunted_formats::test_support::mkanm())
                .unwrap(),
        );
        for n in ANIMS.iter().chain(&["OEX.ANM"]) {
            s.projectile_anims.insert(n.to_string(), anm.clone());
        }
        s.explosion_anim = Some(anm.clone());
        s.jake_sprite = Some(crate::Sprite {
            layer: crate::Layer::Object,
            player: crate::AnimPlayer::from_frame(&anm, 0),
            anim: anm,
            x,
            y: 53,
            frozen: false,
            scale: (1, 1),
            mirror: false,
        });
        let solids = vec![
            Rect {
                x: -100,
                y: 100,
                w: 2200,
                h: 20,
            },
            Rect {
                x: 400,
                y: -100,
                w: 20,
                h: 300,
            },
        ];
        s.cell_solids = solids.clone();
        s.cell_world = crate::World::new(solids);
        let mut inv = crate::jake::Inventory::for_character(&[(0, 0), (1, 20)], ch);
        inv.slots[w] = (1, 10);
        inv.weapon = w;
        let j = crate::Jake::new(
            x,
            53,
            Rect {
                x: 2,
                y: 3,
                w: 18,
                h: 44,
            },
            std::collections::HashMap::new(),
        )
        .with_character(ch)
        .with_inventory(inv);
        s.jake = Some(j);
        s.rebuild_world();
        s.tick(crate::input::RIGHT);
        s.tick(0);
        s
    }

    fn count(s: &Scene, kind: u32) -> usize {
        s.entities
            .iter()
            .filter(|e| e.alive && e.kind == kind)
            .count()
    }

    #[test]
    fn shotgun_pellet_splits_into_four_with_a_shell() {
        let mut s = range(100, crate::jake::SHOTGUN);
        s.tick(crate::input::SHOOT);
        s.tick(0);
        assert_eq!(count(&s, crate::entity::SHELL), 1, "cartucho");
        let mut max = 0;
        for _ in 0..6 {
            s.tick(0);
            max = max.max(count(&s, crate::entity::PELLET));
        }
        assert_eq!(max, 4);
        let vys: Vec<i32> = s
            .entities
            .iter()
            .filter_map(|e| match &e.behaviour {
                Behaviour::Pellet(p) if e.alive => Some(p.vy),
                _ => None,
            })
            .collect();
        // 20 px/tique = 160/8: −11,25°, 0°, 0°, +11,25° (≈ ±31 em 1/8 px)
        let mut vys = vys;
        vys.sort_unstable();
        assert!(vys[1].abs() <= 1 && vys[2].abs() <= 1, "{vys:?}");
        assert!(
            (29..=33).contains(&-vys[0]) && (29..=33).contains(&vys[3]),
            "{vys:?}"
        );
        // batem na parede em x = 400: faíscas e nada sobra
        for _ in 0..20 {
            s.tick(0);
        }
        assert_eq!(count(&s, crate::entity::PELLET), 0);
        assert!(s.events.iter().any(|e| e.sound_id() == Some(3046)));
    }

    #[test]
    fn rocket_blast_near_the_wall_hurts_jake() {
        let mut s = range(330, crate::jake::ROCKETS);
        let hp = s.jake.as_ref().unwrap().health.hp;
        s.tick(crate::input::SHOOT);
        for _ in 0..6 {
            s.tick(0);
        }
        assert_eq!(count(&s, crate::entity::ROCKET), 0, "explodiu na parede");
        s.tick(0);
        let j = s.jake.as_ref().unwrap();
        assert!(
            j.health.hp < hp,
            "a área (R = 96) fere o Jake: {}",
            j.health.hp
        );
        // longe da parede, nada
        let mut far = range(100, crate::jake::ROCKETS);
        far.tick(crate::input::SHOOT);
        for _ in 0..20 {
            far.tick(0);
        }
        assert_eq!(far.jake.as_ref().unwrap().health.hp, hp);
    }

    #[test]
    fn star_sticks_in_the_wall_then_vanishes() {
        let mut s = range(250, crate::jake::STARS);
        s.tick(crate::input::SHOOT);
        let mut stuck = None;
        for t in 0..40 {
            s.tick(0);
            if stuck.is_none()
                && s.entities.iter().any(|e| {
                    e.alive && matches!(&e.behaviour, Behaviour::Star(st) if !st.in_flight())
                })
            {
                stuck = Some(t);
            }
        }
        assert!(stuck.is_some(), "a estrela cravou");
        assert!(s.events.iter().any(|e| e.sound_id() == Some(3030)));
        for _ in 0..110 {
            s.tick(0);
        }
        assert_eq!(count(&s, crate::entity::STAR), 0, "100 tiques depois some");
    }

    #[test]
    fn stuck_star_is_picked_up_for_one_star() {
        let mut s = range(250, crate::jake::STARS);
        s.tick(crate::input::SHOOT);
        for _ in 0..40 {
            s.tick(0);
        }
        let ammo = s.jake.as_ref().unwrap().inventory.ammo(crate::jake::STARS);
        // leva o Jake até a estrela cravada
        let st = s
            .entities
            .iter()
            .find(|e| e.alive && e.kind == crate::entity::STAR)
            .map(|e| e.center())
            .unwrap();
        if let Some(j) = s.jake.as_mut() {
            j.body.x = st.0 - 10;
            j.body.y = st.1 - 20;
        }
        s.tick(0);
        assert_eq!(count(&s, crate::entity::STAR), 0);
        assert_eq!(
            s.jake.as_ref().unwrap().inventory.ammo(crate::jake::STARS),
            ammo + 1
        );
    }

    #[test]
    fn grenade_bounces_off_the_wall_and_explodes_on_the_fuse() {
        let mut s = range(300, crate::jake::GRENADES);
        s.tick(crate::input::SHOOT);
        let mut gone = None;
        let mut bounced = false;
        for t in 0..60 {
            s.tick(0);
            bounced |= s.events.iter().any(|e| e.sound_id() == Some(3042));
            if gone.is_none() && t > 4 && count(&s, crate::entity::GRENADE) == 0 {
                gone = Some(t);
            }
        }
        assert!(bounced, "quicou na parede");
        let t = gone.expect("explodiu");
        assert!((30..=50).contains(&t), "pavio de 35–44 tiques: {t}");
        assert!(
            s.entities
                .iter()
                .any(|e| e.kind == crate::entity::EXPLOSION)
                || t < 60
        );
    }

    fn grenade_vx(s: &Scene) -> Option<i32> {
        s.entities.iter().find_map(|e| match &e.behaviour {
            Behaviour::Grenade(g) if e.alive => Some(g.vx),
            _ => None,
        })
    }

    /// Na parede a granada inverte vx (sem perder rapidez) e espelha; o som 3042 só toca
    /// no choque de parede (`0x40707c`: normal mais horizontal que vertical).
    #[test]
    fn grenade_wall_bounce_inverts_vx_and_mirrors() {
        let mut s = range(300, crate::jake::GRENADES);
        s.tick(crate::input::SHOOT);
        let mut last: Option<i32> = None;
        let mut seen = false;
        for _ in 0..40 {
            s.tick(0);
            let Some(vx) = grenade_vx(&s) else {
                if last.is_some() {
                    break;
                }
                continue;
            };
            let wall = s.events.iter().any(|e| e.sound_id() == Some(3042));
            if let (Some(before), true) = (last, wall) {
                if !seen {
                    assert_eq!(vx, -before, "inverte vx sem perder rapidez");
                    let g = s.entities.iter().find(|e| e.kind == crate::entity::GRENADE);
                    assert_eq!(
                        g.and_then(|e| e.sprite.as_ref()).map(|sp| sp.mirror),
                        Some(before > 0),
                        "espelha ao quicar"
                    );
                }
                seen = true;
            }
            last = Some(vx);
        }
        assert!(seen, "quicou na parede");
        // sem parede: só o chão, nenhum som de quique
        let mut s = range(100, crate::jake::GRENADES);
        let floor = vec![s.cell_solids[0]];
        s.cell_solids = floor.clone();
        s.cell_world = crate::World::new(floor);
        s.rebuild_world();
        s.tick(crate::input::SHOOT);
        for _ in 0..30 {
            s.tick(0);
            assert!(
                !s.events.iter().any(|e| e.sound_id() == Some(3042)),
                "quique no chão não toca o som"
            );
        }
    }

    /// Explosão sobre a granada e a estrela cravada (`armas-2.md`): a granada com dano de
    /// explosão pendente detona no tique seguinte (`0x40d819`); a estrela cravada some. Em
    /// voo ou fora do raio, nada.
    #[test]
    fn blast_detonates_grenades_and_removes_stuck_stars() {
        use crate::behaviour::{grenade::GrenadeState, star::StarState};
        let b = Blast {
            center: (100, 100),
            layer: 0,
            radius: 30,
            push: 30,
            damage: 20,
        };
        let mk = |kind, x, st: Behaviour| {
            let mut e = logic(kind, x, 84, &[]);
            e.behaviour = st;
            e
        };
        let mut stuck = StarState::new(80, 0);
        stuck.stuck = Some(50);
        let mut es = [
            mk(
                crate::entity::GRENADE,
                84,
                Behaviour::Grenade(GrenadeState::new(0, 0, 30)),
            ),
            mk(
                crate::entity::GRENADE,
                400,
                Behaviour::Grenade(GrenadeState::new(0, 0, 30)),
            ),
            mk(crate::entity::STAR, 84, Behaviour::Star(stuck.clone())),
            mk(
                crate::entity::STAR,
                84,
                Behaviour::Star(StarState::new(80, 0)),
            ),
            mk(crate::entity::STAR, 400, Behaviour::Star(stuck)),
        ];
        for e in es.iter_mut() {
            behaviour::interact::blast(&b, e);
        }
        let fuse = |e: &Entity| match &e.behaviour {
            Behaviour::Grenade(g) => g.fuse,
            _ => u32::MAX,
        };
        assert_eq!((fuse(&es[0]), fuse(&es[1])), (0, 30), "a do raio detona");
        let alive: Vec<bool> = es[2..].iter().map(|e| e.alive).collect();
        assert_eq!(alive, vec![false, true, true], "só a cravada no raio some");
    }

    /// Na cena: o foguete que explode na parede detona a granada pousada ao lado.
    #[test]
    fn rocket_blast_sets_off_a_resting_grenade() {
        use crate::behaviour::grenade::GrenadeState;
        let mut s = range(100, crate::jake::ROCKETS);
        let mut g = logic(crate::entity::GRENADE, 380, 92, &[]);
        g.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        }];
        g.behaviour = Behaviour::Grenade(GrenadeState::new(0, 0, 1000));
        s.entities.push(g);
        s.tick(crate::input::SHOOT);
        for _ in 0..30 {
            s.tick(0);
        }
        assert_eq!(
            count(&s, crate::entity::GRENADE),
            0,
            "detonou pela explosão"
        );
    }

    /// Os projéteis das armas novas ferem os inimigos da M7a com crédito ao jogador: um
    /// foguete mata a mina saltadora (tipo 11) e conta.
    #[test]
    fn rocket_kills_a_hop_mine_for_the_player() {
        use crate::behaviour::hop_mine::{HEALTH, HopMineState};
        for w in [crate::jake::ROCKETS, crate::jake::SHOTGUN] {
            let mut s = range(100, w);
            // caixa alta (40 px) pousada no chão, na altura da boca da arma
            let mut m = logic(crate::entity::HOP_MINE, 220, 60, &[0, 0, 0, 0, 0, 4, 0, 5]);
            m.flags = 0x80801;
            m.health = HEALTH;
            m.solid_shapes = true;
            m.boxes = vec![Rect {
                x: 0,
                y: 0,
                w: 17,
                h: 40,
            }];
            m.behaviour = Behaviour::HopMine(HopMineState::default());
            s.entities.push(m);
            s.tick(crate::input::SHOOT);
            for _ in 0..12 {
                s.tick(0);
            }
            assert_eq!(count(&s, crate::entity::HOP_MINE), 0, "arma {w}");
            assert_eq!(s.kills, 1, "arma {w}");
        }
    }

    /// De ponta a ponta: SOCO (Z) com os punhos na cena fere a mina dentro da caixa do golpe
    /// (frente do Jake, terço do meio) e poupa a de fora.
    #[test]
    fn punch_in_the_scene_hurts_the_mine_in_the_box_only() {
        use crate::behaviour::mine::MineState;
        let mut s = range(100, crate::jake::FISTS);
        let mine = |x| {
            let mut m = logic(crate::entity::MINE, x, 66, &[0]);
            m.flags = 0x80801;
            m.boxes = vec![Rect {
                x: 0,
                y: 0,
                w: 16,
                h: 16,
            }];
            m.health = 100;
            m.behaviour = Behaviour::Mine(MineState::new(0));
            m
        };
        for _ in 0..10 {
            s.tick(0); // o Jake para de deslizar
        }
        // à frente do corpo: a mina vem ao Jake ~4 px por tique e entra na caixa do golpe
        // (5 px além do corpo) no tique do acerto
        let jb = s.jake.as_ref().unwrap().body.bounds();
        s.entities.push(mine(jb.x + jb.w + 9));
        s.entities.push(mine(600));
        let n = s.entities.len();
        // a vida da mina no tique do som do soco (depois ela pode chegar ao Jake e explodir)
        let mut hit = None;
        for _ in 0..12 {
            s.tick(crate::input::PUNCH);
            if hit.is_none() && s.events.iter().any(|e| e.sound_id() == Some(SOUND_PUNCH)) {
                hit = Some(s.entities[n - 2].health);
            }
        }
        assert_eq!(
            hit,
            Some(100 - PUNCH_DAMAGE),
            "a mina na caixa levou o soco"
        );
        let outside = &s.entities[n - 1];
        assert_eq!(outside.health, 100, "a de fora, não");
    }

    /// O soco fere quem cruza a caixa do golpe, com o som 5062; os de fora não.
    #[test]
    fn punch_hurts_what_the_blow_box_touches() {
        let mut s = Scene::empty(1000, 1000);
        s.entities = vec![eye(100), eye(300)];
        s.apply_punch(Rect {
            x: 120,
            y: 10,
            w: 14,
            h: 14,
        });
        let hurt: Vec<i32> = s
            .entities
            .iter()
            .map(|e| match &e.behaviour {
                Behaviour::LaserEye(st) => st.hurt.amount,
                _ => -1,
            })
            .collect();
        assert_eq!(hurt, vec![PUNCH_DAMAGE, 0]);
        assert_eq!(s.events.len(), 1);
        assert_eq!(s.events[0].sound_id(), Some(SOUND_PUNCH));
    }

    #[test]
    fn garathe_knife_sticks_and_is_picked_up_again() {
        use crate::character::Character;
        use crate::jake::weapons::KNIVES;
        let mut s = range_as(250, KNIVES, Character::Garathe);
        s.tick(crate::input::SHOOT);
        for _ in 0..40 {
            s.tick(0);
        }
        let knife = s
            .entities
            .iter()
            .find(|e| e.alive && e.kind == crate::entity::KNIFE)
            .expect("a faca cravada na parede");
        assert!(matches!(&knife.behaviour, Behaviour::Star(st) if st.knife && !st.in_flight()));
        let c = knife.center();
        let ammo = s.jake.as_ref().unwrap().inventory.ammo(KNIVES);
        assert_eq!(ammo, 9);
        if let Some(j) = s.jake.as_mut() {
            j.body.x = c.0 - 10;
            j.body.y = c.1 - 20;
        }
        s.tick(0);
        assert_eq!(s.jake.as_ref().unwrap().inventory.ammo(KNIVES), 10);
        assert_eq!(count(&s, crate::entity::KNIFE), 0);
    }

    #[test]
    fn garathe_whip_splits_until_the_fourth_generation() {
        use crate::character::Character;
        use crate::jake::weapons::WHIP;
        let mut s = range_as(1000, WHIP, Character::Garathe);
        s.tick(crate::input::LEFT);
        s.tick(crate::input::SHOOT);
        let mut most = (0, 0);
        for _ in 0..80 {
            s.tick(0);
            for e in s.entities.iter().filter(|e| e.alive) {
                if let Behaviour::Pellet(p) = &e.behaviour {
                    assert!(p.whip && p.generation <= 4);
                    most.0 = most.0.max(p.generation);
                }
            }
            most.1 = most.1.max(count(&s, crate::entity::WHIP_SHOT));
        }
        assert_eq!(most.0, 4, "até a 4ª geração");
        assert!(most.1 >= 4, "dividiu-se: {}", most.1);
        assert_eq!(
            count(&s, crate::entity::WHIP_SHOT),
            0,
            "somem no cenário ou fora"
        );
    }

    #[test]
    fn garathe_club_blow_uses_its_damage_row() {
        let mut s = Scene::empty(1000, 1000);
        s.entities = vec![eye(100)];
        let b = Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        };
        let mut j = crate::Jake::new(0, 0, b, Default::default());
        j.blow = Some(Pending {
            amount: 15,
            knockdown: true,
            push: (20, 10),
            ..Default::default()
        });
        s.jake = Some(j);
        s.apply_punch(Rect {
            x: 100,
            y: 10,
            w: 14,
            h: 14,
        });
        match &s.entities[0].behaviour {
            Behaviour::LaserEye(st) => assert_eq!(st.hurt.amount, 15),
            _ => unreachable!(),
        }
    }
}
