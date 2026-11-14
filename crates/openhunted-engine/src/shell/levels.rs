// SPDX-License-Identifier: GPL-3.0-or-later
//! Escolher missão (`SLEVL.ITF`, `0x43fa7c` + `0x43fc68`; `docs/re/menu.md`,
//! `menus-restantes.md`). Decisão do port: as sete áreas 22019–22025 mostram a lista de
//! missões solo (título; marca nas vencidas), com rolagem pelas setas e pela barra; CIMA/BAIXO
//! do teclado mudam a escolhida; LOAD escolhe. Os grupos 22016–22018 trocam o modo
//! (`0x4413aa`): a aplicação reabre a tela com a lista dele (M15b).

use std::rc::Rc;

use openhunted_formats::Font;

use super::Party;
use super::page::{Page, Reply};
use super::screen::{CANCEL, Dir, Input, Screen, draw_text_in, flags};
use crate::physics::Rect;
use crate::render::Framebuffer;

pub const LOAD: u16 = 22005;
pub const CANCEL_BUTTON: u16 = 22006;
pub const UP: u16 = 22007;
pub const DOWN: u16 = 22008;
pub const SLIDER: u16 = 22009;
pub const PORTRAIT: u16 = 22010;
pub const ONE_PLAYER: u16 = 22016;
pub const HEAD2HEAD: u16 = 22017;
pub const COOP: u16 = 22018;
pub const LINE1: u16 = 22019;
pub const LINE7: u16 = 22025;
/// Linhas visíveis.
pub const VISIBLE: usize = 7;
/// Rótulos do `Slevl.anm`: marca de vencida, alça da barra, retratos do Jake e do Garathe.
const CHECK: u16 = 5;
const HANDLE: u16 = 15;
pub const JAKE_PORTRAIT: u16 = 17;
pub const GARATHE_PORTRAIT: u16 = 18;
/// Cores do texto (decisão do port: dourado e, na escolhida, quase branco da `WORLD.PAL`).
pub const TEXT_COLOR: u8 = 211;
pub const SELECTED_COLOR: u8 = 221;
/// Espaço da marca à esquerda do título.
const CHECK_W: i32 = 30;

/// Uma missão da lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub title: String,
    /// Tem recorde (`0x47fa8c[i] ≠ 0`).
    pub won: bool,
    /// Missão do Garathe (`p=2`): retrato 18.
    pub garathe: bool,
    /// Tabela de cor do retrato (2º número do `p=`, `DLS.TBL`; `0x47f5bc`).
    pub colors: Option<Rc<[u8; 256]>>,
}

#[derive(Debug, Clone)]
pub struct LevelSelect {
    pub screen: Screen,
    font: Rc<Font>,
    entries: Vec<Entry>,
    selected: usize,
    first: usize,
    party: Party,
}

