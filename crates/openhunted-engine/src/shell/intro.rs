// SPDX-License-Identifier: GPL-3.0-or-later
//! Abertura (`docs/re/casca-acabamento.md`): logotipo e título (`0x439bdd`) e as telas que
//! rolam texto, história e créditos (`0x43a18f`). O relógio é o da aplicação (ms); as
//! telas não tocam sons: dizem quais tocar.

use std::collections::HashMap;
use std::rc::Rc;

use openhunted_formats::{Animation, Bitmap, Font, Result};

use super::screen::{label_range, physical};
use crate::anim::draw_ops;
use crate::assets::Assets;
use crate::render::{Framebuffer, SCREEN_H, SCREEN_W, draw_text, measure_text};
use crate::rng::Rng;

/// Animação das telas da abertura e da história.
pub const SIERRA: &str = "SIERRA.ANM";
/// Logotipo: rótulos, paleta, teto, som e a espera depois do "tp" (`0x47d6cc`, `0x439fe5`).
pub mod logo {
    pub const BACKGROUND: u16 = 1;
    pub const TP: u16 = 3;
    pub const PALETTE: &str = "SIERRA.PAL";
    pub const MAX_MS: u32 = 30_000;
    pub const SOUND: u16 = 3403;
    pub const AFTER_TP_MS: u32 = 500;
    /// Sem interrupção: tela preta e a voz 4100 (`0x43a039`).
    pub const BLACK_MS: u32 = 1000;
    pub const VOICE: u16 = 4100;
}
/// Título (`0x47d6e4`).
pub mod title {
    pub const BACKGROUND: u16 = 4;
    pub const PALETTE: &str = "STITLE.PAL";
    pub const MS: u32 = 1500;
    pub const SOUNDS: [u16; 4] = [3081, 3045, 3220, 3086];
    /// Depois: a voz 4101 com o título na tela e 500 ms (`0x43a07e`).
    pub const VOICE: u16 = 4101;
    pub const AFTER_MS: u32 = 500;
}
/// Duração de um quadro da abertura (uma volta do laço do original = um tique do port).
pub const FRAME_MS: u32 = 50;

/// Desenha o quadro lógico `f` de `anim` em (0, 0).
fn draw_frame(
    fb: &mut Framebuffer,
    anim: &Animation,
    bitmaps: &HashMap<String, Rc<Bitmap>>,
    f: usize,
) {
    let Some(p) = physical(anim, f) else { return };
    for op in draw_ops(anim, p) {
        if let Some(fr) = bitmaps
            .get(&op.bitmap.to_uppercase())
            .and_then(|b| b.frames.get(op.frame as usize))
        {
            fb.blit(fr, op.x, op.y, true);
        }
    }
}

/// Bitmaps de uma animação pelo nome em maiúsculas.
type Bitmaps = HashMap<String, Rc<Bitmap>>;

fn load_anim(assets: &mut Assets, name: &str) -> Result<(Rc<Animation>, Bitmaps)> {
    let a = assets.animation(name)?;
    let mut bitmaps = HashMap::new();
    for (_, b) in &a.bitmaps {
        if let Ok(bmp) = assets.bitmap(b) {
            bitmaps.insert(b.to_uppercase(), bmp);
        }
    }
    Ok((a, bitmaps))
}

/// Logotipo ou título (`0x439bdd`).
#[derive(Debug, Clone)]
pub struct Splash {
    anim: Rc<Animation>,
    bitmaps: HashMap<String, Rc<Bitmap>>,
    background: (usize, usize),
    /// Quadros extras do fundo em laço a partir do 2º (`0x439eb9`).
    bg_extra: usize,
    tp: Option<(usize, usize)>,
    /// Quadros do "tp" já desenhados.
    tp_shown: usize,
    /// O "tp" só começa depois do som 3403.
    tp_wait: u32,
    elapsed: u32,
    acc: u32,
    end: u32,
    done: bool,
}

