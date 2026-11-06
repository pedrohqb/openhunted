// SPDX-License-Identifier: GPL-3.0-or-later
//! Barril (tipo 28, `0x0040db74`/`0x0040dc00`; `docs/re/barril-fogo-cerca.md`): em pé,
//! tombando, rolando; explode com batida rápida, com dano de explosão ou fogo e por gatilho.

use super::{Blast, Ctx, Out, grow};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};
use crate::triggers::{armed, satisfied};

/// Estados (`+0x180`), que são também os rótulos do `OOB.ANM`.
pub const STANDING: u8 = 1;
pub const TIPPING: u8 = 2;
pub const ROLLING: u8 = 3;
/// Linha do `DAMAGE.TBL` do atacante 28: raio 96, empurrão 40, dano de área 50.
pub const BLAST_RADIUS: i32 = 96;
pub const BLAST_PUSH: i32 = 40;
pub const BLAST_DAMAGE: i32 = 50;
/// Batida acima de 10 px/tique (80 em 1/8 px) explode.
pub const CRASH_SPEED: i32 = 80;
/// Empurrão do personagem no barril que rola (`+0x164 −= 5` / `+= 7`).
pub const PUSH_LEFT: i32 = 5;
pub const PUSH_RIGHT: i32 = 7;
/// Rolando a ≥ 10 px/tique, o barril derruba quem encontra.
pub const KNOCK_SPEED: i32 = 10;

#[derive(Debug, Clone)]
pub struct BarrelState {
    pub state: u8,
    /// x ao começar a tombar (`+0x27c`): o quadro do rolamento segue a distância.
    pub start_x: i32,
    /// Velocidade em 1/8 px.
    pub vx: i32,
    pub vy: i32,
    /// Dano pendente (`+0x24c`, natureza `+0x252`).
    pub hurt: Pending,
    /// Caixas do `OOB02.BRD` (deitado), trocadas ao fim do tombo.
    pub rolling_boxes: Vec<Rect>,
}

impl BarrelState {
    /// Estado 3 se a fase deu 3, senão 1.
    pub fn new(state: i32, rolling_boxes: Vec<Rect>) -> Self {
        Self {
            state: if state == 3 { ROLLING } else { STANDING },
            start_x: 0,
            vx: 0,
            vy: 0,
            hurt: Pending::default(),
            rolling_boxes,
        }
    }
}

/// Caixa envolvente relativa das formas.
fn shape(boxes: &[Rect]) -> Rect {
    let x0 = boxes.iter().map(|b| b.x).min().unwrap_or(0);
    let y0 = boxes.iter().map(|b| b.y).min().unwrap_or(0);
    let x1 = boxes.iter().map(|b| b.x + b.w).max().unwrap_or(1);
    let y1 = boxes.iter().map(|b| b.y + b.h).max().unwrap_or(1);
    Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    }
}

