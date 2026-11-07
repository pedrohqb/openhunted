// SPDX-License-Identifier: GPL-3.0-or-later
//! Bruiser (tipo 7, `obr.anm`, `0x00416af4`/`0x00416c68`; `docs/re/bruiser.md`): robô bípede
//! que acorda pelos gatilhos, anda pela tabela de passos, atira balas 100 e foguetes 21,
//! põe fogo no chão, faz o chão tremer e esmaga quem está embaixo ao virar-se.

use super::enemy::{self, Clip, Label, Spawn, Wound};
use super::{Blast, Ctx, Event, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity, FIRE};
use crate::physics::{Body, Rect, step};
use crate::sound::{Cue, PRIORITY_FRAME};

/// Rótulos do `OBR.ANM` (id, início, quadros).
pub const LABELS: [Label; 12] = [
    (1, 0, 12),
    (2, 12, 20),
    (3, 32, 14),
    (4, 46, 10),
    (5, 56, 22),
    (6, 78, 14),
    (7, 92, 22),
    (10, 114, 10),
    (11, 124, 6),
    (12, 130, 13),
    (13, 143, 5),
    (15, 148, 1),
];
/// `SOUND.TBL` lista 7 (quadro, som, prioridade): passos, fogo, virada, tranco, morte.
pub const SOUNDS: [(usize, u16, u16); 13] = [
    (1, 3107, 5),
    (2, 3111, 5),
    (12, 3107, 5),
    (22, 3107, 0),
    (40, 3107, 5),
    (58, 3105, 8),
    (83, 3106, 5),
    (95, 3106, 0),
    (96, 3111, 5),
    (115, 3143, 9),
    (124, 3109, 5),
    (130, 3140, 5),
    (144, 3140, 5),
];
/// Velocidade dos passos por quadro absoluto dos rótulos 1–3 (1/8 px, `0x472498`); o fim do
/// rótulo 3 não foi lido (spec M7a: 0).
pub const STEPS: [i32; 46] = [
    9, 20, 22, 22, 22, 22, 22, 22, 20, 22, 0, 18, // rótulo 1
    27, 27, 27, 27, 16, 16, 18, 18, 16, 13, 13, 13, 11, 11, 25, 29, 27, 27, 27, 11, // 2
    22, 20, 18, 67, 43, 40, 0, 27, 34, 0, 0, 0, 0, 0, // 3
];
/// Vida padrão (`0x416af4`).
pub const HEALTH: i32 = 100;
/// Percepção (`0x417c0d`): `|dy|` ≤ 50, `|dx|` ≤ 350, dos dois lados.
pub const SIGHT_DY: i32 = 50;
pub const SIGHT_DX: i32 = 350;
/// Fogo no chão até 65 px; 50 px à frente; espera 70 tiques; altura 25–49.
pub const FIRE_REACH: i32 = 65;
pub const FIRE_AHEAD: i32 = 50;
pub const FIRE_WAIT: i32 = 70;
/// Tiros: 20 px/tique; som 3143.
pub const SHOT_SPEED: i32 = 20;
pub const SOUND_SHOT: u16 = 3143;
/// Bocas olhando à direita / à esquerda (`0x417e47`).
pub const MUZZLE_A: [(i32, i32); 2] = [(55, 12), (0, 12)];
pub const MUZZLE_B: [(i32, i32); 2] = [(72, 15), (-12, 15)];
pub const MUZZLE_C: [(i32, i32); 2] = [(68, 5), (-10, 5)];
/// Terremoto (`0x418229`): som 3131 e cambaleio de 20–39 tiques a até 300 px.
pub const SOUND_QUAKE: u16 = 3131;
pub const QUAKE_RANGE: i32 = 300;
/// O 3131 não recomeça enquanto toca (`0x44e130` antes de `0x44dc20`): sem saber do mixer, a
/// cena o filtra pela duração do `SX_3131.WAV` (23 448 amostras a 11 025 Hz ≈ 2,13 s).
pub const SOUND_QUAKE_TICKS: u32 = 43;
/// Esmagar (linha `(7, 0, 0)`): derrubada, 15 de dano, empurrão fixo (48, −3).
pub const SMASH: i32 = 15;
pub const SMASH_PUSH: (i32, i32) = (48, -3);
/// Morte: 30 explosões `(7, 0, 2)` a ±30 px do canto (raio 48, empurrão 15, área 10).
pub const DEATH_BLASTS: usize = 30;
pub const DEATH_SPREAD: i32 = 30;
pub const BLAST_RADIUS: i32 = 48;
pub const BLAST_PUSH: i32 = 15;
pub const BLAST_DAMAGE: i32 = 10;
/// Frases (`0x416ed6`): "Halt", "Put down your weapon", "You can not win".
pub const PHRASES: [u16; 3] = [3127, 3125, 3126];
/// Silêncio depois de uma frase (spec M7a).
pub const TALK_GAP: u32 = 60;
/// Passo de 17 px à frente e sonda de queda de 15 px (`0x417836`, `0x41817d`).
const STEP_AHEAD: i32 = 17;
const FLOOR_PROBE: i32 = 15;