impl Splash {
    fn new(
        anim: Rc<Animation>,
        bitmaps: HashMap<String, Rc<Bitmap>>,
        label: u16,
        tp: Option<(u16, u32)>,
        end: u32,
    ) -> Self {
        let background = label_range(&anim, label).unwrap_or((0, 1));
        let tp_range = tp.and_then(|(l, _)| label_range(&anim, l));
        Self {
            anim,
            bitmaps,
            background,
            bg_extra: 0,
            tp: tp_range,
            tp_shown: 0,
            tp_wait: tp.map_or(0, |(_, w)| w),
            elapsed: 0,
            acc: 0,
            end,
            done: false,
        }
    }

    /// Logotipo: fundo e "tp" do `SIERRA.ANM`; `sound_ms` = duração do som 3403.
    pub fn logo(assets: &mut Assets, sound_ms: u32) -> Result<Self> {
        let (a, b) = load_anim(assets, SIERRA)?;
        Ok(Self::new(
            a,
            b,
            logo::BACKGROUND,
            Some((logo::TP, sound_ms)),
            logo::MAX_MS,
        ))
    }

    pub fn title(assets: &mut Assets) -> Result<Self> {
        let (a, b) = load_anim(assets, SIERRA)?;
        Ok(Self::new(a, b, title::BACKGROUND, None, title::MS))
    }

    /// Avança `ms` (em quadros de 50 ms).
    pub fn advance(&mut self, ms: u32) {
        self.acc = self.acc.saturating_add(ms);
        while self.acc >= FRAME_MS && !self.done {
            self.acc -= FRAME_MS;
            self.elapsed += FRAME_MS;
            self.frame();
        }
    }

    fn frame(&mut self) {
        if self.elapsed >= self.end {
            self.done = true;
            return;
        }
        if self.background.1 > 1 {
            self.bg_extra = 1 + self.bg_extra % (self.background.1 - 1);
        }
        if let Some((_, n)) = self.tp {
            if self.elapsed > self.tp_wait {
                self.tp_shown += 1;
                if self.tp_shown >= n {
                    self.end = self.end.min(self.elapsed + logo::AFTER_TP_MS);
                    self.tp_wait = u32::MAX;
                }
            }
        }
    }

    /// Quadros do "tp" mostrados (testes).
    pub fn tp_shown(&self) -> usize {
        self.tp_shown
    }

    pub fn done(&self) -> bool {
        self.done
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        fb.clear(0);
        let (s, _) = self.background;
        draw_frame(fb, &self.anim, &self.bitmaps, s + self.bg_extra);
        if let (Some((s, n)), true) = (self.tp, self.tp_shown > 0) {
            draw_frame(fb, &self.anim, &self.bitmaps, s + self.tp_shown.min(n) - 1);
        }
    }
}

/// História ou créditos (`0x43a18f`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roll {
    Story,
    Credits,
}

impl Roll {
    /// Animação, texto, rótulos de fundo e de animação, entrelinha e velocidade (px/s).
    pub fn params(self) -> (&'static str, &'static str, u16, u16, i32, u32) {
        match self {
            Roll::Story => (SIERRA, "STORY.RES", 2, 6, 3, 30),
            Roll::Credits => ("SCREDIT.ANM", "CREDITS.RES", 1, 2, 10, 120),
        }
    }
}

/// Cor do texto (`0x43a61e`) e margem direita da história.
pub const TEXT_COLOR: u8 = 255;
pub const STORY_MARGIN: i32 = 12;
/// Animação da história: começa aos 3 000 ms, 50 ms por quadro, pausa 5 000 + rand%1000.
pub const STORY_START_MS: u32 = 3000;
pub const STORY_FRAME_MS: u32 = 50;
pub const STORY_PAUSE_MS: u32 = 5000;
/// Animação dos créditos: 200 ms por quadro desde o início.
pub const CREDITS_FRAME_MS: u32 = 200;