impl LevelSelect {
    pub fn new(mut screen: Screen, font: Rc<Font>, entries: Vec<Entry>, selected: usize) -> Self {
        for id in [ONE_PLAYER, HEAD2HEAD, COOP] {
            screen.make_option(id);
        }
        screen.set_marked(ONE_PLAYER, true);
        for id in LINE1..=LINE7 {
            screen.make_option(id);
            screen.clear_flags(id, flags::NAV);
        }
        screen.make_button(UP);
        screen.make_button(DOWN);
        for id in [LOAD, CANCEL_BUTTON] {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        screen.point_at(LOAD);
        let selected = selected.min(entries.len().saturating_sub(1));
        let mut l = Self {
            screen,
            font,
            entries,
            selected,
            first: 0,
            party: Party::Solo,
        };
        l.select(selected);
        l
    }

    /// A lista é a do modo `party`: marca o grupo dele.
    pub fn with_party(mut self, party: Party) -> Self {
        self.party = party;
        for (id, p) in Self::GROUPS {
            self.screen.set_marked(id, p == party);
        }
        self
    }

    /// Grupos do modo (22016 um jogador, 22017 disputa, 22018 cooperação).
    const GROUPS: [(u16, Party); 3] = [
        (ONE_PLAYER, Party::Solo),
        (HEAD2HEAD, Party::Deathmatch),
        (COOP, Party::Coop),
    ];

    pub fn party(&self) -> Party {
        self.party
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn first(&self) -> usize {
        self.first
    }

    fn max_first(&self) -> usize {
        self.entries.len().saturating_sub(VISIBLE)
    }

    /// Escolhe a missão `i`, rolando para que fique visível, e troca o retrato.
    pub fn select(&mut self, i: usize) {
        if self.entries.is_empty() {
            return;
        }
        self.selected = i.min(self.entries.len() - 1);
        if self.selected < self.first {
            self.first = self.selected;
        } else if self.selected >= self.first + VISIBLE {
            self.first = self.selected + 1 - VISIBLE;
        }
        let label = if self.entries[self.selected].garathe {
            GARATHE_PORTRAIT
        } else {
            JAKE_PORTRAIT
        };
        self.screen.set_label(PORTRAIT, label);
        let colors = self.entries[self.selected].colors.clone();
        self.screen.set_remap(PORTRAIT, colors);
    }

    fn scroll(&mut self, up: bool) {
        self.first = if up {
            self.first.saturating_sub(1)
        } else {
            (self.first + 1).min(self.max_first())
        };
    }

    fn line_rect(&self, k: usize) -> Option<Rect> {
        self.screen.rect(LINE1 + k as u16)
    }
}

impl Page for LevelSelect {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        match input {
            Input::Nav(Dir::Up) if self.selected > 0 => {
                self.select(self.selected - 1);
                return None;
            }
            Input::Nav(Dir::Down) => {
                self.select(self.selected + 1);
                return None;
            }
            Input::Nav(Dir::Up) => return None,
            _ => {}
        }
        match self.screen.handle(input)? {
            LOAD if !self.entries.is_empty() => Some(Reply::Pick(self.selected)),
            CANCEL_BUTTON | CANCEL => Some(Reply::Close),
            id @ (ONE_PLAYER | HEAD2HEAD | COOP) => Self::GROUPS
                .iter()
                .find(|g| g.0 == id && g.1 != self.party)
                .map(|g| Reply::Party(g.1)),
            UP => {
                self.scroll(true);
                None
            }
            DOWN => {
                self.scroll(false);
                None
            }
            id @ LINE1..=LINE7 => {
                let i = self.first + usize::from(id - LINE1);
                if i < self.entries.len() {
                    self.select(i);
                }
                None
            }
            _ => None,
        }
    }

    fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
        let (ox, oy) = self.screen.origin();
        let check = self.screen.label_frame(CHECK);
        let ch = self.screen.label_size(CHECK).map_or(0, |s| s.1);
        for k in 0..VISIBLE {
            let i = self.first + k;
            let (Some(e), Some(r)) = (self.entries.get(i), self.line_rect(k)) else {
                break;
            };
            if let (true, Some(f)) = (e.won, check) {
                self.screen
                    .draw_anim_frame(fb, f, r.x + 4 - ox, r.y + (r.h - ch) / 2 - oy);
            }
            let color = if i == self.selected {
                SELECTED_COLOR
            } else {
                TEXT_COLOR
            };
            let area = Rect {
                x: r.x + CHECK_W,
                w: r.w - CHECK_W,
                ..r
            };
            draw_text_in(fb, &self.font, area, &e.title, color, false);
        }
        // alça da barra de rolagem
        if let (Some(r), Some(f)) = (self.screen.rect(SLIDER), self.screen.label_frame(HANDLE)) {
            let hh = self.screen.label_size(HANDLE).map_or(25, |s| s.1);
            let max = self.max_first().max(1) as i32;
            let y = r.y + (r.h - hh) * self.first.min(self.max_first()) as i32 / max;
            self.screen.draw_anim_frame(fb, f, r.x - ox, y - oy);
        }
    }
}
