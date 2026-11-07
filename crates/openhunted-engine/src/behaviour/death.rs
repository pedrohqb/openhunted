// SPDX-License-Identifier: GPL-3.0-or-later
//! Morte (tipo 3, `ODE.ANM`, `0x415700`/`0x415855`; `docs/re/inimigos-menores.md`,
//! `inimigos-menores-2.md`): fica oculta, aparece ao lado do jogador, irradia, atira balas 99
//! e some de novo.

use super::enemy::{self, Clip, Label, Spawn, Wound};
use super::{Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, step};
use crate::sound::{Cue, PRIORITY_FRAME};

pub const HEALTH: i32 = 100;
/// Rótulos do `ODE.ANM` (rótulo = estado).
pub const LABELS: [Label; 9] = [
    (1, 0, 9),
    (2, 9, 5),
    (3, 14, 12),
    (4, 26, 5),
    (5, 31, 9),
    (6, 40, 6),
    (7, 46, 7),
    (8, 53, 4),
    (11, 57, 15),
];
/// `SOUND.TBL` tipo 3.
pub const SOUNDS: [(usize, u16, u16); 4] =
    [(38, 3052, 0), (40, 3052, 0), (51, 3052, 0), (58, 3053, 0)];
/// "Death ascending" (aparece e some), radiação, falas e a fala da morte.
pub const SOUND_ASCEND: u16 = 3051;
pub const SOUND_RADIATION: u16 = 3156;
pub const TAUNTS: [u16; 4] = [3760, 3761, 3762, 3763];
pub const SOUND_DIES: u16 = 3764;
/// Radiação (`0x416a48`): `|Δx|`, `|Δy|` < 45; linha `(3, 1)`: 4 com empurrão (3, 0).
pub const RADIATION_RANGE: i32 = 45;
pub const RADIATION: i32 = 4;
pub const RADIATION_PUSH: i32 = 3;
/// Tiro (`0x416696`): bala 99 a ±20 px/tique.
pub const SHOT_SPEED: i32 = 20;
/// Alvo válido (`0x41678d`): `|dy|` ≤ 40, `|dx|` ≤ 1000, à vista.
pub const TARGET_DY: i32 = 40;
pub const TARGET_DX: i32 = 1000;
/// Queda rápida (1/8 px por tique) que a faz sumir (estado −2).
pub const FAST_FALL: i32 = 150;
/// Silêncio entre falas (o original espera a fala acabar; a cena não sabe).
pub const TAUNT_GAP: i32 = 60;

pub const VANISH: i8 = -2;
pub const HIDDEN: i8 = -1;
pub const RISE: i8 = 1;
pub const DIE: i8 = 11;

#[derive(Debug, Clone)]
pub struct DeathState {
    /// `+0x278`: os gatilhos já a soltaram.
    pub active: bool,
    pub state: i8,
    pub clip: Clip,
    /// `+0x264`: espera até a próxima aparição.
    pub wait: i32,
    /// Olha para a direita (sprite espelhado).
    pub right: bool,
    pub vx: i32,
    pub vy: i32,
    pub quiet: i32,
    pub wound: Wound,
    pub last_frame: Option<usize>,
}

impl Default for DeathState {
    fn default() -> Self {
        Self {
            active: false,
            state: HIDDEN,
            clip: Clip::new(1, false),
            wait: 0,
            right: false,
            vx: 0,
            vy: 0,
            quiet: 0,
            wound: Wound::default(),
            last_frame: None,
        }
    }
}

impl DeathState {
    /// Oculta (estado −1): sem desenho, colisão nem radiação.
    pub fn hidden(&self) -> bool {
        self.state == HIDDEN
    }

    fn set(&mut self, state: i8) {
        self.state = state;
        self.clip = match state {
            VANISH => Clip::new(1, true),
            s if s > 0 => Clip::new(s as u16, false),
            _ => Clip::new(1, false),
        };
    }

    /// `0x416753`: `(2 − dif)·75 + rand%100` tiques.
    fn rearm(&mut self, ctx: &mut Ctx) {
        self.wait = ctx.difficulty.death_wait_base() + ctx.rng.below(100) as i32;
    }
}

