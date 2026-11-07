// SPDX-License-Identifier: GPL-3.0-or-later
//! Anticorpo (tipo 105, `OAB.ANM`, `0x41129d`) e Manta (89, `OMR.ANM`, `0x421475`;
//! `docs/re/anticorpo-manta.md`, `inimigos-menores-2.md`): voadores com o mesmo código —
//! descansam, passeiam em laços, mergulham no jogador ou o perseguem e dão o bote.

use super::enemy::{self, Clip, Label, Wound};
use super::{Blast, Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{ANTIBODY, Behaviour, Entity};
use crate::physics::{Body, Rect, step};
use crate::sound::{Cue, PRIORITY_FRAME};

pub const HEALTH: i32 = 1;
/// Rótulos: Anticorpo 1 `oab00` (5), 2 `oab01` (8, bote); Manta 1–5 (5 depois do bote).
pub const ANTIBODY_LABELS: [Label; 2] = [(1, 0, 5), (2, 5, 8)];
pub const MANTA_LABELS: [Label; 5] = [(1, 0, 10), (2, 10, 12), (3, 22, 12), (4, 34, 5), (5, 39, 5)];
/// `SOUND.TBL` tipo 89.
pub const MANTA_SOUNDS: [(usize, u16, u16); 3] = [(0, 3161, 4), (12, 3162, 5), (37, 3160, 8)];
/// Anticorpo: 3071 no bote; 3072/3073 no descanso, a cada ≥ 250 ms.
pub const SOUND_BITE: u16 = 3071;
pub const SOUND_REST_A: u16 = 3072;
pub const SOUND_REST_B: u16 = 3073;
pub const SOUND_REST_TICKS: u32 = 5;
/// Bote da perseguição a menos de 40 px (`0x4225ab`).
pub const LUNGE_RANGE: i32 = 40;
/// Limite de `vx` (1/8 px).
pub const VX_RANGE: (i32, i32) = (-32, 37);
/// Varredura da Manta: além da borda, 75 px à direita, 60 à esquerda.
pub const SWEEP: (i32, i32) = (75, 60);

/// Variante (registros de 0x48: `0x4720c0` / `0x473180`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    pub range: i32,
    pub rest: i32,
    pub rest_again: u32,
    pub attack: u32,
    pub loops: i32,
    pub speed: i32,
    pub max_ticks: i32,
    pub dive: i32,
    /// Dano do bote (`(105|89, 0, v)`).
    pub bite: i32,
    /// % do dano recebido na Manta (o Anticorpo leva 100).
    pub manta_applied: i32,
}

pub const VARIANTS: [Variant; 4] = [
    Variant {
        range: 200,
        rest: 1,
        rest_again: 0,
        attack: 2,
        loops: 2,
        speed: 8,
        max_ticks: 50,
        dive: 60,
        bite: 15,
        manta_applied: 100,
    },
    Variant {
        range: 150,
        rest: 5,
        rest_again: 10,
        attack: 3,
        loops: 3,
        speed: 3,
        max_ticks: 40,
        dive: 40,
        bite: 20,
        manta_applied: 70,
    },
    Variant {
        range: 200,
        rest: 1,
        rest_again: 0,
        attack: 2,
        loops: 2,
        speed: 8,
        max_ticks: 50,
        dive: 60,
        bite: 25,
        manta_applied: 50,
    },
    Variant {
        range: 250,
        rest: 10,
        rest_again: 10,
        attack: 2,
        loops: 1,
        speed: 15,
        max_ticks: 30,
        dive: 80,
        bite: 30,
        manta_applied: 40,
    },
];

/// Modos (`+0x270`).
pub const REST: u8 = 0;
pub const WANDER: u8 = 1;
pub const DIVE: u8 = 2;
pub const CHASE: u8 = 3;
pub const LUNGE: u8 = 4;
pub const RETURN: u8 = 5;

#[derive(Debug, Clone)]
pub struct FlyerState {
    pub manta: bool,
    pub variant: usize,
    /// Anticorpo dormente (`values[6] & 0xff = 0`: o tique não faz nada).
    pub dormant: bool,
    pub mode: u8,
    pub home: (i32, i32),
    /// Ângulo de 16 bits e sentido do giro.
    pub angle: i32,
    pub spin: i32,
    /// Tiques no modo e no descanso.
    pub ticks: i32,
    pub rest: i32,
    pub clip: Clip,
    /// Pontos da varredura da Manta ainda por visitar.
    pub sweep: Vec<(i32, i32)>,
    pub vx: i32,
    pub vy: i32,
    pub seed: u32,
    pub wound: Wound,
    pub last_frame: Option<usize>,
}

