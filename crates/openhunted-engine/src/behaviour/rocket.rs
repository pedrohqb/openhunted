// SPDX-License-Identifier: GPL-3.0-or-later
//! Foguete (tipo 21, `0x0040d678`; `docs/re/armas-2.md`): reto, sem gravidade; ao tocar
//! algo, 20 de dano direto e explosão de raio 64 (área 50, empurrão 30), que fere o Jake.

use super::{Ctx, Out, projectile};
use crate::entity::{Behaviour, Entity};

/// Linha `(21, 0)` do `DAMAGE.TBL`.
pub const DAMAGE: i32 = 20;
pub const RADIUS: i32 = 64;
pub const PUSH: i32 = 30;
pub const AREA: i32 = 50;

#[derive(Debug, Clone)]
pub struct RocketState {
    pub vx: i32,
    pub vy: i32,
    pub prev: Option<(i32, i32)>,
    pub spent: bool,
}

impl RocketState {
    pub fn new(vx: i32, vy: i32) -> Self {
        Self {
            vx,
            vy,
            prev: None,
            spent: false,
        }
    }
}

pub fn tick(e: &mut Entity, ctx: &Ctx, _out: &mut Out) {
    let pos = (e.x, e.y);
    let Behaviour::Rocket(st) = &mut e.behaviour else {
        return;
    };
    st.prev = Some(pos);
    let v = (st.vx, st.vy);
    if projectile::inside_wall(e, ctx) {
        if let Behaviour::Rocket(st) = &mut e.behaviour {
            st.spent = true;
        }
        return;
    }
    let Some(f) = projectile::fly(e, v, ctx, false) else {
        e.alive = false;
        return;
    };
    if let Behaviour::Rocket(st) = &mut e.behaviour {
        // fora da fase também explode (`0x43d629` sem vítima)
        st.spent = f.blocked || f.outside;
    }
}
