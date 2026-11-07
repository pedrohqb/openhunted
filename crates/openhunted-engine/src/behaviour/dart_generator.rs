// SPDX-License-Identifier: GPL-3.0-or-later
//! Gerador de dardos (tipo 87, `ODG.ANM`, `0x420a14`/`0x420bc2`;
//! `docs/re/inimigos-menores.md`): um furo de parede que atira dardos (tipo 29) no jogador
//! alinhado com ele.

use super::enemy::{self, Spawn, Wound};
use super::{Ctx, Out};
use crate::entity::{Behaviour, Entity};

/// Variante (`0x473080`, registros de 0x30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    pub health: i32,
    /// Exige linha de visão (só o Delta).
    pub sight: bool,
    pub interval: i32,
    pub ammo: i32,
    /// % do dano recebido.
    pub applied: i32,
    /// Velocidade do dardo (px/tique).
    pub speed: i32,
}

pub const VARIANTS: [Variant; 5] = [
    // Alpha
    Variant {
        health: 35,
        sight: false,
        interval: 15,
        ammo: 50,
        applied: 100,
        speed: 20,
    },
    // Beta
    Variant {
        health: 30,
        sight: false,
        interval: 15,
        ammo: 40,
        applied: 70,
        speed: 25,
    },
    // Gamma
    Variant {
        health: 25,
        sight: false,
        interval: 10,
        ammo: 100,
        applied: 60,
        speed: 25,
    },
    // Delta
    Variant {
        health: 50,
        sight: true,
        interval: 5,
        ammo: 100,
        applied: 40,
        speed: 30,
    },
    // All
    Variant {
        health: 40,
        sight: false,
        interval: 2,
        ammo: 500,
        applied: 50,
        speed: 35,
    },
];

/// Tolerância perpendicular e alcance padrão (`values[2]`/`values[5]` = 0).
pub const TOLERANCE: i32 = 15;
pub const RANGE: i32 = 350;

#[derive(Debug, Clone, Default)]
pub struct DartGeneratorState {
    pub variant: usize,
    /// Atira na vertical (`values[1] = 1`, rótulo 2); senão na horizontal (rótulo 1).
    pub vertical: bool,
    pub tolerance: i32,
    /// Munição restante (−1 = infinita).
    pub ammo: i32,
    pub interval: i32,
    pub range: i32,
    /// Tiques até o próximo dardo.
    pub wait: i32,
    /// Ligado pelos gatilhos (`0x40`).
    pub armed: bool,
    pub wound: Wound,
}

impl DartGeneratorState {
    /// `0x420a14`: os campos 0 vêm da variante.
    pub fn new(values: &[i32]) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        let variant = v(0).clamp(0, 4) as usize;
        let var = VARIANTS[variant];
        let or = |x: i32, d: i32| if x == 0 { d } else { x };
        Self {
            variant,
            vertical: v(1) == 1,
            tolerance: or(v(2), TOLERANCE),
            ammo: or(v(3), var.ammo),
            interval: or(v(4), var.interval),
            range: or(v(5), RANGE),
            wait: 0,
            armed: false,
            wound: Wound::default(),
        }
    }

    pub fn variant(&self) -> Variant {
        VARIANTS[self.variant.min(VARIANTS.len() - 1)]
    }
}

/// Vida padrão da variante (`+0x18`).
pub fn health(values: &[i32]) -> i32 {
    DartGeneratorState::new(values).variant().health
}

