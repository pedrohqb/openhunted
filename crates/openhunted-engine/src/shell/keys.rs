// SPDX-License-Identifier: GPL-3.0-or-later
//! Teclas (`SKEYB.ITF`, `0x4419e0`) e botões do joystick (`SJOYS.ITF`, `0x442638`) de um
//! jogador (`docs/re/casca-acabamento.md`): clique numa ação espera a tecla ou o botão; a
//! tecla já usada troca de lugar; RESET volta ao padrão; DONE e Esc devolvem a configuração.

use std::rc::Rc;

use openhunted_formats::Font;

use super::config::Config;
use super::page::{Page, Reply};
use super::screen::{CANCEL, Input, Screen, draw_text_in, flags};
use crate::devices::{DEFAULT_BUTTONS, DEFAULT_KEYS, KEYS, assign_button, assign_key};
use crate::render::{Framebuffer, draw_text, wrap_text};

/// Ids das duas telas.
pub mod ids {
    pub const BACKGROUND: u16 = 17000;
    /// Botões das ações (cima … próxima arma) e as áreas dos nomes (+10).
    pub const ACTION: u16 = 17001;
    pub const NAME: u16 = 17011;
    pub const DESCRIPTION: u16 = 17021;
    pub const CHARACTER: u16 = 17023;
    pub const DONE: u16 = 17024;
    pub const RESET: u16 = 17025;
    pub const PLATE: u16 = 17026;
    /// Primeira ação de botão (atirar): o `SJOYS` só tem 17005–17010.
    pub const FIRST_BUTTON_ACTION: u16 = 17005;
    /// Cor dos nomes (`0x442417`) e da descrição (`0x442444`, [inferido]).
    pub const NAME_COLOR: u8 = 209;
    pub const DESCRIPTION_COLOR: u8 = 186;
    /// Rótulos do RESET: uma ação (na espera) e todas.
    pub const RESET_ONE: u16 = 34;
    pub const RESET_ALL: u16 = 38;
    /// Placas: "play 1, keyb" / "play 2, keyb"; joysticks 1–4 dos jogadores (`0x480290`).
    pub const PLATE_KEYBOARD: [u16; 2] = [47, 48];
    pub const PLATE_JOYSTICK: [[u16; 4]; 2] = [[39, 40, 43, 44], [41, 42, 45, 46]];
    /// Rótulos do retrato por ação e parado: Jake (`0x4800c4`) e Garathe (`0x4800f0`).
    pub const JAKE: [u16; 11] = [12, 14, 13, 15, 16, 17, 18, 19, 21, 19, 35];
    pub const GARATHE: [u16; 11] = [22, 23, 24, 25, 26, 27, 28, 29, 31, 29, 36];
    /// Textos da combinação do pulo, desenhados com menos de 4 botões (`0x442e17`).
    pub const COMBO_TEXTS: [u16; 2] = [20, 30];
    /// Ações de arma escondidas com até 2 botões (botão e nome).
    pub const WEAPON_CONTROLS: [u16; 4] = [17008, 17018, 17010, 17020];
    /// O nome pisca a cada 6 quadros (`0x48011c`).
    pub const BLINK: i32 = 6;
}

/// Teclado (10 ações) ou joystick (as 6 de botão).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyboard,
    /// Joystick com `n` botões.
    Joystick(u32),
}

#[derive(Debug, Clone)]
pub struct KeyScreen {
    pub screen: Screen,
    font: Rc<Font>,
    pub kind: Kind,
    player: usize,
    pub config: Config,
    /// `KEYS.RES` (teclado: linha *v* = tecla VK *v*) ou `JOYSTICK.RES`.
    names: Vec<String>,
    /// `SHELL.RES` (teclado) ou `JOYSTICK.RES`: linha 1 geral, 2… por ação.
    descriptions: Vec<String>,
    labels: [u16; 11],
    capture: Option<usize>,
    blink: (i32, bool),
    description: String,
}

