// SPDX-License-Identifier: GPL-3.0-or-later
//! Catapulta (tipo 41, `OCA00.ANM`, `0x4137e4`; `docs/re/inimigos-menores.md`): uma
//! plataforma que lança quem está em cima — o jogador, um barril ou uma formiga.

use super::{Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{ANT, BARREL, Behaviour, Entity};
use crate::physics::SUB;
use crate::sound::{Cue, PRIORITY_FRAME};

/// Alcance (centros, px).
pub const RANGE: i32 = 30;
/// Rótulo do lançamento (5 quadros) e quadro em que empurra.
pub const LABEL: u16 = 1;
pub const LAUNCH_FRAME: usize = 2;
const LABEL_LEN: usize = 5;
/// Som "catapult".
pub const SOUND: u16 = 3137;

/// Lançamento pedido à cena: centro da catapulta, camada e velocidade (px/tique).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Launch {
    pub center: (i32, i32),
    pub layer: i32,
    pub vel: (i32, i32),
}

#[derive(Debug, Clone, Default)]
pub struct CatapultState {
    /// Empurrão (`values[0]`, `values[1]`), x com o sinal do lado.
    pub push: (i32, i32),
    /// Dispara uma vez só (`values[2] = 1`).
    pub once: bool,
    /// Vira-se para quem lança (`values[3]`).
    pub turns: bool,
    /// Estado 1 (lançando) e o quadro do rótulo.
    pub launching: bool,
    pub step: usize,
    /// Espelhada (o x do empurrão troca de sinal).
    pub mirrored: bool,
    pub spent: bool,
    /// Barril ou formiga vivos a ≤ 30 px (`sync`, antes do tique).
    pub cargo: bool,
}

impl CatapultState {
    pub fn new(values: &[i32]) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        Self {
            push: (v(0), v(1)),
            once: v(2) == 1,
            turns: v(3) != 0,
            ..Default::default()
        }
    }

    fn vel(&self) -> (i32, i32) {
        let x = if self.mirrored {
            -self.push.0
        } else {
            self.push.0
        };
        (x, self.push.1)
    }
}

fn near(a: (i32, i32), b: (i32, i32)) -> bool {
    super::mine::distance(a, b) <= RANGE
}

fn is_cargo(e: &Entity) -> bool {
    e.alive && e.health >= 0 && matches!(e.kind, BARREL | ANT)
}

/// Marca nas catapultas se há carga (barril ou formiga) por perto.
pub fn sync(entities: &mut [Entity]) {
    let cargo: Vec<((i32, i32), i32)> = entities
        .iter()
        .filter(|e| is_cargo(e))
        .map(|e| (e.center(), e.layer))
        .collect();
    for e in entities.iter_mut() {
        let (c, layer) = (e.center(), e.layer);
        if let Behaviour::Catapult(st) = &mut e.behaviour {
            st.cargo = cargo.iter().any(|&(p, l)| l == layer && near(c, p));
        }
    }
}

/// Aplica os lançamentos ao primeiro barril ou formiga perto de cada catapulta: linha `(41,
/// 28)` (derrubada, 1 de dano) no barril; na formiga, ⅔ do empurrão em x.
pub fn apply(entities: &mut [Entity], launches: &[Launch]) {
    for l in launches {
        let Some(e) = entities
            .iter_mut()
            .find(|e| is_cargo(e) && e.layer == l.layer && near(l.center, e.center()))
        else {
            continue;
        };
        match &mut e.behaviour {
            Behaviour::Barrel(st) => {
                st.vx = l.vel.0 * SUB;
                st.vy = l.vel.1 * SUB;
                st.hurt.add(&Pending {
                    amount: 1,
                    knockdown: true,
                    push: l.vel,
                    ..Default::default()
                });
            }
            Behaviour::Ant(st) => st.launch(l.vel),
            _ => {}
        }
    }
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let mut st = match &e.behaviour {
        Behaviour::Catapult(s) => s.clone(),
        _ => return,
    };
    e.on = crate::triggers::satisfied(&e.triggers, ctx.on);
    let c = e.center();
    let jake = super::enemy::jake_target(ctx, e.layer)
        .map(|j| super::enemy::center(&j))
        .filter(|&jc| near(c, jc));
    if !st.launching {
        if e.on && !st.spent && (jake.is_some() || st.cargo) {
            if let (true, Some(jc)) = (st.turns, jake) {
                // a arte olha para a direita: virada para a esquerda, espelha
                st.mirrored = jc.0 < c.0;
            }
            st.launching = true;
            st.step = 0;
            out.events
                .push(Event::Sound(Cue::at(SOUND, PRIORITY_FRAME, c, e.layer)));
        }
    } else {
        st.step += 1;
        if st.step == LAUNCH_FRAME {
            let vel = st.vel();
            if jake.is_some() {
                // linha (41, …): derrubada sem dano, com o empurrão imposto
                out.jake_damage.add(&Pending {
                    knockdown: true,
                    push: vel,
                    ..Default::default()
                });
            }
            out.launches.push(Launch {
                center: c,
                layer: e.layer,
                vel,
            });
        }
        if st.step >= LABEL_LEN {
            st.launching = false;
            st.step = 0;
            st.spent = st.once;
        }
    }
    super::show(e, LABEL, if st.launching { st.step } else { 0 });
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = st.mirrored;
    }
    e.behaviour = Behaviour::Catapult(st);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::barrel::BarrelState;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::physics::Rect;

    fn catapult(x: i32, y: i32, values: &[i32]) -> Entity {
        let mut e = logic(crate::entity::CATAPULT, x, y, values);
        e.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 25,
            h: 6,
        }];
        e.behaviour = Behaviour::Catapult(CatapultState::new(values));
        e
    }

    #[test]
    fn catapult_launches_the_jake() {
        let on = |_: i32| None;
        // Jake (centro 111, 25); catapulta com centro (112, 44)
        let (j, w) = ctx_with_jake(100, 0);
        let mut c = ctx(&j, &w, &on);
        let mut k = catapult(100, 41, &[5, -40]);
        let mut out = Out::default();
        for _ in 0..6 {
            tick(&mut k, &mut c, &mut out);
        }
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND)));
        assert!(out.jake_damage.knockdown);
        assert_eq!(out.jake_damage.push, (5, -40));
        assert_eq!(out.jake_damage.amount, 0);
        // longe: nada
        let mut far = catapult(400, 41, &[5, -40]);
        let mut out = Out::default();
        for _ in 0..6 {
            tick(&mut far, &mut c, &mut out);
        }
        assert!(out.launches.is_empty() && out.events.is_empty());
    }

    #[test]
    fn catapult_launches_a_barrel_once() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut barrel = logic(BARREL, 100, 20, &[]);
        barrel.behaviour = Behaviour::Barrel(BarrelState::new(1, vec![]));
        let mut v = vec![catapult(100, 41, &[-5, -30, 1]), barrel];
        let mut launches = 0;
        for _ in 0..30 {
            sync(&mut v);
            let mut out = Out::default();
            tick(&mut v[0], &mut c, &mut out);
            launches += out.launches.len();
            apply(&mut v, &out.launches);
        }
        assert_eq!(launches, 1, "values[2] = 1: uma vez só");
        let Behaviour::Barrel(b) = &v[1].behaviour else {
            panic!()
        };
        assert_eq!(b.vy, -30 * SUB);
        assert!(b.hurt.knockdown && b.hurt.amount == 1);
    }
}
