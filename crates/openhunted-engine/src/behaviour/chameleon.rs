// SPDX-License-Identifier: GPL-3.0-or-later
//! Camaleão (tipo 4, `OCH.ANM`, `0x41853c`/`0x418704`; `docs/re/inimigos-menores-2.md`):
//! anda, vira, morde o jogador perto (30), recua ferido, fica invisível e se clona.

use super::enemy::{self, Clip, Label, Wound};
use super::{Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};
use crate::sound::{Cue, PRIORITY_FRAME};

/// Rótulos do `OCH.ANM` usados (rótulo = estado).
pub const LABELS: [Label; 12] = [
    (1, 0, 5),
    (3, 15, 13),
    (4, 28, 6),
    (7, 50, 9),
    (12, 98, 10),
    (13, 108, 12),
    (18, 120, 53),
    (26, 212, 1),
    (27, 213, 6),
    (28, 219, 15),
    (34, 251, 27),
    (10, 87, 3),
];
/// `SOUND.TBL` tipo 4.
pub const SOUNDS: [(usize, u16, u16); 3] = [(5, 3147, 8), (77, 3147, 0), (82, 3147, 0)];
pub const SOUND_BITE: u16 = 3147;
pub const SOUND_INVISIBLE: u16 = 3155;
pub const SOUND_SPAWNS: u16 = 3154;
pub const SOUND_DIES: u16 = 5119;
pub const SOUNDS_AMBIENT: [u16; 3] = [5120, 5121, 5122];
/// Mordida (`0x41a624`): a ≤ 50 px (centros); linha `(4, 1)`: 30, natureza `0x10`.
pub const BITE_RANGE: i32 = 50;
pub const BITE: i32 = 30;
/// `|dy|` entre centros para morder (`0x41a4ca`).
pub const BITE_DY: i32 = 30;
/// Recarga do recuo ferido (`+0x26a`).
pub const FLINCH_RECHARGE: i32 = 40;
/// Velocidade do andar (px/tique, inferida).
pub const SPEED: i32 = 3;
/// Clones vivos no máximo.
pub const MAX_CLONES: usize = 3;
/// Tiques até sumir de todo (o limiar desce 7 por quadro de 0xf7 a 0x91).
pub const FADE_TICKS: i32 = 15;
/// Queda (1/8 px por tique) que põe o estado 26.
pub const FALLING: i32 = 40;
/// Tiques no mesmo x que o dão por preso (`0x41a6cd`).
pub const STUCK_TICKS: i32 = 13;

pub const IDLE: u16 = 34;
pub const WALK: u16 = 4;
pub const TURN: u16 = 28;
pub const BITING: u16 = 13;
pub const LOOK: u16 = 7;
pub const FLINCH: u16 = 12;
pub const DYING: u16 = 18;
pub const FALL: u16 = 26;
pub const LAND: u16 = 27;
pub const STUCK: u16 = 3;
pub const PAUSE: u16 = 1;

/// Variante (`0x47256c`, registros de 0x38): alcance (`|dx|`) e vida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant {
    pub range: i32,
    pub health: i32,
}

pub const VARIANTS: [Variant; 5] = [
    Variant {
        range: 250,
        health: 80,
    },
    Variant {
        range: 300,
        health: 50,
    },
    Variant {
        range: 250,
        health: 80,
    },
    Variant {
        range: 250,
        health: 80,
    },
    Variant {
        range: 250,
        health: 80,
    },
];

#[derive(Debug, Clone)]
pub struct ChameleonState {
    pub variant: usize,
    /// Marca do original (0 = é o original).
    pub clone_of: u32,
    /// Marca própria (índice + 1, posta por `sync`) e clones vivos.
    pub tag: u32,
    pub clones: usize,
    pub state: u16,
    pub clip: Clip,
    pub right: bool,
    pub vx: i32,
    pub vy: i32,
    pub recharge: i32,
    /// Clonagem em curso (`+0x26e`, decresce a cada 3 tiques).
    pub cloning: Option<i32>,
    pub invisible: bool,
    pub fade: i32,
    pub still: i32,
    pub last_x: i32,
    pub wound: Wound,
    pub last_frame: Option<usize>,
}

