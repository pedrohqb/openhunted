// SPDX-License-Identifier: GPL-3.0-or-later
//! Arsenal do Jake na cena (M7b): projéteis das armas 3–6 e o soco
//! (`docs/re/armas-2.md`, `furtividade.md`).

use super::Scene;
use crate::behaviour::{self, Event};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::sound::{Cue, PRIORITY_FRAME};

/// Linha `(1, 0, 0)` do `DAMAGE.TBL` (soco do Jake): 8 de dano, empurrão (2, 0), som 5062.
pub const PUNCH_DAMAGE: i32 = 8;
pub const PUNCH_PUSH: i32 = 2;
pub const SOUND_PUNCH: u16 = 5062;

impl Scene {
    /// Entidade de um projétil das armas 3–6 (chumbo, foguete, estrela, granada, cartucho).
    pub(super) fn weapon_entity(&mut self, _s: &crate::jake::Shot, _layer: i32) -> Option<Entity> {
        None
    }

    /// Acerto do soco (`0x0043afe1`): cada objeto da camada (presente, `0x80001`, sem
    /// `0x8000`) cuja caixa cruza a do golpe leva a linha `(1, 0, 0)`; só quem trata dano
    /// (`interact::hurt`) reage.
    pub(super) fn apply_punch(&mut self, r: Rect) {
        let right = self.jake.as_ref().is_some_and(|j| j.facing_right());
        let p = Pending {
            amount: PUNCH_DAMAGE,
            shove: true,
            push: (if right { PUNCH_PUSH } else { -PUNCH_PUSH }, 0),
            ..Default::default()
        };
        for e in self.entities.iter_mut() {
            if !e.alive
                || e.layer != self.jake_layer
                || e.flags & 0x80001 != 0x80001
                || e.flags & 0x8000 != 0
                || !e.bounds().is_some_and(|b| b.overlaps(&r))
            {
                continue;
            }
            let mine_alive = matches!(e.behaviour, Behaviour::Mine(_)) && e.health > 0;
            if behaviour::interact::hurt(e, &p) {
                if mine_alive && e.health <= 0 {
                    self.kills += 1;
                }
                self.events.push(Event::Sound(Cue::at(
                    SOUND_PUNCH,
                    PRIORITY_FRAME,
                    e.center(),
                    e.layer,
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::laser_eye::LaserEyeState;
    use crate::behaviour::test_support::logic;
    use crate::entity::LASER_EYE;

    fn eye(x: i32) -> Entity {
        let mut e = logic(LASER_EYE, x, 0, &[]);
        e.flags = 0x80809;
        e.solid_shapes = true;
        e.behaviour = Behaviour::LaserEye(LaserEyeState::new(&[0]));
        e
    }

    /// O soco fere quem cruza a caixa do golpe, com o som 5062; os de fora não.
    #[test]
    fn punch_hurts_what_the_blow_box_touches() {
        let mut s = Scene::empty(1000, 1000);
        s.entities = vec![eye(100), eye(300)];
        s.apply_punch(Rect {
            x: 120,
            y: 10,
            w: 14,
            h: 14,
        });
        let hurt: Vec<i32> = s
            .entities
            .iter()
            .map(|e| match &e.behaviour {
                Behaviour::LaserEye(st) => st.hurt.amount,
                _ => -1,
            })
            .collect();
        assert_eq!(hurt, vec![PUNCH_DAMAGE, 0]);
        assert_eq!(s.events.len(), 1);
        assert_eq!(s.events[0].sound_id(), Some(SOUND_PUNCH));
    }
}
