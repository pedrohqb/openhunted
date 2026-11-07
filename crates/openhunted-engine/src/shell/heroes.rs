// SPDX-License-Identifier: GPL-3.0-or-later
//! Telas pequenas da casca (`docs/re/menus-restantes.md`): lista de heróis (`SHERO`), nome
//! (`SHIGH`), carro de fuga (`S_CAR`) e as informativas (`SABOUT`, `SMULT`, `SINPU`,
//! `SKEYB`).

use std::rc::Rc;

use openhunted_formats::Font;

use super::page::{Page, Reply};
use super::screen::{CANCEL, Input, Screen, draw_text_in, flags};
use crate::physics::Rect;
use crate::render::{Framebuffer, measure_text};

/// Som de cada linha nova da lista e da contagem do resultado.
pub const SOUND_LINE: u16 = 3093;

/// `SHERO.ITF` (`0x444c97`/`0x444ced`): uma linha nova por quadro, nome à esquerda e
/// pontuação à direita, cor 209.
pub mod hero_ids {
    pub const OK: u16 = 31001;
    pub const HEROES: u16 = 31002;
    pub const PISTON: u16 = 31005;
    pub const COLOR: u8 = 209;
    /// Altura de uma linha (14 linhas na área de 296 px).
    pub const ROW: i32 = 21;
}

#[derive(Debug, Clone)]
pub struct Heroes {
    pub screen: Screen,
    font: Rc<Font>,
    rows: Vec<(String, i32)>,
    shown: usize,
}

impl Heroes {
    pub fn new(mut screen: Screen, font: Rc<Font>, rows: Vec<(String, i32)>) -> Self {
        screen.set_flags(hero_ids::OK, flags::UPDATE | flags::NAV);
        screen.point_at(hero_ids::OK);
        Self {
            screen,
            font,
            rows,
            shown: 0,
        }
    }

    /// Linhas já mostradas.
    pub fn shown(&self) -> usize {
        self.shown
    }
}

impl Page for Heroes {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, sounds: &mut Vec<u16>) -> Option<Reply> {
        if input == Input::Tick && self.shown < self.rows.len() {
            self.shown += 1;
            sounds.push(SOUND_LINE);
        }
        match self.screen.handle(input)? {
            hero_ids::OK | CANCEL => Some(Reply::Close),
            _ => None,
        }
    }

    fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
        let Some(r) = self.screen.rect(hero_ids::HEROES) else {
            return;
        };
        for (i, (name, score)) in self.rows.iter().take(self.shown).enumerate() {
            let row = Rect {
                x: r.x,
                y: r.y + i as i32 * hero_ids::ROW,
                w: r.w,
                h: hero_ids::ROW,
            };
            draw_text_in(fb, &self.font, row, name, hero_ids::COLOR, false);
            let s = score.to_string();
            let (tw, _) = measure_text(&self.font, &s);
            let right = Rect {
                x: row.x + row.w - tw - 1,
                w: tw + 1,
                ..row
            };
            draw_text_in(fb, &self.font, right, &s, hero_ids::COLOR, false);
        }
    }
}

/// `SHIGH.ITF` (`0x44530c`): diálogo com o campo de edição 21002 e DONE.
pub mod name_ids {
    pub const DONE: u16 = 21001;
    pub const FIELD: u16 = 21002;
    /// Caracteres aceitos (decisão do port; o campo do original guarda até 128 bytes).
    pub const MAX: usize = 31;
    /// Cor do texto sobre o painel claro (marrom escuro da `WORLD.PAL`).
    pub const COLOR: u8 = 145;
    /// Área do painel com o texto "CONGRATULATIONS!…" da arte (relativa ao diálogo) e um
    /// ponto do painel liso: com título, o port cobre o texto da arte com essa cor.
    pub const PANEL_TEXT: (i32, i32, i32, i32) = (52, 50, 292, 62);
    pub const PANEL_SAMPLE: (i32, i32) = (330, 100);
}

#[derive(Debug, Clone)]
pub struct NameEntry {
    pub screen: Screen,
    font: Rc<Font>,
    /// Título acima do campo (decisão do port: "Load Game:", "Save Game As:"; o nome do
    /// herói vai sem título, como no original).
    title: Option<String>,
    text: String,
}

