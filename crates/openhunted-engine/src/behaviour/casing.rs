// SPDX-License-Identifier: GPL-3.0-or-later
//! Cápsula da pistola (tipo 22, `0x0040c9f9`; `docs/re/projeteis.md`).

use super::{Ctx, Event, Out};
use crate::entity::{Behaviour, Entity};
use crate::physics::{Body, SUB, step};

/// Sons "Bullet shell clink" (sorteio entre os dois).
pub const SOUNDS: [u16; 2] = [3020, 3021];

#[derive(Debug, Clone)]
pub struct CasingState {
    pub vx: i32,
    pub vy: i32,
    /// Tiques seguidos abaixo de 1 px/tique nos dois eixos.
    pub slow: u32,
    pub clinked: bool,
}

/// Um tique: cai com gravidade contra as células; ao tocar algo, o tilintar; some quando
/// fica lenta por dois tiques ou sai da fase.
pub fn tick(e: &mut Entity, ctx: &mut Ctx, out: &mut Out) {
    let Some(boxr) = e.boxes.first().copied() else {
        e.alive = false;
        return;
    };
    let Behaviour::Casing(st) = &mut e.behaviour else {
        return;
    };
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = st.vx;
    body.vy = st.vy;
    step(&mut body, ctx.cells_for(e.layer), true);
    e.x = body.x;
    e.y = body.y;
    st.vx = body.vx;
    st.vy = body.vy;
    if (body.hit_wall || body.hit_ceiling || body.on_ground) && !st.clinked {
        st.clinked = true;
        let id = SOUNDS[ctx.rng.below(2) as usize];
        // centro da caixa (a única da cápsula)
        let c = (body.x + boxr.x + boxr.w / 2, body.y + boxr.y + boxr.h / 2);
        out.events
            .push(Event::Sound(crate::sound::Cue::at(id, 0, c, e.layer)));
    }
    if body.vx.abs() < SUB && body.vy.abs() < SUB {
        st.slow += 1;
    } else {
        st.slow = 0;
    }
    let out_of_level = e.x < 0 || e.y < 0 || e.x >= ctx.level.0 || e.y >= ctx.level.1;
    if st.slow >= 2 || out_of_level {
        e.alive = false;
    }
}
