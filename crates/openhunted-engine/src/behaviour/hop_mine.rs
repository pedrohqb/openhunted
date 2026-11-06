// SPDX-License-Identifier: GPL-3.0-or-later
//! Mina saltadora (tipo 11, "Mine", `0x0041bdb8`/`0x0041bed9`; `docs/re/mina-saltadora.md`):
//! pula pelo chão — cinco pulinhos e um pulo grande — e explode ao encostar no Jake.

use super::enemy::{self, Clip, Label, Wound};
use super::{Blast, Ctx, Out, grow};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, Rect, SUB, step};

/// Rótulos do `ohs.anm`: 1 `ohs00` (pulo grande), 2 `ohs01` (parada), 3 `ohs02` (pulinho),
/// 4 `ohs03` (virar; o −1 é ele ao contrário).
pub const LABELS: [Label; 4] = [(1, 0, 10), (2, 10, 8), (3, 18, 11), (4, 29, 7)];
/// `SOUND.TBL` tipo 11.
pub const SOUNDS: [(usize, u16, u16); 4] =
    [(1, 3014, 0), (10, 3010, 5), (20, 3011, 0), (29, 3012, 0)];
pub const HEALTH: i32 = 10;
/// Pulinhos até o pulo grande (`+0x27c`).
pub const HOPS: i32 = 5;
/// Percepção (`0x41c2a2`): `|dy|` ≤ 50, `|dx|` ≤ 450, só para a frente.
pub const SIGHT_DY: i32 = 50;
pub const SIGHT_DX: i32 = 450;
/// Linha `(11, 0)`: 20 diretos em quem tocou; raio 12, empurrão 20, área 20.
pub const DIRECT: i32 = 20;
pub const BLAST_RADIUS: i32 = 12;
pub const BLAST_PUSH: i32 = 20;
pub const BLAST_DAMAGE: i32 = 20;
/// Ainda caindo: `vy` > 7 (1/8 px).
const FALLING: i32 = 7;

pub const BIG: i8 = 1;
pub const REST: i8 = 2;
pub const SMALL: i8 = 3;
pub const TURN: i8 = 4;
pub const TURN_BACK: i8 = -1;

#[derive(Debug, Clone)]
pub struct HopMineState {
    pub state: i8,
    pub clip: Clip,
    /// Sentido `+0x264` (`true` = direita).
    pub right: bool,
    /// No ar `+0x26c`.
    pub in_air: bool,
    /// Pulinhos restantes `+0x27c`.
    pub hops: i32,
    /// Sprite espelhado (inverte ao fim de cada virada).
    pub mirrored: bool,
    /// Velocidade (1/8 px por tique).
    pub vx: i32,
    pub vy: i32,
    pub wound: Wound,
    pub last_frame: Option<usize>,
}

impl Default for HopMineState {
    fn default() -> Self {
        Self {
            state: REST,
            clip: Clip::new(2, false),
            right: false,
            in_air: false,
            hops: HOPS,
            mirrored: false,
            vx: 0,
            vy: 0,
            wound: Wound::default(),
            last_frame: None,
        }
    }
}

impl HopMineState {
    fn set(&mut self, state: i8) {
        self.state = state;
        self.clip = match state {
            TURN_BACK => Clip::new(4, true),
            s => Clip::new(s as u16, false),
        };
    }

    fn side(&self) -> i32 {
        if self.right { 1 } else { -1 }
    }

    /// Anda: pulinho (`0x41c5aa`) ou, gastos os cinco, o pulo grande (`0x41c543`).
    fn walk(&mut self, ctx: &mut Ctx) {
        if self.hops > 0 {
            self.hops -= 1;
            self.set(SMALL);
            self.vx = self.side() * (4 + ctx.rng.below(4) as i32 - 2) * SUB;
            self.vy = -8 * SUB;
        } else {
            self.hops = HOPS;
            self.set(BIG);
            self.vx = self.side() * (9 + ctx.rng.below(6) as i32 - 3) * SUB;
            self.vy = (-10 + ctx.rng.below(4) as i32 - 2) * SUB;
        }
    }
}

/// Alvo (`0x41c2a2`): o Jake na camada, `|dy|` ≤ 50, `|dx|` ≤ 450, do lado do sentido, à
/// vista. Sem teste de esconderijo.
fn sees(e: &Entity, st: &HopMineState, ctx: &Ctx) -> bool {
    let (Some(me), Some(j)) = (e.bounds(), enemy::jake_target(ctx, e.layer)) else {
        return false;
    };
    let (c, jc) = (enemy::center(&me), enemy::center(&j));
    let (dx, dy) = (jc.0 - c.0, jc.1 - c.1);
    dy.abs() <= SIGHT_DY
        && dx.abs() <= SIGHT_DX
        && (if st.right { dx >= 0 } else { dx <= 0 })
        && enemy::line_of_sight(ctx, c, jc, &[me, j])
}

/// Fim de animação ou verificação da aterrissagem (`0x0041c093`).
fn landing(e: &Entity, st: &mut HopMineState, ctx: &mut Ctx) {
    if st.state != REST {
        st.vx = 0;
    }
    if st.vy > FALLING {
        st.in_air = true;
        return;
    }
    st.in_air = false;
    if matches!(st.state, TURN | TURN_BACK) {
        st.mirrored = !st.mirrored;
    }
    if sees(e, st, ctx) {
        st.walk(ctx);
        return;
    }
    match ctx.rng.below(100) {
        0..33 => {
            st.set(if st.right { TURN_BACK } else { TURN });
            st.vy = -7 * SUB;
            st.right = !st.right;
        }
        33..66 => st.set(REST),
        _ => st.walk(ctx),
    }
}

pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Some(me) = e.bounds() else {
        e.alive = false;
        return;
    };
    let c = enemy::center(&me);
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.alive = false;
        return;
    }
    let mut st = match &e.behaviour {
        Behaviour::HopMine(s) => s.clone(),
        _ => return,
    };
    // contato (`0x41c49a`): o Jake e as balas a detonam; o resto do dano subtrai
    let touched = enemy::jake_target(ctx, e.layer).is_some_and(|j| grow(me, 1).overlaps(&j));
    let w = st.wound.take();
    if touched || (w.by_player && !w.is_empty()) {
        e.health = 0;
    } else {
        e.health -= w.pending.amount;
    }
    if e.health <= 0 {
        if touched {
            out.jake_damage.add(&Pending {
                amount: DIRECT,
                ..Default::default()
            });
        }
        out.blasts.push(Blast {
            center: c,
            layer: e.layer,
            radius: BLAST_RADIUS,
            push: BLAST_PUSH,
            damage: BLAST_DAMAGE,
        });
        if w.by_player {
            out.kills += 1;
        }
        e.alive = false;
        e.on = true;
        return;
    }
    if st.in_air {
        landing(e, &mut st, ctx);
    } else {
        let (_, _, len) = enemy::label(&LABELS, st.clip.label);
        if st.clip.advance(len) {
            landing(e, &mut st, ctx);
        }
    }
    // física com gravidade contra as células
    let boxr = e.boxes.first().copied().unwrap_or(Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    });
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = st.vx;
    body.vy = st.vy;
    step(&mut body, ctx.cells, true);
    e.x = body.x;
    e.y = body.y;
    st.vx = body.vx;
    st.vy = body.vy;
    let frame = enemy::show(e, &LABELS, &st.clip);
    if let Some(sp) = e.sprite.as_mut() {
        sp.mirror = st.mirrored;
    }
    enemy::sound_frame(&SOUNDS, frame, &mut st.last_frame, c, e.layer, out);
    e.behaviour = Behaviour::HopMine(st);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::HOP_MINE;

    fn hopper(x: i32, y: i32) -> Entity {
        let mut e = logic(HOP_MINE, x, y, &[0, 0, 0, 0, 0, 4, 0, 5]);
        e.flags = 0x80801;
        e.health = HEALTH;
        e.solid_shapes = true;
        e.boxes = vec![Rect {
            x: 0,
            y: 0,
            w: 17,
            h: 17,
        }];
        e.behaviour = Behaviour::HopMine(HopMineState::default());
        e
    }

    fn st(e: &Entity) -> &HopMineState {
        match &e.behaviour {
            Behaviour::HopMine(s) => s,
            _ => panic!(),
        }
    }

    #[test]
    fn hop_pattern_five_small_one_big_towards_the_jake() {
        let on = |_: i32| None;
        // Jake à esquerda, no mesmo chão (y + 47)
        let (j, w) = ctx_with_jake(0, 400);
        let mut c = ctx(&j, &w, &on);
        let mut m = hopper(400, 430);
        let mut out = Out::default();
        let mut seq = vec![];
        let mut prev = st(&m).clip;
        for _ in 0..200 {
            tick(&mut m, &mut c, &mut out);
            if !m.alive {
                break;
            }
            let s = st(&m);
            if s.clip != prev && s.clip.step == 0 {
                seq.push(s.state);
            }
            prev = s.clip;
        }
        assert_eq!(
            &seq[..6],
            &[SMALL, SMALL, SMALL, SMALL, SMALL, BIG],
            "{seq:?}"
        );
        assert!(m.x < 400, "pula para a esquerda, para o Jake");
    }

    #[test]
    fn hop_mine_explodes_on_jake() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 400);
        let mut c = ctx(&j, &w, &on);
        // caixa do Jake x 102..120: a mina encostada em x 120
        let mut m = hopper(120, 420);
        let mut out = Out::default();
        tick(&mut m, &mut c, &mut out);
        assert!(!m.alive);
        assert_eq!(out.jake_damage.amount, DIRECT);
        assert_eq!(out.blasts.len(), 1);
        assert_eq!(out.blasts[0].damage, BLAST_DAMAGE);
        assert_eq!(out.kills, 0, "encostar não é morte do jogador");
        // a bala da pistola detona e conta
        let mut m = hopper(600, 420);
        let p = Pending {
            amount: 2,
            ..Default::default()
        };
        assert!(enemy::hurt(&mut m, &p, true));
        tick(&mut m, &mut c, &mut out);
        assert!(!m.alive);
        assert_eq!(out.kills, 1);
    }

    #[test]
    fn hop_mine_sees_only_ahead() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(300, 400);
        let c = ctx(&j, &w, &on);
        let m = hopper(100, 430);
        assert!(
            !sees(&m, st(&m), &c),
            "olha para a esquerda; o Jake está à direita"
        );
        let mut r = hopper(100, 430);
        let Behaviour::HopMine(s) = &mut r.behaviour else {
            panic!()
        };
        s.right = true;
        assert!(sees(&r, st(&r), &c));
        let (far, w2) = ctx_with_jake(700, 400);
        assert!(!sees(&r, st(&r), &ctx(&far, &w2, &on)), "|dx| > 450");
    }
}
