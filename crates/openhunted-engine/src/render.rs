// SPDX-License-Identifier: GPL-3.0-or-later
//! Framebuffer indexado e desenho por software.

use openhunted_formats::Frame;

/// Resolução da tela original.
pub const SCREEN_W: usize = 640;
pub const SCREEN_H: usize = 480;

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

    fn put(&mut self, x: i64, y: i64, color: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = color;
        }
    }

    /// Contorno de retângulo (w × h), recortado.
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
