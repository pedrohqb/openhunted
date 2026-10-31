// SPDX-License-Identifier: GPL-3.0-or-later
//! Bala da pistola (tipo 19, `0x0040c964`; `docs/re/projeteis.md`).

use super::{Ctx, Out};
use crate::entity::Entity;
use crate::physics::{Body, Rect, SUB, step};

/// Som "Bullet Hitting Wall" (quadro 148 do `OEX.ANM`, `oex15`).
pub const SOUND_WALL: u16 = 3046;
/// Dano direto da linha (19, qualquer) do `DAMAGE.TBL`.
pub const DAMAGE: i32 = 2;
/// Raio do efeito `oex15` do impacto (sem dano de área).
pub const IMPACT_RADIUS: i32 = 5;
/// Rótulo do `OEX.ANM` da faísca.
pub const IMPACT_LABEL: u16 = 15;

#[derive(Debug, Clone)]
pub struct BulletState {
    /// Velocidade em 1/8 px por tique.
    pub vx: i32,
    pub vy: i32,
}

/// Rótulo do `OBU.ANM` e espelhamento (bit 1 horizontal, bit 2 vertical) pela direção
/// (`0x0040bff8`): |vx| > 2|vy| → horizontal (1); |vy| > 2|vx| → vertical (3); senão
/// diagonal (2). A arte aponta para a direita e para cima.
pub fn label_and_mirror(vx: i32, vy: i32) -> (u16, u8) {
    let (ax, ay) = (vx.abs(), vy.abs());
    let h = u8::from(vx < 0);
    let v = u8::from(vy > 0) << 1;
    if ax > 2 * ay {
        (1, h)
    } else if ay > 2 * ax {
        (3, v)
    } else {
        (2, h | v)
    }
}

/// Um tique: voa reto contra as células; parede → impacto; fora da fase → some.
pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let Some(boxr) = e.boxes.first().copied() else {
        e.alive = false;
        return;
    };
    let (vx, vy) = match &e.behaviour {
        crate::entity::Behaviour::Bullet(b) => (b.vx, b.vy),
        _ => return,
    };
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = vx;
    body.vy = vy;
    step(&mut body, ctx.cells, false);
    e.x = body.x;
    e.y = body.y;
    let moved = (body.x, body.y) != (e.x - (vx >> 3), e.y - (vy >> 3));
    let c = e.center();
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.alive = false;
        return;
    }
    if body.hit_wall || body.hit_ceiling || !moved {
        e.alive = false;
        out.impacts.push((c.0, c.1, e.layer));
    }
    let _ = SUB;
}

/// Caixa da bala em px da fase.
pub fn bounds(e: &Entity) -> Option<Rect> {
    e.bounds()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_and_mirror_follow_direction() {
        assert_eq!(label_and_mirror(20, 0), (1, 0));
        assert_eq!(label_and_mirror(-20, 1), (1, 1));
        assert_eq!(label_and_mirror(0, -20), (3, 0));
        assert_eq!(label_and_mirror(0, 20), (3, 2));
        assert_eq!(label_and_mirror(20, -20), (2, 0));
        assert_eq!(label_and_mirror(-20, 20), (2, 3));
    }
}
