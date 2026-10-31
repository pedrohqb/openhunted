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

/// O objeto `other` barra quem se move com os sinalizadores `mover` (`0x00436caf`): o outro
/// tem o bit 0 e não o bit 1, não é só de contato (0x20) e o par passa pelas regras de
/// exclusão. O sorteio do par 0x40000 × 0x40000 conta como "não barra". Presença e camada são
/// verificadas por quem chama.
pub fn blocks(mover: i32, other: &LevelObject) -> bool {
    blocks_flags(mover, flags(other))
}

/// `blocks` sobre os sinalizadores do outro.
pub fn blocks_flags(mover: i32, o: i32) -> bool {
    let m = mover;
    let both = |bit: i32| m & bit != 0 && o & bit != 0;
    let (actor, ghost, phase) = (0x4000, 0x10000, 0x40000);
    let excluded = both(0x2000)
        || both(actor)
        || (m & ghost != 0 && o & ghost == 0 && o & actor != 0)
        || (o & ghost != 0 && m & ghost == 0 && m & actor != 0)
        || (m & phase != 0 && o & actor != 0)
        || (m & actor != 0 && o & phase != 0)
        || both(phase);
    o & 3 == 1 && o & 0x20 == 0 && !excluded
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

    const JAKE: i32 = 0x94801;

    #[test]
    fn solid_objects_block_jake() {
        assert!(blocks(JAKE, &obj(0x80089, 0xfe))); // peças OPL, plataformas OPO
        assert!(blocks(JAKE, &obj(0x80801, 0xfe))); // OHS, minas OHM
        assert!(!blocks(JAKE, &obj(0x84801, 0xfe))); // personagens (0x4000 nos dois)
        assert!(!blocks(JAKE, &obj(0x8002b, 0xfe))); // itens OGF (bit 1)
        assert!(!blocks(JAKE, &obj(0x80021, 0xfe))); // contato (0x20)
        assert!(!blocks(JAKE, &obj(0x80080, 0xfe))); // sem o bit 0
    }

    #[test]
    fn pair_rules_on_flags() {
        let o = |f| obj(f, 0xfe);
        assert!(!blocks(0x2001, &o(0x2001))); // 0x2000 nos dois
        assert!(blocks(0x2001, &o(0x0001)));
        assert!(!blocks(0x10001, &o(0x4001))); // m 0x10000, o 0x4000 sem 0x10000
        assert!(blocks(0x10001, &o(0x14001))); // os dois com 0x10000: a regra não se aplica
        assert!(!blocks(0x4001, &o(0x10001))); // o 0x10000, m 0x4000 sem 0x10000
        assert!(!blocks(0x40001, &o(0x4001))); // m 0x40000, o 0x4000
        assert!(!blocks(0x4001, &o(0x40001))); // m 0x4000, o 0x40000
        assert!(!blocks(0x40001, &o(0x40001))); // sorteio no original: não barra aqui
        assert!(blocks(0x10001, &o(0x0001)));
    }

    #[test]
    fn player_start_is_a_slot_in_the_mode() {
        assert!(is_player_start(&obj(0x94801, 0x3), SOLO));
        assert!(is_player_start(&obj(0x94801, 0x6f), SOLO));
        assert!(!is_player_start(&obj(0x94801, 0x21), SOLO));
        assert!(!is_player_start(&obj(0x94801, 0xfe), SOLO));
    }
}
