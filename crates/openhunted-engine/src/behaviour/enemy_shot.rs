// SPDX-License-Identifier: GPL-3.0-or-later
//! Projéteis dos inimigos (`docs/re/projeteis.md`): bala da Morte (99), bala do Bruiser
//! (100), bala da sentinela (106, `0x40caf6`), o foguete inimigo (21) e o dardo (29,
//! `0x421154`). Voam retos sem gravidade (`0x40c964`; o dardo cai), param na parede e nos
//! objetos sólidos e ferem o Jake pela linha do `DAMAGE.TBL`.

use super::{Blast, Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};

pub const ROCKET: u32 = 21;
pub const DEATH_BULLET: u32 = 99;
pub const BRUISER_BULLET: u32 = 100;
pub const SENTINEL_BULLET: u32 = 106;
pub const DART: u32 = crate::entity::DART;
/// Faísca do nascimento do dardo (`0x43d629(dardo, nulo, 1)`: efeito 14).
pub const DART_SPARK: u16 = 14;

/// Linha `(atacante, 0)` do `DAMAGE.TBL`: dano direto, empurrão (reação 2), raio do efeito
/// (sorteado 1–4 no `oex.anm`), empurrão e dano de área.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub direct: i32,
    pub shove: bool,
    pub radius: i32,
    pub area_push: i32,
    pub area_damage: i32,
}

pub fn row(kind: u32) -> Row {
    match kind {
        // (21, 0): 20 direto, raio 64, empurrão 30, área 50
        ROCKET => Row {
            direct: 20,
            shove: false,
            radius: 64,
            area_push: 30,
            area_damage: 50,
        },
        // (100, 0): 15 direto, raio 28, empurrão de área 30, dano de área 10
        BRUISER_BULLET => Row {
            direct: 15,
            shove: false,
            radius: 28,
            area_push: 30,
            area_damage: 10,
        },
        // (106, 0): 15 com empurrão, raio 18
        SENTINEL_BULLET => Row {
            direct: 15,
            shove: true,
            radius: 18,
            area_push: 0,
            area_damage: 0,
        },
        // (29, …): 1 com empurrão, sem efeito
        DART => Row {
            direct: 1,
            shove: true,
            radius: 0,
            area_push: 0,
            area_damage: 0,
        },
        // (99, 0): 15 com empurrão, raio 28
        _ => Row {
            direct: 15,
            shove: true,
            radius: 28,
            area_push: 0,
            area_damage: 0,
        },
    }
}

/// Animação, BRD e tamanho-alvo (`+0x1f0`) por tipo (`0x0040bff8`).
pub fn art(kind: u32) -> (&'static str, &'static str, i32) {
    match kind {
        ROCKET => ("ORO.ANM", "ORO00.BRD", 25),
        BRUISER_BULLET => ("OBB00.ANM", "OBB.BRD", 15),
        SENTINEL_BULLET => ("OSB00.ANM", "OSB00.BRD", 5),
        DART => ("ODA.ANM", "ODA.BRD", 12),
        _ => ("ODS00.ANM", "ODS.BRD", 20),
    }
}

#[derive(Debug, Clone)]
pub struct EnemyShotState {
    /// Velocidade em 1/8 px por tique.
    pub vx: i32,
    pub vy: i32,
    /// Posição antes do último passo (acerto varrido).
    pub prev: Option<(i32, i32)>,
    /// Já passou o primeiro tique (o dardo solta a faísca ao nascer).
    pub started: bool,
}

/// Caixa de reserva (sem BRD).
pub const BOX: Rect = Rect {
    x: 0,
    y: 0,
    w: 4,
    h: 4,
};