impl NameEntry {
    pub fn new(mut screen: Screen, font: Rc<Font>, title: Option<String>, initial: &str) -> Self {
        screen.set_flags(name_ids::DONE, flags::UPDATE | flags::NAV);
        screen.point_at(name_ids::DONE);
        let mut n = Self {
            screen,
            font,
            title,
            text: String::new(),
        };
        n.text(initial);
        n
    }

    pub fn value(&self) -> &str {
        &self.text
    }
}

impl Page for NameEntry {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        // Enter confirma de onde estiver o ponteiro
        if input == Input::Activate {
            return Some(Reply::Name(self.text.clone()));
        }
        match self.screen.handle(input)? {
            name_ids::DONE => Some(Reply::Name(self.text.clone())),
            CANCEL => Some(Reply::Close),
            _ => None,
        }
    }

    fn text(&mut self, s: &str) {
        for c in s.chars().filter(|c| (' '..='~').contains(c)) {
            if self.text.len() < name_ids::MAX {
                self.text.push(c);
            }
        }
    }

    fn backspace(&mut self) {
        self.text.pop();
    }

    fn wants_text(&self) -> bool {
        true
    }

    fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
        let Some(r) = self.screen.rect(name_ids::FIELD) else {
            return;
        };
        let shown = format!("{}_", self.text);
        let inner = Rect {
            x: r.x + 4,
            w: r.w - 8,
            ..r
        };
        // o fim do texto fica sempre à vista
        let (tw, _) = measure_text(&self.font, &shown);
        let area = if tw > inner.w {
            Rect {
                x: inner.x + inner.w - tw,
                w: tw,
                ..inner
            }
        } else {
            inner
        };
        let mut clip = crate::render::Framebuffer::new(fb.width, fb.height);
        clip.pixels.copy_from_slice(&fb.pixels);
        draw_text_in(&mut clip, &self.font, area, &shown, name_ids::COLOR, false);
        // recorta no campo
        for y in inner.y.max(0)..(inner.y + inner.h).min(fb.height as i32) {
            for x in inner.x.max(0)..(inner.x + inner.w).min(fb.width as i32) {
                let i = y as usize * fb.width + x as usize;
                fb.pixels[i] = clip.pixels[i];
            }
        }
        if let Some(t) = &self.title {
            let b = self.screen.bounds();
            let (sx, sy) = name_ids::PANEL_SAMPLE;
            let (px, py, pw, ph) = name_ids::PANEL_TEXT;
            let si = (b.y + sy) as usize * fb.width + (b.x + sx) as usize;
            let fill = fb.pixels.get(si).copied().unwrap_or(0);
            let tr = Rect {
                x: b.x + px,
                y: b.y + py,
                w: pw,
                h: ph,
            };
            for y in tr.y.max(0)..(tr.y + tr.h).min(fb.height as i32) {
                for x in tr.x.max(0)..(tr.x + tr.w).min(fb.width as i32) {
                    fb.pixels[y as usize * fb.width + x as usize] = fill;
                }
            }
            draw_text_in(fb, &self.font, tr, t, name_ids::COLOR, true);
        }
    }
}

/// `S_CAR.ITF` (`0x445ac0` + `0x445b08`): o carro com as peças ganhas.
pub mod car_ids {
    pub const DONE: u16 = 38001;
    pub const CAR: u16 = 38002;
    /// Rótulo do carro = peças + 3 ("part 01" = rótulo 4).
    pub const LABEL_BASE: u16 = 3;
}

#[derive(Debug, Clone)]
pub struct Car {
    pub screen: Screen,
}

impl Car {
    pub fn new(mut screen: Screen, pieces: u16) -> Self {
        screen.set_label(car_ids::CAR, pieces.clamp(1, 15) + car_ids::LABEL_BASE);
        screen.play_once(car_ids::CAR);
        screen.set_flags(car_ids::DONE, flags::UPDATE | flags::NAV);
        screen.point_at(car_ids::DONE);
        Self { screen }
    }
}

impl Page for Car {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        match self.screen.handle(input)? {
            car_ids::DONE | CANCEL => Some(Reply::Close),
            _ => None,
        }
    }
}

/// Telas só de consulta: "Sobre" (`SABOUT`), dois jogadores (`SMULT`), dispositivos
/// (`SINPU`) e teclas (`SKEYB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoKind {
    About,
    Multi,
    Input,
    Keys,
}

