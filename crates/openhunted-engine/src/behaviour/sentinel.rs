// SPDX-License-Identifier: GPL-3.0-or-later
//! Robô sentinela (tipo 10, "Sentinel Robot", `0x0041b51c`/`0x0041b640`;
//! `docs/re/robo-sentinela.md`): casulo fixo que se abre diante do Jake e dispara rajadas de
//! três balas 106; fechado é invulnerável.

use super::enemy::{self, Clip, Label, Spawn, Wound};
use super::{Blast, Ctx, Out};
use crate::entity::{Behaviour, Entity};

/// Rótulos do `ose.anm`: 1 `ose00` (fechado), 2 `ose01` (abrindo), 3 `ose02` (atirando),
/// 4 `ose03` (outra espera), 5 `ose04` (destruído).
pub const LABELS: [Label; 5] = [(1, 0, 4), (2, 4, 8), (3, 12, 7), (4, 19, 9), (5, 28, 1)];
/// `SOUND.TBL` tipo 10: (quadro, som, prioridade).
pub const SOUNDS: [(usize, u16, u16); 5] = [
    (1, SOUND_IDLE, 3),
    (5, 3062, 0),
    (9, 3060, 3),
    (14, 3063, 0),
    (18, 3063, 0),
];
/// Zumbido de espera: no máximo um a cada 200 ms entre todos os casulos (`0x488820`).
pub const SOUND_IDLE: u16 = 3061;
pub const SOUND_IDLE_TICKS: u32 = 4;
/// Vida padrão (`0x0041b51c`).
pub const HEALTH: i32 = 4;
/// Percepção (`0x41bacb`): `|dy|` ≤ 50, `|dx|` ≤ 150.
pub const SIGHT_DY: i32 = 50;
pub const SIGHT_DX: i32 = 150;
/// Balas por disparo (`values[0]`, posto a 3 quando `values[10] ≠ 1`).
pub const SHOTS: i32 = 3;
/// Velocidade das balas (px/tique).
pub const SHOT_SPEED: i32 = 20;
/// Chances: em alerta, abrir de novo com `rand % 100 ≥ 70`; nova rajada com 45 %.
pub const REOPEN: u32 = 70;
pub const REFIRE: u32 = 45;
/// Linha `(10, 0)`: raio 28, empurrão 15, dano de área 10.
pub const BLAST_RADIUS: i32 = 28;
pub const BLAST_PUSH: i32 = 15;
pub const BLAST_DAMAGE: i32 = 10;

pub const CLOSED: i8 = 1;
pub const OPENING: i8 = 2;
pub const FIRING: i8 = 3;
pub const IDLE2: i8 = 4;
pub const CLOSING: i8 = -1;
pub const DESTROYED: i8 = 5;

#[derive(Debug, Clone)]
pub struct SentinelState {
    /// Estado `+0x180`.
    pub state: i8,
    pub clip: Clip,
    /// Alerta `+0x28c`.
    pub alert: bool,
    /// Lado `+0x194` (1 = olhando à direita).
    pub facing_right: bool,
    /// Destroço (`+0x270`).
    pub dead: bool,
    pub wound: Wound,
    pub last_frame: Option<usize>,
    /// `values[0]`: balas por disparo.
    pub shots: i32,
}

impl SentinelState {
    pub fn new(values: &[i32]) -> Self {
        let v = |i: usize| values.get(i).copied().unwrap_or(0);
        Self {
            state: CLOSED,
            clip: Clip::new(1, false),
            alert: false,
            facing_right: false,
            dead: false,
            wound: Wound::default(),
            last_frame: None,
            shots: if v(10) != 1 || v(0) <= 0 { SHOTS } else { v(0) },
        }
    }

    fn set(&mut self, state: i8) {
        self.state = state;
        self.clip = match state {
            CLOSING => Clip::new(2, true),
            s => Clip::new(s as u16, false),
        };
    }
}

