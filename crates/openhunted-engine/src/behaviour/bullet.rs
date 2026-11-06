// SPDX-License-Identifier: GPL-3.0-or-later
//! Bala da pistola (tipo 19, `0x0040c964`; `docs/re/projeteis.md`).

use super::{Ctx, Out};
use crate::entity::Entity;
use crate::physics::{Body, step};

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
    /// Posição antes do último deslocamento (para o teste de acerto varrer o caminho).
    pub prev: Option<(i32, i32)>,
}

/// Caixa varrida pela bala no último tique (união da caixa anterior com a atual), para o
/// teste de acerto não pular alvos menores que o passo de 20 px.
pub fn swept_bounds(e: &Entity) -> Option<crate::physics::Rect> {
    let now = e.bounds()?;
    let prev = match &e.behaviour {
        crate::entity::Behaviour::Bullet(b) => b.prev,
        _ => None,
    };
    let Some((px, py)) = prev else {
        return Some(now);
    };
    let (dx, dy) = (px - e.x, py - e.y);
    let before = crate::physics::Rect {
        x: now.x + dx,
        y: now.y + dy,
        ..now
    };
    let x0 = now.x.min(before.x);
    let y0 = now.y.min(before.y);
    let x1 = (now.x + now.w).max(before.x + before.w);
    let y1 = (now.y + now.h).max(before.y + before.h);
    Some(crate::physics::Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    })
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
    let (vx, vy) = match &mut e.behaviour {
        crate::entity::Behaviour::Bullet(b) => {
            b.prev = Some((e.x, e.y));
            (b.vx, b.vy)
        }
        _ => return,
    };
    // nasceu dentro de um sólido (boca na parede): impacto imediato
    if let Some(b) = e.bounds() {
        if ctx.cells.blocked(&b) {
            let c = e.center();
            e.alive = false;
            out.impacts.push((c.0, c.1, e.layer));
            return;
        }
    }
    let mut body = Body::new(e.x, e.y, boxr);
    body.vx = vx;
    body.vy = vy;
    step(&mut body, ctx.cells, false);
    // qualquer desvio do deslocamento pedido (parede, teto ou chão) é um toque
    let full = (vx >> 3, vy >> 3);
    let moved = (body.x - e.x, body.y - e.y) == full;
    e.x = body.x;
    e.y = body.y;
    let c = e.center();
    if c.0 < 0 || c.1 < 0 || c.0 >= ctx.level.0 || c.1 >= ctx.level.1 {
        e.alive = false;
        return;
    }
    if !moved {
        e.alive = false;
        out.impacts.push((c.0, c.1, e.layer));
    }
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