pub const WALK: i32 = 1;
pub const STRIDE: i32 = 2;
pub const HALT: i32 = 3;
pub const SHOOT: i32 = 4;
pub const GROUND_FIRE: i32 = 5;
pub const TURN: i32 = 6;
pub const QUAKE: i32 = 7;
pub const STAGGER: i32 = 10;
pub const DIE: i32 = 11;
pub const LOOK2: i32 = 12;
pub const LOOK: i32 = 13;
pub const SLEEP: i32 = 15;
pub const TURN_END: i32 = -1;
pub const LOOK_BACK: i32 = -2;
pub const WAKE: i32 = -3;

#[derive(Debug, Clone)]
pub struct BruiserState {
    pub state: i32,
    pub clip: Clip,
    /// `+0x27c`: olhando à direita (o sprite sem espelho olha à direita).
    pub facing_right: bool,
    /// Ativado pelos gatilhos (`+0x27a`).
    pub awake: bool,
    /// Destroço (`+0x290`).
    pub dead: bool,
    /// Espera do fogo (`+0x299`).
    pub fire_wait: i32,
    /// x e tiques sem andar no estado 2.
    pub stuck: (i32, u32),
    /// Capacidades `values[0..3]`: fogo, foguetes, balas, terremoto.
    pub abilities: [bool; 4],
    pub talk: u32,
    pub vy: i32,
    pub wound: Wound,
    pub last_frame: Option<usize>,
}

impl BruiserState {
    pub fn new(values: &[i32]) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        let all = v(15) != 1;
        Self {
            state: SLEEP,
            clip: Clip::new(15, false),
            facing_right: true,
            awake: false,
            dead: false,
            fire_wait: 0,
            stuck: (i32::MIN, 0),
            abilities: [0, 1, 2, 3].map(|i| all || v(i) != 0),
            talk: 0,
            vy: 0,
            wound: Wound::default(),
            last_frame: None,
        }
    }

    pub fn set(&mut self, state: i32) {
        self.state = state;
        self.clip = match state {
            WAKE => Clip::new(11, true),
            LOOK_BACK => Clip::new(13, true),
            TURN_END => Clip::new(6, true),
            s => Clip::new(s as u16, false),
        };
        if matches!(state, TURN | WALK) {
            self.fire_wait = 0;
        }
    }

    fn dir(&self) -> i32 {
        if self.facing_right { 1 } else { -1 }
    }
}