/// Alvo: o jogador vivo na camada, a ≤ alcance, alinhado (`|dy|` ou `|dx|` ≤ tolerância) e,
/// no Delta, à vista. Devolve o lado (+1/−1) para onde atirar.
fn aim(e: &Entity, st: &DartGeneratorState, ctx: &Ctx) -> Option<i32> {
    let (me, j) = (e.bounds()?, enemy::jake_target(ctx, e.layer)?);
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    let (dx, dy) = (jc.0 - c.0, jc.1 - c.1);
    let (along, across) = if st.vertical { (dy, dx) } else { (dx, dy) };
    if super::mine::distance(c, jc) > st.range || across.abs() > st.tolerance {
        return None;
    }
    if st.variant().sight && !enemy::line_of_sight(ctx, c, jc, &[me, j]) {
        return None;
    }
    Some(if along >= 0 { 1 } else { -1 })
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let mut st = match &e.behaviour {
        Behaviour::DartGenerator(s) => s.clone(),
        _ => return,
    };
    if !st.armed && crate::triggers::satisfied(&e.triggers, ctx.on) {
        st.armed = true;
        e.on = true;
    }
    // dano × % (`0x420dc5`: 0 não faz nada; sem dificuldade nem mínimo)
    let w = st.wound.take();
    if w.pending.amount != 0 {
        e.health -= w.pending.amount * st.variant().applied / 100;
        if e.health <= 0 {
            e.alive = false;
            e.behaviour = Behaviour::DartGenerator(st);
            return;
        }
    }
    if st.wait > 0 {
        st.wait -= 1;
    }
    if st.armed && st.ammo != 0 && st.wait == 0 {
        if let Some(side) = aim(e, &st, ctx) {
            if st.ammo > 0 {
                st.ammo -= 1;
            }
            st.wait = st.interval.max(1);
            let speed = st.variant().speed;
            let (at, vel) = if st.vertical {
                let y = if side > 0 { e.y + 13 } else { e.y - 6 };
                ((e.x + 5, y), (0, side * speed))
            } else {
                let x = if side > 0 { e.x + 8 } else { e.x - 7 };
                ((x, e.y + 5), (side * speed, 0))
            };
            out.enemy_spawns.push(Spawn::Dart {
                at,
                vel,
                layer: e.layer,
                vertical: st.vertical,
            });
        }
    }
    let label = if st.vertical { 2 } else { 1 };
    super::show(e, label, 0);
    e.behaviour = Behaviour::DartGenerator(st);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::physics::Rect;

    fn generator(x: i32, y: i32, values: &[i32]) -> Entity {
        let mut e = logic(crate::entity::DART_GENERATOR, x, y, values);
        e.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 10,
            h: 10,
        }];
        e.health = health(values);
        e.behaviour = Behaviour::DartGenerator(DartGeneratorState::new(values));
        e
    }

    #[test]
    fn generator_fires_darts_at_interval_until_ammo() {
        let on = |_: i32| None;
        // Jake em x 302..320, y 3..47 (centro y 25): gerador na altura do peito, à esquerda
        let (j, w) = ctx_with_jake(300, 0);
        let mut c = ctx(&j, &w, &on);
        let mut g = generator(100, 20, &[0, 0, 0, 3, 4, 0]);
        let mut out = Out::default();
        let mut ticks = vec![];
        for t in 0..20 {
            let n = out.enemy_spawns.len();
            tick(&mut g, &mut c, &mut out);
            if out.enemy_spawns.len() > n {
                ticks.push(t);
            }
        }
        assert_eq!(ticks, vec![0, 4, 8], "intervalo 4, munição 3");
        let Spawn::Dart { at, vel, .. } = out.enemy_spawns[0].clone() else {
            panic!()
        };
        assert_eq!(
            (at, vel),
            ((108, 25), (20, 0)),
            "boca x+8, y+5; Alpha 20 px"
        );
        // fora da tolerância (|dy| > 15) não atira
        let mut high = generator(100, -40, &[0]);
        let mut out = Out::default();
        tick(&mut high, &mut c, &mut out);
        assert!(out.enemy_spawns.is_empty());
    }

    #[test]
    fn generator_takes_damage_by_percent_and_dies() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut g = generator(100, 20, &[3]); // Delta: 50 de vida, 40 %
        let p = crate::damage::Pending {
            amount: 10,
            ..Default::default()
        };
        assert!(enemy::hurt(&mut g, &p, true));
        tick(&mut g, &mut c, &mut Out::default());
        assert_eq!(g.health, 46);
        g.health = 2;
        enemy::hurt(&mut g, &p, true);
        tick(&mut g, &mut c, &mut Out::default());
        assert!(!g.alive);
        // dano 0 (borda de explosão) não fere (`0x420dc5`); sem mínimo 1: 1 × 40 % = 0
        let mut g = generator(100, 20, &[3]);
        g.health = 1;
        for amount in [0, 1] {
            let p = crate::damage::Pending {
                amount,
                explosion: amount == 0,
                ..Default::default()
            };
            enemy::hurt(&mut g, &p, false);
            tick(&mut g, &mut c, &mut Out::default());
        }
        assert!(g.alive && g.health == 1);
    }
}
