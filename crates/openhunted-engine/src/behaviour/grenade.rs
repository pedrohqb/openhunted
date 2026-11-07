// SPDX-License-Identifier: GPL-3.0-or-later
//! Granada lançada (tipo 26, `0x0040d7ab`; `docs/re/armas-2.md`): com gravidade, pavio de
//! 35–44 tiques, quica nas paredes (som 3042) e explode ao tocar um objeto que não seja das
//! exceções; explosão de raio 55 (área 30, empurrão 30).

use super::{Blast, Ctx, Event, Out, projectile};
use crate::entity::{Behaviour, Entity};

/// Linha `(26, 0)`: sem dano direto; raio 55, empurrão 30, área 30.
pub const RADIUS: i32 = 55;
pub const PUSH: i32 = 30;
pub const AREA: i32 = 30;
/// Pavio: 35 + `rand % 10`.
pub const FUSE_MIN: u32 = 35;
pub const FUSE_RAND: u32 = 10;
/// "Grenade bouncing".
pub const SOUND_BOUNCE: u16 = 3042;
/// Rótulo do `OGR.ANM` da granada lançada.
pub const LABEL_THROWN: u16 = 2;

/// Tipos que a granada atravessa sem explodir (`0x40d85a`).
pub fn ignores(kind: u32) -> bool {
    matches!(
        kind,
        5 | 6 | 9 | 22..=24 | 30 | 38 | 39 | 41 | 46 | 50..=52 | 56 | 68..=74
    )
}

#[derive(Debug, Clone)]
pub struct GrenadeState {
    pub vx: i32,
    pub vy: i32,
    pub fuse: u32,
    pub prev: Option<(i32, i32)>,
}

impl GrenadeState {
    pub fn new(vx: i32, vy: i32, fuse: u32) -> Self {
        Self {
            vx,
            vy,
            fuse,
            prev: None,
        }
    }
}

/// A explosão da granada no centro dela.
pub fn blast(e: &Entity) -> Blast {
    Blast {
        center: e.center(),
        layer: e.layer,
        radius: RADIUS,
        push: PUSH,
        damage: AREA,
    }
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let pos = (e.x, e.y);
    let Behaviour::Grenade(st) = &mut e.behaviour else {
        return;
    };
    st.prev = Some(pos);
    if st.fuse == 0 {
        out.blasts.push(blast(e));
        e.alive = false;
        return;
    }
    st.fuse -= 1;
    let v = (st.vx, st.vy);
    let Some(f) = projectile::fly(e, v, ctx, true) else {
        e.alive = false;
        return;
    };
    if f.outside {
        e.alive = false;
        return;
    }
    if let Behaviour::Grenade(st) = &mut e.behaviour {
        // parede (`0x40707c`: contato com a normal mais horizontal que vertical): inverte vx
        // sem perder rapidez (a resposta comum da física daria meio rebote)
        st.vx = if f.hit_wall { -v.0 } else { f.v.0 };
        st.vy = if f.on_ground && f.v.1 > 0 { 0 } else { f.v.1 };
    }
    if f.hit_wall {
        // `0x434815(granada, 1)`: inverte o espelho; som posicional na base da caixa
        if let Some(sp) = e.sprite.as_mut() {
            sp.mirror = !sp.mirror;
        }
        let b = e.bounds().unwrap_or(crate::physics::Rect {
            x: e.x,
            y: e.y,
            w: 0,
            h: 0,
        });
        out.events.push(Event::Sound(crate::sound::Cue::at(
            SOUND_BOUNCE,
            crate::sound::PRIORITY_FRAME,
            (b.x + b.w / 2, b.y + b.h),
            e.layer,
        )));
    }
}
