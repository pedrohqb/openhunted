// SPDX-License-Identifier: GPL-3.0-or-later
//! Créditos (`0x43a18f(0)`, `docs/re/menu.md`): fundo `SCREDIT.ANM` (rótulo 1) e o texto do
//! `CREDITS.RES` em `credits.pft`. A rolagem do original não foi lida; decisão do port: as
//! linhas, centradas, sobem 2 px por tique a partir de baixo, e os créditos acabam quando a
//! última passa do topo (ou com uma tecla).

use std::collections::HashMap;
use std::rc::Rc;

use openhunted_formats::{Animation, Bitmap, Font, Result};

use super::screen::physical;
use crate::anim::draw_ops;
use crate::assets::Assets;
use crate::render::{Framebuffer, SCREEN_H, SCREEN_W, draw_text, measure_text};

/// Rótulo do fundo.
pub const BACKGROUND: u16 = 1;
/// Subida por tique.
pub const SPEED: i32 = 2;
/// Espaço entre linhas.
pub const LEADING: i32 = 4;

pub struct Credits {
    anim: Option<Rc<Animation>>,
    bitmaps: HashMap<String, Rc<Bitmap>>,
    font: Rc<Font>,
    lines: Vec<String>,
    /// Topo da primeira linha.
    y: i32,
    done: bool,
}

impl Credits {
    pub fn new(
        anim: Option<Rc<Animation>>,
        bitmaps: HashMap<String, Rc<Bitmap>>,
        font: Rc<Font>,
        lines: Vec<String>,
    ) -> Self {
        Self {
            anim,
            bitmaps,
            font,
            lines,
            y: SCREEN_H as i32,
            done: false,
        }
    }

    /// Carrega o fundo e a fonte dos recursos (sem o fundo, só o texto).
    pub fn load(assets: &mut Assets, lines: Vec<String>) -> Result<Self> {
        let font = assets.font("CREDITS.PFT")?;
        let anim = assets.animation("SCREDIT.ANM").ok();
        let mut bitmaps = HashMap::new();
        if let Some(a) = &anim {
            for (_, name) in &a.bitmaps {
                if let Ok(b) = assets.bitmap(name) {
                    bitmaps.insert(name.to_uppercase(), b);
                }
            }
        }
        Ok(Self::new(anim, bitmaps, font, lines))
    }

    fn line_h(&self) -> i32 {
        i32::from(self.font.height) + LEADING
    }

    pub fn tick(&mut self) {
        self.y -= SPEED;
        if self.y + self.line_h() * self.lines.len() as i32 <= 0 {
            self.done = true;
        }
    }

    pub fn skip(&mut self) {
        self.done = true;
    }

    pub fn done(&self) -> bool {
        self.done
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        fb.clear(0);
        if let Some(a) = &self.anim {
            // o rótulo conta quadros lógicos: a tabela A dá o físico (como `Screen`)
            let start = a
                .labels
                .iter()
                .find(|l| l.id == BACKGROUND)
                .map_or(0, |l| usize::from(l.frame));
            for op in physical(a, start)
                .map(|f| draw_ops(a, f))
                .unwrap_or_default()
            {
                if let Some(f) = self
                    .bitmaps
                    .get(&op.bitmap.to_uppercase())
                    .and_then(|b| b.frames.get(op.frame as usize))
                {
                    fb.blit(f, op.x, op.y, true);
                }
            }
        }
        let h = self.line_h();
        for (i, l) in self.lines.iter().enumerate() {
            let y = self.y + i as i32 * h;
            if y + h < 0 || y > SCREEN_H as i32 {
                continue;
            }
            let (w, _) = measure_text(&self.font, l);
            draw_text(fb, &self.font, (SCREEN_W as i32 - w) / 2, y, l, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openhunted_formats::test_support::mkpft;

    #[test]
    fn the_background_label_counts_logical_frames() {
        let (anim, bmps) = crate::shell::screen::tests::synth_anim();
        let font = Rc::new(Font::parse(&mkpft('a' as u32, &[(2, vec![7; 4])])).unwrap());
        let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
        let plain = Credits::new(Some(anim.clone()), bmps.clone(), font.clone(), vec![]);
        plain.draw(&mut fb);
        assert!(fb.pixels.contains(&1), "sem a tabela A, o quadro 0");
        // com a tabela A, o rótulo 1 (lógico 0) aponta o quadro físico 4 (cor 5)
        let mut a = (*anim).clone();
        a.table_a = vec![4; a.frames.len()];
        let c = Credits::new(Some(Rc::new(a)), bmps, font, vec![]);
        c.draw(&mut fb);
        assert!(fb.pixels.contains(&5));
        assert!(!fb.pixels.contains(&1));
    }

    #[test]
    fn scrolls_up_and_ends_after_the_last_line() {
        let font = Rc::new(Font::parse(&mkpft('a' as u32, &[(2, vec![7; 4])])).unwrap());
        let h = i32::from(font.height) + LEADING;
        let mut c = Credits::new(None, HashMap::new(), font, vec!["a".into(), "a".into()]);
        let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
        let ticks = (SCREEN_H as i32 + 2 * h) / SPEED;
        for _ in 0..ticks - 1 {
            c.tick();
        }
        assert!(!c.done());
        c.tick();
        assert!(c.done());
        c.draw(&mut fb);
        let mut c2 = Credits::new(None, HashMap::new(), c.font.clone(), vec!["a".into()]);
        for _ in 0..100 {
            c2.tick();
        }
        c2.draw(&mut fb);
        assert!(fb.pixels.contains(&7), "texto na tela");
        c2.skip();
        assert!(c2.done());
    }
}
