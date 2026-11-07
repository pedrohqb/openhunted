// SPDX-License-Identifier: GPL-3.0-or-later
//! Dispositivos de entrada (`SINPU.ITF`, `0x43ea1f`; `docs/re/casca-acabamento.md`): o
//! grupo de cada jogador marca teclado ou Joystick 1–4; o mesmo dispositivo nos dois troca;
//! CUSTOMIZE leva às teclas ou aos botões.

use super::config::Config;
use super::page::{Page, Reply};
use super::screen::{CANCEL, Input, Screen, flags};
use crate::devices::{Device, resolve};

pub mod ids {
    /// Teclado e Joystick 1–4 do jogador 1 (29001–29005) e do 2 (29006–29010).
    pub const FIRST: [u16; 2] = [29001, 29006];
    pub const DONE: u16 = 29011;
    pub const FIRE: u16 = 29012;
    pub const CUSTOMIZE: [u16; 2] = [29013, 29014];
}

#[derive(Debug, Clone)]
pub struct DeviceScreen {
    pub screen: Screen,
    pub config: Config,
    joysticks: usize,
}

impl DeviceScreen {
    /// `joysticks`: quantos estão ligados (os outros não se escolhem).
    pub fn new(mut screen: Screen, config: Config, joysticks: usize) -> Self {
        for p in 0..2 {
            for i in 0..5u16 {
                let id = ids::FIRST[p] + i;
                screen.make_option(id);
                if usize::from(i) > joysticks {
                    // Ruling: o joystick ausente não se escolhe nem se navega
                    screen.clear_flags(id, flags::NAV);
                }
            }
        }
        for id in [ids::DONE, ids::CUSTOMIZE[0], ids::CUSTOMIZE[1]] {
            screen.set_flags(id, flags::UPDATE | flags::NAV);
        }
        screen.point_at(ids::DONE);
        let mut s = Self {
            screen,
            config,
            joysticks,
        };
        s.config.devices = s.devices().map(Some);
        s.mark();
        s
    }

    /// Os dispositivos em vigor (os gravados, acertados como o `0x43edc1`).
    pub fn devices(&self) -> [Device; 2] {
        resolve(self.config.devices, self.joysticks)
    }

    fn option(p: usize, d: Device) -> u16 {
        ids::FIRST[p]
            + match d {
                Device::Joystick(n) => u16::from(n),
                _ => 0,
            }
    }

    fn mark(&mut self) {
        for (p, d) in self.devices().into_iter().enumerate() {
            for i in 0..5 {
                let id = ids::FIRST[p] + i;
                self.screen.set_marked(id, id == Self::option(p, d));
            }
        }
    }

    /// `0x43e7af(jogador, dispositivo)`: o outro jogador, se tinha este, fica com o antigo.
    pub fn pick(&mut self, p: usize, d: Device) {
        let mut cur = self.devices();
        let d = match d {
            Device::Joystick(_) => d,
            _ => Device::keyboard_of(p),
        };
        let old = cur[p];
        cur[p] = d;
        if cur[1 - p] == d {
            cur[1 - p] = match old {
                Device::Joystick(_) => old,
                _ => Device::keyboard_of(1 - p),
            };
        }
        self.config.devices = cur.map(Some);
        self.mark();
    }
}

impl Page for DeviceScreen {
    fn screen(&self) -> &Screen {
        &self.screen
    }

    fn handle(&mut self, input: Input, _sounds: &mut Vec<u16>) -> Option<Reply> {
        let id = self.screen.handle(input)?;
        match id {
            ids::DONE | CANCEL => Some(Reply::Close),
            c if c == ids::CUSTOMIZE[0] => Some(Reply::Customize(0)),
            c if c == ids::CUSTOMIZE[1] => Some(Reply::Customize(1)),
            o => {
                let p = usize::from(o >= ids::FIRST[1]);
                let i = o.checked_sub(ids::FIRST[p]).filter(|&i| i < 5)?;
                if usize::from(i) > self.joysticks {
                    return None;
                }
                let d = if i == 0 {
                    Device::keyboard_of(p)
                } else {
                    Device::Joystick(i as u8)
                };
                self.pick(p, d);
                Some(Reply::Config(self.config.clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::keys::tests::real;

    fn click(d: &mut DeviceScreen, id: u16) -> Option<Reply> {
        let r = d.screen.rect(id).unwrap();
        let mut snd = Vec::new();
        d.handle(Input::PointerMove(r.x + r.w / 2, r.y + r.h / 2), &mut snd);
        d.handle(Input::PointerDown, &mut snd);
        let mut out = None;
        for i in 0..6 {
            if i == 1 {
                d.handle(Input::PointerUp, &mut snd);
            }
            out = out.or(d.handle(Input::Tick, &mut snd));
        }
        out
    }

    #[test]
    fn sinpu_marks_swaps_and_greys_missing_joysticks() {
        let Some((s, _, _)) = real("SINPU.ITF") else {
            return;
        };
        let mut d = DeviceScreen::new(s, Config::default(), 1);
        assert_eq!(d.devices(), [Device::Keyboard1, Device::Joystick(1)]);
        // Joystick 2 não existe: o clique não faz nada
        assert_eq!(click(&mut d, ids::FIRST[0] + 2), None);
        // o jogador 1 pega o Joystick 1: o 2 fica com o teclado dele
        match click(&mut d, ids::FIRST[0] + 1) {
            Some(Reply::Config(c)) => {
                assert_eq!(
                    c.devices,
                    [Some(Device::Joystick(1)), Some(Device::Keyboard2)]
                )
            }
            r => panic!("{r:?}"),
        }
        assert_eq!(click(&mut d, ids::CUSTOMIZE[0]), Some(Reply::Customize(0)));
        assert_eq!(click(&mut d, ids::DONE), Some(Reply::Close));
    }
}