#[derive(Debug, Clone)]
pub struct Scroller {
    pub roll: Roll,
    anim: Option<Rc<Animation>>,
    bitmaps: HashMap<String, Rc<Bitmap>>,
    font: Rc<Font>,
    /// (texto, x, y antes da rolagem).
    lines: Vec<(String, i32, i32)>,
    total: i32,
    background: Option<usize>,
    animation: Option<(usize, usize)>,
    frame: usize,
    next_ms: u32,
    elapsed: u32,
    rng: Rng,
    done: bool,
}

impl Scroller {
    pub fn new(
        roll: Roll,
        anim: Option<Rc<Animation>>,
        bitmaps: HashMap<String, Rc<Bitmap>>,
        font: Rc<Font>,
        text: &[String],
    ) -> Self {
        let (_, _, bg, al, gap, _) = roll.params();
        let h = i32::from(font.height);
        let mut y = SCREEN_H as i32;
        let mut lines = Vec::new();
        for t in text {
            let (w, _) = measure_text(&font, t);
            let x = match roll {
                Roll::Story => SCREEN_W as i32 - w - STORY_MARGIN,
                Roll::Credits => (SCREEN_W as i32 - w) / 2,
            };
            lines.push((t.clone(), x, y));
            y += h + gap;
        }
        let background = anim.as_ref().and_then(|a| label_range(a, bg)).map(|r| r.0);
        let animation = anim.as_ref().and_then(|a| label_range(a, al));
        Self {
            roll,
            anim,
            bitmaps,
            font,
            lines,
            total: y,
            background,
            animation,
            frame: 0,
            next_ms: match roll {
                Roll::Story => STORY_START_MS,
                Roll::Credits => 0,
            },
            elapsed: 0,
            rng: Rng::new(0x43a4a2),
            done: false,
        }
    }

    pub fn load(assets: &mut Assets, roll: Roll, text: &[String]) -> Result<Self> {
        let font = assets.font("CREDITS.PFT")?;
        let (a, b) = match load_anim(assets, roll.params().0) {
            Ok((a, b)) => (Some(a), b),
            Err(_) => (None, HashMap::new()),
        };
        Ok(Self::new(roll, a, b, font, text))
    }

    /// Subida em px.
    pub fn scroll(&self) -> i32 {
        (u64::from(self.elapsed) * u64::from(self.roll.params().5) / 1000) as i32
    }

    pub fn advance(&mut self, ms: u32) {
        if self.done {
            return;
        }
        let end = self.elapsed.saturating_add(ms);
        // a animação anda pelo relógio: um quadro por vencimento (`0x43a454`)
        while let Some((_, n)) = self.animation.filter(|_| self.next_ms < end) {
            let now = self.next_ms.max(self.elapsed);
            self.next_ms = now
                + match self.roll {
                    Roll::Story => STORY_FRAME_MS,
                    Roll::Credits => CREDITS_FRAME_MS,
                };
            self.frame += 1;
            if self.frame >= n {
                self.frame = 0;
                if self.roll == Roll::Story {
                    self.next_ms = now + STORY_PAUSE_MS + self.rng.below(1000);
                }
            }
        }
        self.elapsed = end;
        self.done = self.scroll() >= self.total;
    }

    pub fn skip(&mut self) {
        self.done = true;
    }

    pub fn done(&self) -> bool {
        self.done
    }

    /// Quadro da animação (testes).
    pub fn anim_frame(&self) -> usize {
        self.frame
    }