fn hide(e: &mut Entity, st: &mut DeathState, ctx: &mut Ctx) {
    st.set(HIDDEN);
    st.rearm(ctx);
    st.vx = 0;
    st.vy = 0;
    e.flags |= 0x2;
}

fn vanish(st: &mut DeathState, c: (i32, i32), layer: i32, out: &mut Out) {
    st.set(VANISH);
    out.events.push(Event::Sound(Cue::at(
        SOUND_ASCEND,
        PRIORITY_FRAME,
        c,
        layer,
    )));
}

/// Alvo válido (`0x41678d`).
fn valid_target(e: &Entity, ctx: &Ctx) -> Option<Rect> {
    let me = e.bounds()?;
    let j = enemy::jake_target(ctx, e.layer)?;
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    ((jc.1 - c.1).abs() <= TARGET_DY
        && (jc.0 - c.0).abs() <= TARGET_DX
        && enemy::line_of_sight(ctx, c, jc, &[me, j]))
    .then_some(j)
}

/// Aparição (`0x416251`): passa à camada do jogador e procura, ao lado da caixa dele, um
/// lugar livre com chão até 15 px abaixo (recuando de 10 em 10 px, os dois lados).
fn appear(e: &mut Entity, st: &mut DeathState, ctx: &mut Ctx, out: &mut Out) -> bool {
    let (Some(j), Some(rel)) = (ctx.jake_bounds(), e.boxes.first().copied()) else {
        return false;
    };
    let layer = ctx.jake_layer;
    let first = if ctx.rng.below(2) == 0 { 1 } else { -1 };
    let shift = 10 + ctx.rng.below(10) as i32;
    let cells = ctx.cells_for(layer);
    for side in [first, -first] {
        for back in 0..4 {
            let off = shift + back * 10;
            let bx = if side > 0 {
                j.x + j.w + off
            } else {
                j.x - rel.w - off
            };
            let by = j.y + j.h - rel.h - 5;
            let r = Rect {
                x: bx,
                y: by,
                w: rel.w,
                h: rel.h,
            };
            let c = enemy::center(&r);
            if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
                continue;
            }
            if cells.blocked(&r) || !cells.standing_on(&r, 15) {
                continue;
            }
            e.x = bx - rel.x;
            e.y = by - rel.y;
            e.layer = layer;
            e.flags &= !0x2;
            st.right = side < 0;
            st.set(RISE);
            st.vx = 0;
            st.vy = 0;
            out.events.push(Event::Sound(Cue::at(
                SOUND_ASCEND,
                PRIORITY_FRAME,
                c,
                layer,
            )));
            return true;
        }
    }
    false
}

/// Tiro (`0x416696`): bala 99 da boca, `vy` sorteado no espalhamento da dificuldade.
fn shoot(e: &Entity, st: &DeathState, ctx: &mut Ctx, out: &mut Out) {
    let s = ctx.difficulty.death_spread();
    let vy = ctx.rng.below(s as u32) as i32 - s / 2;
    let (x, vx) = if st.right {
        (e.x + 30, SHOT_SPEED)
    } else {
        (e.x - 20, -SHOT_SPEED)
    };
    out.enemy_spawns.push(Spawn::Shot {
        kind: super::enemy_shot::DEATH_BULLET,
        at: (x, e.y + 2),
        vel: (vx, vy),
        layer: e.layer,
    });
}

