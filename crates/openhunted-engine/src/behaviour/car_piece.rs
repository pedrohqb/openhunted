// SPDX-License-Identifier: GPL-3.0-or-later
//! Peça do carro (tipo 9, `0x00414ce0`/`0x00414d99`; `docs/re/objetos-tutorial.md`): flutua
//! 16 px em torno do meio da célula; apanhada, cumpre o objetivo "acionar".

use super::{Ctx, Event, Out};
use crate::entity::Entity;
use crate::sound::{Cue, PRIORITY_PICKUP};

/// "Pickup flag or key".
pub const SOUND_PICKUP: u16 = 5060;
/// Oscilação: amplitude 16 px, período 22 tiques, em torno de y = 64 da célula.
pub const AMPLITUDE: i64 = 16;
pub const PERIOD: i32 = 22;
/// O Jake pode pegar (`+0x178 & 0x100`).
const JAKE_CAN_TAKE: i32 = 0x100;

/// Velocidade vertical (1/8 px) que persegue o ponto da oscilação.
pub fn bob_velocity(tick: u32, x: i32, y: i32) -> i32 {
    let phase = f64::from((tick as i32).wrapping_add(x).rem_euclid(PERIOD)) / f64::from(PERIOD);
    let s = ((phase * std::f64::consts::TAU).sin() * 16384.0).round() as i64;
    let target = 64 + AMPLITUDE * s / 16384;
    (target - i64::from(y.rem_euclid(128))).clamp(-64, 64) as i32
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let touching = e.flags & JAKE_CAN_TAKE != 0
        && e.layer == ctx.jake_layer
        && e.bounds()
            .zip(ctx.jake_bounds())
            .is_some_and(|(p, j)| p.overlaps(&j));
    if touching {
        out.events
            .push(Event::Sound(Cue::ui(SOUND_PICKUP, PRIORITY_PICKUP)));
        e.alive = false;
        e.on = true;
        return;
    }
    e.y += bob_velocity(ctx.tick, e.x, e.y) >> 3;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::entity::CAR_PIECE;

    #[test]
    fn piece_bobs_and_is_picked_up() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(1000, 0);
        let mut c = ctx(&j, &w, &on);
        let mut p = logic(CAR_PIECE, 100, 64, &[]);
        p.flags = 0x80bad;
        let mut ys = vec![];
        for t in 0..88 {
            c.tick = t;
            tick(&mut p, &c, &mut Out::default());
            ys.push(p.y);
        }
        let (lo, hi) = (
            ys[44..].iter().min().unwrap(),
            ys[44..].iter().max().unwrap(),
        );
        assert!(
            *lo >= 64 - 16 && *hi <= 64 + 16 && hi - lo >= 8,
            "{lo}..{hi}"
        );
        let (j, w) = ctx_with_jake(100, 40);
        let c = ctx(&j, &w, &on);
        let mut out = Out::default();
        tick(&mut p, &c, &mut out);
        assert!(!p.alive && p.on);
        assert_eq!(
            out.events,
            vec![Event::Sound(Cue::ui(SOUND_PICKUP, PRIORITY_PICKUP))]
        );
    }
}
