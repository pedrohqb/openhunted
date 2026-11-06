// SPDX-License-Identifier: GPL-3.0-or-later
//! Projéteis dos inimigos (`docs/re/projeteis.md`): bala da Morte (99), bala do Bruiser
//! (100), bala da sentinela (106, `0x40caf6`) e o foguete inimigo (21). Voam retos sem
//! gravidade (`0x40c964`), param na parede e ferem o Jake pela linha do `DAMAGE.TBL`.

use super::{Blast, Ctx, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};

pub const ROCKET: u32 = 21;
pub const DEATH_BULLET: u32 = 99;
pub const BRUISER_BULLET: u32 = 100;
pub const SENTINEL_BULLET: u32 = 106;

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
fn impact(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let r = row(e.kind);
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
        out.effects.push((b, 1 + ctx.rng.below(4) as u16));
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
    st.prev = Some((e.x, e.y));
    if e.bounds().is_some_and(|b| ctx.cells.blocked(&b)) {
        impact(e, ctx, out);
        return;
    }
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = vx;
    body.vy = vy;
    step(&mut body, ctx.cells, false);
    let moved = (body.x - e.x, body.y - e.y) == (vx >> 3, vy >> 3);
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
        impact(e, ctx, out);
    } else if !moved {
        impact(e, ctx, out);
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
}