/// O fogo no chão: tipo 42 já iniciado, altura `height`, que míngua 1 a cada 2 tiques.
pub fn fire(base: (i32, i32), height: i32, layer: i32, frame_h: i32) -> Entity {
    let values = vec![0, 21, 0, 0, height, 0, 0, 0, 0, 0, base.0, base.1, 0, 1000];
    let st = super::fire::FireState::new(&values, base, (0, 0), frame_h);
    let r = st.rect();
    Entity {
        kind: FIRE,
        x: r.x,
        y: r.y,
        flags: 0x8002d,
        layer,
        id: 0,
        values,
        triggers: vec![],
        objective: 0,
        health: 0,
        armor: 0,
        boxes: vec![Rect {
            x: 0,
            y: 0,
            w: r.w,
            h: r.h,
        }],
        solid_shapes: false,
        on: true,
        alive: true,
        sprite: None,
        behaviour: Behaviour::Fire(st),
    }
}

/// Alvo (`0x417c0d`): o Jake na camada, `|dy|` ≤ 50, `|dx|` ≤ 350, à vista.
fn target(e: &Entity, ctx: &Ctx) -> Option<Rect> {
    let me = e.bounds()?;
    let j = enemy::jake_target(ctx, e.layer)?;
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    ((jc.1 - c.1).abs() <= SIGHT_DY
        && (jc.0 - c.0).abs() <= SIGHT_DX
        && enemy::line_of_sight(ctx, c, jc, &[me, j]))
    .then_some(j)
}

/// "Pode andar" (`0x417836`): 17 px à frente sem parede e com chão a até 15 px.
fn can_walk(e: &Entity, st: &BruiserState, ctx: &Ctx) -> bool {
    let Some(me) = e.bounds() else {
        return false;
    };
    let ahead = Rect {
        x: me.x + STEP_AHEAD * st.dir(),
        ..me
    };
    ctx.cells_for(e.layer).floor_at(&ahead, FLOOR_PROBE)
}

/// Tiro (`0x417e47`): 35 % balas (uma em B e, com 48 %, outra em A), senão um foguete em C.
fn shoot(e: &Entity, st: &BruiserState, ctx: &mut Ctx, out: &mut Out) -> bool {
    let side = usize::from(!st.facing_right);
    let vel = (SHOT_SPEED * st.dir(), (ctx.rng.next_u16() & 3) as i32);
    let at = |m: [(i32, i32); 2]| (e.x + m[side].0, e.y + m[side].1);
    let mut shots = vec![];
    if ctx.rng.below(100) < 35 {
        if st.abilities[2] {
            shots.push((super::enemy_shot::BRUISER_BULLET, at(MUZZLE_B)));
            if ctx.rng.below(100) < 48 {
                shots.push((super::enemy_shot::BRUISER_BULLET, at(MUZZLE_A)));
            }
        }
    } else if st.abilities[1] {
        shots.push((super::enemy_shot::ROCKET, at(MUZZLE_C)));
    }
    if shots.is_empty() {
        return false;
    }
    out.events.push(Event::Sound(Cue::at(
        SOUND_SHOT,
        PRIORITY_FRAME,
        e.center(),
        e.layer,
    )));
    for (kind, at) in shots {
        out.enemy_spawns.push(Spawn::Shot {
            kind,
            at,
            vel,
            layer: e.layer,
        });
    }
    true
}

/// Escolha do ataque (`0x0041750b`).
fn attack(e: &Entity, st: &mut BruiserState, j: Rect, ctx: &mut Ctx, out: &mut Out) {
    let (c, jc) = (e.center(), enemy::center(&j));
    let behind = if st.facing_right {
        jc.0 <= c.0
    } else {
        jc.0 > c.0
    };
    if behind {
        st.set(TURN);
    } else if (jc.0 - c.0).abs() <= FIRE_REACH && st.abilities[0] {
        st.set(GROUND_FIRE);
    } else if can_walk(e, st, ctx) && ctx.rng.below(100) < 30 {
        st.set(WALK);
    } else if ctx.rng.below(100) < 75 {
        if ctx.rng.below(100) < 75 && (st.abilities[1] || st.abilities[2]) {
            let fired = shoot(e, st, ctx, out);
            st.set(if fired { SHOOT } else { LOOK });
        } else {
            st.set(if st.abilities[3] { QUAKE } else { LOOK });
        }
    } else {
        st.set(LOOK);
    }
}