impl KeyScreen {
    /// `texts`: as linhas de nomes e de descrições (no joystick, as duas do `JOYSTICK.RES`);
    /// `garathe`: o retrato do jogador (o 2 no padrão); `colors`: a tabela dele.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mut screen: Screen,
        font: Rc<Font>,
        kind: Kind,
        player: usize,
        joystick: usize,
        config: Config,
        names: Vec<String>,
        descriptions: Vec<String>,
        garathe: bool,
    ) -> Self {
        let labels = if garathe { ids::GARATHE } else { ids::JAKE };
        let player = player.min(1);
        for id in [ids::DONE, ids::RESET] {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        let first = match kind {
            Kind::Keyboard => 0,
            Kind::Joystick(_) => 4,
        };
        for s in first..KEYS {
            screen.set_flags(ids::ACTION + s as u16, flags::UPDATE | flags::NAV);
        }
        screen.set_label(ids::RESET, ids::RESET_ALL);
        screen.set_label(
            ids::PLATE,
            match kind {
                Kind::Keyboard => ids::PLATE_KEYBOARD[player],
                Kind::Joystick(_) => ids::PLATE_JOYSTICK[player][joystick.min(3)],
            },
        );
        if let Kind::Joystick(n) = kind {
            if n <= 2 {
                for id in ids::WEAPON_CONTROLS {
                    screen.set_flags(id, flags::HIDDEN);
                }
            }
        }
        screen.point_at(ids::DONE);
        let mut k = Self {
            screen,
            font,
            kind,
            player,
            config,
            names,
            descriptions,
            labels,
            capture: None,
            blink: (ids::BLINK - 1, false),
            description: String::new(),
        };
        k.idle();
        k
    }

    fn line(v: &[String], n: usize) -> String {
        v.get(n.wrapping_sub(1)).cloned().unwrap_or_default()
    }

    /// Primeira ação da tela (0 no teclado, 4 = atirar no joystick).
    fn first(&self) -> usize {
        match self.kind {
            Kind::Keyboard => 0,
            Kind::Joystick(_) => 4,
        }
    }

    /// Sem espera: retrato parado, descrição geral, RESET de todas.
    fn idle(&mut self) {
        self.capture = None;
        self.screen.set_label(ids::CHARACTER, self.labels[10]);
        self.screen.set_label(ids::RESET, ids::RESET_ALL);
        self.description = Self::line(&self.descriptions, 1);
    }

    /// A ação em espera (0–9), se houver.
    pub fn capturing(&self) -> Option<usize> {
        self.capture
    }

    /// Nome mostrado para a ação `slot` (0–9).
    pub fn name(&self, slot: usize) -> String {
        match self.kind {
            Kind::Keyboard => {
                let vk = self.config.keys[self.player][slot];
                Self::line(&self.names, usize::from(vk))
            }
            Kind::Joystick(_) => {
                let b = self.config.buttons[self.player][slot - 4];
                usize::try_from(b)
                    .ok()
                    .filter(|&b| b > 0)
                    .map(|b| Self::line(&self.names, 9 + b))
                    .unwrap_or_default()
            }
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    fn start(&mut self, slot: usize) {
        self.capture = Some(slot);
        self.blink = (ids::BLINK - 1, false);
        self.screen.set_label(ids::RESET, ids::RESET_ONE);
        self.screen.set_label(ids::CHARACTER, self.labels[slot]);
        // Ruling: a descrição da própria ação (o original troca soco e próxima arma)
        let line = match self.kind {
            Kind::Keyboard => slot + 2,
            Kind::Joystick(_) => slot - 4 + 2,
        };
        self.description = Self::line(&self.descriptions, line);
    }

    fn reset(&mut self) {
        let p = self.player;
        match (self.kind, self.capture) {
            (Kind::Keyboard, Some(s)) => {
                assign_key(&mut self.config.keys, p, s, DEFAULT_KEYS[p][s]);
            }
            (Kind::Keyboard, None) => self.config.keys = DEFAULT_KEYS,
            (Kind::Joystick(_), Some(s)) => {
                let b = DEFAULT_BUTTONS[s - 4];
                assign_button(&mut self.config.buttons[p], s - 4, b);
            }
            (Kind::Joystick(_), None) => self.config.buttons[p] = DEFAULT_BUTTONS,
        }
        self.idle();
    }

    /// Uma tecla apertada (código VK) durante a espera: Esc cancela, as outras ficam.
    pub fn key(&mut self, vk: u8) {
        let Some(s) = self.capture else { return };
        if vk != 27 && self.kind == Kind::Keyboard {
            assign_key(&mut self.config.keys, self.player, s, vk);
        }
        self.idle();
    }

    /// Botões apertados do joystick (bit 0 = botão 1): o menor fica com a ação em espera.
    pub fn buttons(&mut self, pressed: u32) {
        let (Some(s), Kind::Joystick(_)) = (self.capture, self.kind) else {
            return;
        };
        if pressed != 0 {
            let b = pressed.trailing_zeros() as i32 + 1;
            assign_button(&mut self.config.buttons[self.player], s - 4, b);
            self.idle();
        }
    }
}

impl Page for KeyScreen {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn capturing(&self) -> bool {
        self.capture.is_some()
    }

    fn raw_key(&mut self, vk: u8) {
        self.key(vk);
    }

    fn joy_buttons(&mut self, pressed: u32) {
        self.buttons(pressed);
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        if self.capture.is_some() {
            match input {
                Input::Tick => {
                    self.blink.0 -= 1;
                    if self.blink.0 < 0 {
                        self.blink = (ids::BLINK - 1, !self.blink.1);
                    }
                    self.screen.handle(Input::Tick);
                }
                Input::PointerMove(..) => {
                    self.screen.handle(input);
                }
                // Ruling: um clique no RESET volta só esta ação; noutro lugar, cancela
                Input::PointerDown if self.screen.hit(ids::RESET) => self.reset(),
                Input::PointerDown | Input::Cancel => self.idle(),
                _ => {}
            }
            return None;
        }
        let id = self.screen.handle(input)?;
        let first = ids::ACTION + self.first() as u16;
        match id {
            ids::DONE | CANCEL => Some(Reply::Config(self.config.clone())),
            ids::RESET => {
                self.reset();
                None
            }
            ids::BACKGROUND => {
                self.screen.set_label(ids::CHARACTER, self.labels[10]);
                None
            }
            a if (first..ids::ACTION + KEYS as u16).contains(&a) => {
                self.start(usize::from(a - ids::ACTION));
                None
            }
            _ => None,
        }
    }

    fn draw(&self, fb: &mut Framebuffer) {
        self.screen.draw(fb);
        if let Kind::Joystick(n) = self.kind {
            if n < crate::devices::COMBO_BUTTONS {
                for l in ids::COMBO_TEXTS {
                    if let Some(f) = self.screen.label_frame(l) {
                        self.screen.draw_anim_frame(fb, f, 0, 0);
                    }
                }
            }
        }
        for s in self.first()..KEYS {
            let id = ids::NAME + s as u16;
            if self.capture == Some(s) && self.blink.1 {
                continue;
            }
            if self.screen.flags(id) & flags::HIDDEN != 0 {
                continue;
            }
            if let Some(r) = self.screen.rect(id) {
                draw_text_in(fb, &self.font, r, &self.name(s), ids::NAME_COLOR, true);
            }
        }
        if let Some(r) = self.screen.rect(ids::DESCRIPTION) {
            let h = i32::from(self.font.height) + 2;
            for (i, l) in wrap_text(&self.font, &self.description, r.w)
                .iter()
                .enumerate()
                .take((r.h / h.max(1)).max(1) as usize)
            {
                draw_text(
                    fb,
                    &self.font,
                    r.x,
                    r.y + i as i32 * h,
                    l,
                    Some(ids::DESCRIPTION_COLOR),
                );
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::devices::slot;
    use crate::shell::screen::Placement;
    use openhunted_formats::{Interface, parse_text_list};

    /// Tela montada dos dados reais (`OPENHUNTED_DATA`), com a fonte e os textos.
    pub(crate) fn real(itf: &str) -> Option<(Screen, Rc<Font>, Assets)> {
        let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
            eprintln!("pulado: OPENHUNTED_DATA não definido");
            return None;
        };
        let gd = openhunted_formats::open_data(std::path::Path::new(&p)).unwrap();
        let mut a = Assets::new(gd.resources().unwrap());
        let i = Interface::parse(a.archive().get(itf).unwrap()).unwrap();
        let s = Screen::new(&i, &mut a, Placement::Full).unwrap();
        let f = a.font("SLEVD.PFT").unwrap();
        Some((s, f, a))
    }

    fn texts(a: &Assets, name: &str) -> Vec<String> {
        parse_text_list(a.archive().get(name).unwrap())
    }

    fn click(k: &mut KeyScreen, id: u16) -> Option<Reply> {
        let r = k.screen.rect(id).unwrap();
        let mut snd = Vec::new();
        k.handle(Input::PointerMove(r.x + r.w / 2, r.y + r.h / 2), &mut snd);
        k.handle(Input::PointerDown, &mut snd);
        let mut out = None;
        for i in 0..6 {
            if i == 1 {
                k.handle(Input::PointerUp, &mut snd);
            }
            out = out.or(k.handle(Input::Tick, &mut snd));
        }
        out
    }

    #[test]
    fn skeyb_waits_for_a_key_swaps_resets_and_returns_the_config() {
        let Some((s, f, a)) = real("SKEYB.ITF") else {
            return;
        };
        let (names, descs) = (texts(&a, "KEYS.RES"), texts(&a, "SHELL.RES"));
        let mut k = KeyScreen::new(
            s,
            f,
            Kind::Keyboard,
            0,
            0,
            Config::default(),
            names,
            descs,
            false,
        );
        assert_eq!(k.name(slot::JUMP), "SHIFT", "o pular padrão do original");
        assert_eq!(k.name(slot::SHOOT), "CTRL");
        assert_eq!(k.description(), "Assign keyboard to game actions.");
        assert!(click(&mut k, ids::ACTION + slot::JUMP as u16).is_none());
        assert_eq!(k.capturing(), Some(slot::JUMP));
        assert!(k.description().contains("JUMP"));
        // pisca: o nome some e volta a cada 6 quadros
        let mut snd = Vec::new();
        for _ in 0..6 {
            k.handle(Input::Tick, &mut snd);
        }
        assert!(k.blink.1, "apagado");
        // Esc cancela sem mudar
        k.raw_key(27);
        assert_eq!(k.capturing(), None);
        assert_eq!(k.config.keys[0][slot::JUMP], 16);
        // Z (soco) no pular: troca
        click(&mut k, ids::ACTION + slot::JUMP as u16);
        k.raw_key(90);
        assert_eq!(k.name(slot::JUMP), "Z");
        assert_eq!(k.name(slot::PUNCH), "SHIFT");
        // a descrição do soco é a do soco (Ruling: o original a troca com a da próxima arma)
        click(&mut k, ids::ACTION + slot::PUNCH as u16);
        assert!(k.description().contains("PUNCH"));
        // RESET na espera: só esta ação (Shift volta ao pular por troca? não: o soco volta a Z)
        click(&mut k, ids::RESET);
        assert_eq!(k.capturing(), None);
        assert_eq!(k.config.keys[0][slot::PUNCH], 90);
        assert_eq!(k.config.keys[0][slot::JUMP], 16, "a troca devolve o Shift");
        // RESET fora da espera: tudo
        click(&mut k, ids::ACTION + slot::UP as u16);
        k.raw_key(87); // W
        click(&mut k, ids::RESET);
        assert_eq!(k.config.keys, DEFAULT_KEYS);
        match click(&mut k, ids::DONE) {
            Some(Reply::Config(c)) => assert_eq!(c.keys, DEFAULT_KEYS),
            r => panic!("{r:?}"),
        }
    }

    #[test]
    fn sjoys_takes_the_lowest_button_and_hides_weapons_with_two() {
        let Some((s, f, a)) = real("SJOYS.ITF") else {
            return;
        };
        let j = texts(&a, "JOYSTICK.RES");
        let mut k = KeyScreen::new(
            s,
            f,
            Kind::Joystick(2),
            1,
            0,
            Config::default(),
            j.clone(),
            j,
            true,
        );
        assert_eq!(k.name(slot::SHOOT), "Button 1");
        assert_ne!(
            k.screen.flags(17008) & flags::HIDDEN,
            0,
            "arma com 2 botões"
        );
        click(&mut k, ids::ACTION + slot::SHOOT as u16);
        assert_eq!(k.capturing(), Some(slot::SHOOT));
        k.raw_key(65); // tecla na espera do joystick: cancela
        assert_eq!(k.capturing(), None);
        click(&mut k, ids::ACTION + slot::SHOOT as u16);
        k.joy_buttons(0b110);
        assert_eq!(
            k.config.buttons[1],
            [2, 1, 3, 4, 5, 6],
            "o botão 2, com troca"
        );
        assert_eq!(k.config.buttons[0], DEFAULT_BUTTONS, "só o jogador da tela");
        let mut fb = Framebuffer::new(640, 480);
        k.draw(&mut fb);
    }
}
