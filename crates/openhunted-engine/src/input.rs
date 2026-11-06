// SPDX-License-Identifier: GPL-3.0-or-later
//! Bits de entrada do jogador, como no original (`0x00407e2d`, máscara em `0x00487a2c`).

pub const UP: u32 = 0x1;
pub const DOWN: u32 = 0x2;
pub const LEFT: u32 = 0x4;
pub const RIGHT: u32 = 0x8;
pub const SHOOT: u32 = 0x100;
pub const JUMP: u32 = 0x200;
pub const STEALTH: u32 = 0x400;
/// Soco (tecla Z). O soco ainda não existe: o bit chega ao Jake e é ignorado.
pub const PUNCH: u32 = 0x40000;