/// Terremoto (`0x418229`): só com alvo (`+0x280`); som 3131 sem posição e todo Jake vivo a
/// até 300 px cambaleia.
fn quake(e: &Entity, j: Option<Rect>, ctx: &mut Ctx, out: &mut Out) {
    if j.is_none() {
        return;
    }
    out.events
        .push(Event::Sound(Cue::ui(SOUND_QUAKE, PRIORITY_FRAME)));
    let near = ctx
        .jake_bounds()
        .is_some_and(|j| super::mine::distance(e.center(), enemy::center(&j)) < QUAKE_RANGE);
    if near {
        let t = 20 + ctx.rng.below(20);
        out.quake = Some(out.quake.unwrap_or(0).max(t));
    }
}

/// Fim de cada animação (`0x00417061`).
fn end_of_animation(
    e: &mut Entity,
    st: &mut BruiserState,
    j: Option<Rect>,
    ctx: &mut Ctx,
    out: &mut Out,
) {
    let like_shoot = |e: &Entity, st: &mut BruiserState, ctx: &mut Ctx, out: &mut Out| match j {
        Some(j) => attack(e, st, j, ctx, out),
        None => st.set(TURN),
    };
    match st.state {
        WAKE => st.set(LOOK),
        LOOK => st.set(LOOK2),
        LOOK2 => match j {
            Some(j) => attack(e, st, j, ctx, out),
            None if ctx.rng.below(100) < 80 => st.set(LOOK2),
            None => st.set(LOOK_BACK),
        },
        LOOK_BACK => match j {
            Some(j) => attack(e, st, j, ctx, out),
            None if ctx.rng.below(100) < 70 => st.set(LOOK),
            None if can_walk(e, st, ctx) => st.set(WALK),
            None => st.set(TURN),
        },
        WALK | STRIDE => {
            let next = if can_walk(e, st, ctx) { STRIDE } else { HALT };
            st.set(next);
        }
        HALT => match j {
            Some(j) => attack(e, st, j, ctx, out),
            None => st.set(if ctx.rng.below(2) == 0 { TURN } else { LOOK }),
        },
        GROUND_FIRE => {
            if st.fire_wait <= 0 {
                if let Some(me) = e.bounds() {
                    let base = (e.center().0 + FIRE_AHEAD * st.dir(), me.y + me.h);
                    out.enemy_spawns.push(Spawn::Fire {
                        base,
                        height: 25 + ctx.rng.below(25) as i32,
                        layer: e.layer,
                    });
                }
                st.fire_wait = FIRE_WAIT;
            }
            like_shoot(e, st, ctx, out);
        }
        SHOOT | QUAKE | STAGGER => like_shoot(e, st, ctx, out),
        TURN => {
            // esmagar (`0x417308`): o alvo sob a largura do Bruiser
            if let (Some(t), Some(me)) = (j, e.bounds()) {
                let tx = enemy::center(&t).0;
                if tx >= me.x && tx < me.x + me.w {
                    let side = if tx >= e.center().0 { 1 } else { -1 };
                    out.jake_damage.add(&Pending {
                        amount: SMASH,
                        knockdown: true,
                        push: (SMASH_PUSH.0 * side, SMASH_PUSH.1),
                        ..Default::default()
                    });
                }
            }
            st.facing_right = match j {
                Some(t) => enemy::center(&t).0 > e.center().0,
                None => !st.facing_right,
            };
            st.set(TURN_END);
        }
        TURN_END => match j {
            Some(j) => attack(e, st, j, ctx, out),
            None if can_walk(e, st, ctx) => st.set(WALK),
            None => st.set(if ctx.rng.below(2) == 0 { TURN } else { LOOK }),
        },
        DIE => {
            st.dead = true;
            e.on = true;
            e.solid_shapes = false;
            e.flags = (e.flags & !0x8) | 0x60;
            let corner = e.bounds().map_or((e.x, e.y), |b| (b.x, b.y));
            for _ in 0..DEATH_BLASTS {
                out.blasts.push(Blast {
                    center: enemy::scatter(ctx, corner, DEATH_SPREAD),
                    layer: e.layer,
                    radius: BLAST_RADIUS,
                    push: BLAST_PUSH,
                    damage: BLAST_DAMAGE,
                });
            }
        }
        _ => {}
    }
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let mut st = match &e.behaviour {
        Behaviour::Bruiser(s) if !s.dead => s.clone(),
        _ => return,
    };
    // 2. ativação pelos gatilhos
    if !st.awake {
        if crate::triggers::satisfied(&e.triggers, ctx.on) {
            st.awake = true;
            st.set(WAKE);
        } else {
            display(e, &mut st, out);
            e.behaviour = Behaviour::Bruiser(st);
            return;
        }
    }
    st.fire_wait -= 1;
    // 3. dano (`0x417ae0`): imune a fogo; dificuldade, depois metade
    let w = st.wound.take();
    if !w.is_empty() && !w.pending.fire && st.state != DIE {
        e.health -= ctx.difficulty.damage(w.pending.amount) * 50 / 100;
        if e.health <= 0 {
            st.set(DIE);
        } else if !matches!(st.state, STAGGER | TURN | GROUND_FIRE) {
            st.set(STAGGER);
        }
    }
    let j = if st.state == DIE {
        None
    } else {
        target(e, ctx)
    };
    // 5. fala
    st.talk = st.talk.saturating_sub(1);
    if j.is_some() && st.talk == 0 && ctx.rng.below(1000) <= 10 {
        let id = PHRASES[ctx.rng.below(3) as usize];
        out.events.push(Event::Sound(Cue::ui(id, PRIORITY_FRAME)));
        st.talk = TALK_GAP;
    }
    // 6. parado no estado 2 sem alvo
    if st.state == STRIDE {
        if st.stuck.0 == e.x {
            st.stuck.1 += 1;
        } else {
            st.stuck = (e.x, 0);
        }
        if st.stuck.1 >= 6 && j.is_none() {
            st.stuck = (i32::MIN, 0);
            st.set(if ctx.rng.below(2) == 0 { TURN } else { LOOK });
        }
    }
    // 7. animação, ataques nos quadros e fim
    let (_, _, len) = enemy::label(&LABELS, st.clip.label);
    // `0x417761` (QUAKE) e `0x417718` (TURN) chamam `0x418229` sem olhar a capacidade 3,
    // que só decide se o ataque escolhe o estado QUAKE
    if st.state == QUAKE && st.clip.step + 5 == len {
        quake(e, j, ctx, out);
    }
    if st.state == TURN && st.clip.step + 1 == len {
        quake(e, j, ctx, out);
    }
    if st.clip.advance(len) {
        end_of_animation(e, &mut st, j, ctx, out);
    }
    // passos (`0x417a68`): vx pela tabela, vy = −16
    let (_, start2, len2) = enemy::label(&LABELS, st.clip.label);
    let mut body = Body::new(
        e.x,
        e.y,
        e.boxes.first().copied().unwrap_or(Rect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        }),
    );
    body.vy = st.vy;
    if matches!(st.state, WALK | STRIDE | HALT) {
        let f = start2 + st.clip.offset(len2);
        body.vx = st.dir() * STEPS.get(f).copied().unwrap_or(0);
        body.vy = -16;
    }
    if !st.dead {
        step(&mut body, ctx.cells_for(e.layer), true);
        e.x = body.x;
        e.y = body.y;
        st.vy = body.vy;
    }
    display(e, &mut st, out);
    e.behaviour = Behaviour::Bruiser(st);
}