/// Projétil novo em `at` com velocidade `vel` em px/tique (sinalizadores `0x2011 | 0x8`).
pub fn entity(kind: u32, at: (i32, i32), vel: (i32, i32), layer: i32) -> Entity {
    Entity {
        kind,
        x: at.0,
        y: at.1,
        flags: 0x2019,
        layer,
        id: 0,
        values: vec![],
        triggers: vec![],
        objective: 0,
        health: 1,
        armor: 0,
        boxes: vec![BOX],
        solid_shapes: false,
        on: false,
        alive: true,
        sprite: None,
        behaviour: Behaviour::EnemyShot(EnemyShotState {
            vx: vel.0 * SUB,
            vy: vel.1 * SUB,
            prev: None,
            started: false,
        }),
    }
}

fn swept(e: &Entity, prev: Option<(i32, i32)>) -> Option<Rect> {
    let now = e.bounds()?;
    let Some((px, py)) = prev else {
        return Some(now);
    };
    let (bx, by) = (now.x + px - e.x, now.y + py - e.y);
    let x0 = now.x.min(bx);
    let y0 = now.y.min(by);
    Some(Rect {
        x: x0,
        y: y0,
        w: (now.x + now.w).max(bx + now.w) - x0,
        h: (now.y + now.h).max(by + now.h) - y0,
    })
}

/// Impacto: efeito sorteado 1–4 e, com dano de área, a explosão (`0x0040d20c`).
fn impact(e: &mut Entity, rng: &mut crate::rng::Rng, out: &mut Out) {
    let r = row(e.kind);
    if e.kind == DART {
        e.alive = false;
        return;
    }
    let b = Blast {
        center: e.center(),
        layer: e.layer,
        radius: r.radius,
        push: r.area_push,
        damage: r.area_damage,
    };
    if r.area_damage > 0 {
        out.blasts.push(b);
    } else {
        out.effects.push((b, 1 + rng.below(4) as u16));
    }
    e.alive = false;
}

/// Um tique: nasceu na parede → impacto; voa; parede → impacto; fora da fase → some; Jake
/// na caixa varrida → dano direto e impacto.
pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let boxr = e.boxes.first().copied().unwrap_or(BOX);
    let Behaviour::EnemyShot(st) = &mut e.behaviour else {
        return;
    };
    let (vx, vy) = (st.vx, st.vy);
    let gravity = e.kind == DART;
    let spark = gravity && !st.started;
    st.started = true;
    st.prev = Some((e.x, e.y));
    if spark {
        out.effects.push((
            Blast {
                center: e.center(),
                layer: e.layer,
                radius: 12,
                push: 0,
                damage: 0,
            },
            DART_SPARK,
        ));
    }
    if e.bounds()
        .is_some_and(|b| ctx.cells_for(e.layer).blocked(&b))
    {
        impact(e, ctx.rng, out);
        return;
    }
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = vx;
    body.vy = vy;
    step(&mut body, ctx.cells_for(e.layer), gravity);
    let fall = if gravity { crate::physics::GRAVITY } else { 0 };
    let moved = (body.x - e.x, body.y - e.y) == (vx >> 3, (vy + fall) >> 3);
    if let Behaviour::EnemyShot(st) = &mut e.behaviour {
        st.vy = vy + fall;
    }
    e.x = body.x;
    e.y = body.y;
    let c = e.center();
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.alive = false;
        return;
    }
    let prev = match &e.behaviour {
        Behaviour::EnemyShot(s) => s.prev,
        _ => None,
    };
    let hit = super::enemy::jake_target(ctx, e.layer)
        .zip(swept(e, prev))
        .is_some_and(|(j, b)| j.overlaps(&b));
    if hit {
        let r = row(e.kind);
        out.jake_damage.add(&Pending {
            amount: r.direct,
            shove: r.shove,
            ..Default::default()
        });
        impact(e, ctx.rng, out);
    } else if !moved {
        impact(e, ctx.rng, out);
    }
}

/// Linha `(106, 28)` do `DAMAGE.TBL`: a bala da sentinela no barril dá 5 de natureza 2
/// (explosão: detona), derrubada, e toca 5056.
pub const SENTINEL_ON_BARREL: i32 = 5;
pub const SOUND_SENTINEL_ON_BARREL: u16 = 5056;

