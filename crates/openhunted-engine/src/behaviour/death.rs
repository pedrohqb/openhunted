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
/// Velocidade que a sonda de chão da aparição deixa (`0x416646`: `+0x168 = 0x78`).
pub const PROBE_VY: i32 = 0x78;
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

/// Some (fim do −2 ou do −1): sem rearmar, a espera é a da última tentativa de aparição.
fn hide(e: &mut Entity, st: &mut DeathState) {
    st.set(HIDDEN);
    st.vx = 0;
    st.vy = 0;
    e.flags |= 0x2;
}

/// Sons da Morte: todos sem posição (`0x43c93d(id, 0, …)`), salvo a radiação.
fn say(id: u16, out: &mut Out) {
    out.events.push(Event::Sound(Cue::ui(id, 0)));
}

fn vanish(st: &mut DeathState, out: &mut Out) {
    st.set(VANISH);
    say(SOUND_ASCEND, out);
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

/// Deslocamento horizontal livre de `r` contra as células, até `d` px (varredura
/// `0x432e5d` com `+0x15c = 8·d`): devolve o `+0x15c` resultante, em 1/8 px — inteiro se
/// livre; no contato, o ponto de contato menos 1/8 px.
fn sweep_x(cells: &crate::physics::World, r: &Rect, d: i32) -> i32 {
    let unit = d.signum();
    let mut m = 0;
    while m != d {
        if cells.blocked_from(r, &r.moved(m + unit, 0)) {
            return 8 * m - unit;
        }
        m += unit;
    }
    8 * d
}

/// Chão até 15 px abaixo (`0x416646`: varredura com `vy = 0x78`).
fn ground(cells: &crate::physics::World, e: &Entity, rel: &Rect) -> bool {
    let r = Rect {
        x: e.x + rel.x,
        y: e.y + rel.y,
        ..*rel
    };
    cells.standing_on(&r, 15)
}

/// Aparição (`0x416251`): sorteia o alvo (os jogadores vivos; no solo, o Jake) e passa à
/// camada dele; alinha a borda da caixa com a do alvo, do lado sorteado, e os pés − 5
/// (`0x416517`); varre `d` = 10–19 px para o lado e **soma o deslocamento em 1/8 px à
/// posição em px** (`0x4163ac`: na prática, 80–152 px); procura chão recuando 10 px por vez
/// para o alvo enquanto estiver a ≥ 20 px da origem dele. Duas tentativas: a falha volta à
/// origem do alvo (sem realinhar) e troca o lado; uma varredura de 0 < |v| < 20 (1/8 px)
/// também. Fora da fase ou da grade → falha. Na falha a Morte fica onde parou (a origem do
/// alvo; oráculo `D_J_012` t1).
fn appear(e: &mut Entity, st: &mut DeathState, ctx: &mut Ctx) -> bool {
    let (Some(j), Some(rel)) = (ctx.jake.filter(|j| j.is_active()), e.boxes.first().copied())
    else {
        return false;
    };
    // `rand % n` dos jogadores vivos (n = 1 no solo)
    ctx.rng.below(1);
    let t = (j.body.x, j.body.y);
    let tb = j.body.boxr;
    e.layer = ctx.jake_layer;
    let mut d = ctx.rng.below(10) as i32 + 10;
    if ctx.rng.below(2) == 1 {
        d = -d;
    }
    // `0x416517`
    e.x = t.0
        + if d > 0 {
            (tb.x + tb.w) - (rel.x + rel.w)
        } else {
            tb.x - rel.x
        };
    e.y = t.1 + (tb.y + tb.h) - (rel.y + rel.h) - 5;
    let cells = ctx.cells_for(e.layer);
    let mut found = false;
    let mut tries = 0;
    while !found && tries < 2 {
        tries += 1;
        let r = Rect {
            x: e.x + rel.x,
            y: e.y + rel.y,
            ..rel
        };
        let v = sweep_x(cells, &r, d);
        if v != 8 * d && v != 0 && v.abs() < 20 {
            d = -d;
            (e.x, e.y) = t;
            continue;
        }
        e.x += v;
        let near = if v > 0 { t.0 + 20 } else { t.0 - 20 };
        while !found {
            let far = if v > 0 { e.x >= near } else { e.x <= near };
            if !far {
                break;
            }
            if ground(cells, e, &rel) {
                found = true;
            } else if near > t.0 {
                e.x -= 10;
            } else {
                e.x += 10;
            }
        }
        if !found {
            d = -d;
            (e.x, e.y) = t;
        }
    }
    // `0x4165d5`: posição fora da fase
    if e.x < 0 || e.y < 0 || e.x >= ctx.level.0 || e.y >= ctx.level.1 || !found {
        return false;
    }
    st.right = e.x < t.0;
    ground(cells, e, &rel)
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

/// Saída da aparição bem-sucedida: estado 1, `0x2` desligado, som 3051 sem posição; a
/// velocidade é a que a sonda de chão deixou (`(0, 0x78)`).
fn rise(e: &mut Entity, st: &mut DeathState, out: &mut Out) {
    e.flags &= !0x2;
    st.set(RISE);
    st.vx = 0;
    st.vy = PROBE_VY;
    say(SOUND_ASCEND, out);
}

/// Fim de animação (`0x415d63`).
fn end_of_clip(e: &mut Entity, st: &mut DeathState, ctx: &mut Ctx, out: &mut Out) {
    let has = valid_target(e, ctx).is_some();
    let third = ctx.rng.below(3);
    match st.state {
        VANISH => {
            // ½ some; ½ reaparece (sem rearmar) ou some
            if ctx.rng.below(2) == 1 && appear(e, st, ctx) {
                rise(e, st, out);
            } else {
                hide(e, st);
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
            _ => vanish(st, out),
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
            _ => vanish(st, out),
        },
    }
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let mut st = match &e.behaviour {
        Behaviour::Death(s) => s.as_ref().clone(),
        _ => return,
    };
    if !st.active {
        // o tique que solta os gatilhos não faz mais nada (`0x41588b`)
        st.active = crate::triggers::satisfied(&e.triggers, ctx.on);
        e.behaviour = Behaviour::Death(Box::new(st));
        return;
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
            say(id, out);
            st.quiet = TAUNT_GAP;
        }
        if st.wait > 0 {
            st.wait -= 1;
            e.behaviour = Behaviour::Death(Box::new(st));
            return;
        }
        // zerada: tenta aparecer e rearma nos dois casos (`0x415bb2`/`0x415bbb`); se
        // aparece, o tique segue no estado 1
        let shown = appear(e, &mut st, ctx);
        st.rearm(ctx);
        if !shown {
            e.behaviour = Behaviour::Death(Box::new(st));
            return;
        }
        rise(e, &mut st, out);
    }
    let Some(me) = e.bounds() else {
        e.alive = false;
        return;
    };
    let c = enemy::center(&me);
    if st.state == RISE {
        st.vy = st.vy.max(-10);
    }
    // dano (`0x416979`): 0 não faz nada; dificuldade, só o negativo vira 1
    let w = st.wound.take();
    if let Some(d) = ctx.difficulty.hit(w.pending.amount)
        && st.state != DIE
    {
        e.health -= d;
        if e.health <= 0 {
            st.set(DIE);
            e.on = true;
            say(SOUND_DIES, out);
        }
    } else if st.state != DIE && st.state != VANISH && st.vy >= FAST_FALL {
        // queda rápida: some, com o som, e rearma (`0x415c10`)
        vanish(&mut st, out);
        st.rearm(ctx);
    }
    // radiação (`0x416a48`), também no −2 e no 11: posições (`+0x154/+0x158`), não centros
    if let Some(j) = ctx
        .jake
        .filter(|j| j.is_active() && ctx.jake_layer == e.layer)
    {
        let (jx, jy) = (j.body.x, j.body.y);
        if (jx - e.x).abs() < RADIATION_RANGE && (jy - e.y).abs() < RADIATION_RANGE {
            let side = if jx >= e.x { 1 } else { -1 };
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
        hide(e, &mut st);
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
        let (j, w) = ctx_with_jake(1000, 200);
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
        assert!((b.x - jb.x).abs() < 160, "perto do Jake: {b:?} {jb:?}");
        assert_eq!(b.y + b.h, jb.y + jb.h, "pés alinhados − 5, e cai no chão");
        assert!(
            out.events
                .iter()
                .any(|e| e.sound_id() == Some(SOUND_ASCEND))
        );
    }

    /// Oráculo `D_J_012`: a Morte aparece com a borda da caixa alinhada à do alvo e depois
    /// desloca-se `8·d` px (`d` = 10–19: o deslocamento em 1/8 px da varredura `0x432e5d`
    /// somado à posição em px, `0x4163ac`): x 1083 (d −11), 1302 (16), 1254 (10) com o Jake
    /// em 1202; y com os pés alinhados − 5.
    #[test]
    fn death_appears_eight_times_the_shift_away() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(1000, 200);
        let (mut left, mut right) = (false, false);
        for seed in 0..40 {
            let mut c = ctx(&j, &w, &on);
            *c.rng = crate::rng::Rng::new(seed);
            let mut d = death(100, 100);
            let mut out = Out::default();
            for _ in 0..2 {
                tick(&mut d, &mut c, &mut out);
            }
            assert!(!st(&d).hidden(), "chão plano: aparece no 2º tique");
            let dx = d.x - j.body.x;
            if st(&d).right {
                // borda esquerda alinhada: 2 − 33 = −31
                assert!(
                    (dx + 31) % 8 == 0 && (-152..=-80).contains(&(dx + 31)),
                    "{dx}"
                );
                left = true;
            } else {
                // borda direita alinhada: (2 + 18) − (33 + 15) = −28
                assert!(
                    (dx + 28) % 8 == 0 && (80..=152).contains(&(dx + 28)),
                    "{dx}"
                );
                right = true;
            }
            assert_eq!(d.y, 200 + 47 - 43, "pés − 5, e cai 5 px no mesmo tique");
            assert_eq!(
                st(&d).clip.step,
                1,
                "o tique continua: quadro 1 (oráculo t161 q1)"
            );
            assert!(out.events.iter().any(|e| matches!(
                e,
                Event::Sound(c) if c.id == SOUND_ASCEND && c.at.is_none()
            )));
        }
        assert!(left && right, "os dois lados");
    }

    /// Oráculo `D_J_012` t1: com o Jake no ar a aparição falha e a Morte fica na origem
    /// dele (1202, 309), oculta; o tique da ativação pelos gatilhos não faz mais nada.
    #[test]
    fn failed_appearance_leaves_death_on_the_target() {
        let on = |_: i32| None;
        let (mut j, w) = ctx_with_jake(1000, 0);
        j.body = crate::physics::Body::new(500, 300, j.body.boxr);
        let mut c = ctx(&j, &w, &on);
        let mut d = death(100, 100);
        tick(&mut d, &mut c, &mut Out::default());
        assert!(st(&d).active);
        assert_eq!((d.x, d.y), (100, 100), "tique da ativação");
        tick(&mut d, &mut c, &mut Out::default());
        assert!(st(&d).hidden());
        assert_eq!((d.x, d.y), (500, 300), "na origem do alvo");
        assert!(st(&d).wait >= 75, "rearma (`0x416753`)");
    }

    /// Radiação (`0x416a48`): `|Δ|` das posições (`+0x154/+0x158`), não dos centros.
    #[test]
    fn radiation_measures_origins() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(1000, 0);
        for (x, hit) in [(1044, true), (950, false)] {
            let mut c = ctx(&j, &w, &on);
            let mut d = death(x, 0);
            let Behaviour::Death(s) = &mut d.behaviour else {
                panic!()
            };
            s.active = true;
            s.set(RISE);
            d.flags &= !0x2;
            let mut out = Out::default();
            tick(&mut d, &mut c, &mut out);
            assert_eq!(out.jake_damage.amount == RADIATION, hit, "x {x}");
        }
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
    fn zero_damage_does_nothing_and_hard_halves_one_to_zero() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        c.difficulty = crate::Difficulty::Hard;
        let mut d = death(100, 7);
        let Behaviour::Death(s) = &mut d.behaviour else {
            panic!()
        };
        s.active = true;
        s.set(RISE);
        d.flags &= !0x2;
        d.health = 1;
        // borda de explosão (`explosion::area_effect`): dano 0, natureza explosão
        let rim = Pending {
            amount: 0,
            explosion: true,
            ..Default::default()
        };
        enemy::hurt(&mut d, &rim, false);
        tick(&mut d, &mut c, &mut Out::default());
        assert_eq!(d.health, 1, "dano 0 não fere (`0x416979`)");
        let one = Pending {
            amount: 1,
            ..Default::default()
        };
        enemy::hurt(&mut d, &one, true);
        tick(&mut d, &mut c, &mut Out::default());
        assert_eq!(d.health, 1, "difícil: 1 / 2 = 0, sem mínimo");
        assert_ne!(st(&d).state, DIE);
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
        // oculta (`0x2`), não recolhe dano de explosão
        let hp = d.health;
        let b = crate::behaviour::Blast {
            center: d.center(),
            layer: d.layer,
            radius: 60,
            push: 10,
            damage: 30,
        };
        crate::behaviour::interact::blast(&b, &mut d);
        assert!(
            st(&d).wound.is_empty(),
            "oculta, não guarda o dano para quando aparecer"
        );
        tick(&mut d, &mut c, &mut Out::default());
        assert_eq!(d.health, hp);
    }
}