/// Fim de animação (`0x415d63`).
fn end_of_clip(e: &mut Entity, st: &mut DeathState, ctx: &mut Ctx, out: &mut Out) {
    let c = e.center();
    let has = valid_target(e, ctx).is_some();
    let third = ctx.rng.below(3);
    match st.state {
        VANISH => {
            if ctx.rng.below(2) == 0 || !appear(e, st, ctx, out) {
                hide(e, st, ctx);
            }
        }
        5 => {
            shoot(e, st, ctx, out);
            st.set(6);
        }
        7 => {
            shoot(e, st, ctx, out);
            st.set(8);
        }
        8 => match (has, third) {
            (true, 1) => st.set(7),
            (true, 2) => st.set(5),
            _ => vanish(st, c, e.layer, out),
        },
        DIE => {
            e.alive = false;
        }
        2 => st.set(if ctx.rng.below(2) == 0 { 4 } else { 3 }),
        3 => st.set(4),
        // 1, 4, 6
        _ => match (has, third) {
            (true, 1) => st.set(5),
            (true, 2) => st.set(7),
            _ => vanish(st, c, e.layer, out),
        },
    }
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let mut st = match &e.behaviour {
        Behaviour::Death(s) => s.as_ref().clone(),
        _ => return,
    };
    if !st.active {
        if !crate::triggers::satisfied(&e.triggers, ctx.on) {
            return;
        }
        st.active = true;
    }
    if st.quiet > 0 {
        st.quiet -= 1;
    }
    if st.state == HIDDEN {
        e.flags |= 0x2;
        if st.quiet == 0 && ctx.rng.below(300) < 2 && st.wait > 30 {
            let id = if ctx.rng.below(100) < 40 {
                TAUNTS[3]
            } else {
                TAUNTS[ctx.rng.below(3) as usize]
            };
            out.events.push(Event::Sound(Cue::ui(id, PRIORITY_FRAME)));
            st.quiet = TAUNT_GAP;
        }
        if st.wait > 0 {
            st.wait -= 1;
        } else if !appear(e, &mut st, ctx, out) {
            st.rearm(ctx);
        }
        e.behaviour = Behaviour::Death(Box::new(st));
        return;
    }
    let Some(me) = e.bounds() else {
        e.alive = false;
        return;
    };
    let c = enemy::center(&me);
    if st.state == RISE {
        st.vy = st.vy.max(-10);
    }
    // dano (`0x416979`): dificuldade, mínimo 1
    let w = st.wound.take();
    if !w.is_empty() && st.state != DIE {
        e.health -= ctx.difficulty.damage(w.pending.amount).max(1);
        if e.health <= 0 {
            st.set(DIE);
            e.on = true;
            out.events
                .push(Event::Sound(Cue::ui(SOUND_DIES, PRIORITY_FRAME)));
        }
    }
    if st.state != DIE && st.state != VANISH && st.vy >= FAST_FALL {
        vanish(&mut st, c, e.layer, out);
    }
    // radiação (`0x416a48`), também no −2 e no 11
    if let Some(j) = enemy::jake_target(ctx, e.layer) {
        let jc = enemy::center(&j);
        if (jc.0 - c.0).abs() < RADIATION_RANGE && (jc.1 - c.1).abs() < RADIATION_RANGE {
            let side = if jc.0 >= c.0 { 1 } else { -1 };
            out.jake_damage.add(&Pending {
                amount: RADIATION,
                shove: true,
                push: (side * RADIATION_PUSH, 0),
                ..Default::default()
            });
            out.events.push(Event::Sound(Cue::at(
                SOUND_RADIATION,
                PRIORITY_FRAME,
                c,
                e.layer,
            )));
        }
    }
    let (_, _, len) = enemy::label(&LABELS, st.clip.label);
    if st.clip.advance(len) {
        end_of_clip(e, &mut st, ctx, out);
    }
    if !e.alive || st.state == HIDDEN {
        e.behaviour = Behaviour::Death(Box::new(st));
        return;
    }
    // física com gravidade contra as células da camada
    let boxr = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = st.vx;
    body.vy = st.vy;
    step(&mut body, ctx.cells_for(e.layer), true);
    let nc = enemy::center(&body.bounds());
    if nc.0 < 0 || nc.1 < 0 || nc.0 >= ctx.level.0 || nc.1 >= ctx.level.1 {
        // não sai da fase: some e espera a próxima aparição
        hide(e, &mut st, ctx);
    } else {
        e.x = body.x;
        e.y = body.y;
        st.vx = body.vx;
        st.vy = body.vy;
    }
    let frame = enemy::show(e, &LABELS, &st.clip);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = st.right;
    }
    enemy::sound_frame(&SOUNDS, frame, &mut st.last_frame, c, e.layer, out);
    e.behaviour = Behaviour::Death(Box::new(st));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::DEATH;

    fn death(x: i32, y: i32) -> Entity {
        let mut e = logic(DEATH, x, y, &[]);
        e.flags = 0x84803;
        e.health = HEALTH;
        e.solid_shapes = true;
        e.boxes = vec![Rect {
            x: 33,
            y: -4,
            w: 15,
            h: 47,
        }];
        e.behaviour = Behaviour::Death(Box::default());
        e
    }

    fn st(e: &Entity) -> &DeathState {
        match &e.behaviour {
            Behaviour::Death(s) => s,
            _ => panic!(),
        }
    }

    #[test]
    fn death_waits_triggers_and_appears_beside_the_target() {
        let on = |id: i32| Some(id == 5);
        let (j, w) = ctx_with_jake(1000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut d = death(100, 100);
        d.triggers = vec![(4, Some(11))];
        for _ in 0..300 {
            tick(&mut d, &mut c, &mut Out::default());
        }
        assert!(
            !st(&d).active && st(&d).hidden(),
            "o gatilho 4 está desligado"
        );
        d.triggers = vec![];
        let mut out = Out::default();
        for _ in 0..200 {
            tick(&mut d, &mut c, &mut out);
            if !st(&d).hidden() {
                break;
            }
        }
        assert!(!st(&d).hidden(), "não apareceu");
        assert_eq!(d.flags & 0x2, 0, "visível tem colisão");
        let b = d.bounds().unwrap();
        let jb = j.body.bounds();
        assert!((b.x - jb.x).abs() < 60, "ao lado do Jake: {b:?} {jb:?}");
        assert_eq!(b.y + b.h, jb.y + jb.h - 5, "pés alinhados, y − 5");
        assert!(
            out.events
                .iter()
                .any(|e| e.sound_id() == Some(SOUND_ASCEND))
        );
    }

    #[test]
    fn radiation_hurts_within_45_and_bullets_fly() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(1000, 0);
        let mut c = ctx(&j, &w, &on);
        // caixa x 1028..1043, a 20 px do centro do Jake (1011)
        let mut d = death(995, 0);
        let Behaviour::Death(s) = &mut d.behaviour else {
            panic!()
        };
        s.active = true;
        s.set(5);
        d.flags &= !0x2;
        let mut out = Out::default();
        for _ in 0..9 {
            tick(&mut d, &mut c, &mut out);
        }
        assert!(out.jake_damage.amount >= 9 * RADIATION);
        assert!(out.jake_damage.shove);
        assert!(
            out.events
                .iter()
                .any(|e| e.sound_id() == Some(SOUND_RADIATION))
        );
        assert!(
            out.enemy_spawns
                .iter()
                .any(|s| matches!(s, Spawn::Shot { kind: 99, .. })),
            "o fim do 5 atira"
        );
    }

    #[test]
    fn death_dies_with_its_line_and_easy_doubles_damage() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        c.difficulty = crate::Difficulty::Easy;
        let mut d = death(100, 7);
        let Behaviour::Death(s) = &mut d.behaviour else {
            panic!()
        };
        s.active = true;
        s.set(RISE);
        d.flags &= !0x2;
        let p = Pending {
            amount: 2,
            ..Default::default()
        };
        enemy::hurt(&mut d, &p, true);
        tick(&mut d, &mut c, &mut Out::default());
        assert_eq!(d.health, HEALTH - 4, "fácil: ×2");
        d.health = 3;
        enemy::hurt(&mut d, &p, true);
        let mut out = Out::default();
        tick(&mut d, &mut c, &mut out);
        assert_eq!(st(&d).state, DIE);
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND_DIES)));
        for _ in 0..20 {
            if d.alive {
                tick(&mut d, &mut c, &mut Out::default());
            }
        }
        assert!(!d.alive && enemy::killed(&d));
    }

    #[test]
    fn hidden_death_has_no_collision_and_stays_present() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        c.jake = None;
        let mut d = death(100, 7);
        for _ in 0..400 {
            tick(&mut d, &mut c, &mut Out::default());
        }
        assert!(d.alive && st(&d).hidden() && d.flags & 0x2 != 0);
        assert!(!d.visible());
    }
}