/// O Jake como alvo válido (`0x41bacb`): mesma camada, `|dy|` ≤ 50, `|dx|` ≤ 150, do lado
/// para onde olha (abrindo ou aberta) e à vista. Sem teste de esconderijo.
fn target(e: &Entity, st: &SentinelState, ctx: &Ctx) -> Option<(i32, i32)> {
    let me = e.bounds()?;
    let j = enemy::jake_target(ctx, e.layer)?;
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    let (dx, dy) = (jc.0 - c.0, jc.1 - c.1);
    if dy.abs() > SIGHT_DY || dx.abs() > SIGHT_DX {
        return None;
    }
    if matches!(st.state, OPENING | FIRING) && (dx > 0) != st.facing_right && dx != 0 {
        return None;
    }
    enemy::line_of_sight(ctx, c, jc, &[me, j]).then_some(jc)
}

/// Disparo (`0x0041bcdb`): balas 106 para o lado, cada uma 1–12 px acima da anterior.
fn fire(e: &Entity, st: &SentinelState, ctx: &mut Ctx, out: &mut Out) {
    let side = if st.facing_right { 1 } else { -1 };
    let mut y = e.y;
    for i in 0..st.shots {
        let x = e.x + ctx.rng.below(5) as i32 + 10 * side;
        if i > 0 {
            y += ctx.rng.below(12) as i32 - 12;
        }
        let vy = ctx.rng.below(6) as i32;
        out.enemy_spawns.push(Spawn::Shot {
            kind: super::enemy_shot::SENTINEL_BULLET,
            at: (x, y),
            vel: (SHOT_SPEED * side, vy),
            layer: e.layer,
        });
    }
}

