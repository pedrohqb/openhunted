// SPDX-License-Identifier: GPL-3.0-or-later
//! Raio laser (tipo 36, criado em `0x00410a18`, atualização `0x00410bda`;
//! `docs/re/olho-laser.md`): voa reto, para na parede, fere o Jake com tranco e detona barris.

use super::{Blast, Ctx, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity, LASER};
use crate::physics::{Body, Rect, SUB, step};

/// Dano por variante (linhas do atacante 36 no `DAMAGE.TBL`): 5 / 7 / 10 / 5.
pub const DAMAGE: [i32; 4] = [5, 7, 10, 5];
/// Sons "Laser" na criação: 3037 (variantes 0 e 2) ou 3036 (1 e 3), `0x47209c`.
pub const SOUNDS: [u16; 2] = [3037, 3036];
/// Tiques que o raio parado continua na tela antes de sumir (`+0x270 = 5`).
pub const DYING_TICKS: u32 = 5;
/// Caixa do `OLA.BRD` (8 × 8).
pub const BOX: Rect = Rect {
    x: 0,
    y: 0,
    w: 8,
    h: 8,
};
/// Efeito do acerto no jogador: `oex01`, raio 5.
pub const HIT_LABEL: u16 = 1;
pub const HIT_RADIUS: i32 = 5;
/// Empurrão do tranco no Jake: a linha (36, 1) do `DAMAGE.TBL` pede a reação 2 com
/// empurrão fixo (0, 0) e módulo 0 — o Jake toma o tranco no lugar.
pub const SHOVE: i32 = 0;
/// Comprimento máximo do traço desenhado (`0x410dae`: a cauda fica a ≤ 25 px da cabeça).
pub const TRAIL: i32 = 25;
/// Cores do traço por variante (tabela `0x004720ac`: 3 índices da paleta, miolo → borda).
pub const COLORS: [[u8; 3]; 4] = [
    [0xb7, 0xba, 0xbc],
    [0xc2, 0xc6, 0xc7],
    [0xd8, 0xda, 0xdc],
    [0xe1, 0xe7, 0xe9],
];

#[derive(Debug, Clone)]
pub struct LaserState {
    /// Velocidade em 1/8 px por tique.
    pub vx: i32,
    pub vy: i32,
    pub variant: usize,
    /// Parado (acertou algo): tiques até sumir.
    pub dying: Option<u32>,
    /// Posição antes do último passo (teste de acerto varrido).
    pub prev: Option<(i32, i32)>,
    /// Ponto de partida (a cauda do traço).
    pub tail: (i32, i32),
}

impl LaserState {
    pub fn damage(&self) -> Pending {
        Pending {
            amount: DAMAGE[self.variant.min(3)],
            explosion: true,
            ..Default::default()
        }
    }

    /// O raio parou (parede ou alvo): fica 5 tiques e some.
    pub fn stop(&mut self) {
        if self.dying.is_none() {
            self.dying = Some(DYING_TICKS);
        }
        self.vx = 0;
        self.vy = 0;
    }

    pub fn moving(&self) -> bool {
        self.dying.is_none()
    }
}

/// Som de criação da variante.
pub fn sound(variant: usize) -> u16 {
    SOUNDS[variant & 1]
}

/// Raio novo na boca `at` com velocidade `vel` em px/tique.
pub fn bolt(at: (i32, i32), vel: (i32, i32), layer: i32, variant: usize) -> Entity {
    Entity {
        kind: LASER,
        x: at.0,
        y: at.1,
        flags: 0x83031,
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
        behaviour: Behaviour::Laser(LaserState {
            vx: vel.0 * SUB,
            vy: vel.1 * SUB,
            variant: variant.min(3),
            dying: None,
            prev: None,
            tail: at,
        }),
    }
}

