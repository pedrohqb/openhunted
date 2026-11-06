// SPDX-License-Identifier: GPL-3.0-or-later
//! Caixa genérica (`0x00445dc8`, `docs/re/interface-itf.md`): moldura `SGENERIC.BMP`, texto
//! em `slevd.pft` e um botão (OKAY), centrada na tela. Usada pela pausa ("GAME PAUSED").
//!
//! A geometria do original está em aberto; aqui (decisão do port): moldura 3 × 3 de peças
//! (cantos 60/70 px de largura, 55/95 de altura, miolo repetido de 50 × 29), o texto
//! centrado na área clara e o botão de três colunas (74 + 18·n + 38 px, 29 de altura) no
//! rebaixo da peça de baixo.

use std::rc::Rc;

use openhunted_formats::{Bitmap, Font};

use super::screen::draw_text_in;
use crate::physics::Rect;
use crate::render::{Framebuffer, SCREEN_H, SCREEN_W, measure_text};

/// Cor do texto da caixa e do botão.
pub const TEXT_COLOR: u8 = 162;
/// Peças da moldura: cantos, bordas e miolo.
const TL: usize = 0;
const TOP: usize = 1;
const TR: usize = 2;
const LEFT: usize = 3;
const RIGHT: usize = 4;
const BL: usize = 5;
const BOTTOM: usize = 6;
const BR: usize = 7;
const FILL: usize = 32;
/// Primeiro quadro do botão por estado (normal, em cima, apertado).
const BUTTON: [usize; 3] = [8, 16, 24];
const BUTTON_H: i32 = 29;

/// Caixa aberta.
#[derive(Debug, Clone)]
pub struct GenericBox {
    pub text: String,
    pub button: String,
    /// Retângulo da caixa e do botão na tela.
    pub rect: Rect,
    pub button_rect: Rect,
    cols: i32,
    rows: i32,
    button_cols: i32,
    hover: bool,
    pressed: bool,
}

fn fw(bmp: &Bitmap, i: usize, default: i32) -> i32 {
    bmp.frames.get(i).map_or(default, |f| i32::from(f.width))
}

fn fh(bmp: &Bitmap, i: usize, default: i32) -> i32 {
    bmp.frames.get(i).map_or(default, |f| i32::from(f.height))
}

impl GenericBox {
    pub fn new(bmp: &Bitmap, font: &Font, text: &str, button: &str) -> Self {
        // `~x`/`~l` no começo do texto: formatação (em aberto); aqui só se removem
        let text = text
            .strip_prefix("~x")
            .or_else(|| text.strip_prefix("~l"))
            .unwrap_or(text)
            .to_string();
        let (tw, th) = measure_text(font, &text);
        let (cw, ch) = (fw(bmp, FILL, 50), fh(bmp, FILL, 29));
        let cols = ((tw + 20 + cw - 1) / cw).clamp(2, 9);
        let rows = ((th + 8 + ch - 1) / ch).max(1);
        let w = fw(bmp, TL, 60) + cols * cw + fw(bmp, TR, 70);
        let h = fh(bmp, TL, 55) + rows * ch + fh(bmp, BL, 95);
        let rect = Rect {
            x: (SCREEN_W as i32 - w) / 2,
            y: (SCREEN_H as i32 - h) / 2,
            w,
            h,
        };
        let (bw, _) = measure_text(font, button);
        let button_cols = ((bw + 8 - 38 + 17) / 18).max(1);
        let bwidth = 74 + 18 * button_cols + 38;
        let button_rect = Rect {
            x: rect.x + (w - fw(bmp, TR, 70) + 12 - bwidth) / 2,
            y: rect.y + h - 39,
            w: bwidth,
            h: BUTTON_H,
        };
        Self {
            text,
            button: button.to_string(),
            rect,
            button_rect,
            cols,
            rows,
            button_cols,
            hover: false,
            pressed: false,
        }
    }

    /// Ponteiro movido: realça o botão.
    pub fn pointer(&mut self, x: i32, y: i32) {
        let r = self.button_rect;
        self.hover = x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h;
    }

    /// Botão do mouse: devolve `true` ao soltar em cima do botão (fecha).
    pub fn click(&mut self, down: bool) -> bool {
        if down {
            self.pressed = self.hover;
            false
        } else {
            let hit = self.pressed && self.hover;
            self.pressed = false;
            hit
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer, bmp: &Bitmap, font: &Font) {
        let f = |i: usize| bmp.frames.get(i);
        let Rect { x, y, w, h } = self.rect;
        let (cw, ch) = (fw(bmp, FILL, 50), fh(bmp, FILL, 29));
        let (lw, th) = (fw(bmp, TL, 60), fh(bmp, TL, 55));
        let rw = fw(bmp, TR, 70);
        let bh = fh(bmp, BL, 95);
        let mut put = |i: usize, px: i32, py: i32| {
            if let Some(fr) = f(i) {
                fb.blit(fr, px, py, true);
            }
        };
        put(TL, x, y);
        put(TR, x + w - rw, y);
        put(BL, x, y + h - bh);
        put(BR, x + w - rw, y + h - bh);
        for c in 0..self.cols {
            put(TOP, x + lw + c * cw, y);
            put(BOTTOM, x + lw + c * cw, y + h - bh);
            for r in 0..self.rows {
                put(FILL, x + lw + c * cw, y + th + r * ch);
            }
        }
        for r in 0..self.rows {
            put(LEFT, x, y + th + r * ch);
            put(RIGHT, x + w - rw, y + th + r * ch);
        }
        // botão: topo (13 px) e base (16 px) de três colunas
        let state = if self.pressed && self.hover {
            2
        } else {
            usize::from(self.hover)
        };
        let b0 = BUTTON[state];
        let br = self.button_rect;
        put(b0, br.x, br.y);
        put(b0 + 5, br.x, br.y + 13);
        for c in 0..self.button_cols {
            put(b0 + 1, br.x + 74 + 18 * c, br.y);
            put(b0 + 6, br.x + 74 + 18 * c, br.y + 13);
        }
        let rx = br.x + 74 + 18 * self.button_cols;
        put(b0 + 2, rx, br.y);
        put(b0 + 7, rx, br.y + 13);
        let text_area = Rect {
            x: x + lw,
            y: y + th,
            w: self.cols * cw,
            h: self.rows * ch,
        };
        draw_text_in(fb, font, text_area, &self.text, TEXT_COLOR, true);
        let label = Rect {
            x: br.x + 74,
            y: br.y,
            w: br.w - 74 - 6,
            h: br.h,
        };
        draw_text_in(fb, font, label, &self.button, TEXT_COLOR, true);
    }
}

/// Fonte e moldura da caixa, compartilhadas.
#[derive(Debug, Clone)]
pub struct BoxArt {
    pub frame: Rc<Bitmap>,
    pub font: Rc<Font>,
}
