// SPDX-License-Identifier: GPL-3.0-or-later
//! Vistas da tela (`docs/re/multijogador.md`): o layout `0x4361c8` (uma vista por jogador,
//! J1 em cima) e o lugar da barra de cada uma (`0x40a4ef`, alturas pela metade com dois ou
//! mais jogadores, `0x471cd0`).

use super::hud::BAR_H;
use crate::physics::Rect;
use crate::render::{SCREEN_H, SCREEN_W};

/// Barra achatada (37 px) e altura de referência da escala de cada metade (`0x436a70`).
pub const HALF_BAR_H: i32 = BAR_H / 2;
pub const SPLIT_REFERENCE_H: i32 = 240;

/// Retângulos de uma vista na tela 640×480: o jogo e a barra logo abaixo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewRect {
    pub game: Rect,
    pub bar: Rect,
}

/// Layout de `n` vistas (`0x4361c8`): colunas 2 se `n > 2`, senão 1; linhas `(n + 1) / 2`
/// ou `n`; a barra (75 px, 37 com `n ≥ 2`) tira a altura do jogo. Ordem: jogador 1, 2…
pub fn layout(n: usize) -> Vec<ViewRect> {
    let n = n.max(1);
    let cols = if n > 2 { 2 } else { 1 };
    let rows = if n > 2 { n.div_ceil(2) } else { n };
    let (w, h) = (SCREEN_W as i32 / cols as i32, SCREEN_H as i32 / rows as i32);
    let bar_h = if n >= 2 { HALF_BAR_H } else { BAR_H };
    (0..n)
        .map(|i| {
            let (x, y) = ((i % cols) as i32 * w, (i / cols) as i32 * h);
            ViewRect {
                game: Rect {
                    x,
                    y,
                    w,
                    h: h - bar_h,
                },
                bar: Rect {
                    x,
                    y: y + h - bar_h,
                    w,
                    h: bar_h,
                },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn solo_and_split_layouts() {
        assert_eq!(
            layout(1),
            [ViewRect {
                game: r(0, 0, 640, 405),
                bar: r(0, 405, 640, 75)
            }]
        );
        // dois: J1 em cima (640×240), J2 embaixo; jogo 640×203, barras em y = 203 e 443
        assert_eq!(
            layout(2),
            [
                ViewRect {
                    game: r(0, 0, 640, 203),
                    bar: r(0, 203, 640, 37)
                },
                ViewRect {
                    game: r(0, 240, 640, 203),
                    bar: r(0, 443, 640, 37)
                },
            ]
        );
        let four = layout(4);
        assert_eq!(four[3].game, r(320, 240, 320, 203));
        assert_eq!(layout(3).len(), 3);
    }
}
