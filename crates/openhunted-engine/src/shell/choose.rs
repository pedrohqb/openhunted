// SPDX-License-Identifier: GPL-3.0-or-later
//! Escolher personagem e cor de um jogador da disputa (`SEGOC.ITF`, `0x4430a0` +
//! `0x443386`, retratos `0x4430c7`; `docs/re/menus-restantes.md`, `multijogador.md`).
//! A aplicação abre a tela uma vez por jogador; OK devolve a escolha, CANCEL fecha.

use std::rc::Rc;

use super::page::{Page, Reply};
use super::screen::{CANCEL, Input, Screen, flags};
use crate::character::Character;
use crate::devices::Device;
use crate::rng::Rng;

pub mod ids {
    pub const BACKGROUND: u16 = 15000;
    pub const OK: u16 = 15003;
    pub const CANCEL_BUTTON: u16 = 15004;
    pub const INPUT: u16 = 15009;
    /// Retratos do Jake e do Garathe.
    pub const PORTRAIT: [u16; 2] = [15010, 15011];
    pub const BROWN: u16 = 15012;
    pub const RED: u16 = 15013;
    pub const BLACK: u16 = 15014;
    pub const WHITE: u16 = 15015;
    /// Botões do Jake e do Garathe.
    pub const PICK: [u16; 2] = [15016, 15017];
    /// Botões navegáveis (`0x480428`).
    pub const NAV: [u16; 8] = [
        CANCEL_BUTTON,
        OK,
        WHITE,
        BLACK,
        RED,
        BROWN,
        PICK[0],
        PICK[1],
    ];
}

/// Botões de cor e a tabela `DLS.TBL` de cada (`0x44367d`…: marrom 36, vermelho 38, preto
/// 35, branco 37).
pub const COLOURS: [(u16, u8); 4] = [
    (ids::BROWN, 36),
    (ids::RED, 38),
    (ids::BLACK, 35),
    (ids::WHITE, 37),
];
/// Primeira tabela de cor dos jogadores (preto).
pub const FIRST_COLOUR: u8 = 35;
/// Rótulos "fade" dos retratos (`0x443524`, `0x443559`).
pub const FADE: [u16; 2] = [18, 19];
/// Rótulos de espera sorteados ao fim de cada um (`0x4803d4`: 13 do Jake; `0x480408`: 8 do
/// Garathe).
pub const IDLE_JAKE: [u16; 13] = [14, 34, 17, 34, 21, 34, 26, 34, 27, 34, 28, 34, 29];
pub const IDLE_GARATHE: [u16; 8] = [36, 30, 36, 31, 36, 32, 36, 33];
/// Rótulo do botão 15009 por jogador e dispositivo (`0x480394[7·p + dev]`, dev = teclado 1,
/// teclado 2, Joystick 1–4).
pub const DEVICE_LABELS: [[u16; 6]; 2] = [[10, 10, 6, 9, 8, 11], [12, 12, 22, 23, 24, 25]];

/// Padrões (`0x43d33b`): J1 Jake/35, J2 Garathe/36.
pub fn defaults(player: usize) -> (Character, u8) {
    if player == 0 {
        (Character::Jake, 35)
    } else {
        (Character::Garathe, 36)
    }
}

/// O que um retrato faz a cada quadro (`0x4803cc`/`0x4803d0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fade {
    /// Parado no quadro 0.
    Off,
    /// Para trás até o quadro 0 (o outro personagem foi escolhido).
    Out,
    /// Para a frente; no fim do rótulo, sorteia um de espera.
    In,
    /// Para trás até 0, troca a cor e volta a acender.
    Recolour,
}

#[derive(Debug, Clone)]
pub struct Choose {
    pub screen: Screen,
    pub player: usize,
    pick: Character,
    /// Cor de cada retrato (`0x48038c`, `0x480390`).
    colours: [u8; 2],
    /// Cor que o retrato aceso recebe ao apagar (`0x480384`).
    next_colour: u8,
    fades: [Fade; 2],
    /// Tabelas de cor 35–38 do `DLS.TBL`.
    tables: [Option<Rc<[u8; 256]>>; 4],
    rng: Rng,
}

fn index(c: Character) -> usize {
    match c {
        Character::Jake => 0,
        Character::Garathe => 1,
    }
}

fn device_index(d: Device) -> usize {
    match d {
        Device::Keyboard1 => 0,
        Device::Keyboard2 => 1,
        Device::Joystick(n) => usize::from(n.clamp(1, 4)) + 1,
    }
}