fn display(e: &mut Entity, st: &mut BruiserState, out: &mut Out) {
    let frame = enemy::show(e, &LABELS, &st.clip);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = !st.facing_right;
    }
    let c = e.center();
    enemy::sound_frame(&SOUNDS, frame, &mut st.last_frame, c, e.layer, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::BRUISER;

    /// Bruiser de pé no chão do `ctx_with_jake(_, y)` (chão em y + 47).
    fn bruiser(x: i32, floor: i32, triggers: Vec<(i32, Option<i32>)>) -> Entity {
        let mut e = logic(BRUISER, x, floor - 66, &[]);
        e.flags = 0x84801;
        e.health = HEALTH;
        e.solid_shapes = true;
        e.triggers = triggers;
        e.boxes = vec![Rect {
            x: 10,
            y: 10,
            w: 40,
            h: 56,
        }];
        e.behaviour = Behaviour::Bruiser(Box::new(BruiserState::new(&[])));
        e
    }

    fn st(e: &Entity) -> &BruiserState {
        match &e.behaviour {
            Behaviour::Bruiser(s) => s,
            _ => panic!(),
        }
    }

    #[test]
    fn bruiser_sleeps_until_triggers() {
        let off = |id: i32| (id == 3).then_some(false);
        let on = |id: i32| (id == 3).then_some(true);
        let (j, w) = ctx_with_jake(300, 0);
        let mut b = bruiser(100, 47, vec![(3, Some(11))]);
        let mut c = ctx(&j, &w, &off);
        for _ in 0..20 {
            tick(&mut b, &mut c, &mut Out::default());
        }
        assert_eq!(st(&b).state, SLEEP);
        let mut c = ctx(&j, &w, &on);
        tick(&mut b, &mut c, &mut Out::default());
        assert!(st(&b).awake);
        assert_eq!(st(&b).state, WAKE);
    }

    #[test]
    fn bruiser_takes_half_and_ignores_fire() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(5000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut b = bruiser(100, 47, vec![]);
        let mut out = Out::default();
        tick(&mut b, &mut c, &mut out); // acorda
        let bullet = Pending {
            amount: 2,
            ..Default::default()
        };
        enemy::hurt(&mut b, &bullet, true);
        tick(&mut b, &mut c, &mut out);
        assert_eq!(b.health, 99, "pistola: 1 por bala");
        assert_eq!(st(&b).state, STAGGER);
        let burn = Pending {
            amount: 1,
            fire: true,
            ..Default::default()
        };
        enemy::hurt(&mut b, &burn, false);
        tick(&mut b, &mut c, &mut out);
        assert_eq!(b.health, 99);
        b.health = 1;
        enemy::hurt(&mut b, &bullet, true);
        tick(&mut b, &mut c, &mut out);
        assert_eq!(st(&b).state, DIE);
        assert!(enemy::killed(&b), "a morte conta");
        for _ in 0..10 {
            tick(&mut b, &mut c, &mut out);
        }
        assert!(st(&b).dead && b.alive && !b.solid_shapes);
        assert_eq!(out.blasts.len(), DEATH_BLASTS);
    }

    #[test]
    fn bruiser_attacks_and_quakes() {
        let on = |_: i32| None;
        // Jake a ~200 px à direita, no mesmo chão, numa sala de x 0 a 600 (no estado 2 ele
        // só para numa parede ou beirada)
        let (j, w) = ctx_with_jake(330, 0);
        let w = crate::World::new(
            w.solids()
                .iter()
                .copied()
                .chain([
                    Rect {
                        x: -20,
                        y: -200,
                        w: 20,
                        h: 400,
                    },
                    Rect {
                        x: 600,
                        y: -200,
                        w: 20,
                        h: 400,
                    },
                ])
                .collect(),
        );
        let mut c = ctx(&j, &w, &on);
        let mut b = bruiser(100, 47, vec![]);
        let mut out = Out::default();
        let mut quaked = false;
        for t in 0..1500 {
            c.tick = t;
            tick(&mut b, &mut c, &mut out);
            quaked |= out.quake.is_some();
        }
        let shots = out
            .enemy_spawns
            .iter()
            .filter(|s| matches!(s, Spawn::Shot { .. }))
            .count();
        assert!(shots > 0, "atira");
        assert!(quaked, "faz tremer");
        assert!(out.events.iter().any(|e| e.sound_id() == Some(SOUND_SHOT)));
        let Some(Spawn::Shot { vel, .. }) = out
            .enemy_spawns
            .iter()
            .find(|s| matches!(s, Spawn::Shot { .. }))
        else {
            panic!()
        };
        assert_eq!(vel.0, SHOT_SPEED, "para a direita, onde está o Jake");
    }

    #[test]
    fn bruiser_burns_and_smashes_up_close() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(140, 0);
        let mut c = ctx(&j, &w, &on);
        let mut b = bruiser(100, 47, vec![]);
        let mut out = Out::default();
        for t in 0..300 {
            c.tick = t;
            tick(&mut b, &mut c, &mut out);
        }
        assert!(
            out.enemy_spawns
                .iter()
                .any(|s| matches!(s, Spawn::Fire { .. })),
            "fogo no chão a menos de 65 px"
        );
        // atrás dele, embaixo: vira e esmaga
        let (j2, w2) = ctx_with_jake(115, 0);
        let mut c2 = ctx(&j2, &w2, &on);
        let mut b2 = bruiser(100, 47, vec![]);
        let Behaviour::Bruiser(s) = &mut b2.behaviour else {
            panic!()
        };
        s.awake = true;
        s.facing_right = true;
        s.set(TURN);
        let mut out2 = Out::default();
        for t in 0..14 {
            c2.tick = t;
            tick(&mut b2, &mut c2, &mut out2);
        }
        assert_eq!(out2.jake_damage.amount, SMASH);
        assert!(out2.jake_damage.knockdown);
        assert!(!st(&b2).facing_right, "encara o Jake, à esquerda do centro");
    }

    /// Bruiser acordado no estado `state`, a um tique do quadro do terremoto.
    fn about_to_quake(state: i32, abilities: [bool; 4]) -> Entity {
        let mut b = bruiser(100, 47, vec![]);
        let Behaviour::Bruiser(s) = &mut b.behaviour else {
            panic!()
        };
        s.awake = true;
        s.abilities = abilities;
        s.set(state);
        let (_, _, len) = enemy::label(&LABELS, s.clip.label);
        s.clip.step = if state == QUAKE { len - 5 } else { len - 1 };
        b
    }

    fn quake_cues(out: &Out) -> Vec<Cue> {
        out.events
            .iter()
            .filter_map(|e| match e {
                Event::Sound(c) if c.id == SOUND_QUAKE => Some(*c),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn quake_needs_a_target_not_the_ability() {
        let on = |_: i32| None;
        // sem alvo (Jake longe): nada, nem som (`0x418229`: `+0x280 = 0` → sai)
        let (far, w) = ctx_with_jake(5000, 0);
        for state in [QUAKE, TURN] {
            let mut c = ctx(&far, &w, &on);
            let mut b = about_to_quake(state, [true; 4]);
            let mut out = Out::default();
            tick(&mut b, &mut c, &mut out);
            assert!(
                out.quake.is_none() && quake_cues(&out).is_empty(),
                "{state}"
            );
        }
        // com alvo e sem a capacidade 3: o quadro do estado faz tremer mesmo assim
        // (`0x417718`/`0x417761` não olham a capacidade); som sem posição
        let (near, w) = ctx_with_jake(250, 0);
        for state in [QUAKE, TURN] {
            let mut c = ctx(&near, &w, &on);
            let mut b = about_to_quake(state, [true, true, true, false]);
            let mut out = Out::default();
            tick(&mut b, &mut c, &mut out);
            assert!(out.quake.is_some(), "{state}");
            assert_eq!(
                quake_cues(&out),
                vec![Cue::ui(SOUND_QUAKE, PRIORITY_FRAME)],
                "{state}"
            );
        }
        assert_eq!(enemy::throttle(SOUND_QUAKE), Some(SOUND_QUAKE_TICKS));
    }
}
