// SPDX-License-Identifier: GPL-3.0-or-later
//! Explosão (tipo 86, `0x0040d20c`).

#[derive(Debug, Clone, Default)]
pub struct ExplosionState {
    pub applied: bool,
}
