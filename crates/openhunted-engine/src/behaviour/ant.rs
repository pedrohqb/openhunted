// SPDX-License-Identifier: GPL-3.0-or-later
//! Formigueiro (tipo 88, `OAH00.ANM`, `0x4149a0`/`0x414aed`) e formiga (109, `OAN.ANM`,
//! `0x41fe70`; `docs/re/inimigos-menores-2.md`): o formigueiro solta formigas em ritmo
//! fixo; a formiga anda, morde o jogador perto e morre com qualquer dano.

use super::enemy::{self, Clip, Label, Spawn, Wound};
use super::{Blast, Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{ANT, Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};
use crate::sound::{Cue, PRIORITY_FRAME};

/// Variantes do formigueiro (`0x47234c`, registros de 0x1c): intervalo (`+0x14`) e número
/// de formigas (`+0x18`).
pub const HILLS: [(i32, i32); 3] = [(100, 9), (50, 5), (55, 3)];
/// Formiga (`0x472fb8`): alcance 60, vida 1.
pub const RANGE: i32 = 60;
pub const HEALTH: i32 = 1;
/// Linha `(109, 0)`: 5 de dano.
pub const BITE: i32 = 5;
/// Tiques entre mordidas e velocidade do andar (inferidos).
pub const BITE_TICKS: i32 = 10;
pub const SPEED: i32 = 2;
/// Sons: 3151 (10 % ao andar, por volta do rótulo) e 3153 (mordida).
pub const SOUND_WALK: u16 = 3151;
pub const SOUND_BITE: u16 = 3153;
/// Morte: fumaça `oex` de raio 15 (`0x43d629(f, nulo, 4)`).
pub const SMOKE_RADIUS: i32 = 15;
/// Rótulos do `OAN.ANM` [inferido]: 2 anda, 3 vira, 4 morde, 5 derrubada.
pub const LABELS: [Label; 6] = [
    (1, 0, 4),
    (2, 4, 8),
    (3, 12, 8),
    (4, 20, 8),
    (5, 28, 16),
    (6, 44, 5),
];
const WALK: u16 = 2;
const BITE_LABEL: u16 = 4;
const KNOCKED: u16 = 5;

#[derive(Debug, Clone, Default)]
pub struct AnthillState {
    pub variant: usize,
    /// Formigas restantes (`+0x264`; −1 = sem fim).
    pub left: i32,
    pub interval: i32,
    pub wait: i32,
    /// Variante das formigas: campo 5 − 1 (`+0x180`).
    pub ant_variant: i32,
}

impl AnthillState {
    pub fn new(values: &[i32], field5: i32) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        let variant = v(0).clamp(0, 2) as usize;
        let (interval, count) = HILLS[variant];
        let interval = if v(2) == 0 { interval } else { v(2) };
        Self {
            variant,
            left: if v(1) == 0 { count } else { v(1) },
            interval,
            wait: interval,
            ant_variant: (field5 - 1).clamp(0, 3),
        }
    }
}

/// Formigueiro (`0x414aed`): espera os gatilhos; ligado, a cada intervalo cria uma formiga
/// na sua posição até acabar a conta.
pub fn tick_hill(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Behaviour::Anthill(st) = &mut e.behaviour else {
        return;
    };
    if !e.on {
        if crate::triggers::satisfied(&e.triggers, ctx.on) {
            e.on = true;
        }
        return;
    }
    if st.left == 0 {
        return;
    }
    st.wait -= 1;
    if st.wait > 0 {
        return;
    }
    if st.left != -1 {
        st.left -= 1;
    }
    st.wait = st.interval.max(1);
    out.enemy_spawns.push(Spawn::Ant {
        at: (e.x, e.y),
        variant: st.ant_variant,
        layer: e.layer,
        mirror: e.sprite.as_ref().is_some_and(|s| s.mirror),
    });
}

#[derive(Debug, Clone)]
pub struct AntState {
    pub variant: i32,
    /// Criada por um formigueiro (não conta no placar).
    pub spawned: bool,
    pub right: bool,
    pub clip: Clip,
    pub vx: i32,
    pub vy: i32,
    pub bite_wait: i32,
    pub knocked: bool,
    pub wound: Wound,
}

impl AntState {
    pub fn new(variant: i32, spawned: bool) -> Self {
        Self {
            variant: variant.clamp(0, 3),
            spawned,
            right: false,
            clip: Clip::new(WALK, false),
            vx: 0,
            vy: 0,
            bite_wait: 0,
            knocked: false,
            wound: Wound::default(),
        }
    }

    /// Lançada pela catapulta: ⅔ do empurrão em x, derrubada.
    pub fn launch(&mut self, vel: (i32, i32)) {
        self.vx = vel.0 * 2 * SUB / 3;
        self.vy = vel.1 * SUB;
        self.knocked = true;
        self.clip = Clip::new(KNOCKED, false);
    }
}

