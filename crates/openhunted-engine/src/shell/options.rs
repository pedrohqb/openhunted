// SPDX-License-Identifier: GPL-3.0-or-later
//! Tela de opções (`SOPTI.ITF`, `0x43dfdc` + `0x43e26a`; `docs/re/opcoes-salvamento.md`):
//! dificuldade e brilho em grupos de opção, volumes da música e dos efeitos em barras de
//! 0–100 com setas de ±10, DONE. Cada mudança volta à aplicação na hora (grava o
//! `HUNTER.INI`).

use super::config::{BRIGHTNESS_MIN, Config};
use super::page::{Page, Reply};
use super::screen::{CANCEL, Input, Screen, flags};
use crate::render::Framebuffer;

pub const EASY: u16 = 12001;
pub const MEDIUM: u16 = 12002;
pub const HARD: u16 = 12003;
pub const LEVEL1: u16 = 12004;
pub const LEVEL5: u16 = 12008;
pub const SFX_BAR: u16 = 12018;
pub const MUSIC_BAR: u16 = 12020;
pub const SFX_DOWN: u16 = 12021;
pub const SFX_UP: u16 = 12022;
pub const MUSIC_DOWN: u16 = 12024;
pub const MUSIC_UP: u16 = 12025;
pub const DONE: u16 = 12026;
/// Alças das barras (rótulos "slider1" e "slider2" do `Sopti.anm`).
const MUSIC_KNOB: u16 = 5;
const SFX_KNOB: u16 = 19;
/// Passo das setas (`0x43deb5(seta, barra, ±10)`).
pub const STEP: i32 = 10;

/// Posição x da alça (relativa à barra) para o valor `v` (0–100).
pub fn knob_x(v: i32, bar_w: i32, knob_w: i32) -> i32 {
    v.clamp(0, 100) * (bar_w - knob_w).max(0) / 100
}

/// Valor pela posição `x` (relativa à barra) de um clique, arredondado ao passo.
pub fn value_at(x: i32, bar_w: i32, knob_w: i32) -> i32 {
    let span = (bar_w - knob_w).max(1);
    let v = (x - knob_w / 2) * 100 / span;
    ((v + STEP / 2).div_euclid(STEP) * STEP).clamp(0, 100)
}

/// A tela aberta.
#[derive(Debug, Clone)]
pub struct Options {
    pub screen: Screen,
    pub config: Config,
}

impl Options {
    pub fn new(mut screen: Screen, config: Config) -> Self {
        for id in EASY..=LEVEL5 {
            screen.make_option(id);
        }
        for id in [MUSIC_BAR, SFX_BAR] {
            screen.make_option(id);
            screen.clear_flags(id, flags::NAV);
        }
        for id in [MUSIC_DOWN, MUSIC_UP, SFX_DOWN, SFX_UP] {
            screen.make_button(id);
            screen.set_flags(id, flags::NAV);
        }
        screen.set_flags(DONE, flags::UPDATE | flags::NAV);
        screen.point_at(DONE);
        let mut o = Self { screen, config };
        o.mark();
        o
    }

    fn mark(&mut self) {
        let d = self.config.difficulty;
        for (i, id) in (EASY..=HARD).enumerate() {
            self.screen.set_marked(id, i as i32 == d);
        }
        let b = self.config.brightness - BRIGHTNESS_MIN;
        for (i, id) in (LEVEL1..=LEVEL5).enumerate() {
            self.screen.set_marked(id, i as i32 == b);
        }
    }

    fn knob_w(&self, label: u16) -> i32 {
        self.screen.label_size(label).map_or(14, |s| s.0)
    }

    fn bar_click(&self, bar: u16, knob: u16) -> Option<i32> {
        let r = self.screen.rect(bar)?;
        Some(value_at(
            self.screen.pointer().0 - r.x,
            r.w,
            self.knob_w(knob),
        ))
    }
}

impl Page for Options {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        let id = self.screen.handle(input)?;
        let music_at = self.bar_click(MUSIC_BAR, MUSIC_KNOB);
        let sfx_at = self.bar_click(SFX_BAR, SFX_KNOB);
        let c = &mut self.config;
        match id {
            DONE | CANCEL => return Some(Reply::Close),
            EASY..=HARD => c.difficulty = i32::from(id - EASY),
            LEVEL1..=LEVEL5 => c.brightness = BRIGHTNESS_MIN + i32::from(id - LEVEL1),
            MUSIC_DOWN => c.music_volume = (c.music_volume - STEP).max(0),
            MUSIC_UP => c.music_volume = (c.music_volume + STEP).min(100),
            SFX_DOWN => c.sfx_volume = (c.sfx_volume - STEP).max(0),
            SFX_UP => c.sfx_volume = (c.sfx_volume + STEP).min(100),
            MUSIC_BAR => c.music_volume = music_at?,
            SFX_BAR => c.sfx_volume = sfx_at?,
            _ => return None,
        }
        self.mark();
        Some(Reply::Config(self.config.clone()))
    }

    fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
        for (bar, knob, v) in [
            (MUSIC_BAR, MUSIC_KNOB, self.config.music_volume),
            (SFX_BAR, SFX_KNOB, self.config.sfx_volume),
        ] {
            let (Some(r), Some(f)) = (self.screen.rect(bar), self.screen.label_frame(knob)) else {
                continue;
            };
            let x = r.x + knob_x(v, r.w, self.knob_w(knob));
            let (ox, oy) = self.screen.origin();
            self.screen.draw_anim_frame(fb, f, x - ox, r.y - oy);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knob_and_click_positions() {
        assert_eq!(knob_x(0, 258, 14), 0);
        assert_eq!(knob_x(100, 258, 14), 244);
        assert_eq!(knob_x(50, 258, 14), 122);
        assert_eq!(value_at(7 + 122, 258, 14), 50);
        assert_eq!(value_at(0, 258, 14), 0);
        assert_eq!(value_at(300, 258, 14), 100);
        assert_eq!(value_at(7 + 30, 258, 14), 10, "arredonda ao passo");
    }
}