/// Ids das telas informativas.
pub mod info_ids {
    pub const ABOUT_DONE: u16 = 33001;
    pub const ABOUT_VERSION: u16 = 33002;
    pub const ABOUT_COLOR: u8 = 255;
    pub const MULTI_H2H: u16 = 13001;
    pub const MULTI_COOP: u16 = 13002;
    pub const MULTI_CANCEL: u16 = 13004;
    pub const INPUT_P1_KEYBOARD: u16 = 29001;
    pub const INPUT_P2_KEYBOARD: u16 = 29006;
    pub const INPUT_P2_GAMEPAD2: u16 = 29010;
    pub const INPUT_DONE: u16 = 29011;
    pub const INPUT_P1_CUSTOMIZE: u16 = 29013;
    pub const INPUT_P2_CUSTOMIZE: u16 = 29014;
    /// Botões das ações (cima, baixo, esquerda, direita, atirar, pular, furtivo, arma
    /// anterior, soco, próxima arma) e as áreas dos nomes das teclas.
    pub const KEYS_FIRST: u16 = 17001;
    pub const KEYS_LAST: u16 = 17010;
    pub const KEYS_STR: u16 = 17011;
    pub const KEYS_DONE: u16 = 17024;
    pub const KEYS_RESET: u16 = 17025;
    pub const KEYS_COLOR: u8 = 211;
    pub const KEYS_CHARACTER: u16 = 17023;
    pub const KEYS_NAMEPLATE: u16 = 17026;
    /// Rótulo da placa "play 1, keyb" do `Skeyb.anm`.
    pub const KEYS_PLATE_P1_KEYBOARD: u16 = 47;
}

#[derive(Debug, Clone)]
pub struct Info {
    pub screen: Screen,
    pub kind: InfoKind,
}

impl Info {
    /// `texts`: "Sobre" → a versão; teclas → os dez nomes, na ordem dos botões 17001–17010.
    pub fn new(
        mut screen: Screen,
        kind: InfoKind,
        font: Option<Rc<Font>>,
        texts: &[String],
    ) -> Self {
        use info_ids::*;
        let buttons: &[u16] = match kind {
            InfoKind::About => &[ABOUT_DONE],
            InfoKind::Multi => &[MULTI_H2H, MULTI_COOP, MULTI_CANCEL],
            InfoKind::Input => &[INPUT_DONE, INPUT_P1_CUSTOMIZE, INPUT_P2_CUSTOMIZE],
            InfoKind::Keys => &[KEYS_DONE, KEYS_RESET],
        };
        for &id in buttons {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        match kind {
            InfoKind::About => {
                if let (Some(f), Some(t)) = (font, texts.first()) {
                    screen.set_text(ABOUT_VERSION, t, f, ABOUT_COLOR);
                }
            }
            InfoKind::Input => {
                // só o teclado (jogador 1 no `Keyboard1`, jogador 2 no `Keyboard2`)
                for id in INPUT_P1_KEYBOARD..=INPUT_P2_GAMEPAD2 {
                    screen.make_option(id);
                    screen.clear_flags(id, flags::NAV);
                }
                screen.set_marked(INPUT_P1_KEYBOARD, true);
                screen.set_marked(INPUT_P2_KEYBOARD, true);
            }
            InfoKind::Keys => {
                screen.set_label(KEYS_NAMEPLATE, KEYS_PLATE_P1_KEYBOARD);
                if let Some(f) = font {
                    for (k, t) in texts.iter().take(10).enumerate() {
                        screen.set_text(KEYS_STR + k as u16, t, f.clone(), KEYS_COLOR);
                    }
                }
            }
            InfoKind::Multi => {}
        }
        screen.point_at(buttons[0]);
        Self { screen, kind }
    }
}

impl Page for Info {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        use info_ids::*;
        let id = self.screen.handle(input)?;
        match (self.kind, id) {
            (_, CANCEL)
            | (InfoKind::About, ABOUT_DONE)
            | (InfoKind::Multi, MULTI_CANCEL)
            | (InfoKind::Input, INPUT_DONE)
            | (InfoKind::Keys, KEYS_DONE) => Some(Reply::Close),
            (InfoKind::Multi, MULTI_H2H | MULTI_COOP) | (InfoKind::Input, INPUT_P2_CUSTOMIZE) => {
                Some(Reply::Unavailable)
            }
            (InfoKind::Input, INPUT_P1_CUSTOMIZE) => Some(Reply::Customize),
            _ => None,
        }
    }
}
