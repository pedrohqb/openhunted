// SPDX-License-Identifier: GPL-3.0-or-later
//! Framebuffer indexado e desenho por software.

use openhunted_formats::Frame;

use crate::physics::Rect;

/// Resolução da tela original.
pub const SCREEN_W: usize = 640;
pub const SCREEN_H: usize = 480;

/// Modo de escrita dos pixels (`docs/re/camadas-desenho.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// Escreve tudo, inclusive o índice 0 (cenário da camada da vista).
    Opaque,
    /// Pula o índice 0 (objetos, decalques).
    Transparent,
    /// Modo `0x80`: só escreve onde o destino é 0 e a origem não é (camadas de trás).
    Behind,
}

/// Recorte (x0, y0, x1, y1) de `r` nos limites do quadro.
fn clip_to(r: Rect, width: usize, height: usize) -> (i64, i64, i64, i64) {
    let (w, h) = (width as i64, height as i64);
    (
        i64::from(r.x).clamp(0, w),
        i64::from(r.y).clamp(0, h),
        (i64::from(r.x) + i64::from(r.w)).clamp(0, w),
        (i64::from(r.y) + i64::from(r.h)).clamp(0, h),
    )
}

/// Framebuffer de 8 bits (índices de paleta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
        }
    }

    pub fn clear(&mut self, color: u8) {
        self.pixels.fill(color);
    }

    /// Copia `frame` com o canto superior esquerdo em (x, y), recortando nas bordas; com
    /// `transparent`, o índice 0 não é copiado.
    pub fn blit(&mut self, frame: &Frame, x: i32, y: i32, transparent: bool) {
        let (fw, fh) = (frame.width as i64, frame.height as i64);
        let (x, y) = (i64::from(x), i64::from(y));
        let (w, h) = (self.width as i64, self.height as i64);
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + fw).min(w), (y + fh).min(h));
        if x0 >= x1 || y0 >= y1 || frame.pixels.len() < (fw * fh) as usize {
            return;
        }
        for dy in y0..y1 {
            let src = ((dy - y) * fw + (x0 - x)) as usize;
            let dst = (dy * w + x0) as usize;
            let n = (x1 - x0) as usize;
            let (s, d) = (&frame.pixels[src..src + n], &mut self.pixels[dst..dst + n]);
            if transparent {
                for (o, &p) in d.iter_mut().zip(s) {
                    if p != 0 {
                        *o = p;
                    }
                }
            } else {
                d.copy_from_slice(s);
            }
        }
    }

    /// Como `blit`, com escala `num/den` (vizinho mais próximo) e espelhamento horizontal.
    /// Escala inválida (zero ou negativa) vale 1; o tamanho de destino é limitado a 4096.
    pub fn blit_scaled(
        &mut self,
        frame: &Frame,
        x: i32,
        y: i32,
        scale: (i32, i32),
        mirror: bool,
        transparent: bool,
    ) {
        let (num, den) = if scale.0 > 0 && scale.1 > 0 {
            (i64::from(scale.0), i64::from(scale.1))
        } else {
            (1, 1)
        };
        if num == den && !mirror {
            return self.blit(frame, x, y, transparent);
        }
        let (fw, fh) = (i64::from(frame.width), i64::from(frame.height));
        if frame.pixels.len() < (fw * fh) as usize || fw == 0 || fh == 0 {
            return;
        }
        let dw = (fw * num / den).clamp(1, 4096);
        let dh = (fh * num / den).clamp(1, 4096);
        let (x, y) = (i64::from(x), i64::from(y));
        let (w, h) = (self.width as i64, self.height as i64);
        for dy in y.max(0)..(y + dh).min(h) {
            let sy = ((dy - y) * den / num).min(fh - 1);
            for dx in x.max(0)..(x + dw).min(w) {
                let mut sx = ((dx - x) * den / num).min(fw - 1);
                if mirror {
                    sx = fw - 1 - sx;
                }
                let p = frame.pixels[(sy * fw + sx) as usize];
                if !transparent || p != 0 {
                    self.pixels[(dy * w + dx) as usize] = p;
                }
            }
        }
    }

    /// Como `blit_scaled`, passando cada pixel pela tabela `map` (`0x481048`); é
    /// transparente o pixel cujo valor **remapeado** é 0 (`0x44f641`).
    pub fn blit_scaled_mapped(
        &mut self,
        frame: &Frame,
        x: i32,
        y: i32,
        scale: (i32, i32),
        mirror: bool,
        map: &[u8; 256],
    ) {
        let (num, den) = if scale.0 > 0 && scale.1 > 0 {
            (i64::from(scale.0), i64::from(scale.1))
        } else {
            (1, 1)
        };
        let (fw, fh) = (i64::from(frame.width), i64::from(frame.height));
        if frame.pixels.len() < (fw * fh) as usize || fw == 0 || fh == 0 {
            return;
        }
        let dw = (fw * num / den).clamp(1, 4096);
        let dh = (fh * num / den).clamp(1, 4096);
        let (x, y) = (i64::from(x), i64::from(y));
        let (w, h) = (self.width as i64, self.height as i64);
        for dy in y.max(0)..(y + dh).min(h) {
            let sy = ((dy - y) * den / num).min(fh - 1);
            for dx in x.max(0)..(x + dw).min(w) {
                let mut sx = ((dx - x) * den / num).min(fw - 1);
                if mirror {
                    sx = fw - 1 - sx;
                }
                let p = map[usize::from(frame.pixels[(sy * fw + sx) as usize])];
                if p != 0 {
                    self.pixels[(dy * w + dx) as usize] = p;
                }
            }
        }
    }

    /// Estica `frame` ao retângulo `dst` (vizinho mais próximo), recortado por `clip`, com o
    /// modo de `blend`; com `map`, cada pixel passa pela tabela antes do teste de
    /// transparência (`0x44f641`) e do modo atrás (`0x44f6d4`).
    pub fn blit_view(
        &mut self,
        frame: &Frame,
        dst: Rect,
        mirror: bool,
        map: Option<&[u8; 256]>,
        blend: Blend,
        clip: Rect,
    ) {
        let ratio = (
            (i64::from(dst.w), i64::from(frame.width)),
            (i64::from(dst.h), i64::from(frame.height)),
        );
        self.blit_ratio(frame, dst, ratio, mirror, map, blend, clip);
    }

    /// Como `blit_view`, amostrando pela escala `num/den` de cada eixo (`((num, den), (num,
    /// den))`): o pixel `d` do destino vem de `d·den/num` (como `blit_scaled`).
    #[allow(clippy::too_many_arguments)]
    pub fn blit_ratio(
        &mut self,
        frame: &Frame,
        dst: Rect,
        ratio: ((i64, i64), (i64, i64)),
        mirror: bool,
        map: Option<&[u8; 256]>,
        blend: Blend,
        clip: Rect,
    ) {
        let (fw, fh) = (i64::from(frame.width), i64::from(frame.height));
        let ((nx, dxr), (ny, dyr)) = ratio;
        if frame.pixels.len() < (fw * fh) as usize
            || fw == 0
            || fh == 0
            || dst.w <= 0
            || dst.h <= 0
            || nx <= 0
            || ny <= 0
        {
            return;
        }
        let (dw, dh) = (i64::from(dst.w), i64::from(dst.h));
        let (x, y) = (i64::from(dst.x), i64::from(dst.y));
        let c = clip_to(clip, self.width, self.height);
        let (x0, x1) = (x.max(c.0), (x + dw).min(c.2));
        let (y0, y1) = (y.max(c.1), (y + dh).min(c.3));
        let w = self.width as i64;
        for dy in y0..y1 {
            let sy = ((dy - y) * dyr / ny).clamp(0, fh - 1);
            for dx in x0..x1 {
                let mut sx = ((dx - x) * dxr / nx).clamp(0, fw - 1);
                if mirror {
                    sx = fw - 1 - sx;
                }
                let src = frame.pixels[(sy * fw + sx) as usize];
                let p = map.map_or(src, |m| m[usize::from(src)]);
                let o = &mut self.pixels[(dy * w + dx) as usize];
                match blend {
                    Blend::Opaque => *o = p,
                    Blend::Transparent if p != 0 => *o = p,
                    Blend::Behind if p != 0 && *o == 0 => *o = p,
                    _ => {}
                }
            }
        }
    }

    /// Ponto na cor `color`, recortado por `clip`, com o modo de `blend`.
    pub fn plot(&mut self, x: i32, y: i32, color: u8, blend: Blend, clip: Rect) {
        let c = clip_to(clip, self.width, self.height);
        let (x, y) = (i64::from(x), i64::from(y));
        if x < c.0 || x >= c.2 || y < c.1 || y >= c.3 {
            return;
        }
        let o = &mut self.pixels[(y * self.width as i64 + x) as usize];
        if blend != Blend::Behind || *o == 0 {
            *o = color;
        }
    }

    /// Preenche `r ∩ clip`.
    pub fn fill_clipped(&mut self, r: Rect, color: u8, clip: Rect) {
        let c = clip_to(clip, self.width, self.height);
        let (x0, y0) = (i64::from(r.x).max(c.0), i64::from(r.y).max(c.1));
        let x1 = (i64::from(r.x) + i64::from(r.w)).min(c.2);
        let y1 = (i64::from(r.y) + i64::from(r.h)).min(c.3);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for row in y0..y1 {
            let base = (row * self.width as i64) as usize;
            self.pixels[base + x0 as usize..base + x1 as usize].fill(color);
        }
    }

    fn put(&mut self, x: i64, y: i64, color: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }

    /// Contorno de retângulo (w × h), recortado.
    /// Preenche o retângulo (recortado nas bordas).
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        if w <= 0 || h <= 0 {
            return;
        }
        let x0 = x.clamp(0, self.width as i32) as usize;
        let y0 = y.clamp(0, self.height as i32) as usize;
        let x1 = (x.saturating_add(w)).clamp(0, self.width as i32) as usize;
        let y1 = (y.saturating_add(h)).clamp(0, self.height as i32) as usize;
        for row in y0..y1 {
            self.pixels[row * self.width + x0..row * self.width + x1].fill(color);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        if w <= 0 || h <= 0 {
            return;
        }
        let (x, y, w, h) = (i64::from(x), i64::from(y), i64::from(w), i64::from(h));
        let span_x = (x.max(-1))..=((x + w - 1).min(self.width as i64));
        for i in span_x {
            self.put(i, y, color);
            self.put(i, y + h - 1, color);
        }
        let span_y = (y.max(-1))..=((y + h - 1).min(self.height as i64));
        for j in span_y {
            self.put(x, j, color);
            self.put(x + w - 1, j, color);
        }
    }
}

