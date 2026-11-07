// SPDX-License-Identifier: GPL-3.0-or-later
//! Barra de estado (`INVBAR01.BMP`) embaixo da vista do jogo (`docs/re/hud.md`).

use openhunted_formats::{Bitmap, Frame};

use crate::render::Framebuffer;

/// Altura da barra (quadro 0) e altura da vista do jogo em 640×480 solo.
pub const BAR_H: i32 = 75;
pub const VIEW_H: i32 = 480 - BAR_H;

/// Personagem (escolhe a tabela de armas).
pub use crate::character::Character;

/// O que a barra mostra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudState {
    pub character: Character,
    pub health: i32,
    pub armor: i32,
    /// Rosto alternativo (`+0x270 ≠ 0`, quadro 93).
    pub face_alt: bool,
    /// Pares (tem, quantidade) do inventário, índice = número do par.
    pub slots: [(i32, i32); 30],
    /// Par da arma corrente.
    pub weapon: usize,
    /// Cronômetro em tiques (`0x4972fc`); 0 nas fases sem limite.
    pub timer: i32,
}

/// Tabela das armas por personagem: (quadro se tem, par, quadro se não tem), na ordem da
/// barra (`0x471cd1` / `0x471ce9`).
const JAKE_WEAPONS: [(usize, usize, usize); 6] = [
    (52, 1, 196),
    (58, 6, 199),
    (56, 5, 198),
    (60, 2, 200),
    (62, 3, 201),
    (64, 4, 202),
];
const GARATHE_WEAPONS: [(usize, usize, usize); 6] = [
    (66, 1, 203),
    (68, 2, 204),
    (58, 5, 199),
    (54, 4, 197),
    (56, 6, 198),
    (70, 3, 205),
];

/// Estilo do número de vida/armadura pelo valor.
fn health_style(v: i32) -> usize {
    if v >= 50 {
        19
    } else if v >= 20 {
        18
    } else {
        17
    }
}

fn frame(bar: &Bitmap, i: usize) -> Option<&Frame> {
    bar.frames.get(i)
}

fn blit(fb: &mut Framebuffer, bar: &Bitmap, i: usize, x: i32, y: i32) {
    if let Some(f) = frame(bar, i) {
        fb.blit(f, x, y, true);
    }
}

fn width(bar: &Bitmap, i: usize) -> i32 {
    frame(bar, i).map_or(0, |f| i32::from(f.width))
}

/// Desenha um número (`0x0040a716`): o quadro `style` como fundo em (x, y) e os dígitos da
/// direita para a esquerda, com zeros à esquerda, limitado ao máximo de dígitos do estilo.
pub fn draw_number(fb: &mut Framebuffer, bar: &Bitmap, value: i32, style: usize, x: i32, y: i32) {
    let (digits, glyphs, ux, uy) = match style {
        17 => (3, 20, 32, 4),
        18 => (3, 30, 32, 4),
        19 => (3, 40, 32, 4),
        50 => (2, 72, 14, 8),
        51 => (2, 82, 14, 8),
        196 => (2, 20, 20, 4),
        _ => return,
    };
    blit(fb, bar, style, x, y);
    let max = 10i32.pow(digits) - 1;
    let mut v = value.clamp(0, max);
    let mut cx = x + ux;
    for _ in 0..digits {
        let g = glyphs + (v % 10) as usize;
        blit(fb, bar, g, cx, y + uy);
        v /= 10;
        // o próximo dígito fica à esquerda: avança a largura do glifo + 1
        let next = glyphs + (v % 10) as usize;
        cx -= width(bar, next) + 1;
    }
}

/// Quadros dos 4 dígitos do cronômetro, das unidades para os milhares: contador de rolo,
/// quadro `193 − 10·d − r` com `r = (tiques % 20) / 2`, que só passa ao dígito seguinte
/// enquanto o anterior é 9.
pub fn timer_frames(ticks: i32) -> [usize; 4] {
    let ticks = ticks.max(0);
    let mut secs = (ticks / 20).min(9999);
    let mut r = (ticks % 20) / 2;
    let mut out = [0; 4];
    for o in out.iter_mut() {
        let d = secs % 10;
        *o = (193 - 10 * d - r) as usize;
        if d != 9 {
            r = 0;
        }
        secs /= 10;
    }
    out
}

