// SPDX-License-Identifier: GPL-3.0-or-later
//! O Garathe na cena (M8): sons pedidos pelo personagem e a investida do pulo longo
//! (`0x429bda`, `docs/re/garathe-movimentos.md`).

use super::Scene;
use crate::behaviour::{self, Event};
use crate::character::Character;
use crate::damage::Pending;
use crate::entity::Behaviour;
use crate::jake::{JumpKind, State, garathe};
use crate::sound::{Cue, PRIORITY_FRAME};

impl Scene {
    /// Depois do tique do jogador: o som posicional que ele pediu e a investida.
    pub(super) fn tick_player_extras(&mut self) {
        let layer = self.jake_layer;
        let Some(j) = self.jake.as_mut() else { return };
        if let Some(id) = j.sound.take() {
            let b = j.body.bounds();
            let c = (b.x + b.w / 2, b.y + b.h / 2);
            self.events
                .push(Event::Sound(Cue::at(id, PRIORITY_FRAME, c, layer)));
        }
        if j.character() != Character::Garathe || j.state() != State::Jump(JumpKind::Long) {
            self.body_checked.clear();
            return;
        }
        // investida: quem ele toca (objeto sólido, não só de toque) leva 7, tranco e
        // empurrão pelo lado; Ruling: uma vez por objeto por pulo
        let b = j.body.bounds();
        let dir = if j.facing_right() { 1 } else { -1 };
        let p = Pending {
            amount: garathe::BODY_CHECK_DAMAGE,
            jolt: true,
            shove: true,
            push: (dir * garathe::BODY_CHECK_PUSH, 0),
            ..Default::default()
        };
        for (i, e) in self.entities.iter_mut().enumerate() {
            if !e.alive
                || e.layer != layer
                || e.flags & 0x80001 != 0x80001
                || e.flags & 0x8020 != 0
                || self.body_checked.contains(&i)
                || !e.bounds().is_some_and(|r| r.overlaps(&b))
            {
                continue;
            }
            self.body_checked.push(i);
            let mine_alive = matches!(e.behaviour, Behaviour::Mine(_)) && e.health > 0;
            if (behaviour::enemy::hurt(e, &p, true) || behaviour::interact::hurt(e, &p))
                && mine_alive
                && e.health <= 0
            {
                self.kills += 1;
            }
        }
    }
}