/// Largura e altura de `text` na fonte: soma das larguras dos glifos com 1 px entre eles;
/// caracteres sem glifo valem a largura do espaço (ou 4 px).
pub fn measure_text(font: &openhunted_formats::Font, text: &str) -> (i32, i32) {
    let mut w = 0;
    for (i, c) in text.chars().enumerate() {
        if i > 0 {
            w += 1;
        }
        w += glyph_width(font, c);
    }
    (w, i32::from(font.height))
}

fn glyph_width(font: &openhunted_formats::Font, c: char) -> i32 {
    let glyph = |ch: char| {
        Some(u32::from(ch))
            .filter(|&n| n < 256)
            .and_then(|n| font.glyphs.get(n as usize))
            .and_then(|g| g.as_ref())
    };
    glyph(c)
        .or_else(|| glyph(' '))
        .map_or(4, |g| i32::from(g.width))
}

/// Desenha `text` com o canto superior esquerdo em (x, y); o índice 0 dos glifos é
/// transparente e `color`, se dado, recolore os demais.
pub fn draw_text(
    fb: &mut Framebuffer,
    font: &openhunted_formats::Font,
    x: i32,
    y: i32,
    text: &str,
    color: Option<u8>,
) {
    let mut cx = x;
    for c in text.chars() {
        let n = Some(u32::from(c)).filter(|&n| n < 256).map(|n| n as usize);
        if let Some(g) = n.and_then(|n| font.glyphs.get(n)).and_then(|g| g.as_ref()) {
            let mut pixels = g.pixels.clone();
            if let Some(col) = color {
                for p in pixels.iter_mut().filter(|p| **p != 0) {
                    *p = col;
                }
            }
            let frame = Frame {
                width: g.width,
                height: font.height,
                pixels,
            };
            fb.blit(&frame, cx, y, true);
        }
        cx += glyph_width(font, c) + 1;
    }
}