/// Formiga nova de um formigueiro (`0x414bd4`): sinalizadores da formiga da fase, vida 1.
pub fn spawned(at: (i32, i32), variant: i32, layer: i32, mirror: bool) -> Entity {
    let mut st = AntState::new(variant, true);
    st.right = mirror;
    Entity {
        kind: ANT,
        x: at.0,
        y: at.1,
        flags: 0xd4805,
        layer,
        id: 0,
        values: vec![variant],
        triggers: vec![],
        objective: 0,
        health: HEALTH,
        armor: 0,
        boxes: vec![Rect {
            x: 0,
            y: 0,
            w: 13,
            h: 15,
        }],
        solid_shapes: true,
        on: false,
        alive: true,
        sprite: None,
        behaviour: Behaviour::Ant(st),
    }
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Some(me) = e.bounds() else {
        e.alive = false;
        return;
    };
    let c = enemy::center(&me);
    let mut st = match &e.behaviour {
        Behaviour::Ant(s) => s.clone(),
        _ => return,
    };
    let outside = c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1;
    // vida 1: qualquer dano mata (fumaça de raio 15)
    if outside || !st.wound.take().is_empty() {
        e.health = 0;
        e.alive = false;
        e.on = true;
        if !outside {
            out.effects.push((
                Blast {
                    center: c,
                    layer: e.layer,
                    radius: SMOKE_RADIUS,
                    push: 0,
                    damage: 0,
                },
                1 + ctx.rng.below(4) as u16,
            ));
        }
        return;
    }
    let target = enemy::jake_target(ctx, e.layer).filter(|j| {
        let jc = enemy::center(j);
        super::mine::distance(c, jc) <= RANGE && enemy::line_of_sight(ctx, c, jc, &[me, *j])
    });
    let walk_sound = ctx.rng.below(10) == 0;
    let cells = ctx.cells_for(e.layer);
    let on_floor = cells.standing_on(&me, 2);
    if st.bite_wait > 0 {
        st.bite_wait -= 1;
    }
    let (_, _, len) = enemy::label(&LABELS, st.clip.label);
    let ended = st.clip.advance(len);
    if st.knocked {
        if on_floor && st.vy >= 0 {
            st.knocked = false;
            st.clip = Clip::new(WALK, false);
        }
    } else if let Some(j) = target {
        st.right = enemy::center(&j).0 > c.0;
        if st.vx.abs() > 0 && on_floor {
            st.vx = 0;
        }
        if st.bite_wait == 0 {
            st.bite_wait = BITE_TICKS;
            st.clip = Clip::new(BITE_LABEL, false);
            out.jake_damage.add(&Pending {
                amount: BITE,
                ..Default::default()
            });
            out.events.push(Event::Sound(Cue::at(
                SOUND_BITE,
                PRIORITY_FRAME,
                c,
                e.layer,
            )));
        }
    } else {
        if ended {
            st.clip = Clip::new(WALK, false);
            if walk_sound {
                out.events.push(Event::Sound(Cue::at(
                    SOUND_WALK,
                    PRIORITY_FRAME,
                    c,
                    e.layer,
                )));
            }
        }
        if on_floor {
            let side = if st.right { 1 } else { -1 };
            let ahead = me.moved(side * (SPEED + 1), 0);
            if cells.blocked(&ahead) || !cells.standing_on(&ahead, 16) {
                st.right = !st.right;
            }
            st.vx = if st.right { SPEED } else { -SPEED } * SUB;
        }
    }
    let boxr = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = st.vx;
    body.vy = st.vy;
    step(&mut body, cells, true);
    e.x = body.x;
    e.y = body.y;
    st.vx = body.vx;
    st.vy = body.vy;
    enemy::show(e, &LABELS, &st.clip);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = st.right;
    }
    e.behaviour = Behaviour::Ant(st);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};

    #[test]
    fn anthill_spawns_on_interval_until_count() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut hill = logic(crate::entity::ANTHILL, 100, 20, &[0, 3, 10, 10]);
        hill.behaviour = Behaviour::Anthill(AnthillState::new(&hill.values, 2));
        let mut out = Out::default();
        let mut at = vec![];
        for t in 0..60 {
            let n = out.enemy_spawns.len();
            tick_hill(&mut hill, &mut c, &mut out);
            if out.enemy_spawns.len() > n {
                at.push(t);
            }
        }
        assert_eq!(
            at,
            vec![10, 20, 30],
            "liga no 1º tique; 3 formigas, uma a cada 10"
        );
        let Spawn::Ant { variant, at, .. } = out.enemy_spawns[0].clone() else {
            panic!()
        };
        assert_eq!((variant, at), (1, (100, 20)), "variante = campo 5 − 1");
        let d = AnthillState::new(&[1], 0);
        assert_eq!((d.interval, d.left, d.ant_variant), (50, 5, 0));
    }

    #[test]
    fn ant_bites_for_5_and_dies_to_any_hit() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(140, 0);
        let mut c = ctx(&j, &w, &on);
        let mut a = spawned((100, 32), 0, 0, false);
        let mut out = Out::default();
        for _ in 0..12 {
            tick(&mut a, &mut c, &mut out);
        }
        assert_eq!(
            out.jake_damage.amount,
            2 * BITE,
            "duas mordidas em 12 tiques"
        );
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND_BITE)));
        let p = Pending {
            amount: 1,
            ..Default::default()
        };
        assert!(enemy::hurt(&mut a, &p, true));
        tick(&mut a, &mut c, &mut out);
        assert!(!a.alive && a.health == 0);
        assert!(
            !enemy::counted(&a),
            "criada pelo formigueiro: fora do placar"
        );
    }
}
