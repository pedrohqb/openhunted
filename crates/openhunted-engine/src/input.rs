// SPDX-License-Identifier: GPL-3.0-or-later
//! Bits de entrada do jogador, como no original (`0x00407e2d`, máscara em `0x00487a2c`).

pub const UP: u32 = 0x1;
pub const DOWN: u32 = 0x2;
pub const LEFT: u32 = 0x4;
pub const RIGHT: u32 = 0x8;
pub const SHOOT: u32 = 0x100;
pub const JUMP: u32 = 0x200;
pub const STEALTH: u32 = 0x400;
/// Arma anterior / próxima (PgUp / PgDn no padrão do jogador 1).
pub const PREV_WEAPON: u32 = 0x800;
pub const NEXT_WEAPON: u32 = 0x1000;
/// Soco (tecla Z).
pub const PUNCH: u32 = 0x40000;
/// Extensão do port: escolha direta da arma (1–6) nos bits 24–26 (`arma << SELECT_SHIFT`).
pub const SELECT_SHIFT: u32 = 24;
pub const SELECT_MASK: u32 = 0x0700_0000;

/// Troca CIMA↔BAIXO e ESQUERDA↔DIREITA (confusão do jogador, `+0x264`).
pub fn swap_directions(input: u32) -> u32 {
    let pair = |a: u32, b: u32| {
        let (ha, hb) = (input & a != 0, input & b != 0);
        (if hb { a } else { 0 }) | (if ha { b } else { 0 })
    };
    (input & !(UP | DOWN | LEFT | RIGHT)) | pair(UP, DOWN) | pair(LEFT, RIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_directions_inverts_both_pairs() {
        assert_eq!(swap_directions(LEFT | SHOOT), RIGHT | SHOOT);
        assert_eq!(swap_directions(UP | RIGHT), DOWN | LEFT);
        assert_eq!(swap_directions(LEFT | RIGHT), LEFT | RIGHT);
        assert_eq!(swap_directions(JUMP), JUMP);
    }
}
