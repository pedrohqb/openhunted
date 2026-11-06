// SPDX-License-Identifier: GPL-3.0-or-later
//! Briefing da missão (`SLEVD.ITF`, `docs/re/briefing.md`): diálogo centrado com o título
//! e a descrição, 5 linhas visíveis com setas, GO e CANCEL.

use std::rc::Rc;

use openhunted_formats::Font;

use super::screen::{CANCEL, Dir, Input, Screen, draw_text_in, flags};
use crate::physics::Rect;
use crate::render::{Framebuffer, wrap_text};

pub const GO: u16 = 21501;
pub const CANCEL_BUTTON: u16 = 21502;
pub const TEXT_AREA: u16 = 21503;
pub const FAN: u16 = 21504;
pub const UP: u16 = 21505;
pub const DOWN: u16 = 21506;
pub const SLIDER: u16 = 21507;
/// Rótulo da alça da barra de rolagem.
const HANDLE_LABEL: u16 = 10;
/// Cor do texto (`0x497d94`).
pub const TEXT_COLOR: u8 = 162;
/// Linhas visíveis.
pub const VISIBLE: usize = 5;
/// Anúncio ao começar a partida (solo/cooperativo).
pub const SOUND_GO: u16 = 3795;

/// O que o jogador escolheu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Go,
    Cancel,
}

/// O briefing aberto.
#[derive(Debug, Clone)]
pub struct Briefing {
    pub screen: Screen,
    font: Rc<Font>,
    lines: Vec<String>,
    first: usize,
}

impl Briefing {
    /// Monta o texto `"título\n\ndescrição"` quebrado na largura da área 21503; com mais
    /// de 5 linhas, liga as setas e a barra; senão as remove. O ponteiro vai para GO.
    pub fn new(mut screen: Screen, font: Rc<Font>, title: &str, description: &str) -> Self {
        let text = format!("{title}\n\n{description}");
        let width = screen.rect(TEXT_AREA).map_or(238, |r| r.w);
        let lines = wrap_text(&font, &text, width);
        screen.set_flags(FAN, flags::UPDATE);
        for id in [GO, CANCEL_BUTTON] {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        if lines.len() > VISIBLE {
            screen.make_button(UP);
            screen.make_button(DOWN);
        } else {
            for id in [UP, DOWN, SLIDER] {
                screen.remove(id);
            }
        }
        screen.point_at(GO);
        Self {
            screen,
            font,
            lines,
            first: 0,
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Primeira linha visível.
    pub fn first_line(&self) -> usize {
        self.first
    }

    fn max_first(&self) -> usize {
        self.lines.len().saturating_sub(VISIBLE)
    }

    pub fn handle(&mut self, input: Input) -> Option<Choice> {
        match self.screen.handle(input)? {
            GO => Some(Choice::Go),
            CANCEL_BUTTON | CANCEL => Some(Choice::Cancel),
            UP => {
                self.first = self.first.saturating_sub(1);
                None
            }
            DOWN => {
                self.first = (self.first + 1).min(self.max_first());
                None
            }
            _ => None,
        }
    }

    /// Atalho do teclado: as setas para cima/baixo navegam entre GO e CANCEL como no
    /// original (só esquerda/direita os separam); aqui também rolam o texto.
    pub fn nav(&mut self, d: Dir) {
        match d {
            Dir::Up if self.lines.len() > VISIBLE => self.first = self.first.saturating_sub(1),
            Dir::Down if self.lines.len() > VISIBLE => {
                self.first = (self.first + 1).min(self.max_first())
            }
            _ => {
                self.screen.handle(Input::Nav(d));
            }
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
        let Some(area) = self.screen.rect(TEXT_AREA) else {
            return;
        };
        let line_h = area.h / VISIBLE as i32;
        for (i, line) in self.lines.iter().skip(self.first).take(VISIBLE).enumerate() {
            let r = Rect {
                x: area.x,
                y: area.y + line_h * i as i32,
                w: area.w,
                h: line_h,
            };
            draw_text_in(fb, &self.font, r, line, TEXT_COLOR, true);
        }
        // alça da barra: faixa 0…(linhas − 5), posição proporcional na trilha
        if let (Some(track), Some(handle)) = (
            self.screen.rect(SLIDER),
            self.screen.label_frame(HANDLE_LABEL),
        ) {
            let b = self.screen.bounds();
            let hh = 26;
            let span = (track.h - hh).max(0);
            let max = self.max_first().max(1) as i32;
            let y = track.y + span * self.first as i32 / max;
            self.screen
                .draw_anim_frame(fb, handle, track.x - b.x, y - b.y);
        }
    }
}