/// Fim de cada animação (`0x0041b77c`).
fn end_of_animation(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Behaviour::Sentinel(st) = &e.behaviour else {
        return;
    };
    let seen = target(e, st, ctx);
    let mut st = st.clone();
    if seen.is_none() {
        st.alert = false;
    }
    let face = |st: &mut SentinelState, jc: (i32, i32)| {
        st.facing_right = jc.0 > e.center().0;
    };
    match st.state {
        CLOSED | IDLE2 | CLOSING => {
            let open = match seen {
                Some(_) if !st.alert => true,
                Some(_) => ctx.rng.below(100) >= REOPEN,
                None => false,
            };
            if let (true, Some(jc)) = (open, seen) {
                st.alert = true;
                face(&mut st, jc);
                st.set(OPENING);
            } else {
                st.set(if ctx.rng.below(2) == 0 { CLOSED } else { IDLE2 });
            }
        }
        OPENING => {
            fire(e, &st, ctx, out);
            st.set(FIRING);
        }
        FIRING => {
            if seen.is_some() && ctx.rng.below(100) < REFIRE {
                fire(e, &st, ctx, out);
                st.set(FIRING);
            } else {
                st.set(CLOSING);
            }
        }
        _ => {
            st.dead = true;
            e.solid_shapes = false;
            e.flags |= 0x48;
        }
    }
    e.behaviour = Behaviour::Sentinel(st);
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let center = e.center();
    let Behaviour::Sentinel(st) = &mut e.behaviour else {
        return;
    };
    if st.dead {
        return;
    }
    // dano (`0x41b9c9`): fechada (1 e 4) não sente nada
    let w = st.wound.take();
    if !w.is_empty() {
        if st.state != CLOSED && st.state != IDLE2 {
            e.health -= w.pending.amount;
        }
        if e.health <= 0 && st.state != DESTROYED {
            st.set(DESTROYED);
            e.on = true;
            out.blasts.push(Blast {
                center,
                layer: e.layer,
                radius: BLAST_RADIUS,
                push: BLAST_PUSH,
                damage: BLAST_DAMAGE,
            });
        }
    }
    let (_, _, len) = enemy::label(&LABELS, st.clip.label);
    if st.clip.advance(len) {
        end_of_animation(e, ctx, out);
    }
    let Behaviour::Sentinel(st) = &e.behaviour else {
        return;
    };
    let (clip, right) = (st.clip, st.facing_right);
    let frame = enemy::show(e, &LABELS, &clip);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = right;
    }
    let Behaviour::Sentinel(st) = &mut e.behaviour else {
        return;
    };
    enemy::sound_frame(&SOUNDS, frame, &mut st.last_frame, center, e.layer, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::SENTINEL;
    use crate::physics::Rect;

    pub fn sentinel(x: i32, y: i32) -> Entity {
        let mut e = logic(SENTINEL, x, y, &[3, 0, 0, 0, 0, 4, 0, 0, 0, 0, 1]);
        e.flags = 0x84809;
        e.health = HEALTH;
        e.solid_shapes = true;
        e.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 20,
            h: 40,
        }];
        e.behaviour = Behaviour::Sentinel(SentinelState::new(&e.values));
        e
    }

    fn st(e: &Entity) -> &SentinelState {
        match &e.behaviour {
            Behaviour::Sentinel(s) => s,
            _ => panic!(),
        }
    }

    fn shots(out: &Out) -> usize {
        out.enemy_spawns
            .iter()
            .filter(|s| matches!(s, Spawn::Shot { kind: 106, .. }))
            .count()
    }

    #[test]
    fn sentinel_opens_and_fires_three_bullets() {
        let on = |_: i32| None;
        // Jake a ~100 px à direita, na mesma altura
        let (j, w) = ctx_with_jake(200, 0);
        let mut c = ctx(&j, &w, &on);
        let mut s = sentinel(90, 5);
        let mut out = Out::default();
        let mut t = 0;
        while shots(&out) == 0 && t < 40 {
            c.tick = t;
            tick(&mut s, &mut c, &mut out);
            t += 1;
        }
        // 4 quadros fechada + 8 abrindo
        assert_eq!(t, 12, "abre no fim da espera e dispara ao acabar de abrir");
        assert_eq!(shots(&out), 3);
        assert!(st(&s).facing_right && st(&s).alert);
        assert_eq!(st(&s).state, FIRING);
        let Spawn::Shot { vel, at, .. } = out.enemy_spawns[0].clone() else {
            panic!()
        };
        assert_eq!(vel.0, SHOT_SPEED);
        assert!((0..6).contains(&vel.1));
        assert!((100..105).contains(&at.0));
        let ids: Vec<_> = out
            .events
            .iter()
            .filter_map(super::super::Event::sound_id)
            .collect();
        assert!(ids.contains(&3062) && ids.contains(&3060), "{ids:?}");
    }

    #[test]
    fn closed_sentinel_is_invulnerable() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(2000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut s = sentinel(90, 5);
        let p = crate::damage::Pending {
            amount: 2,
            ..Default::default()
        };
        let mut out = Out::default();
        for _ in 0..3 {
            enemy::hurt(&mut s, &p, true);
            tick(&mut s, &mut c, &mut out);
        }
        assert_eq!(s.health, HEALTH);
        // aberta: dois tiros de pistola matam e contam
        let Behaviour::Sentinel(x) = &mut s.behaviour else {
            panic!()
        };
        x.set(FIRING);
        for _ in 0..2 {
            enemy::hurt(&mut s, &p, true);
            tick(&mut s, &mut c, &mut out);
        }
        assert!(s.health <= 0);
        assert!(enemy::killed(&s));
        assert_eq!(out.blasts.len(), 1);
        tick(&mut s, &mut c, &mut out);
        assert!(st(&s).dead && s.alive && !s.solid_shapes, "destroço");
        enemy::hurt(&mut s, &p, true);
        tick(&mut s, &mut c, &mut out);
        assert_eq!(out.blasts.len(), 1, "o destroço não explode de novo");
    }

    #[test]
    fn sentinel_sees_only_ahead_when_open_and_ignores_hiding() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(40, 0);
        let c = ctx(&j, &w, &on);
        let mut s = sentinel(90, 5);
        assert!(
            target(&s, st(&s), &c).is_some(),
            "fechada vê dos dois lados"
        );
        let Behaviour::Sentinel(x) = &mut s.behaviour else {
            panic!()
        };
        x.state = FIRING;
        x.facing_right = true;
        assert!(target(&s, st(&s), &c).is_none(), "aberta para a direita");
        let (far, w2) = ctx_with_jake(400, 0);
        let c2 = ctx(&far, &w2, &on);
        assert!(target(&s, st(&s), &c2).is_none(), "|dx| > 150");
    }
}
