// SPDX-License-Identifier: GPL-3.0-or-later
//! Entidade (tipo 107, `OES.ANM`; `docs/re/inimigos-menores.md`, `inimigos-menores-2.md`):
//! invulnerável, vagueia e, perto do jogador e na tela dele, inverte-lhe os controles.

use super::enemy::{self, Clip, Label, Wound};
use super::{Ctx, Out};
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};

/// Rótulos do `OES.ANM`: 1 (13, sem alvo), 2 (13) e 3 (10, em laço com alvo).
pub const LABELS: [Label; 3] = [(1, 0, 13), (2, 13, 13), (3, 26, 10)];
/// `SOUND.TBL` tipo 107.
pub const SOUNDS: [(usize, u16, u16); 3] = [(1, 3113, 0), (13, 3114, 7), (26, 3115, 7)];
/// Tiques de confusão postos no jogador (`+0x264 = 3`).
pub const CONFUSION_TICKS: u32 = 3;
/// Meia vista do jogador (640 × 405, câmera centrada nele).
pub const VIEW_HALF: (i32, i32) = (320, 202);

/// Variante (`0x4721f0`, registros de 0x28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    pub range: i32,
    pub confusion: i32,
    pub turn_chance: u32,
    /// px/tique.
    pub speed: i32,
}

pub const VARIANTS: [Variant; 4] = [
    Variant {
        range: 200,
        confusion: 250,
        turn_chance: 100,
        speed: 4,
    },
    Variant {
        range: 150,
        confusion: 200,
        turn_chance: 50,
        speed: 4,
    },
    Variant {
        range: 250,
        confusion: 300,
        turn_chance: 50,
        speed: 2,
    },
    Variant {
        range: 150,
        confusion: 200,
        turn_chance: 50,
        speed: 8,
    },
];

#[derive(Debug, Clone)]
pub struct TheEntityState {
    pub variant: usize,
    /// Vagueia (`values[2] = 0`).
    pub wanders: bool,
    pub right: bool,
    pub clip: Clip,
    /// Velocidade (1/8 px); o empurrão de um golpe soma em `vx`.
    pub vx: i32,
    pub vy: i32,
    pub shoved: i32,
    pub wound: Wound,
    pub last_frame: Option<usize>,
}

impl TheEntityState {
    pub fn new(values: &[i32]) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        Self {
            variant: v(0).clamp(0, 3) as usize,
            wanders: v(2) == 0,
            right: false,
            clip: Clip::new(1, false),
            vx: 0,
            vy: 0,
            shoved: 0,
            wound: Wound::default(),
            last_frame: None,
        }
    }

    pub fn variant(&self) -> Variant {
        VARIANTS[self.variant.min(3)]
    }
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Some(me) = e.bounds() else {
        e.alive = false;
        return;
    };
    let c = enemy::center(&me);
    // só some fora da fase
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.alive = false;
        return;
    }
    let mut st = match &e.behaviour {
        Behaviour::TheEntity(s) => s.clone(),
        _ => return,
    };
    let v = st.variant();
    let jake = enemy::jake_target(ctx, e.layer);
    // um golpe só empurra (`0x41293a`): para longe do jogador, 5–12 px/tique
    if !st.wound.take().is_empty() {
        let away = match jake {
            Some(j) if enemy::center(&j).0 > c.0 => -1,
            Some(_) => 1,
            None if st.right => -1,
            None => 1,
        };
        st.vx = away * (5 + ctx.rng.below(8) as i32) * SUB;
        st.shoved = 6;
    }
    let target = jake.filter(|j| {
        let jc = enemy::center(j);
        super::mine::distance(c, jc) <= v.range && enemy::line_of_sight(ctx, c, jc, &[me, *j])
    });
    if let Some(j) = target {
        let jc = enemy::center(&j);
        let (dx, dy) = (c.0 - jc.0, c.1 - jc.1);
        if dx.abs() <= VIEW_HALF.0
            && dy.abs() <= VIEW_HALF.1
            && super::mine::distance(c, jc) < v.confusion
        {
            out.confuse = true;
        }
    }
    let (_, _, len) = enemy::label(&LABELS, st.clip.label);
    if st.clip.advance(len) {
        st.clip = match (target.is_some(), st.clip.label) {
            (true, 2) => Clip::new(3, false),
            (true, _) => Clip::new(if st.clip.label == 3 { 3 } else { 2 }, false),
            (false, _) => Clip::new(1, false),
        };
        if st.wanders && ctx.rng.below(100) < v.turn_chance {
            st.right = !st.right;
        }
    }
    // movimento: vagueia na horizontal com gravidade contra as células da camada
    let boxr = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    let cells = ctx.cells_for(e.layer);
    if st.shoved > 0 {
        st.shoved -= 1;
    } else if st.wanders {
        let side = if st.right { 1 } else { -1 };
        let ahead = me.moved(side * (v.speed + 1), 0);
        let floor = cells.standing_on(&me, 2);
        if floor && (cells.blocked(&ahead) || !cells.standing_on(&ahead, 16)) {
            st.right = !st.right;
        }
        st.vx = if st.right { v.speed } else { -v.speed } * SUB;
    } else {
        st.vx = 0;
    }
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = st.vx;
    body.vy = st.vy;
    step(&mut body, cells, true);
    e.x = body.x;
    e.y = body.y;
    st.vy = body.vy;
    if body.hit_wall && st.shoved == 0 {
        st.right = !st.right;
    }
    let frame = enemy::show(e, &LABELS, &st.clip);
    enemy::sound_frame(&SOUNDS, frame, &mut st.last_frame, c, e.layer, out);
    e.behaviour = Behaviour::TheEntity(st);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};

    fn orb(x: i32, y: i32, values: &[i32]) -> Entity {
        let mut e = logic(crate::entity::THE_ENTITY, x, y, values);
        e.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 25,
            h: 25,
        }];
        e.behaviour = Behaviour::TheEntity(TheEntityState::new(values));
        e
    }

    #[test]
    fn entity_confuses_only_near_and_on_screen() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(300, 0);
        let mut c = ctx(&j, &w, &on);
        let mut near = orb(150, 22, &[0, 3, 1]);
        let mut out = Out::default();
        tick(&mut near, &mut c, &mut out);
        assert!(out.confuse, "a 150 px, Alpha (alcance 200, confusão 250)");
        let mut far = orb(-200, 22, &[0, 3, 1]);
        let mut out = Out::default();
        tick(&mut far, &mut c, &mut out);
        assert!(!out.confuse, "fora do alcance");
        let mut other = orb(150, 22, &[0, 3, 1]);
        other.layer = 1;
        let mut out = Out::default();
        tick(&mut other, &mut c, &mut out);
        assert!(!out.confuse, "outra camada");
    }

    #[test]
    fn entity_is_pushed_not_hurt_and_wanders_on_the_floor() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut e = orb(100, 22, &[0, 3, 0]);
        let p = crate::damage::Pending {
            amount: 50,
            ..Default::default()
        };
        assert!(enemy::hurt(&mut e, &p, true));
        let x0 = e.x;
        tick(&mut e, &mut c, &mut Out::default());
        assert!(e.alive && e.health == 0 && !enemy::killed(&e));
        assert!(e.x < x0 - 4, "empurrada para longe do Jake (à direita)");
        let mut xs = vec![];
        for _ in 0..80 {
            tick(&mut e, &mut c, &mut Out::default());
            xs.push(e.x);
        }
        assert!(xs.iter().any(|&x| x != xs[0]), "vagueia");
        assert!(e.y <= 22, "fica no chão (y + 47)");
    }
}