impl Choose {
    /// Tela do jogador `player` (0 ou 1) com o dispositivo dele e a escolha anterior (cor
    /// < 35 → 36, `0x4433d2`). `tables`: tabelas 35–38; `seed`: sorteio dos retratos.
    pub fn new(
        mut screen: Screen,
        player: usize,
        device: Device,
        (pick, colour): (Character, u8),
        tables: [Option<Rc<[u8; 256]>>; 4],
        seed: u32,
    ) -> Self {
        let colour = if colour < FIRST_COLOUR {
            36
        } else {
            colour.min(38)
        };
        let label = DEVICE_LABELS[player.min(1)][device_index(device)];
        screen.set_label(ids::INPUT, label);
        for id in [ids::OK, ids::CANCEL_BUTTON, ids::INPUT] {
            screen.set_flags(id, flags::UPDATE);
        }
        for id in ids::PICK.into_iter().chain(COLOURS.iter().map(|c| c.0)) {
            screen.make_option(id);
        }
        for id in ids::NAV {
            screen.set_flags(id, flags::NAV);
        }
        for (k, id) in ids::PORTRAIT.into_iter().enumerate() {
            screen.set_label(id, FADE[k]);
            screen.set_frame(id, 0);
        }
        let mut colours = [36, 36];
        colours[index(pick)] = colour;
        let mut c = Self {
            screen,
            player,
            pick,
            colours,
            next_colour: colour,
            fades: [Fade::Off; 2],
            tables,
            rng: Rng::new(seed),
        };
        c.fades[index(pick)] = Fade::In;
        c.mark();
        let id = ids::PICK[index(pick)];
        c.screen.point_at(id);
        c
    }

    pub fn character(&self) -> Character {
        self.pick
    }

    /// Cor escolhida. Ruling: a do botão marcado, mesmo com o retrato ainda trocando de cor
    /// (o original grava a do retrato, `0x4437cf`, que só muda no fim do apagar).
    pub fn colour(&self) -> u8 {
        self.next_colour
    }

    fn table(&self, colour: u8) -> Option<Rc<[u8; 256]>> {
        self.tables
            .get(usize::from(colour.saturating_sub(FIRST_COLOUR)))
            .cloned()
            .flatten()
    }

    /// Marca o personagem e a cor dele; os retratos com as cores deles.
    fn mark(&mut self) {
        for (k, id) in ids::PICK.into_iter().enumerate() {
            self.screen.set_marked(id, k == index(self.pick));
        }
        let want = self.next_colour;
        for (id, c) in COLOURS {
            self.screen.set_marked(id, c == want);
        }
        for k in 0..2 {
            let t = self.table(self.colours[k]);
            self.screen.set_remap(ids::PORTRAIT[k], t);
        }
    }

    /// Escolhe o personagem (`0x4438dd`/`0x44386b`): o outro apaga do fim do fade.
    fn pick(&mut self, c: Character) {
        if c == self.pick {
            return;
        }
        let old = index(self.pick);
        self.screen.set_label(ids::PORTRAIT[old], FADE[old]);
        let last = self.screen.frame(ids::PORTRAIT[old]).map_or(0, |f| f.1);
        self.screen.set_frame(ids::PORTRAIT[old], last);
        self.fades[old] = Fade::Out;
        self.pick = c;
        let new = index(c);
        if self.fades[new] == Fade::Off {
            self.fades[new] = Fade::In;
        }
        self.next_colour = self.colours[new];
        self.mark();
    }

    /// Escolhe a cor (`0x44367d`…): o retrato aceso apaga, troca de cor e acende.
    fn recolour(&mut self, colour: u8) {
        let k = index(self.pick);
        self.next_colour = colour;
        if self.colours[k] != colour {
            self.fades[k] = Fade::Recolour;
        }
        self.mark();
    }

    /// Um quadro dos retratos (`0x4430c7`).
    fn animate(&mut self) {
        for (k, id) in ids::PORTRAIT.into_iter().enumerate() {
            let Some((f, n)) = self.screen.frame(id) else {
                continue;
            };
            match self.fades[k] {
                Fade::Off => {}
                Fade::Out | Fade::Recolour if f > 0 => self.screen.set_frame(id, f - 1),
                Fade::Out => self.fades[k] = Fade::Off,
                Fade::Recolour => {
                    self.colours[k] = self.next_colour;
                    self.fades[k] = Fade::In;
                    self.screen.set_label(id, FADE[k]);
                    self.screen.set_frame(id, 0);
                    self.mark();
                }
                Fade::In if f + 1 < n => self.screen.set_frame(id, f + 1),
                Fade::In => {
                    let label = if k == 0 {
                        IDLE_JAKE[self.rng.below(13) as usize]
                    } else {
                        IDLE_GARATHE[self.rng.below(8) as usize]
                    };
                    self.screen.set_label(id, label);
                    self.screen.set_frame(id, 0);
                }
            }
        }
    }
}

impl Page for Choose {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        if input == Input::Tick {
            self.animate();
        }
        match self.screen.handle(input)? {
            ids::OK => Some(Reply::Chosen(self.pick, self.colour())),
            ids::CANCEL_BUTTON | CANCEL => Some(Reply::Close),
            ids::INPUT => Some(Reply::Devices),
            id if id == ids::PICK[0] => {
                self.pick(Character::Jake);
                None
            }
            id if id == ids::PICK[1] => {
                self.pick(Character::Garathe);
                None
            }
            id => {
                if let Some(&(_, c)) = COLOURS.iter().find(|c| c.0 == id) {
                    self.recolour(c);
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_device_labels() {
        assert_eq!(defaults(0), (Character::Jake, 35));
        assert_eq!(defaults(1), (Character::Garathe, 36));
        assert_eq!(DEVICE_LABELS[0][device_index(Device::Keyboard1)], 10);
        assert_eq!(DEVICE_LABELS[1][device_index(Device::Keyboard2)], 12);
        assert_eq!(DEVICE_LABELS[1][device_index(Device::Joystick(1))], 22);
        assert_eq!(DEVICE_LABELS[0][device_index(Device::Joystick(4))], 11);
    }
}