/// Caixa varrida no último passo.
pub fn swept_bounds(e: &Entity) -> Option<Rect> {
    let now = e.bounds()?;
    let Behaviour::Laser(st) = &e.behaviour else {
        return Some(now);
    };
    let Some((px, py)) = st.prev else {
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

/// Um tique: voa contra as células; parede → para; Jake na caixa → dano e tranco.
pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let layer = e.layer;
    let (x, y) = (e.x, e.y);
    let Behaviour::Laser(st) = &mut e.behaviour else {
        return;
    };
    if let Some(n) = st.dying {
        st.prev = None;
        // a cauda alcança a cabeça enquanto o raio some
        st.tail = (
            st.tail.0 + (x - st.tail.0) / 2,
            st.tail.1 + (y - st.tail.1) / 2,
        );
        if n <= 1 {
            e.alive = false;
        } else {
            st.dying = Some(n - 1);
        }
        return;
    }
    st.prev = Some((x, y));
    let mut body = Body::new(x, y, BOX);
    body.vx = st.vx;
    body.vy = st.vy;
    step(&mut body, ctx.cells, false);
    let moved = (body.x - x, body.y - y) == (st.vx >> 3, st.vy >> 3);
    e.x = body.x;
    e.y = body.y;
    let c = (e.x + BOX.w / 2, e.y + BOX.h / 2);
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.alive = false;
        return;
    }
    let Behaviour::Laser(st) = &mut e.behaviour else {
        return;
    };
    if !moved {
        st.stop();
        return;
    }
    let hit_jake = ctx.jake_layer == layer
        && ctx
            .jake_bounds()
            .zip(swept_bounds(e))
            .is_some_and(|(j, b)| j.overlaps(&b));
    if hit_jake {
        let Behaviour::Laser(st) = &mut e.behaviour else {
            return;
        };
        let mut p = st.damage();
        p.shove = true;
        p.push = (st.vx.signum() * SHOVE, 0);
        out.jake_damage.add(&p);
        out.effects.push((
            Blast {
                center: c,
                layer,
                radius: HIT_RADIUS,
                push: 0,
                damage: 0,
            },
            HIT_LABEL,
        ));
        st.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake};

    #[test]
    fn bolt_flies_hurts_jake_with_a_shove_and_lingers() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 0);
        let c = ctx(&j, &w, &on);
        let mut b = bolt((60, 20), (8, 0), 0, 0);
        let mut out = Out::default();
        tick(&mut b, &c, &mut out);
        assert_eq!(b.x, 68);
        assert!(out.jake_damage.is_empty());
        let mut n = 1;
        while out.jake_damage.is_empty() && n < 20 {
            tick(&mut b, &c, &mut out);
            n += 1;
        }
        assert_eq!(n, 5, "acerta quando a caixa varrida cruza a do Jake");
        assert_eq!(out.jake_damage.amount, 5);
        assert!(out.jake_damage.shove && out.jake_damage.explosion);
        assert_eq!(out.jake_damage.push, (SHOVE, 0));
        assert_eq!(out.effects.len(), 1);
        // parado, fica 5 tiques e some
        let x = b.x;
        let mut lingering = 0;
        while b.alive && lingering < 20 {
            tick(&mut b, &c, &mut out);
            assert_eq!(b.x, x);
            lingering += 1;
        }
        assert_eq!(lingering, DYING_TICKS);
        assert_eq!(out.jake_damage.amount, 5, "um acerto só");
    }

    #[test]
    fn bolt_stops_at_walls_and_leaves_the_level() {
        let on = |_: i32| None;
        let (j, _) = ctx_with_jake(1000, 0);
        let wall = crate::World::new(vec![Rect {
            x: 90,
            y: 0,
            w: 10,
            h: 100,
        }]);
        let c = ctx(&j, &wall, &on);
        let mut b = bolt((60, 20), (20, 0), 0, 2);
        let mut out = Out::default();
        tick(&mut b, &c, &mut out);
        tick(&mut b, &c, &mut out);
        let Behaviour::Laser(st) = &b.behaviour else {
            panic!()
        };
        assert!(!st.moving());
        assert_eq!(b.x + 8, 90);
        assert_eq!(sound(2), 3037);
        assert_eq!(sound(1), 3036);
        let mut gone = bolt((5, 20), (-20, 0), 0, 0);
        tick(&mut gone, &c, &mut out);
        assert!(!gone.alive);
    }
}