/// Tiros inimigos contra os objetos sólidos da camada (portas `OH`, barris, peças…): o tiro
/// para e a vítima leva a linha `(tipo, vítima)` do `DAMAGE.TBL` — a `(106, 28)` no barril,
/// senão a `(tipo, 0)` (dano direto, sem natureza). Ruling: os inimigos da M7a não param
/// nem são feridos pelos tiros (eles nascem nas bocas, dentro das caixas de quem atira).
pub fn object_hits(entities: &mut [Entity], rng: &mut crate::rng::Rng, out: &mut Out) {
    for si in 0..entities.len() {
        let shot = &entities[si];
        let Behaviour::EnemyShot(st) = &shot.behaviour else {
            continue;
        };
        if !shot.alive {
            continue;
        }
        let Some(sb) = swept(shot, st.prev) else {
            continue;
        };
        let (layer, kind) = (shot.layer, shot.kind);
        let victim = (0..entities.len()).find(|&i| {
            let e = &entities[i];
            i != si
                && e.alive
                && e.solid_shapes
                && e.layer == layer
                && e.flags & 0x80003 == 0x80001
                && e.flags & 0x20 == 0
                && !crate::entity::ENEMIES.contains(&e.kind)
                && !matches!(
                    e.behaviour,
                    Behaviour::EnemyShot(_)
                        | Behaviour::Bullet(_)
                        | Behaviour::Casing(_)
                        | Behaviour::Laser(_)
                )
                && e.bounds().is_some_and(|b| b.overlaps(&sb))
        });
        let Some(vi) = victim else { continue };
        let p = if kind == SENTINEL_BULLET && entities[vi].kind == crate::entity::BARREL {
            out.events.push(Event::Sound(crate::sound::Cue::at(
                SOUND_SENTINEL_ON_BARREL,
                crate::sound::PRIORITY_FRAME,
                entities[si].center(),
                layer,
            )));
            Pending {
                amount: SENTINEL_ON_BARREL,
                explosion: true,
                knockdown: true,
                ..Default::default()
            }
        } else {
            let r = row(kind);
            Pending {
                amount: r.direct,
                shove: r.shove,
                ..Default::default()
            }
        };
        super::interact::hurt(&mut entities[vi], &p);
        impact(&mut entities[si], rng, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake};

    #[test]
    fn sentinel_bullet_hurts_jake_with_a_shove() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 0);
        let mut c = ctx(&j, &w, &on);
        let mut b = entity(SENTINEL_BULLET, (40, 20), (20, 0), 0);
        let mut out = Out::default();
        for _ in 0..5 {
            if b.alive {
                tick(&mut b, &mut c, &mut out);
            }
        }
        assert!(!b.alive);
        assert_eq!(out.jake_damage.amount, 15);
        assert!(out.jake_damage.shove);
        assert!(out.blasts.is_empty());
        assert_eq!(out.effects.len(), 1);
        assert_eq!(out.effects[0].0.radius, 18);
    }

    #[test]
    fn bruiser_bullet_and_rocket_blast() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 0);
        let mut c = ctx(&j, &w, &on);
        for (kind, direct, area) in [(BRUISER_BULLET, 15, 10), (ROCKET, 20, 50)] {
            let mut b = entity(kind, (160, 20), (-20, 0), 0);
            let mut out = Out::default();
            while b.alive {
                tick(&mut b, &mut c, &mut out);
            }
            assert_eq!(out.jake_damage.amount, direct);
            assert!(!out.jake_damage.shove);
            assert_eq!(out.blasts.len(), 1);
            assert_eq!(out.blasts[0].damage, area);
        }
    }

    #[test]
    fn shot_dies_in_walls_and_outside() {
        let on = |_: i32| None;
        let (j, _) = ctx_with_jake(1000, 0);
        let wall = crate::World::new(vec![Rect {
            x: 90,
            y: 0,
            w: 10,
            h: 100,
        }]);
        let mut c = ctx(&j, &wall, &on);
        let mut out = Out::default();
        let mut b = entity(BRUISER_BULLET, (60, 20), (20, 0), 0);
        tick(&mut b, &mut c, &mut out);
        tick(&mut b, &mut c, &mut out);
        assert!(!b.alive);
        assert_eq!(out.blasts.len(), 1, "parede: explosão do (100, 0)");
        let mut inner = entity(SENTINEL_BULLET, (92, 20), (20, 0), 0);
        tick(&mut inner, &mut c, &mut out);
        assert!(!inner.alive);
        let mut gone = entity(SENTINEL_BULLET, (5, 20), (-20, 0), 0);
        tick(&mut gone, &mut c, &mut out);
        assert!(!gone.alive);
        assert_eq!(out.jake_damage.amount, 0);
    }

    #[test]
    fn shots_stop_at_solid_objects_and_sentinel_bullets_blow_barrels() {
        use crate::behaviour::barrel::BarrelState;
        use crate::behaviour::test_support::logic;
        let solid = |kind: u32, x: i32| {
            let mut o = logic(kind, x, 0, &[]);
            o.flags = 0x80181;
            o.solid_shapes = true;
            o.boxes = vec![Rect {
                x: 0,
                y: 0,
                w: 20,
                h: 40,
            }];
            o
        };
        let mut barrel = solid(crate::entity::BARREL, 100);
        barrel.behaviour = Behaviour::Barrel(BarrelState::new(1, vec![]));
        let door = solid(50, 100);
        let sentinel = solid(crate::entity::SENTINEL, 100);
        let mut rng = crate::rng::Rng::new(1);
        // (106, 28): 5 de dano, natureza 2 (explosão) → o barril detona
        let mut v = vec![entity(SENTINEL_BULLET, (98, 20), (20, 0), 0), barrel];
        let mut out = Out::default();
        object_hits(&mut v, &mut rng, &mut out);
        assert!(!v[0].alive, "o tiro para no barril");
        let Behaviour::Barrel(b) = &v[1].behaviour else {
            panic!()
        };
        assert!(b.hurt.explosion && b.hurt.amount == 5, "{:?}", b.hurt);
        assert!(out.events.iter().any(|e| e.sound_id() == Some(5056)));
        // porta OH: para (efeito do impacto), sem dano
        let mut v = vec![entity(BRUISER_BULLET, (98, 20), (20, 0), 0), door];
        let mut out = Out::default();
        object_hits(&mut v, &mut rng, &mut out);
        assert!(!v[0].alive);
        assert_eq!(out.blasts.len(), 1, "explosão do (100, 0)");
        // outra camada ou um inimigo: passa
        let mut v = vec![entity(BRUISER_BULLET, (98, 20), (20, 0), 1), solid(50, 100)];
        object_hits(&mut v, &mut rng, &mut Out::default());
        assert!(v[0].alive, "outra camada");
        let mut v = vec![entity(SENTINEL_BULLET, (98, 20), (20, 0), 0), sentinel];
        object_hits(&mut v, &mut rng, &mut Out::default());
        assert!(v[0].alive, "os tiros não ferem nem param nos inimigos");
    }

    #[test]
    fn dart_falls_and_hurts_one_with_a_shove() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 0);
        let mut c = ctx(&j, &w, &on);
        let mut d = entity(DART, (40, 10), (20, 0), 0);
        let mut out = Out::default();
        tick(&mut d, &mut c, &mut out);
        assert_eq!(out.effects[0].1, DART_SPARK, "faísca ao nascer");
        assert!(d.y > 10, "a gravidade puxa o dardo");
        while d.alive {
            tick(&mut d, &mut c, &mut out);
        }
        assert_eq!(out.jake_damage.amount, 1);
        assert!(out.jake_damage.shove);
        assert_eq!(out.effects.len(), 1, "sem efeito no acerto");
    }
}
