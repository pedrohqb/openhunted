// SPDX-License-Identifier: GPL-3.0-or-later
//! Dispositivos de entrada dos jogadores (`docs/re/casca-acabamento.md`): teclado com as 10
//! teclas do `HUNTER.INI` (`InputKeys1/2`) ou joystick com o mapa de 6 botões
//! (`Joystick1/2`), e a conversão do estado do dispositivo nos bits de `crate::input`
//! (`0x407e2d`).

use crate::input::{DOWN, JUMP, LEFT, NEXT_WEAPON, PREV_WEAPON, PUNCH, RIGHT, SHOOT, STEALTH, UP};

/// Ações na ordem dos botões 17001–17010 do `SKEYB`.
pub mod slot {
    pub const UP: usize = 0;
    pub const DOWN: usize = 1;
    pub const LEFT: usize = 2;
    pub const RIGHT: usize = 3;
    pub const SHOOT: usize = 4;
    pub const JUMP: usize = 5;
    pub const STEALTH: usize = 6;
    pub const PREV_WEAPON: usize = 7;
    pub const PUNCH: usize = 8;
    pub const NEXT_WEAPON: usize = 9;
}

/// Teclas por jogador.
pub const KEYS: usize = 10;
/// Botões do joystick por jogador: atirar, pular, furtivo, anterior, soco, próxima (as
/// ações 4–9).
pub const BUTTONS: usize = 6;
/// Padrões (`0x480020`, `0x480070`): códigos VK do Windows.
pub const DEFAULT_KEYS: [[u8; KEYS]; 2] = [
    [38, 40, 37, 39, 17, 16, 88, 33, 90, 34],
    [73, 77, 74, 75, 32, 13, 186, 45, 191, 46],
];
/// Padrão dos dois jogadores (`0x480224`, `0x48023c`): botões 1–6.
pub const DEFAULT_BUTTONS: [i32; BUTTONS] = [1, 2, 3, 4, 5, 6];
/// Limiares dos eixos (−128…127): direção e "meia inclinação anda" (`0x407e5c`, `0x407f91`).
pub const AXIS_DIR: i16 = 50;
pub const AXIS_WALK: i16 = 100;
/// Espera depois da troca de arma pela combinação com o pulo (`0x487a30`), em tiques.
pub const COMBO_WAIT: i32 = 20;
/// Com menos botões que isso, pular + BAIXO + lado troca de arma (`0x407e95`).
pub const COMBO_BUTTONS: u32 = 4;

/// Dispositivo de um jogador (`PlayerNInput`, tabela `0x47f4c0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Keyboard1,
    Keyboard2,
    /// `Joystick1`…`Joystick4` (1–4).
    Joystick(u8),
}

impl Device {
    /// Valor no `HUNTER.INI`.
    pub fn ini(self) -> String {
        match self {
            Device::Keyboard1 => "Keyboard1".into(),
            Device::Keyboard2 => "Keyboard2".into(),
            Device::Joystick(n) => format!("Joystick{n}"),
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "keyboard1" => Some(Device::Keyboard1),
            "keyboard2" => Some(Device::Keyboard2),
            j => j
                .strip_prefix("joystick")?
                .parse::<u8>()
                .ok()
                .filter(|n| (1..=4).contains(n))
                .map(Device::Joystick),
        }
    }

    pub fn is_keyboard(self) -> bool {
        !matches!(self, Device::Joystick(_))
    }

    /// O teclado do jogador `p` (0 ou 1): o 1 usa sempre `Keyboard1`, o 2 `Keyboard2`
    /// (`0x43e83f`, `0x43e8d9`).
    pub fn keyboard_of(p: usize) -> Self {
        if p == 0 {
            Device::Keyboard1
        } else {
            Device::Keyboard2
        }
    }
}

/// Dispositivos dos dois jogadores como o `0x43edc1` os acerta: ausente → padrão (jogador 1
/// teclado; jogador 2 `Joystick1` se existe e o 1 não o usa, senão teclado), teclado do
/// outro → o próprio, joystick que não existe → o teclado, iguais → o 2 fica no teclado.
pub fn resolve(devices: [Option<Device>; 2], joysticks: usize) -> [Device; 2] {
    let fix = |p: usize, d: Device| match d {
        Device::Joystick(n) if usize::from(n) > joysticks => Device::keyboard_of(p),
        Device::Joystick(_) => d,
        _ => Device::keyboard_of(p),
    };
    let a = fix(0, devices[0].unwrap_or(Device::Keyboard1));
    let b = match devices[1] {
        Some(d) => fix(1, d),
        None if joysticks >= 1 && a != Device::Joystick(1) => Device::Joystick(1),
        None => Device::Keyboard2,
    };
    [a, if a == b { Device::Keyboard2 } else { b }]
}