    /// Linhas com a posição na tela (texto, x, y).
    pub fn visible(&self) -> Vec<(&str, i32, i32)> {
        let h = i32::from(self.font.height);
        let s = self.scroll();
        self.lines
            .iter()
            .map(|(t, x, y)| (t.as_str(), *x, y - s))
            .filter(|&(_, _, y)| y > -h && y < SCREEN_H as i32)
            .collect()
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        fb.clear(0);
        if let Some(a) = &self.anim {
            if let Some(b) = self.background {
                draw_frame(fb, a, &self.bitmaps, b);
            }
            if let Some((s, _)) = self.animation {
                draw_frame(fb, a, &self.bitmaps, s + self.frame);
            }
        }
        for (t, x, y) in self.visible() {
            draw_text(fb, &self.font, x, y, t, Some(TEXT_COLOR));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openhunted_formats::test_support::mkpft;

    fn font() -> Rc<Font> {
        Rc::new(Font::parse(&mkpft('a' as u32, &[(2, vec![7; 4])])).unwrap())
    }

    #[test]
    fn credits_scroll_120_px_per_second_and_end_after_the_last_line() {
        let f = font();
        let h = i32::from(f.height);
        let lines = vec!["a".to_string(); 3];
        let mut c = Scroller::new(Roll::Credits, None, HashMap::new(), f.clone(), &lines);
        c.advance(1000);
        assert_eq!(c.scroll(), 120);
        let total = SCREEN_H as i32 + 3 * (h + 10);
        let ms = (total as u32 * 1000).div_ceil(120);
        c.advance(ms - 1000 - 10);
        assert!(!c.done());
        c.advance(10);
        assert!(c.done());
        // centrada; a história alinha à direita com 12 px
        let c = Scroller::new(Roll::Credits, None, HashMap::new(), f.clone(), &lines);
        let (w, _) = measure_text(&f, "a");
        assert_eq!(c.lines[0].1, (640 - w) / 2);
        let s = Scroller::new(Roll::Story, None, HashMap::new(), f.clone(), &lines);
        assert_eq!(s.lines[0].1, 640 - w - 12);
        assert_eq!(s.lines[1].2 - s.lines[0].2, h + 3);
        let mut s = s;
        s.advance(1000);
        assert_eq!(s.scroll(), 30);
        let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
        s.advance(1000);
        s.draw(&mut fb);
        assert!(fb.pixels.contains(&TEXT_COLOR), "texto na cor 255");
    }

    #[test]
    fn real_intro_screens_follow_the_original_timing() {
        let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
            eprintln!("pulado: OPENHUNTED_DATA não definido");
            return;
        };
        let gd = openhunted_formats::open_data(std::path::Path::new(&p)).unwrap();
        let mut assets = Assets::new(gd.resources().unwrap());
        // logotipo: o "tp" espera o som (1 s aqui), um quadro a cada 50 ms, fim 500 ms depois
        let mut l = Splash::logo(&mut assets, 1000).unwrap();
        let n = l.tp.unwrap().1;
        assert_eq!(n, 193, "rótulo 3 do SIERRA.ANM");
        l.advance(1000);
        assert_eq!(l.tp_shown(), 0);
        l.advance(50);
        assert_eq!(l.tp_shown(), 1);
        l.advance(50 * (n as u32 - 1));
        assert_eq!(l.tp_shown(), n);
        assert!(!l.done());
        l.advance(500);
        assert!(l.done(), "500 ms depois do último quadro");
        let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
        l.draw(&mut fb);
        assert!(fb.pixels.iter().any(|&p| p != 0));
        let mut t = Splash::title(&mut assets).unwrap();
        t.advance(1450);
        assert!(!t.done());
        t.advance(50);
        assert!(t.done(), "1 500 ms");
        // história: texto do STORY.RES, animação aos 3 s
        let text = openhunted_formats::parse_text_list(assets.archive().get("STORY.RES").unwrap());
        let mut s = Scroller::load(&mut assets, Roll::Story, &text).unwrap();
        assert!(s.lines[0].0.starts_with("\"The world will not end"));
        s.advance(2950);
        assert_eq!(s.anim_frame(), 0);
        s.advance(100);
        assert!(s.anim_frame() > 0, "começa aos 3 000 ms");
        s.draw(&mut fb);
    }
}