/// Quebra `text` em linhas de até `width` px, por palavras; `\n` força a quebra (linhas
/// vazias ficam). Uma palavra mais larga que `width` ocupa a linha sozinha.
pub fn wrap_text(font: &openhunted_formats::Font, text: &str, width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if line.is_empty() || measure_text(font, &candidate).0 <= width {
                line = candidate;
            } else {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            }
        }
        lines.push(line);
    }
    lines
}

/// As 20 cores estáticas do Windows de 256 cores (índices 0–9 e 246–255).
const WINDOWS_STATIC: [(u8, [u8; 3]); 20] = [
    (0, [0, 0, 0]),
    (1, [128, 0, 0]),
    (2, [0, 128, 0]),
    (3, [128, 128, 0]),
    (4, [0, 0, 128]),
    (5, [128, 0, 128]),
    (6, [0, 128, 128]),
    (7, [192, 192, 192]),
    (8, [192, 220, 192]),
    (9, [166, 202, 240]),
    (246, [255, 251, 240]),
    (247, [160, 160, 164]),
    (248, [128, 128, 128]),
    (249, [255, 0, 0]),
    (250, [0, 255, 0]),
    (251, [255, 255, 0]),
    (252, [0, 0, 255]),
    (253, [255, 0, 255]),
    (254, [0, 255, 255]),
    (255, [255, 255, 255]),
];