/// Desenha a barra inteira com o canto em (x, y) e a largura `w` (`0x40a5b6`).
pub fn draw_hud(fb: &mut Framebuffer, bar: &Bitmap, x: i32, y: i32, w: i32, s: &HudState) {
    // fundo: quadro 0 repetido
    let tile = width(bar, 0).max(1);
    let mut tx = x;
    while tx < x + w {
        blit(fb, bar, 0, tx, y);
        tx += tile;
    }
    // faixas de itens e ícones dos pares 13–18 (esquerda) e 7–12 (direita)
    let strip = width(bar, 1);
    blit(fb, bar, 1, x, y);
    blit(fb, bar, 1, x + w - strip, y);
    for i in 0..6 {
        for (pair, icon, base) in [(13 + i, 2 + i, x), (7 + i, 8 + i, x + w - strip)] {
            if s.slots.get(pair).is_some_and(|p| p.0 != 0) {
                let cx = base + 35 + 51 * i as i32;
                blit(fb, bar, icon, cx - width(bar, icon) / 2, y + 5);
            }
        }
    }
    // vida e armadura
    blit(fb, bar, 14, x, y + 30);
    blit(fb, bar, if s.face_alt { 93 } else { 15 }, x + 6, y + 35);
    draw_number(fb, bar, s.health, health_style(s.health), x + 43, y + 35);
    blit(fb, bar, 14, x + 96, y + 30);
    blit(fb, bar, 16, x + 100, y + 32);
    draw_number(fb, bar, s.armor, health_style(s.armor), x + 139, y + 35);
    // armas
    let table = match s.character {
        Character::Jake => &JAKE_WEAPONS,
        Character::Garathe => &GARATHE_WEAPONS,
    };
    let icon = |&(has, pair, not): &(usize, usize, usize)| {
        if s.slots.get(pair).is_some_and(|p| p.0 != 0) {
            has + usize::from(pair == s.weapon)
        } else {
            not
        }
    };
    let total: i32 = table.iter().map(|t| width(bar, icon(t))).sum::<i32>() + 6 * width(bar, 50);
    let mut wx = x + w / 2 - total / 2 + 63;
    let wy = y + 30;
    for (i, t) in table.iter().enumerate() {
        let f = icon(t);
        blit(fb, bar, f, wx, wy);
        wx += width(bar, f);
        if i > 0 {
            let ammo = s.slots.get(t.1).map_or(0, |p| p.1);
            let style = if t.1 == s.weapon { 50 } else { 51 };
            draw_number(fb, bar, ammo, style, wx, wy);
            wx += width(bar, 50);
        }
    }
    // cronômetro
    let tx = x + w - 93;
    blit(fb, bar, 92, tx, y + 30);
    for (k, f) in timer_frames(s.timer).iter().enumerate() {
        blit(fb, bar, *f, tx + 68 - 22 * k as i32, y + 36);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Barra sintética: 211 quadros 2×2, o quadro i pintado com a cor i (mod 256, sem 0).
    fn bar() -> Bitmap {
        Bitmap {
            frames: (0..211)
                .map(|i| Frame {
                    width: 2,
                    height: 2,
                    pixels: vec![(i % 255 + 1) as u8; 4],
                })
                .collect(),
        }
    }

    fn color_at(fb: &Framebuffer, x: i32, y: i32) -> u8 {
        fb.pixels[y as usize * fb.width + x as usize]
    }

    #[test]
    fn number_draws_units_first_with_leading_zeros() {
        let b = bar();
        let mut fb = Framebuffer::new(100, 60);
        draw_number(&mut fb, &b, 47, 19, 0, 0);
        assert_eq!(color_at(&fb, 0, 0), 20, "fundo: quadro 19");
        // glifos 40–49 de 2 px: unidades em x 32, dezenas em 29, centenas em 26
        assert_eq!(color_at(&fb, 32, 4), 48, "7 → quadro 47");
        assert_eq!(color_at(&fb, 29, 4), 45, "4 → quadro 44");
        assert_eq!(color_at(&fb, 26, 4), 41, "zero à esquerda → quadro 40");
        let mut fb = Framebuffer::new(100, 60);
        draw_number(&mut fb, &b, 1234, 50, 0, 0);
        assert_eq!(color_at(&fb, 14, 8), 82, "limitado a 99: quadro 81");
        assert_eq!(color_at(&fb, 11, 8), 82);
    }

    #[test]
    fn timer_rolls_units_and_carries_on_nines() {
        // 0 tiques: 0000 parado
        assert_eq!(timer_frames(0), [193; 4]);
        // 12 s + 19 tiques: r = 9 rola as unidades (2 → transição de 3)
        assert_eq!(
            timer_frames(12 * 20 + 19),
            [193 - 20 - 9, 193 - 10, 193, 193]
        );
        // 109 s + 10 tiques: unidade 9 com r = 5 → a dezena (0) também rola
        assert_eq!(
            timer_frames(109 * 20 + 10),
            [193 - 90 - 5, 193 - 5, 193 - 10, 193]
        );
        assert_eq!(timer_frames(-5), [193; 4]);
    }

    #[test]
    fn hud_places_panels() {
        let b = bar();
        let mut fb = Framebuffer::new(640, 75);
        let mut slots = [(0, 0); 30];
        slots[1] = (1, 20);
        slots[2] = (1, 12);
        let s = HudState {
            character: Character::Jake,
            health: 100,
            armor: 0,
            face_alt: false,
            slots,
            weapon: 2,
            timer: 0,
        };
        draw_hud(&mut fb, &b, 0, 0, 640, &s);
        assert_eq!(color_at(&fb, 6, 35), 16, "rosto: quadro 15");
        assert_eq!(color_at(&fb, 100, 32), 17, "armadura: quadro 16");
        assert_eq!(
            color_at(&fb, 640 - 93 + 68, 36),
            194,
            "cronômetro: 0 = quadro 193"
        );
        // pistola corrente: quadro 61 (60 + 1) presente em algum lugar da faixa das armas
        assert!(fb.pixels[30 * 640..32 * 640].contains(&62));
    }
}