/// `0x442537`: dá a tecla `vk` à ação `slot` do jogador `p`; em cada jogador, a ação que já
/// usava `vk` recebe a tecla antiga desta ação.
pub fn assign_key(keys: &mut [[u8; KEYS]; 2], p: usize, slot: usize, vk: u8) {
    let old = keys[p][slot];
    for k in keys.iter_mut().flatten() {
        if *k == vk {
            *k = old;
        }
    }
    keys[p][slot] = vk;
}

/// `0x442f37`: o mesmo para os botões do joystick, só no mapa do jogador.
pub fn assign_button(map: &mut [i32; BUTTONS], slot: usize, button: i32) {
    let old = map[slot];
    for b in map.iter_mut() {
        if *b == button {
            *b = old;
        }
    }
    map[slot] = button;
}

/// Estado de um dispositivo num tique: eixos −128…127 e as seis ações de botão na ordem
/// do dispositivo (`+0x98…+0x9d`: atirar, pular, furtivo, **próxima**, **anterior**, soco).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceState {
    pub x: i16,
    pub y: i16,
    pub buttons: [bool; 6],
    /// Botões do dispositivo (`+0x04`); o teclado conta 6.
    pub count: u32,
}

impl DeviceState {
    /// Teclado (`0x448503`): eixos ±128 pelas teclas de direção, botões pelas outras.
    pub fn keyboard(keys: &[u8; KEYS], pressed: impl Fn(u8) -> bool) -> Self {
        let k = |s: usize| pressed(keys[s]);
        let axis = |neg: bool, pos: bool| 128 * i16::from(pos) - 128 * i16::from(neg);
        Self {
            x: axis(k(slot::LEFT), k(slot::RIGHT)).min(127),
            y: axis(k(slot::UP), k(slot::DOWN)).min(127),
            buttons: [
                k(slot::SHOOT),
                k(slot::JUMP),
                k(slot::STEALTH),
                k(slot::NEXT_WEAPON),
                k(slot::PREV_WEAPON),
                k(slot::PUNCH),
            ],
            count: BUTTONS as u32,
        }
    }

    /// Joystick (`0x4483c4`): eixos já em −128…127, `pressed` = bits dos botões (bit 0 =
    /// botão 1) e o mapa de 6 botões do jogador (1-base, na ordem do `SJOYS`: atirar, pular,
    /// furtivo, anterior, soco, próxima). Mapa fora dos botões → o bit da própria posição.
    pub fn joystick(x: i16, y: i16, pressed: u32, count: u32, map: &[i32; BUTTONS]) -> Self {
        // ordem do dispositivo: atirar, pular, furtivo, próxima (map[5]), anterior (map[3]),
        // soco (map[4]) (`0x43efde`)
        let order = [map[0], map[1], map[2], map[5], map[3], map[4]];
        let mut buttons = [false; 6];
        for (i, (b, &m)) in buttons.iter_mut().zip(&order).enumerate() {
            let bit = u32::try_from(m - 1)
                .ok()
                .filter(|&n| n < count)
                .unwrap_or(i as u32);
            *b = bit < 32 && pressed & (1 << bit) != 0;
        }
        Self {
            x,
            y,
            buttons,
            count,
        }
    }
}

/// Conversão do estado em bits, com a espera da combinação de pulo (`0x487a30`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputReader {
    wait: i32,
}