/// Explosão do barril (passo 10): `0x43d629(barril, nulo, 0)`, vida 0, ligado, destruído.
/// O fogo deixado com `values[0] ≠ 0` (`0x40fb58`) ainda não é criado.
fn explode(e: &mut Entity, out: &mut Out) {
    e.alive = false;
    e.health = 0;
    e.on = true;
    out.blasts.push(Blast {
        center: e.center(),
        layer: e.layer,
        radius: BLAST_RADIUS,
        push: BLAST_PUSH,
        damage: BLAST_DAMAGE,
    });
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let c = e.center();
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.alive = false;
        return;
    }
    let Some(bounds) = e.bounds() else { return };
    let same_layer = ctx.jake_layer == e.layer;
    // o Jake encostado (o barril é sólido para ele: contato = caixas a 1 px)
    let jake = ctx
        .jake
        .filter(|j| j.is_active() && same_layer)
        .filter(|j| grow(bounds, 1).overlaps(&j.body.bounds()));
    let triggered = armed(&e.triggers) && satisfied(&e.triggers, ctx.on);
    let Behaviour::Barrel(st) = &mut e.behaviour else {
        return;
    };
    // 6–7. gatilho, dano de explosão ou de fogo → explode
    let hurt = std::mem::take(&mut st.hurt);
    if triggered || hurt.explosion || hurt.fire {
        explode(e, out);
        return;
    }
    // 3. tocado pelo Jake: em pé tomba; rolando, ele empurra quando anda
    if let Some(j) = jake {
        let walking = matches!(j.state(), crate::State::Run | crate::State::Push);
        match st.state {
            STANDING => {
                st.state = TIPPING;
                st.start_x = e.x;
            }
            ROLLING if walking => {
                let jc = j.body.bounds().x + j.body.bounds().w / 2;
                let toward_barrel = (jc < c.0) == j.facing_right();
                if toward_barrel {
                    st.vx += if j.facing_right() {
                        PUSH_RIGHT
                    } else {
                        -PUSH_LEFT
                    };
                }
            }
            _ => {}
        }
        // 5. rolando rápido contra ele: derruba e fere, e o barril para
        let s = st.vx.abs() / SUB;
        let jc = j.body.bounds().x + j.body.bounds().w / 2;
        let ahead = (st.vx > 0 && jc > c.0) || (st.vx < 0 && jc < c.0);
        if st.state == ROLLING && s >= KNOCK_SPEED && ahead {
            out.jake_damage.add(&Pending {
                amount: s / 2,
                push: (st.vx.signum() * s, 0),
                knockdown: true,
                ..Default::default()
            });
            st.vx = 0;
        }
    }
    // física: só na camada do Jake (a única com geometria montada)
    if same_layer {
        let boxr = shape(&e.boxes);
        let mut body = Body::new(e.x, e.y, boxr);
        body.vx = if st.state == ROLLING { st.vx } else { 0 };
        body.vy = st.vy;
        let (vx0, vy0) = (body.vx, body.vy.saturating_add(crate::physics::GRAVITY));
        step(&mut body, ctx.world, true);
        let vertical = body.on_ground && vy0 > 0 || body.hit_ceiling;
        // 4. batida rápida (parede, chão, outro barril, porta, plataforma) → explode
        if (body.hit_wall && vx0.abs() > CRASH_SPEED) || (vertical && vy0.abs() > CRASH_SPEED) {
            e.x = body.x;
            e.y = body.y;
            explode(e, out);
            return;
        }
        e.x = body.x;
        e.y = body.y;
        let Behaviour::Barrel(st) = &mut e.behaviour else {
            return;
        };
        // rolar não tem atrito; a batida zera a componente
        st.vx = if body.hit_wall { 0 } else { vx0 };
        st.vy = if vertical || body.on_ground {
            0
        } else {
            body.vy
        };
    }
    let Behaviour::Barrel(st) = &mut e.behaviour else {
        return;
    };
    // 8–9. animação: tombo até o fim do rótulo 2; rolando, o quadro segue a distância
    let state = st.state;
    let start_x = st.start_x;
    match state {
        TIPPING => {
            let done = match e.sprite.as_mut() {
                Some(sp) if super::label_of(sp) == Some(TIPPING as u16) => sp.player.tick_once(),
                Some(_) => {
                    super::restart(e, TIPPING as u16);
                    false
                }
                None => true,
            };
            if done {
                if let Behaviour::Barrel(st) = &mut e.behaviour {
                    st.state = ROLLING;
                    if !st.rolling_boxes.is_empty() {
                        e.boxes = st.rolling_boxes.clone();
                    }
                }
                super::restart(e, ROLLING as u16);
            }
        }
        ROLLING => {
            let n = e.sprite.as_ref().map_or(1, |sp| {
                crate::AnimPlayer::at_label(&sp.anim, ROLLING as u16).map_or(1, |p| p.len())
            });
            let f = ((start_x - e.x) / 4).rem_euclid(n.max(1) as i32) as usize;
            super::show(e, ROLLING as u16, f);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::BARREL;

    fn barrel(x: i32, y: i32) -> Entity {
        let mut b = logic(BARREL, x, y, &[0]);
        b.flags = 0x80181;
        b.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 20,
            h: 25,
        }];
        b.solid_shapes = true;
        b.health = 1;
        b.behaviour = Behaviour::Barrel(BarrelState::new(
            1,
            vec![Rect {
                x: 1,
                y: 1,
                w: 20,
                h: 22,
            }],
        ));
        b
    }

    fn st(e: &Entity) -> &BarrelState {
        match &e.behaviour {
            Behaviour::Barrel(s) => s,
            _ => unreachable!(),
        }
    }

    #[test]
    fn touch_tips_the_barrel_and_it_starts_rolling() {
        let on = |_: i32| None;
        // Jake em x 100 (caixa 102..120, pés em 47); barril colado à direita, no chão
        let (j, w) = ctx_with_jake(100, 0);
        let c = ctx(&j, &w, &on);
        let mut b = barrel(120, 22);
        let mut out = Out::default();
        tick(&mut b, &c, &mut out);
        assert_ne!(st(&b).state, STANDING, "tombou");
        for _ in 0..3 {
            tick(&mut b, &c, &mut out);
        }
        assert_eq!(st(&b).state, ROLLING);
        assert_eq!(b.boxes[0].h, 22, "colisão deitada (OOB02)");
        assert!(b.alive && out.blasts.is_empty());
    }

    #[test]
    fn explosion_fire_or_trigger_sets_it_off() {
        let on = |_: i32| Some(true);
        let (j, w) = ctx_with_jake(500, 0);
        let c = ctx(&j, &w, &on);
        for p in [
            Pending {
                amount: 2,
                explosion: true,
                ..Default::default()
            },
            Pending {
                amount: 1,
                fire: true,
                ..Default::default()
            },
        ] {
            let mut b = barrel(100, 22);
            if let Behaviour::Barrel(s) = &mut b.behaviour {
                s.hurt = p;
            }
            let mut out = Out::default();
            tick(&mut b, &c, &mut out);
            assert!(!b.alive);
            assert_eq!(out.blasts[0].radius, BLAST_RADIUS);
            assert_eq!(out.blasts[0].damage, BLAST_DAMAGE);
        }
        // soco comum (natureza 0): nada
        let mut b = barrel(100, 22);
        if let Behaviour::Barrel(s) = &mut b.behaviour {
            s.hurt.amount = 3;
        }
        tick(&mut b, &c, &mut Out::default());
        assert!(b.alive);
        // gatilho: só com código > 0
        let mut b = barrel(100, 22);
        b.triggers = vec![(7, Some(11))];
        tick(&mut b, &c, &mut Out::default());
        assert!(!b.alive);
        let mut b = barrel(100, 22);
        b.triggers = vec![(0, Some(0))];
        tick(&mut b, &c, &mut Out::default());
        assert!(b.alive);
    }

    #[test]
    fn fast_fall_or_roll_into_a_wall_explodes() {
        let on = |_: i32| None;
        let (j, _) = ctx_with_jake(2000, 0);
        // chão em y = 400: cair de y = 0 passa de 10 px/tique antes de chegar
        let w = crate::World::new(vec![
            Rect {
                x: 0,
                y: 400,
                w: 1000,
                h: 20,
            },
            Rect {
                x: 600,
                y: 0,
                w: 20,
                h: 400,
            },
        ]);
        let c = ctx(&j, &w, &on);
        let mut b = barrel(100, 0);
        let mut out = Out::default();
        for _ in 0..40 {
            if !b.alive {
                break;
            }
            tick(&mut b, &c, &mut out);
        }
        assert!(!b.alive, "a queda longa explode");
        // queda curta: pousa inteiro
        let mut b = barrel(100, 370);
        for _ in 0..10 {
            tick(&mut b, &c, &mut Out::default());
        }
        assert!(b.alive);
        assert_eq!(b.y + 25, 400);
        // rolando a 12 px/tique contra a parede
        let mut b = barrel(400, 375);
        if let Behaviour::Barrel(s) = &mut b.behaviour {
            s.state = ROLLING;
            s.vx = 12 * SUB;
        }
        let mut out = Out::default();
        for _ in 0..30 {
            if !b.alive {
                break;
            }
            tick(&mut b, &c, &mut out);
        }
        assert!(!b.alive);
        // devagar (5 px/tique): para na parede e não explode
        let mut b = barrel(400, 375);
        if let Behaviour::Barrel(s) = &mut b.behaviour {
            s.state = ROLLING;
            s.vx = 5 * SUB;
        }
        for _ in 0..60 {
            tick(&mut b, &c, &mut Out::default());
        }
        assert!(b.alive);
        assert_eq!(st(&b).vx, 0);
    }
}
