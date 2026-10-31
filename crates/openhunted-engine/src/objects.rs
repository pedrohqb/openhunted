// SPDX-License-Identifier: GPL-3.0-or-later
//! Regras do original sobre os objetos da fase: presença por modo de jogo e jogador
//! (`0x004045a5`, `0x00436caf`). Ver `docs/re/objetos.md`.

use openhunted_formats::LevelObject;

/// Máscara do modo de jogo solo (Jake ou Garathe), `0x4972f8` = 0x2.
pub const SOLO: i32 = 0x2;

/// Sinalizador `+0x178`: presente em qualquer modo.
const ANY_MODE: i32 = 0x10;
/// Bit 0 dos modos: vaga de jogador (só existe com um jogador atribuído).
const PLAYER_SLOT: i32 = 0x1;

/// Sinalizadores do objeto (`+0x178`).
pub fn flags(o: &LevelObject) -> i32 {
    o.fields.get(3).copied().unwrap_or(0)
}

/// O objeto existe no modo `mode` sem ser uma vaga de jogador.
pub fn present(o: &LevelObject, mode: i32) -> bool {
    (o.modes & mode != 0 || flags(o) & ANY_MODE != 0) && o.modes & PLAYER_SLOT == 0
}

/// O objeto é o início do jogador no modo `mode` (vaga de jogador presente no modo).
pub fn is_player_start(o: &LevelObject, mode: i32) -> bool {
    o.modes & PLAYER_SLOT != 0 && o.modes & mode != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(flags: i32, modes: i32) -> LevelObject {
        LevelObject {
            kind: 1,
            x: 0,
            y: 0,
            fields: vec![0, 0, 0, flags, 0],
            start_label: None,
            modes,
            points: vec![],
            values: vec![],
            triggers: vec![],
            names: [String::new(), String::new()],
        }
    }

    #[test]
    fn presence_follows_mode_mask_and_any_mode_flag() {
        assert!(present(&obj(0x80089, 0xfe), SOLO));
        assert!(!present(&obj(0x80089, 0xd8), SOLO)); // só modos de vários jogadores
        assert!(present(&obj(0x80099, 0xd8), SOLO)); // 0x10: qualquer modo
        assert!(!present(&obj(0x94801, 0x3), SOLO)); // vaga de jogador
        assert!(!present(&obj(0x94801, 0x21), SOLO));
    }

    #[test]
    fn player_start_is_a_slot_in_the_mode() {
        assert!(is_player_start(&obj(0x94801, 0x3), SOLO));
        assert!(is_player_start(&obj(0x94801, 0x6f), SOLO));
        assert!(!is_player_start(&obj(0x94801, 0x21), SOLO));
        assert!(!is_player_start(&obj(0x94801, 0xfe), SOLO));
    }
}