impl InputReader {
    /// Bits de entrada de um tique (`0x407e2d`, só os bits segurados).
    pub fn bits(&mut self, s: &DeviceState) -> u32 {
        let mut s = *s;
        let mut bits = 0;
        if s.x < -AXIS_DIR {
            bits |= LEFT;
        } else if s.x > AXIS_DIR {
            bits |= RIGHT;
        }
        if s.y < -AXIS_DIR {
            bits |= UP;
        } else if s.y > AXIS_DIR {
            bits |= DOWN;
        }
        if s.count < COMBO_BUTTONS {
            let jump = s.buttons[1] && bits & DOWN != 0;
            s.buttons[3] = jump && bits & RIGHT != 0;
            s.buttons[4] = jump && bits & LEFT != 0;
            if s.buttons[3] || s.buttons[4] {
                self.wait = COMBO_WAIT;
            } else if !s.buttons[1] {
                self.wait = 0;
            }
            if self.wait > 0 {
                self.wait -= 1;
                s.buttons[1] = false;
            }
        }
        for (on, bit) in
            s.buttons
                .iter()
                .zip([SHOOT, JUMP, STEALTH, NEXT_WEAPON, PREV_WEAPON, PUNCH])
        {
            if *on {
                bits |= bit;
            }
        }
        // meia inclinação: anda
        let half_x = bits & (LEFT | RIGHT) != 0 && s.x.abs() <= AXIS_WALK;
        let half_y = bits & (UP | DOWN) != 0 && s.y.abs() <= AXIS_WALK;
        if half_x || half_y {
            bits |= STEALTH;
        }
        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_names_and_resolution() {
        for d in [
            Device::Keyboard1,
            Device::Keyboard2,
            Device::Joystick(1),
            Device::Joystick(4),
        ] {
            assert_eq!(Device::parse(&d.ini()), Some(d));
        }
        assert_eq!(Device::parse("joystick5"), None);
        assert_eq!(Device::parse("Mouse"), None);
        use Device::*;
        assert_eq!(resolve([None, None], 0), [Keyboard1, Keyboard2]);
        assert_eq!(resolve([None, None], 1), [Keyboard1, Joystick(1)]);
        assert_eq!(
            resolve([Some(Joystick(1)), None], 1),
            [Joystick(1), Keyboard2]
        );
        assert_eq!(
            resolve([Some(Keyboard2), Some(Keyboard1)], 0),
            [Keyboard1, Keyboard2]
        );
        assert_eq!(
            resolve([Some(Joystick(2)), None], 1),
            [Keyboard1, Joystick(1)]
        );
        assert_eq!(
            resolve([Some(Joystick(1)), Some(Joystick(1))], 2),
            [Joystick(1), Keyboard2]
        );
    }

    #[test]
    fn assigning_a_used_key_swaps_in_both_players() {
        let mut k = DEFAULT_KEYS;
        // Z (soco do 1) no pular do 1: o soco fica com o Shift
        assign_key(&mut k, 0, slot::JUMP, 90);
        assert_eq!(k[0][slot::JUMP], 90);
        assert_eq!(k[0][slot::PUNCH], 16);
        // espaço (atirar do 2) no atirar do 1: o 2 recebe o Ctrl
        assign_key(&mut k, 0, slot::SHOOT, 32);
        assert_eq!(k[0][slot::SHOOT], 32);
        assert_eq!(k[1][slot::SHOOT], 17);
        // a mesma tecla de novo: nada muda
        let before = k;
        assign_key(&mut k, 0, slot::SHOOT, 32);
        assert_eq!(k, before);
        let mut m = DEFAULT_BUTTONS;
        assign_button(&mut m, 0, 3);
        assert_eq!(m, [3, 2, 1, 4, 5, 6]);
    }

    #[test]
    fn keyboard_gives_full_axes_without_walking() {
        let held = [37u8, 16, 33];
        let s = DeviceState::keyboard(&DEFAULT_KEYS[0], |vk| held.contains(&vk));
        assert_eq!((s.x, s.y), (-128, 0));
        let mut r = InputReader::default();
        assert_eq!(r.bits(&s), LEFT | JUMP | PREV_WEAPON);
        let s = DeviceState::keyboard(&DEFAULT_KEYS[0], |vk| [39, 40, 34, 90].contains(&vk));
        assert_eq!(r.bits(&s), RIGHT | DOWN | NEXT_WEAPON | PUNCH);
    }

    /// Um joystick falso: eixos, botões pelo mapa, limiar 50, meia inclinação, combinação.
    #[test]
    fn fake_joystick_maps_axes_and_buttons_like_the_original() {
        let mut r = InputReader::default();
        let j = |x, y, b, n| DeviceState::joystick(x, y, b, n, &DEFAULT_BUTTONS);
        assert_eq!(r.bits(&j(-50, 50, 0, 6)), 0, "no limiar não conta");
        assert_eq!(r.bits(&j(-128, 127, 0, 6)), LEFT | DOWN);
        assert_eq!(
            r.bits(&j(80, 0, 0, 6)),
            RIGHT | STEALTH,
            "meia inclinação anda"
        );
        // botões 1–6: atirar, pular, furtivo, anterior, soco, próxima
        for (b, bit) in [SHOOT, JUMP, STEALTH, PREV_WEAPON, PUNCH, NEXT_WEAPON]
            .into_iter()
            .enumerate()
        {
            assert_eq!(r.bits(&j(0, 0, 1 << b, 6)), bit, "botão {}", b + 1);
        }
        // mapa trocado: o botão 3 atira; fora dos botões vale o bit da posição
        let m = [3, 2, 9, 4, 5, 6];
        let s = DeviceState::joystick(0, 0, 0b100, 4, &m);
        assert_eq!(
            r.bits(&s),
            SHOOT | STEALTH,
            "furtivo fora: o bit 2, o da posição"
        );
        let s = DeviceState::joystick(0, 0, 0b1, 4, &m);
        assert_eq!(r.bits(&s), 0, "o botão 1 não tem ação");
        // dois botões: pular + BAIXO + DIREITA troca de arma e segura o pulo 20 tiques
        let combo = j(127, 127, 0b10, 2);
        assert_eq!(r.bits(&combo), RIGHT | DOWN | NEXT_WEAPON);
        let hold = j(0, 0, 0b10, 2);
        for _ in 0..(COMBO_WAIT - 1) {
            assert_eq!(r.bits(&hold), 0, "pulo segurado");
        }
        assert_eq!(r.bits(&hold), JUMP);
        assert_eq!(r.bits(&j(-127, 127, 0b10, 2)), LEFT | DOWN | PREV_WEAPON);
        // soltar o pulo encerra a espera
        r.bits(&j(0, 0, 0, 2));
        assert_eq!(r.bits(&j(0, 0, 0b10, 2)), JUMP);
    }
}
