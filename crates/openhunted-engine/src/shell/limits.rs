// SPDX-License-Identifier: GPL-3.0-or-later
//! Ajustes da disputa (`SHVLD.ITF`, `process_deathmatch_interface`, `0x4454e8` +
//! `0x445501`; `docs/re/multijogador.md`): limite de tempo e de pontos com os dígitos de rolo
//! da `INVBAR01`, nome da fase, GO e CANCEL.

use std::rc::Rc;

use openhunted_formats::{Bitmap, Font};

use super::hud::timer_frames;
use super::page::{Page, Reply};
use super::screen::{CANCEL, Input, Screen, flags};
use crate::render::{Framebuffer, draw_text};

pub mod ids {
    pub const BACKGROUND: u16 = 32000;
    pub const TIME_UP: u16 = 32003;
    pub const TIME_DOWN: u16 = 32004;
    pub const KILLS_UP: u16 = 32005;
    pub const KILLS_DOWN: u16 = 32006;
    pub const TIME: u16 = 32007;
    pub const KILLS: u16 = 32008;
    pub const GO: u16 = 32009;
    pub const CANCEL_BUTTON: u16 = 32010;
    pub const LEVEL_NAME: u16 = 32011;
}

/// Valores guardados × 20 (`0x480560` tempo, `0x480564` pontos), passo 20, teto 1980.
pub const STEP: i32 = 20;
pub const MAX: i32 = 1980;
/// Cor e posição do nome da fase (`0x445a90`).
pub const NAME_COLOR: u8 = 162;
pub const NAME_AT: (i32, i32) = (16, 9);
/// Espera da repetição do botão preso (`0x4458e1`): 5, 4, 3, 2, 1 e depois todo quadro.
pub const REPEAT_FIRST: i32 = 5;
/// Contador: unidades em `x + w − 26, y + 4`, 22 px à esquerda por dígito (`0x4459ba`).
const DIGIT_DX: i32 = 26;
const DIGIT_DY: i32 = 4;
const DIGIT_STEP: i32 = 22;

/// As setas: (controle, contador 0 = tempo / 1 = pontos, sentido).
const ARROWS: [(u16, usize, i32); 4] = [
    (ids::TIME_UP, 0, 1),
    (ids::TIME_DOWN, 0, -1),
    (ids::KILLS_UP, 1, 1),
    (ids::KILLS_DOWN, 1, -1),
];

#[derive(Debug, Clone)]
pub struct Limits {
    pub screen: Screen,
    font: Rc<Font>,
    title: String,
    bar: Option<Rc<Bitmap>>,
    /// Alvos (tempo, pontos) × 20 e o que os contadores mostram (rolam até o alvo).
    value: [i32; 2],
    shown: [i32; 2],
    /// Seta presa, espera da repetição (`0x480568`) e repetições (`0x480570`).
    held: Option<u16>,
    down: bool,
    wait: i32,
    repeats: i32,
}

impl Limits {
    /// `minutes`, `points`: os limites anteriores (`0x497374`, `0x497378`; 0 = sem limite).
    pub fn new(
        mut screen: Screen,
        font: Rc<Font>,
        title: &str,
        bar: Option<Rc<Bitmap>>,
        minutes: i32,
        points: i32,
    ) -> Self {
        for (id, ..) in ARROWS {
            screen.set_frame(id, 0);
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        for id in [ids::GO, ids::CANCEL_BUTTON] {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        // contadores e nome têm desenho próprio (`0x4459ba`, `0x445a90`)
        for id in [ids::TIME, ids::KILLS] {
            screen.set_flags(id, flags::HIDDEN);
        }
        screen.point_at(ids::GO);
        let value = [
            (minutes * STEP).clamp(0, MAX),
            (points * STEP).clamp(0, MAX),
        ];
        Self {
            screen,
            font,
            title: title.to_string(),
            bar,
            value,
            shown: value,
            held: None,
            down: false,
            wait: 0,
            repeats: 0,
        }
    }

    /// Limites escolhidos: minutos e pontos (`0x4457ab`, ÷ 20).
    pub fn limits(&self) -> (i32, i32) {
        (self.value[0] / STEP, self.value[1] / STEP)
    }

    /// Valores mostrados agora (× 20).
    pub fn shown(&self) -> [i32; 2] {
        self.shown
    }

    /// Um passo da seta (`0x44580a`…): ± 20, entre 0 e 1980.
    fn step(&mut self, id: u16) {
        if let Some(&(_, k, d)) = ARROWS.iter().find(|a| a.0 == id) {
            self.value[k] = (self.value[k] + d * STEP).clamp(0, MAX);
        }
    }

    fn arrow_under_pointer(&self) -> Option<u16> {
        ARROWS.iter().map(|a| a.0).find(|&id| self.screen.hit(id))
    }

    /// Um quadro: repetição da seta presa, quadros das setas e rolagem dos contadores
    /// (`0x4458a1`: passo `diff / 5`, no mínimo 1).
    fn tick(&mut self) {
        let under = self.arrow_under_pointer();
        match (self.held, self.down) {
            (Some(id), true) if under == Some(id) => {
                if self.wait > 0 {
                    self.wait -= 1;
                } else {
                    self.step(id);
                    self.wait = (REPEAT_FIRST - self.repeats).max(0);
                    self.repeats += 1;
                }
            }
            _ => {
                self.wait = 0;
                self.repeats = 0;
            }
        }
        for (id, ..) in ARROWS {
            let f = match (under == Some(id), self.down && self.held == Some(id)) {
                (true, true) => 2,
                (true, false) => 1,
                _ => 0,
            };
            self.screen.set_frame(id, f);
        }
        for k in 0..2 {
            let d = self.value[k] - self.shown[k];
            let s = d / 5;
            self.shown[k] += if s == 0 { d.signum() } else { s };
        }
    }

    fn draw_counter(&self, fb: &mut Framebuffer, id: u16, v: i32) {
        let (Some(r), Some(bar)) = (self.screen.rect(id), self.bar.as_ref()) else {
            return;
        };
        let frames = timer_frames(v);
        for (k, &f) in frames.iter().take(2).enumerate() {
            if let Some(fr) = bar.frames.get(f) {
                let x = r.x + r.w - DIGIT_DX - DIGIT_STEP * k as i32;
                fb.blit(fr, x, r.y + DIGIT_DY, true);
            }
        }
    }
}

impl Page for Limits {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        match input {
            Input::Tick => self.tick(),
            Input::PointerDown => {
                self.down = true;
                self.held = self.arrow_under_pointer();
                if self.held.is_some() {
                    return None;
                }
            }
            Input::PointerUp => {
                self.down = false;
                self.held = None;
            }
            // Enter/espaço numa seta: um passo
            Input::Activate => {
                if let Some(id) = self.arrow_under_pointer() {
                    self.step(id);
                    return None;
                }
            }
            _ => {}
        }
        match self.screen.handle(input)? {
            ids::GO => {
                let (minutes, points) = self.limits();
                Some(Reply::Limits { minutes, points })
            }
            ids::CANCEL_BUTTON | CANCEL => Some(Reply::Close),
            _ => None,
        }
    }

    fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
        if let Some(r) = self.screen.rect(ids::LEVEL_NAME) {
            draw_text(
                fb,
                &self.font,
                r.x + NAME_AT.0,
                r.y + NAME_AT.1,
                &self.title,
                Some(NAME_COLOR),
            );
        }
        self.draw_counter(fb, ids::TIME, self.shown[0]);
        self.draw_counter(fb, ids::KILLS, self.shown[1]);
    }
}
