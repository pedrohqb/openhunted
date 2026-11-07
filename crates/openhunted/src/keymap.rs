// SPDX-License-Identifier: GPL-3.0-or-later
//! Teclas da SDL → códigos VK do Windows, os do `InputKeys1/2` do `HUNTER.INI` e das linhas
//! do `KEYS.RES` (`docs/re/casca-acabamento.md`). Ruling: pela posição da tecla (scancode,
//! teclado americano); Shift, Ctrl e Alt dos dois lados são 16, 17 e 18, como no original.

use sdl3::keyboard::Scancode;

/// (scancode, VK).
const TABLE: &[(Scancode, u8)] = {
    use Scancode as S;
    &[
        (S::Backspace, 8),
        (S::Tab, 9),
        (S::Return, 13),
        (S::KpEnter, 13),
        (S::LShift, 16),
        (S::RShift, 16),
        (S::LCtrl, 17),
        (S::RCtrl, 17),
        (S::LAlt, 18),
        (S::RAlt, 18),
        (S::Pause, 19),
        (S::CapsLock, 20),
        (S::Escape, 27),
        (S::Space, 32),
        (S::PageUp, 33),
        (S::PageDown, 34),
        (S::End, 35),
        (S::Home, 36),
        (S::Left, 37),
        (S::Up, 38),
        (S::Right, 39),
        (S::Down, 40),
        (S::PrintScreen, 44),
        (S::Insert, 45),
        (S::Delete, 46),
        (S::_0, 48),
        (S::_1, 49),
        (S::_2, 50),
        (S::_3, 51),
        (S::_4, 52),
        (S::_5, 53),
        (S::_6, 54),
        (S::_7, 55),
        (S::_8, 56),
        (S::_9, 57),
        (S::A, 65),
        (S::B, 66),
        (S::C, 67),
        (S::D, 68),
        (S::E, 69),
        (S::F, 70),
        (S::G, 71),
        (S::H, 72),
        (S::I, 73),
        (S::J, 74),
        (S::K, 75),
        (S::L, 76),
        (S::M, 77),
        (S::N, 78),
        (S::O, 79),
        (S::P, 80),
        (S::Q, 81),
        (S::R, 82),
        (S::S, 83),
        (S::T, 84),
        (S::U, 85),
        (S::V, 86),
        (S::W, 87),
        (S::X, 88),
        (S::Y, 89),
        (S::Z, 90),
        (S::Kp0, 96),
        (S::Kp1, 97),
        (S::Kp2, 98),
        (S::Kp3, 99),
        (S::Kp4, 100),
        (S::Kp5, 101),
        (S::Kp6, 102),
        (S::Kp7, 103),
        (S::Kp8, 104),
        (S::Kp9, 105),
        (S::KpMultiply, 106),
        (S::KpPlus, 107),
        (S::KpMinus, 109),
        (S::KpPeriod, 110),
        (S::KpDivide, 111),
        (S::F1, 112),
        (S::F2, 113),
        (S::F3, 114),
        (S::F4, 115),
        (S::F5, 116),
        (S::F6, 117),
        (S::F7, 118),
        (S::F8, 119),
        (S::F9, 120),
        (S::F10, 121),
        (S::F11, 122),
        (S::F12, 123),
        (S::NumLockClear, 144),
        (S::ScrollLock, 145),
        (S::Semicolon, 186),
        (S::Equals, 187),
        (S::Comma, 188),
        (S::Minus, 189),
        (S::Period, 190),
        (S::Slash, 191),
        (S::Grave, 192),
        (S::LeftBracket, 219),
        (S::Backslash, 220),
        (S::RightBracket, 221),
        (S::Apostrophe, 222),
    ]
};

/// Código VK da tecla (`None` = sem equivalente).
pub fn vk(sc: Scancode) -> Option<u8> {
    TABLE.iter().find(|(s, _)| *s == sc).map(|&(_, v)| v)
}

/// A tecla VK está segurada (alguma das teclas com esse código).
pub fn held(vk: u8, pressed: impl Fn(Scancode) -> bool) -> bool {
    TABLE.iter().any(|&(s, v)| v == vk && pressed(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_default_keys_have_a_scancode() {
        use openhunted_engine::devices::DEFAULT_KEYS;
        for k in DEFAULT_KEYS.iter().flatten() {
            assert!(TABLE.iter().any(|&(_, v)| v == *k), "VK {k}");
        }
        assert_eq!(vk(Scancode::RShift), Some(16));
        assert_eq!(vk(Scancode::Z), Some(90));
        assert_eq!(vk(Scancode::Application), None);
        assert!(held(16, |s| s == Scancode::LShift));
        assert!(!held(17, |s| s == Scancode::LShift));
    }
}