impl FlyerState {
    pub fn new(kind: u32, values: &[i32], at: (i32, i32), seed: i32) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        let manta = kind != ANTIBODY;
        let s = seed as u32;
        let variant = v(0).clamp(0, 3) as usize;
        let r = VARIANTS[variant].rest;
        Self {
            manta,
            variant,
            dormant: !manta && v(6) & 0xff == 0,
            mode: REST,
            home: at,
            angle: (s.wrapping_mul(2_654_435_761) >> 16) as i32 & 0xffff,
            spin: if s & 1 == 0 { 1 } else { -1 },
            ticks: 0,
            rest: (s % r.max(1) as u32) as i32 + r,
            clip: Clip::new(if manta { 3 } else { 1 }, false),
            sweep: vec![],
            vx: 0,
            vy: 0,
            seed: s,
            wound: Wound::default(),
            last_frame: None,
        }
    }

    pub fn variant(&self) -> Variant {
        VARIANTS[self.variant.min(3)]
    }

    fn labels(&self) -> &'static [Label] {
        if self.manta {
            &MANTA_LABELS
        } else {
            &ANTIBODY_LABELS
        }
    }

    fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
        self.ticks = 0;
        let label = match (self.manta, mode) {
            (false, LUNGE) => 2,
            (false, _) => 1,
            (true, LUNGE) => 4,
            (true, REST) => 1,
            (true, _) => 2,
        };
        if self.clip.label != label {
            self.clip = Clip::new(label, false);
        }
    }

    /// Recomeça o passeio (`0x42250b`): casa nova, ângulo sorteado.
    fn wander(&mut self, at: (i32, i32), ctx: &mut Ctx) {
        self.home = at;
        self.angle = ctx.rng.below(65536) as i32;
        self.spin = if ctx.rng.below(2) == 0 { 1 } else { -1 };
        self.set_mode(WANDER);
    }
}

/// Velocidade para um ponto (`0x421e9b`): metade da diferença (1/8 px), limitada só por cima
/// ao mergulho; componentes pequenas dobram. `None` = chegou (distância < largura + 15).
fn steer(c: (i32, i32), p: (i32, i32), w: i32, cap: i32) -> Option<(i32, i32)> {
    if super::mine::distance(c, p) < w + 15 {
        return None;
    }
    let f = |d: i32| {
        let v = (d / 2).min(cap);
        if v.abs() < 10 { v * 2 } else { v }
    };
    Some((f(p.0 - c.0), f(p.1 - c.1)))
}

