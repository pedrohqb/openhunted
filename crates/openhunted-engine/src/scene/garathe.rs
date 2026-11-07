// SPDX-License-Identifier: GPL-3.0-or-later
//! O Garathe na cena (M8): sons pedidos pelo personagem e a investida do pulo longo
//! (`0x429bda`, `docs/re/garathe-movimentos.md`).

use super::Scene;
use crate::behaviour::{self, Event};
use crate::character::Character;
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
        // investida: quem ele toca (objeto sólido, não só de toque) leva 7 e tranco, com o
        // empurrão pelos centros; Ruling: uma vez por objeto por pulo
        let b = j.body.bounds();
        if !self.parked.is_empty() {
            // o outro jogador (marcado como `usize::MAX − índice` na lista do pulo)
            let checked = self.body_checked.clone();
            let mut hit = Vec::new();
            let others: Vec<usize> = self.parked.iter().map(|s| s.player).collect();
            for k in others {
                if checked.contains(&(usize::MAX - k)) {
                    continue;
                }
                let r = self
                    .player_jake(k)
                    .filter(|o| o.is_active() && o.layer() == layer);
                if let Some(r) = r.map(|o| o.body.bounds()).filter(|r| r.overlaps(&b)) {
                    hit.push((k, r));
                }
            }
            for (k, r) in hit {
                self.body_checked.push(usize::MAX - k);
                if let Some(o) = self.player_jake_mut(k) {
                    o.hurt(garathe::body_check(b, r));
                }
                let c = self.active_number();
                self.credit_player(k, c);
            }
        }
        let Some(j) = self.jake.as_ref() else { return };
        let b = j.body.bounds();
        for (i, e) in self.entities.iter_mut().enumerate() {
            let Some(r) = e.bounds() else { continue };
            if !e.alive
                || e.layer != layer
                || e.flags & 0x80001 != 0x80001
                || e.flags & 0x8020 != 0
                || self.body_checked.contains(&i)
                || !r.overlaps(&b)
            {
                continue;
            }
            self.body_checked.push(i);
            if self.players.len() >= 2 && e.health > 0 {
                if self.credits.len() <= i {
                    self.credits.resize(i + 1, 0);
                }
                self.credits[i] = self.players.get(self.active).map_or(0, |p| p.number);
            }
            let p = garathe::body_check(b, r);
            // as mortes contam pela cena (`enemy::kills`)
            if !behaviour::enemy::hurt(e, &p, true) {
                behaviour::interact::hurt(e, &p);
            }
        }
    }
}