/// Paleta como o original a mostra (`0x004518cf`): a paleta lógica do Windows recebe só os
/// índices 10–245 do arquivo; 0–9 e 246–255 são as cores estáticas do sistema (o magenta de
/// marcador da `WORLD.PAL` nesses índices nunca aparece na tela).
pub fn display_palette(pal: &openhunted_formats::Palette) -> openhunted_formats::Palette {
    let mut out = pal.0;
    for (i, c) in WINDOWS_STATIC {
        out[i as usize] = c;
    }
    openhunted_formats::Palette(out)
}

/// Índice da cor mais escura da paleta, usado para limpar a tela fora da fase (o índice 0
/// das paletas do jogo é a cor de transparência, magenta na `WORLD.PAL`).
pub fn background_index(pal: &openhunted_formats::Palette) -> u8 {
    pal.0
        .iter()
        .enumerate()
        .min_by_key(|(_, c)| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]))
        .map_or(0, |(i, _)| i as u8)
}

/// Mantém a câmera dentro da cena (`scene` = largura e altura; `view` = tamanho da janela).
pub fn clamp_camera(scene: (i32, i32), view: (i32, i32), cam: (i32, i32)) -> (i32, i32) {
    let max_x = (scene.0 - view.0).max(0);
    let max_y = (scene.1 - view.1).max(0);
    (cam.0.clamp(0, max_x), cam.1.clamp(0, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_rect_outside_the_screen_is_a_no_op() {
        let mut fb = Framebuffer::new(10, 4);
        fb.fill_rect(20, 1, 5, 2, 7);
        fb.fill_rect(-5, -5, 3, 3, 7);
        fb.fill_rect(8, 3, 50, 50, 7);
        assert_eq!(fb.pixels.iter().filter(|&&p| p == 7).count(), 2);
    }

    #[test]
    fn draw_text_blits_glyphs() {
        use openhunted_formats::test_support::mkpft;
        // 'A' = 2×2 cheio de 5, 'B' = 3×2 cheio de 6 (altura 2 em mkpft)
        let font = openhunted_formats::Font::parse(&mkpft(
            'A' as u32,
            &[(2, vec![5; 4]), (3, vec![6; 6])],
        ))
        .unwrap();
        assert_eq!(measure_text(&font, "AB"), (6, 2));
        let mut fb = Framebuffer::new(10, 4);
        draw_text(&mut fb, &font, 1, 1, "AB", Some(9));
        assert_eq!(fb.pixels[10 + 1], 9); // A em x=1..3
        assert_eq!(fb.pixels[10 + 3], 0); // 1 px de espaço
        assert_eq!(fb.pixels[10 + 4], 9); // B em x=4..7
        assert_eq!(fb.pixels[2 * 10 + 6], 9);
        assert_eq!(fb.pixels[10 + 7], 0);
    }

    #[test]
    fn display_palette_replaces_system_ranges_only() {
        let mut raw = [[255u8, 0, 191]; 256];
        raw[10] = [1, 2, 3];
        raw[245] = [4, 5, 6];
        let p = display_palette(&openhunted_formats::Palette(raw));
        assert_eq!(p.0[0], [0, 0, 0]);
        assert_eq!(p.0[9], [166, 202, 240]);
        assert_eq!(p.0[246], [255, 251, 240]);
        assert_eq!(p.0[255], [255, 255, 255]);
        assert_eq!((p.0[10], p.0[245]), ([1, 2, 3], [4, 5, 6]));
    }
    use openhunted_formats::Frame;

    fn sprite() -> Frame {
        // 3×2: o índice 0 é transparente
        Frame {
            width: 3,
            height: 2,
            pixels: vec![0, 5, 6, 7, 0, 8],
        }
    }

    #[test]
    fn view_blit_modes_and_clip() {
        let all = Rect {
            x: 0,
            y: 0,
            w: 4,
            h: 2,
        };
        let at = Rect {
            x: 0,
            y: 0,
            w: 3,
            h: 2,
        };
        // opaco escreve o 0; transparente o pula
        let mut fb = Framebuffer::new(4, 2);
        fb.clear(9);
        fb.blit_view(&sprite(), at, false, None, Blend::Opaque, all);
        assert_eq!(fb.pixels, vec![0, 5, 6, 9, 7, 0, 8, 9]);
        // atrás: só onde o destino é 0, e a origem (remapeada) não é 0
        let mut map: [u8; 256] = std::array::from_fn(|i| (i as u8).wrapping_add(100));
        map[0] = 0;
        let mut back = Frame {
            width: 3,
            height: 2,
            pixels: vec![1, 1, 1, 0, 2, 2],
        };
        fb.blit_view(&back, at, false, Some(&map), Blend::Behind, all);
        assert_eq!(fb.pixels, vec![101, 5, 6, 9, 7, 102, 8, 9]);
        // recorte: só a coluna 1
        back.pixels = vec![3; 6];
        let mut fb = Framebuffer::new(4, 2);
        fb.blit_view(
            &back,
            at,
            false,
            None,
            Blend::Transparent,
            Rect {
                x: 1,
                y: 0,
                w: 1,
                h: 9,
            },
        );
        assert_eq!(fb.pixels, vec![0, 3, 0, 0, 0, 3, 0, 0]);
        // esticado ao dobro, espelhado
        let mut fb = Framebuffer::new(6, 4);
        fb.blit_view(
            &sprite(),
            Rect {
                x: 0,
                y: 0,
                w: 6,
                h: 4,
            },
            true,
            None,
            Blend::Opaque,
            Rect {
                x: 0,
                y: 0,
                w: 6,
                h: 4,
            },
        );
        assert_eq!(&fb.pixels[..6], &[6, 6, 5, 5, 0, 0]);
        fb.fill_clipped(
            Rect {
                x: -5,
                y: 3,
                w: 100,
                h: 9,
            },
            4,
            Rect {
                x: 2,
                y: 0,
                w: 2,
                h: 9,
            },
        );
        assert_eq!(&fb.pixels[18..], &[8, 8, 4, 4, 7, 7]);
        fb.plot(0, 0, 1, Blend::Behind, all);
        fb.plot(
            4,
            0,
            1,
            Blend::Behind,
            Rect {
                x: 0,
                y: 0,
                w: 6,
                h: 4,
            },
        );
        assert_eq!(&fb.pixels[..6], &[6, 6, 5, 5, 1, 0]);
    }

    #[test]
    fn blit_respects_transparency() {
        let mut fb = Framebuffer::new(4, 3);
        fb.clear(9);
        fb.blit(&sprite(), 1, 1, true);
        assert_eq!(fb.pixels, vec![9, 9, 9, 9, 9, 9, 5, 6, 9, 7, 9, 8]);
        fb.blit(&sprite(), 0, 0, false);
        assert_eq!(&fb.pixels[..3], &[0, 5, 6]);
    }

    #[test]
    fn blit_clips_on_every_edge() {
        for (x, y) in [
            (-2, 0),
            (0, -1),
            (3, 0),
            (0, 2),
            (-3, -2),
            (4, 3),
            (-100, 50),
            (i32::MIN, i32::MAX),
        ] {
            let mut fb = Framebuffer::new(4, 3);
            fb.blit(&sprite(), x, y, true);
        }
        let mut fb = Framebuffer::new(4, 3);
        fb.blit(&sprite(), -2, -1, false);
        assert_eq!(fb.pixels[0], 8);
    }

    #[test]
    fn rect_outline_clips() {
        let mut fb = Framebuffer::new(4, 3);
        fb.rect(-1, -1, 3, 3, 1);
        assert_eq!(fb.pixels, vec![0, 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0]);
        fb.rect(2, 1, 100, 100, 2);
    }

    #[test]
    fn scaled_blit_halves_and_doubles() {
        // 4×2 → metade (2×1): pega os pixels 0 e 2 da primeira linha
        let f = Frame {
            width: 4,
            height: 2,
            pixels: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let mut fb = Framebuffer::new(4, 2);
        fb.blit_scaled(&f, 0, 0, (1, 2), false, false);
        assert_eq!(&fb.pixels[..2], &[1, 3]);
        assert_eq!(fb.pixels[4], 0);
        let g = Frame {
            width: 2,
            height: 1,
            pixels: vec![1, 2],
        };
        let mut fb = Framebuffer::new(4, 2);
        fb.blit_scaled(&g, 0, 0, (2, 1), false, false);
        assert_eq!(fb.pixels, vec![1, 1, 2, 2, 1, 1, 2, 2]);
    }

    #[test]
    fn mirrored_blit_flips_horizontally() {
        let g = Frame {
            width: 3,
            height: 1,
            pixels: vec![1, 0, 3],
        };
        let mut fb = Framebuffer::new(3, 1);
        fb.clear(9);
        fb.blit_scaled(&g, 0, 0, (1, 1), true, true);
        assert_eq!(fb.pixels, vec![3, 9, 1]);
    }

    #[test]
    fn scaled_blit_survives_extremes() {
        let g = Frame {
            width: 3,
            height: 2,
            pixels: vec![1; 6],
        };
        let mut fb = Framebuffer::new(4, 3);
        for (x, y, sc) in [
            (i32::MIN, 0, (1, 1)),
            (0, i32::MAX, (3, 1)),
            (-2, -1, (0, 1)),
            (1, 1, (1, 0)),
            (0, 0, (1000, 1)),
        ] {
            fb.blit_scaled(&g, x, y, sc, true, true);
        }
    }

    #[test]
    fn mapped_blit_remaps_and_skips_mapped_zero() {
        // 7 → 0 (transparente depois do remapeamento), demais somam 100
        let mut map: [u8; 256] = std::array::from_fn(|i| (i as u8).wrapping_add(100));
        map[0] = 0;
        map[7] = 0;
        let mut fb = Framebuffer::new(4, 3);
        fb.clear(9);
        fb.blit_scaled_mapped(&sprite(), 1, 1, (1, 1), false, &map);
        assert_eq!(fb.pixels, vec![9, 9, 9, 9, 9, 9, 105, 106, 9, 9, 9, 108]);
        let mut fb = Framebuffer::new(3, 1);
        let g = Frame {
            width: 3,
            height: 1,
            pixels: vec![1, 0, 3],
        };
        fb.blit_scaled_mapped(&g, 0, 0, (1, 1), true, &map);
        assert_eq!(fb.pixels, vec![103, 0, 101]);
        let mut fb = Framebuffer::new(4, 1);
        fb.blit_scaled_mapped(&g, 0, 0, (2, 1), false, &map);
        assert_eq!(fb.pixels, vec![101, 101, 0, 0]);
        for (x, y, sc) in [
            (i32::MIN, 0, (1, 1)),
            (0, i32::MAX, (3, 1)),
            (-2, -1, (0, 1)),
        ] {
            fb.blit_scaled_mapped(&g, x, y, sc, true, &map);
        }
    }

    #[test]
    fn background_is_darkest_color() {
        let mut pal = [[200u8, 200, 200]; 256];
        pal[0] = [255, 0, 255];
        pal[37] = [2, 1, 0];
        assert_eq!(background_index(&openhunted_formats::Palette(pal)), 37);
    }

    #[test]
    fn camera_clamps_to_scene() {
        assert_eq!(
            clamp_camera((1792, 128), (640, 480), (5000, -30)),
            (1152, 0)
        );
        assert_eq!(clamp_camera((1792, 128), (640, 480), (-10, 400)), (0, 0));
        assert_eq!(clamp_camera((300, 200), (640, 480), (50, 50)), (0, 0));
    }
}