impl ChameleonState {
    pub fn new(variant: i32) -> Self {
        Self {
            variant: variant.clamp(0, 4) as usize,
            clone_of: 0,
            tag: 0,
            clones: 0,
            state: IDLE,
            clip: Clip::new(IDLE, false),
            right: false,
            vx: 0,
            vy: 0,
            recharge: 0,
            cloning: None,
            invisible: false,
            fade: 0,
            still: 0,
            last_x: i32::MIN,
            wound: Wound::default(),
            last_frame: None,
        }
    }

    pub fn variant(&self) -> Variant {
        VARIANTS[self.variant.min(4)]
    }

    /// Sumiu de todo (invisível depois do esmaecer): não é desenhado.
    pub fn faded(&self) -> bool {
        self.fade >= FADE_TICKS
    }

    fn set(&mut self, state: u16) {
        self.state = state;
        self.clip = Clip::new(state, false);
    }
}

/// Marca os originais, conta os clones vivos de cada um e zera a vida dos clones cujo
/// original morreu (`0x41b02c`). Como `mine_generator::sync`, vale porque a cena só
/// acrescenta entidades no fim.
pub fn sync(entities: &mut [Entity]) {
    let n = entities.len();
    let mut live = vec![0usize; n];
    let mut dead_owner = vec![false; n];
    for (i, e) in entities.iter().enumerate() {
        if let Behaviour::Chameleon(c) = &e.behaviour {
            if c.clone_of == 0 {
                dead_owner[i] = !e.alive || e.health <= 0;
            }
        }
    }
    for e in entities.iter_mut() {
        if let Behaviour::Chameleon(c) = &e.behaviour {
            let owner = c.clone_of as usize;
            if owner == 0 || owner > n || !e.alive {
                continue;
            }
            if dead_owner[owner - 1] {
                if e.health > 0 {
                    e.health = 0;
                }
            } else if e.health > 0 {
                live[owner - 1] += 1;
            }
        }
    }
    for (i, e) in entities.iter_mut().enumerate() {
        if let Behaviour::Chameleon(c) = &mut e.behaviour {
            if c.clone_of == 0 {
                c.tag = i as u32 + 1;
                c.clones = live[i];
            }
        }
    }
}

/// Alvo (`0x41918b`/`0x419242`): o jogador vivo na camada, `|dx|` ≤ alcance, à vista.
fn target(e: &Entity, st: &ChameleonState, ctx: &Ctx) -> Option<Rect> {
    let me = e.bounds()?;
    let j = enemy::jake_target(ctx, e.layer)?;
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    ((jc.0 - c.0).abs() <= st.variant().range && enemy::line_of_sight(ctx, c, jc, &[me, j]))
        .then_some(j)
}

/// Caminho livre à frente (`0x41a22c`): passo livre e chão adiante.
fn path_free(me: &Rect, right: bool, ctx: &Ctx, layer: i32) -> bool {
    let cells = ctx.cells_for(layer);
    let ahead = me.moved(if right { SPEED + 2 } else { -SPEED - 2 }, 0);
    !cells.blocked(&ahead) && cells.standing_on(&ahead, 16)
}

/// Decisão (`0x41a79e`).
fn decide(e: &mut Entity, st: &mut ChameleonState, ctx: &mut Ctx, out: &mut Out) {
    let Some(me) = e.bounds() else {
        return;
    };
    let c = enemy::center(&me);
    let Some(j) = target(e, st, ctx) else {
        // Ruling: sem alvo, passeia
        match ctx.rng.below(4) {
            0 | 1 if path_free(&me, st.right, ctx, e.layer) => st.set(WALK),
            0..=2 => st.set(TURN),
            _ => st.set(IDLE),
        }
        return;
    };
    let jc = enemy::center(&j);
    let facing = (jc.0 >= c.0) == st.right;
    if ctx.rng.below(100) < 10 && e.health <= 30 {
        // fuga por porta (39/40) não portada
        st.set(IDLE);
    } else if (jc.1 - c.1).abs() <= BITE_DY {
        if !facing {
            st.set(TURN);
        } else {
            match ctx.rng.below(100) {
                0..85 => {
                    st.set(BITING);
                    if super::mine::distance(c, jc) <= BITE_RANGE {
                        out.jake_damage.add(&Pending {
                            amount: BITE,
                            ..Default::default()
                        });
                        out.events
                            .push(Event::Sound(Cue::at(SOUND_BITE, 8, jc, e.layer)));
                    }
                }
                85..95 => st.set(LOOK),
                _ => st.set(PAUSE),
            }
        }
    } else if !facing {
        st.set(if ctx.rng.below(100) < 30 { LOOK } else { TURN });
    } else if path_free(&me, st.right, ctx, e.layer) {
        st.set(WALK);
        if st.clone_of == 0
            && st.clones < MAX_CLONES
            && st.cloning.is_none()
            && ctx.rng.below(4) == 0
        {
            st.cloning = Some((50 + ctx.rng.below(50) as i32) * 3);
        }
    } else {
        st.set(TURN);
    }
}

