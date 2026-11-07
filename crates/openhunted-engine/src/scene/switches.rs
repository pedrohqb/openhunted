// SPDX-License-Identifier: GPL-3.0-or-later
//! CIMA nos interruptores (tipo 74, `0x0040e4dd`; `docs/re/logica-fase.md`).

use super::Scene;
use crate::entity::Behaviour;
use crate::jake::zone;

impl Scene {
    /// O Jake parado sobre uma zona PR 0x40 inverte os interruptores da camada cuja caixa ele
    /// toca (desigualdades não estritas), como a saída.
    pub(super) fn try_switch(&mut self) {
        let Some(j) = self.jake.as_ref().filter(|j| j.can_use_exit()) else {
            return;
        };
        let jb = j.body.bounds();
        let in_zone = self.prop_zones.iter().any(|z| {
            z.layer == self.jake_layer && z.kind & zone::EXIT != 0 && z.rect.overlaps(&jb)
        });
        if !in_zone {
            return;
        }
        let (layer, tick) = (self.jake_layer, self.tick_count);
        for e in self.entities.iter_mut() {
            if !(e.alive && e.layer == layer && matches!(e.behaviour, Behaviour::Switch(_))) {
                continue;
            }
            let touching = e.bounds().is_some_and(|b| {
                jb.x <= b.x + b.w && jb.x + jb.w >= b.x && jb.y <= b.y + b.h && jb.y + jb.h >= b.y
            });
            if touching {
                crate::behaviour::switch::toggle(e, tick);
            }
        }
    }
}
