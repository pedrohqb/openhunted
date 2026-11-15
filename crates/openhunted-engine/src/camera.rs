// SPDX-License-Identifier: GPL-3.0-or-later
//! Câmera das vistas (`0x00436386`, `0x00436898`, `docs/re/camadas-desenho.md`): o centro da
//! vista persegue o centro horizontal e o topo do quadro do objeto seguido, no máximo 18 px
//! por tique (comprimento do vetor), sem ficar presa à fase; a camada da vista sai na escala
//! absoluta do original (`View::reference = 30`).

use crate::behaviour::mine::distance;
use crate::physics::Rect;
use crate::scene::Scene;
use crate::scene::view::{REFERENCE, View};

/// Passo máximo do centro da vista por tique, em px (`0x4365ae`: `0x12`).
pub const STEP: i32 = 18;

/// Um tique da câmera: anda de `center` até `target`; acima de `STEP` px, o vetor é
/// encurtado a `STEP` (`dx·18/d`, divisão truncada, `0x4365b3`).
pub fn chase(center: (i32, i32), target: (i32, i32)) -> (i32, i32) {
    let (mut dx, mut dy) = (target.0 - center.0, target.1 - center.1);
    let d = distance((0, 0), (dx, dy));
    if d > STEP {
        dx = (i64::from(dx) * i64::from(STEP) / i64::from(d)) as i32;
        dy = (i64::from(dy) * i64::from(STEP) / i64::from(d)) as i32;
    }
    (center.0 + dx, center.1 + dy)
}

/// Retângulo de referência do objeto (`0x0043585c`): o registro do quadro 0 da animação (ou,
/// com o bit `0x80` de `+0x178`, o do 1º quadro do rótulo; o Jake não o tem) na escala do
/// objeto (`+0x1f0/+0x1f4`), somado à posição `(x, y)` (`+0x14c/+0x150`).
pub fn reference_rect(sp: &crate::scene::Sprite, (x, y): (i32, i32)) -> Option<Rect> {
    let f = sp.anim.frames.first()?;
    let (num, den) = sp.scale;
    let sc = |v: i32| (i64::from(v) * i64::from(num) / i64::from(den.max(1))) as i32;
    Some(Rect {
        x: x.saturating_add(sc(i32::from(f.x))),
        y: y.saturating_add(sc(i32::from(f.y))),
        w: sc(i32::from(f.width)),
        h: sc(i32::from(f.height)),
    })
}

/// Alvo da câmera do jogador `i` (`0x436415…0x43645c`): centro horizontal (`w/2` truncado) e
/// topo do retângulo de referência dele. Sem sprite, o da caixa de colisão.
pub fn target(scene: &Scene, i: usize) -> Option<(i32, i32)> {
    let j = scene.player_jake(i)?;
    let r = scene
        .player_sprite(i)
        .and_then(|sp| reference_rect(sp, (j.body.x, j.body.y)))
        .unwrap_or_else(|| j.body.bounds());
    Some((r.x + r.w / 2, r.y))
}

/// A vista solo (640×`VIEW_H`) centrada em `center`, com a escala absoluta do original:
/// a camada da vista a `d` = 23 sai em 30/23 ≈ 1,30 (`0x436a70`).
pub fn solo_view(scene: &Scene, screen: Rect, size: (i32, i32), center: (i32, i32)) -> View {
    let top_left = (center.0 - size.0 / 2, center.1 - size.1 / 2);
    let mut v = scene.view_of(0, top_left, screen, size);
    v.views = 1;
    v.reference = Some(REFERENCE);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chase_is_capped_at_18_px_along_the_vector() {
        assert_eq!(chase((0, 0), (10, -5)), (10, -5), "perto: vai direto");
        assert_eq!(chase((0, 0), (18, 0)), (18, 0));
        assert_eq!(chase((0, 0), (100, 0)), (18, 0));
        // (30, 40): d = 50 → (30·18/50, 40·18/50) = (10, 14), truncado
        assert_eq!(chase((5, 5), (35, 45)), (15, 19));
        assert_eq!(chase((0, 0), (-30, -40)), (-10, -14), "trunca para o zero");
    }
}
