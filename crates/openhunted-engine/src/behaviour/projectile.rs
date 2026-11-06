// SPDX-License-Identifier: GPL-3.0-or-later
//! Partes comuns dos projéteis das armas 3–6 do Jake (`docs/re/armas-2.md`): voo contra as
//! células e caixa varrida para o teste de acerto.

use super::Ctx;
use crate::entity::Entity;
use crate::physics::{Body, Rect, step};

/// Resultado de um tique de voo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flight {
    /// Andou menos que o pedido (parede, teto ou chão).
    pub blocked: bool,
    pub hit_wall: bool,
    pub on_ground: bool,
    /// Saiu da fase (centro fora): o projétil some.
    pub outside: bool,
    /// Velocidade depois do choque (1/8 px).
    pub v: (i32, i32),
}

/// Move a entidade `v` (1/8 px por tique) contra as células, com ou sem gravidade.
pub fn fly(e: &mut Entity, v: (i32, i32), ctx: &Ctx, gravity: bool) -> Option<Flight> {
    let boxr = e.boxes.first().copied()?;
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = v.0;
    body.vy = v.1;
    let full = (v.0 >> 3, v.1 >> 3);
    step(&mut body, ctx.cells, gravity);
    let want = if gravity {
        (full.0, (v.1 + crate::physics::GRAVITY) >> 3)
    } else {
        full
    };
    let blocked = (body.x - e.x, body.y - e.y) != want;
    e.x = body.x;
    e.y = body.y;
    let c = e.center();
    let outside = c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1;
    Some(Flight {
        blocked,
        hit_wall: body.hit_wall,
        on_ground: body.on_ground,
        outside,
        v: (body.vx, body.vy),
    })
}

/// Caixa da entidade unida à do tique anterior (posição `prev`).
pub fn swept(e: &Entity, prev: Option<(i32, i32)>) -> Option<Rect> {
    let now = e.bounds()?;
    let Some((px, py)) = prev else {
        return Some(now);
    };
    let before = now.moved(px - e.x, py - e.y);
    let x0 = now.x.min(before.x);
    let y0 = now.y.min(before.y);
    let x1 = (now.x + now.w).max(before.x + before.w);
    let y1 = (now.y + now.h).max(before.y + before.h);
    Some(Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
}

/// Nasceu dentro de um sólido das células.
pub fn inside_wall(e: &Entity, ctx: &Ctx) -> bool {
    e.bounds().is_some_and(|b| ctx.cells.blocked(&b))
}
