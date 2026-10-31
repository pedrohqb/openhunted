// SPDX-License-Identifier: GPL-3.0-or-later
//! Saída da fase (tipo 5, `0x0040e393`/`0x0040e4dd`; `docs/re/logica-fase.md`).

use crate::entity::Entity;

/// Sons da saída recusada: "MC Objectives not satisfied" e "Electric Fence Zap #1".
pub const SOUND_OBJECTIVES: u16 = 3772;
pub const SOUND_ZAP: u16 = 3135;
/// Dano elétrico da saída recusada.
pub const ZAP_DAMAGE: i32 = 20;
/// Tipo de zona PR diante da saída.
pub const EXIT_PROP: i16 = 0x40;

#[derive(Debug, Clone, Default)]
pub struct ExitState {}

/// Ligada, a alavanca anda até o fim do rótulo; desligada, volta até o começo
/// (`0x0040e4b4`).
pub fn tick(e: &mut Entity) {
    let on = e.on;
    if let Some(sp) = e.sprite.as_mut() {
        if on {
            sp.player.tick_once();
        } else {
            sp.player.tick_back();
        }
    }
}