/// Percepção: jogador vivo na camada, a ≤ alcance, à vista.
fn target(c: (i32, i32), me: Rect, layer: i32, v: &Variant, ctx: &Ctx) -> Option<Rect> {
    let j = enemy::jake_target(ctx, layer)?;
    let jc = enemy::center(&j);
    (super::mine::distance(c, jc) <= v.range && enemy::line_of_sight(ctx, c, jc, &[me, j]))
        .then_some(j)
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let mut st = match &e.behaviour {
        Behaviour::Flyer(s) => s.as_ref().clone(),
        _ => return,
    };
    if st.dormant {
        return;
    }
    let Some(me) = e.bounds() else {
        e.alive = false;
        return;
    };
    let c = enemy::center(&me);
    let v = st.variant();
    // fora da fase → vida 0
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.health = 0;
    }
    let w = st.wound.take();
    if !w.is_empty() {
        let applied = if st.manta { v.manta_applied } else { 100 };
        e.health -= ctx.difficulty.damage(w.pending.amount).max(1) * applied / 100;
    }
    if e.health <= 0 {
        out.effects.push((
            Blast {
                center: c,
                layer: e.layer,
                radius: if st.manta { 12 } else { 15 },
                push: 0,
                damage: 0,
            },
            if st.manta {
                14
            } else {
                1 + ctx.rng.below(4) as u16
            },
        ));
        e.alive = false;
        e.on = true;
        return;
    }
    let jake = target(c, me, e.layer, &v, ctx);
    st.ticks += 1;
    let (mut vx, mut vy) = (0, 0);
    let (_, _, len) = enemy::label(st.labels(), st.clip.label);
    let ended = st.clip.advance(len);
    match st.mode {
        REST => {
            if ended {
                st.clip = Clip::new(st.clip.label, false);
                if !st.manta && ctx.rng.below(100) < 30 {
                    let id = if ctx.rng.below(2) == 0 {
                        SOUND_REST_A
                    } else {
                        SOUND_REST_B
                    };
                    out.events
                        .push(Event::Sound(Cue::at(id, PRIORITY_FRAME, c, e.layer)));
                }
            }
            if st.rest > 0 {
                st.rest -= 1;
            } else if ctx.rng.below(100) < v.rest_again + ctx.rng.below(5) {
                st.rest = ctx.rng.below(v.rest.max(1) as u32) as i32 + v.rest;
            } else if jake.is_some() {
                if ctx.rng.below(100) < v.attack {
                    let mode = if ctx.rng.below(2) == 0 { DIVE } else { CHASE };
                    st.set_mode(mode);
                    st.sweep.clear();
                }
            } else {
                st.wander(c, ctx);
            }
        }
        WANDER => {
            let (cos, sin) = super::cos_sin14(st.angle);
            vx = ((cos * v.speed) >> 11).min(v.dive);
            vy = ((sin * v.speed) >> 11).min(v.dive);
            st.angle = (st.angle + st.spin * 0x1000) & 0xffff;
            // 16 tiques = uma volta do passeio
            if st.ticks >= v.max_ticks || st.ticks >= 16 * v.loops {
                st.set_mode(REST);
                st.rest = ctx.rng.below(v.rest.max(1) as u32) as i32 + v.rest;
            }
        }
        DIVE | CHASE => {
            let Some(j) = jake else {
                st.set_mode(RETURN);
                e.behaviour = Behaviour::Flyer(Box::new(st));
                return;
            };
            let jc = enemy::center(&j);
            let point = if st.mode == DIVE {
                jc
            } else if st.manta {
                if st.sweep.is_empty() {
                    let top = j.y - ctx.rng.below(10) as i32;
                    st.sweep = if c.0 > jc.0 {
                        vec![(j.x + j.w, top), (j.x + j.w - SWEEP.1 - j.w, top)]
                    } else {
                        vec![(j.x, top), (j.x + SWEEP.0, top)]
                    };
                }
                st.sweep[0]
            } else {
                // pelo lado do alvo em que ele está
                let side = if c.0 > jc.0 { 1 } else { -1 };
                (jc.0 + side * (j.w / 2 + 20), jc.1)
            };
            let close = super::mine::distance(c, jc) < LUNGE_RANGE;
            match steer(c, point, me.w, v.dive) {
                None if st.mode == DIVE => st.set_mode(LUNGE),
                None if st.manta && st.sweep.len() > 1 => {
                    st.sweep.remove(0);
                }
                _ if st.mode == CHASE && close => st.set_mode(LUNGE),
                None => st.set_mode(RETURN),
                Some(d) => (vx, vy) = d,
            }
            if st.mode == CHASE && st.ticks >= v.max_ticks
                || st.mode == DIVE && st.ticks >= 2 * v.max_ticks
            {
                st.set_mode(RETURN);
            }
        }
        LUNGE => {
            if ended {
                // `0x412249` / `0x422416`: o bote fere o alvo, sem novo teste de distância
                if let Some(j) = enemy::jake_target(ctx, e.layer) {
                    out.jake_damage.add(&Pending {
                        amount: v.bite,
                        shove: st.manta,
                        ..Default::default()
                    });
                    if !st.manta {
                        out.events.push(Event::Sound(Cue::at(
                            SOUND_BITE,
                            PRIORITY_FRAME,
                            enemy::center(&j),
                            e.layer,
                        )));
                    }
                }
                if st.manta {
                    // `0x4223e2`: a casa espelhada em x pela posição atual
                    st.home = (2 * c.0 - st.home.0, c.1 - ctx.rng.below(10) as i32);
                    st.clip = Clip::new(5, false);
                }
                st.mode = RETURN;
                st.ticks = 0;
            }
        }
        _ => {
            // volta à casa (`0x4221b0`)
            if st.manta && st.clip.label == 5 && ended {
                st.clip = Clip::new(2, false);
            }
            match steer(c, st.home, me.w, v.dive) {
                Some(d) if st.ticks < 4 * v.max_ticks => (vx, vy) = d,
                _ => st.wander(c, ctx),
            }
        }
    }
    if st.clip.label != 5 && !matches!(st.mode, LUNGE) && ended {
        st.clip = Clip::new(st.clip.label, false);
    }
    vx = vx.clamp(VX_RANGE.0, VX_RANGE.1);
    // sem gravidade: a velocidade pedida a compensa
    let boxr = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = vx;
    body.vy = vy;
    step(&mut body, ctx.cells_for(e.layer), false);
    e.x = body.x;
    e.y = body.y;
    // contato com o cenário → para (`+0x220 & 2`)
    if body.hit_wall || body.hit_ceiling || body.on_ground {
        st.vx = 0;
        st.vy = 0;
        if st.mode == WANDER {
            st.spin = -st.spin;
        }
    } else {
        st.vx = body.vx;
        st.vy = body.vy;
    }
    let frame = enemy::show(e, st.labels(), &st.clip);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = st.vx > 0;
    }
    if st.manta {
        enemy::sound_frame(&MANTA_SOUNDS, frame, &mut st.last_frame, c, e.layer, out);
    }
    e.behaviour = Behaviour::Flyer(Box::new(st));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::MANTA;

    fn flyer(kind: u32, x: i32, y: i32, variant: i32, awake: i32) -> Entity {
        let values = [variant, 0, 0, 0, 0, 0, awake];
        let mut e = logic(kind, x, y, &values);
        e.health = HEALTH;
        e.flags = 0x84081;
        e.solid_shapes = true;
        e.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 18,
            h: 18,
        }];
        e.behaviour = Behaviour::Flyer(Box::new(FlyerState::new(kind, &values, (x, y), 7)));
        e
    }

    fn st(e: &Entity) -> &FlyerState {
        match &e.behaviour {
            Behaviour::Flyer(s) => s,
            _ => panic!(),
        }
    }

    #[test]
    fn wander_loops_and_comes_back_to_rest() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 600);
        let mut c = ctx(&j, &w, &on);
        let mut a = flyer(ANTIBODY, 500, 300, 0, 1);
        let mut modes = vec![];
        let mut xs = vec![];
        for _ in 0..120 {
            tick(&mut a, &mut c, &mut Out::default());
            modes.push(st(&a).mode);
            xs.push(a.x);
        }
        assert!(modes.contains(&WANDER) && modes.contains(&REST));
        let spread = xs.iter().max().unwrap() - xs.iter().min().unwrap();
        assert!(
            spread > 10 && spread < 200,
            "laços em volta da casa: {spread}"
        );
    }

    #[test]
    fn antibody_dives_and_bites_the_jake() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(300, 600);
        let mut c = ctx(&j, &w, &on);
        let mut a = flyer(ANTIBODY, 200, 560, 3, 1);
        let mut out = Out::default();
        for _ in 0..600 {
            tick(&mut a, &mut c, &mut out);
            if out.jake_damage.amount > 0 {
                break;
            }
        }
        assert_eq!(out.jake_damage.amount, 30, "Delta: 30");
        assert!(!out.jake_damage.shove, "o Anticorpo não empurra");
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND_BITE)));
    }

    #[test]
    fn manta_sweeps_and_shoves() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(300, 600);
        let mut c = ctx(&j, &w, &on);
        let mut m = flyer(MANTA, 180, 570, 0, 0);
        assert!(!st(&m).dormant, "a Manta usa a vida, não o byte");
        let mut out = Out::default();
        for _ in 0..800 {
            tick(&mut m, &mut c, &mut out);
            if out.jake_damage.amount > 0 {
                break;
            }
        }
        assert_eq!(out.jake_damage.amount, 15);
        assert!(out.jake_damage.shove);
    }

    #[test]
    fn dormant_antibody_does_nothing_and_one_bullet_kills() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(300, 0);
        let mut c = ctx(&j, &w, &on);
        let mut d = flyer(ANTIBODY, 250, 0, 0, 0);
        for _ in 0..50 {
            tick(&mut d, &mut c, &mut Out::default());
        }
        assert_eq!((d.x, d.y), (250, 0));
        let mut a = flyer(ANTIBODY, 250, 200, 0, 1);
        let p = Pending {
            amount: 2,
            ..Default::default()
        };
        assert!(enemy::hurt(&mut a, &p, true));
        let mut out = Out::default();
        tick(&mut a, &mut c, &mut out);
        assert!(!a.alive && enemy::killed(&a));
        assert_eq!(out.effects.len(), 1);
    }
}