/// Clone (`0x41aec0`): tipo 4 marcado, mesma variante, metade da vida.
fn spawn_clone(e: &Entity, st: &ChameleonState, out: &mut Out) {
    let mut c = e.clone();
    let mut cs = ChameleonState::new(st.variant as i32);
    cs.clone_of = st.tag;
    cs.right = st.right;
    c.health = st.variant().health / 2;
    c.id = 0;
    c.triggers = vec![];
    c.objective = 0;
    c.on = false;
    c.behaviour = Behaviour::Chameleon(Box::new(cs));
    out.spawn.push(c);
    out.events.push(Event::Sound(Cue::at(
        SOUND_SPAWNS,
        PRIORITY_FRAME,
        e.center(),
        e.layer,
    )));
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Some(me) = e.bounds() else {
        e.alive = false;
        return;
    };
    let c = enemy::center(&me);
    let mut st = match &e.behaviour {
        Behaviour::Chameleon(s) => s.as_ref().clone(),
        _ => return,
    };
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.health = 0;
        e.alive = false;
        e.behaviour = Behaviour::Chameleon(Box::new(st));
        return;
    }
    if st.state != DYING && ctx.rng.below(200) < 2 {
        let id = SOUNDS_AMBIENT[ctx.rng.below(3) as usize];
        out.events
            .push(Event::Sound(Cue::at(id, PRIORITY_FRAME, c, e.layer)));
    }
    if st.recharge > 0 {
        st.recharge -= 1;
    }
    // clonagem
    if let Some(n) = st.cloning {
        if st.state == DYING || st.clones >= MAX_CLONES {
            st.cloning = None;
        } else if n <= 0 {
            st.cloning = None;
            st.invisible = false;
            spawn_clone(e, &st, out);
            st.clones += 1;
        } else {
            st.cloning = Some(n - 1);
        }
    }
    // dano (`0x419f89`, `0x419fac`)
    let w = st.wound.take();
    if !w.is_empty() && st.state != DYING {
        if !w.pending.poison && !w.pending.explosion {
            if matches!(st.state, 8 | 9 | 30..=33) {
                st.set(10);
            } else if st.state != FLINCH && st.recharge == 0 {
                st.set(FLINCH);
                st.vx = 0;
                st.recharge = FLINCH_RECHARGE;
            }
        }
        e.health -= ctx.difficulty.damage(w.pending.amount).max(1);
    }
    if e.health <= 0 && st.state != DYING {
        st.set(DYING);
        st.vx = 0;
        st.invisible = false;
        st.cloning = None;
        e.on = true;
        out.events.push(Event::Sound(Cue::at(
            SOUND_DIES,
            PRIORITY_FRAME,
            c,
            e.layer,
        )));
    }
    let cells = ctx.cells_for(e.layer);
    let on_floor = cells.standing_on(&me, 2);
    if st.state != DYING {
        if st.vy >= FALLING && st.state != FALL {
            st.set(FALL);
            st.vx = 0;
        } else if st.state == FALL && on_floor {
            st.set(LAND);
        }
    }
    // preso: 13 tiques no mesmo x andando
    if st.state == WALK {
        if e.x == st.last_x {
            st.still += 1;
        } else {
            st.still = 0;
        }
        if st.still >= STUCK_TICKS {
            st.still = 0;
            st.set(STUCK);
            st.vy += 10;
        }
    }
    st.last_x = e.x;
    let (_, _, len) = enemy::label(&LABELS, st.clip.label);
    if st.clip.advance(len) {
        match st.state {
            DYING => {
                e.alive = false;
                e.behaviour = Behaviour::Chameleon(Box::new(st));
                return;
            }
            FALL => st.clip = Clip::new(FALL, false),
            TURN => {
                st.right = !st.right;
                if ctx.rng.below(100) < 20 {
                    if !st.invisible {
                        out.events.push(Event::Sound(Cue::at(
                            SOUND_INVISIBLE,
                            PRIORITY_FRAME,
                            c,
                            e.layer,
                        )));
                    }
                    st.invisible = true;
                } else if ctx.rng.below(100) < 25 {
                    st.invisible = false;
                }
                decide(e, &mut st, ctx, out);
            }
            _ => decide(e, &mut st, ctx, out),
        }
    }
    st.fade = if st.invisible {
        (st.fade + 1).min(FADE_TICKS)
    } else {
        (st.fade - 1).max(0)
    };
    st.vx = if st.state == WALK {
        if st.right { SPEED } else { -SPEED }
    } else {
        0
    } * SUB;
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
    e.x = body.x;
    e.y = body.y;
    st.vy = body.vy;
    let frame = enemy::show(e, &LABELS, &st.clip);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = st.right;
    }
    enemy::sound_frame(&SOUNDS, frame, &mut st.last_frame, c, e.layer, out);
    e.behaviour = Behaviour::Chameleon(Box::new(st));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::CHAMELEON;

    fn chameleon(x: i32, variant: i32) -> Entity {
        let mut e = logic(CHAMELEON, x, 2, &[variant]);
        e.flags = 0x84001;
        e.solid_shapes = true;
        e.boxes = vec![Rect {
            x: 5,
            y: 0,
            w: 25,
            h: 45,
        }];
        let st = ChameleonState::new(variant);
        e.health = st.variant().health;
        e.behaviour = Behaviour::Chameleon(Box::new(st));
        e
    }

    fn st(e: &Entity) -> &ChameleonState {
        match &e.behaviour {
            Behaviour::Chameleon(s) => s,
            _ => panic!(),
        }
    }

    #[test]
    fn bites_the_jake_for_30_up_close() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(140, 0);
        let mut c = ctx(&j, &w, &on);
        let mut k = chameleon(100, 0);
        let Behaviour::Chameleon(s) = &mut k.behaviour else {
            panic!()
        };
        s.right = true;
        let mut out = Out::default();
        for _ in 0..200 {
            tick(&mut k, &mut c, &mut out);
            if out.jake_damage.amount > 0 {
                break;
            }
        }
        assert_eq!(out.jake_damage.amount, BITE);
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND_BITE)));
    }

    #[test]
    fn flinches_when_hit_and_dies_with_easy_damage() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        c.difficulty = crate::Difficulty::Easy;
        let mut k = chameleon(100, 1);
        let p = Pending {
            amount: 2,
            ..Default::default()
        };
        enemy::hurt(&mut k, &p, true);
        tick(&mut k, &mut c, &mut Out::default());
        assert_eq!(st(&k).state, FLINCH);
        assert_eq!(k.health, 46, "Beta 50, fácil ×2");
        k.health = 1;
        enemy::hurt(&mut k, &p, true);
        let mut out = Out::default();
        tick(&mut k, &mut c, &mut out);
        assert_eq!(st(&k).state, DYING);
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND_DIES)));
        for _ in 0..60 {
            if k.alive {
                tick(&mut k, &mut c, &mut Out::default());
            }
        }
        assert!(!k.alive && enemy::killed(&k));
    }

    #[test]
    fn clones_have_half_life_and_die_with_the_original() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut v = vec![chameleon(100, 0)];
        sync(&mut v);
        let Behaviour::Chameleon(s) = &mut v[0].behaviour else {
            panic!()
        };
        s.cloning = Some(0);
        let mut out = Out::default();
        tick(&mut v[0], &mut c, &mut out);
        assert_eq!(out.spawn.len(), 1);
        assert!(
            out.events
                .iter()
                .any(|e| e.sound_id() == Some(SOUND_SPAWNS))
        );
        v.extend(out.spawn);
        assert_eq!(v[1].health, 40, "metade da vida do Alpha");
        assert!(!enemy::counted(&v[1]), "o clone não entra no placar");
        sync(&mut v);
        assert_eq!(st(&v[0]).clones, 1);
        v[0].health = 0;
        sync(&mut v);
        assert_eq!(v[1].health, 0, "morre com o original");
        assert!(!enemy::killed(&v[1]));
    }

    #[test]
    fn invisible_chameleon_stays_present() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(3000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut k = chameleon(100, 0);
        let Behaviour::Chameleon(s) = &mut k.behaviour else {
            panic!()
        };
        s.invisible = true;
        s.fade = FADE_TICKS - 1;
        tick(&mut k, &mut c, &mut Out::default());
        assert!(k.alive && !k.visible() && k.health > 0);
    }
}
