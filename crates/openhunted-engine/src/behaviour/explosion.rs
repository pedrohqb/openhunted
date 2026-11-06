// SPDX-License-Identifier: GPL-3.0-or-later
//! Explosão (tipo 86, `0x0040d20c`/`0x0040d499`; `docs/re/dano-vida.md`).

use super::{Blast, Event, Out};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::sound::{Cue, PRIORITY_FRAME};

#[derive(Debug, Clone, Default)]
pub struct ExplosionState {
    pub applied: bool,
    /// Centro do efeito (origem dos sons por quadro).
    pub center: (i32, i32),
}

/// Sons por quadro absoluto do `OEX.ANM` (lista 86 do `SOUND.TBL`, `docs/re/som.md`). O
/// quadro 148 (`oex15`, a faísca da bala, 3046) fica de fora: a cena o toca no impacto.
pub const FRAME_SOUNDS: [(usize, u16); 11] = [
    (1, 3080),
    (29, 3084),
    (58, 3082),
    (59, 3083),
    (72, 3086),
    (74, 3081),
    (77, 3081),
    (82, 3081),
    (86, 3081),
    (101, 3087),
    (140, 3085),
];

/// Som do quadro `frame` do `OEX.ANM`, se houver.
pub fn frame_sound(frame: usize) -> Option<u16> {
    FRAME_SOUNDS
        .iter()
        .find(|(f, _)| *f == frame)
        .map(|(_, id)| *id)
}

/// Pedido do som do quadro atual da explosão (ao nascer e a cada quadro novo).
pub fn frame_cue(e: &Entity) -> Option<Cue> {
    let (Some(sp), Behaviour::Explosion(st)) = (e.sprite.as_ref(), &e.behaviour) else {
        return None;
    };
    frame_sound(sp.player.frame()).map(|id| Cue::at(id, PRIORITY_FRAME, st.center, e.layer))
}

/// Dano e empurrão que a explosão causa a um objeto com centro em `victim` e na mesma camada
/// (`0x0040d499`): `R = 1,5 · raio`; a `d < R`, dano `dano·(R − d)/R` e empurrão
/// `empurrão·(R − d)/R` na direção centro → vítima; empurrão > 7 derruba.
pub fn area_effect(b: &Blast, victim: (i32, i32), layer: i32) -> Option<Pending> {
    if layer != b.layer {
        return None;
    }
    let r = b.radius * 3 / 2;
    let d = super::mine::distance(b.center, victim);
    if d >= r {
        return None;
    }
    let amount = b.damage * (r - d) / r;
    let push = b.push * (r - d) / r;
    let (dx, dy) = (victim.0 - b.center.0, victim.1 - b.center.1);
    let dir = if d == 0 { (0, -1) } else { (dx, dy) };
    let len = d.max(1);
    Some(Pending {
        amount,
        explosion: true,
        push: (push * dir.0 / len, push * dir.1 / len),
        knockdown: push > 7,
        ..Default::default()
    })
}

/// Avança a animação (com o som do quadro novo, se houver); no fim, a explosão some.
pub fn tick(e: &mut Entity, out: &mut Out) {
    if let Some(sp) = e.sprite.as_mut() {
        if sp.player.tick_once() {
            e.alive = false;
        } else if let Some(c) = frame_cue(e) {
            out.events.push(Event::Sound(c));
        }
    } else {
        e.alive = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_damage_falls_linearly_and_knocks_down_when_close() {
        let b = Blast {
            center: (100, 100),
            layer: 0,
            radius: 28,
            push: 15,
            damage: 10,
        };
        let at0 = area_effect(&b, (100, 100), 0).unwrap();
        assert_eq!((at0.amount, at0.knockdown), (10, true));
        let at21 = area_effect(&b, (121, 100), 0).unwrap();
        assert_eq!((at21.amount, at21.knockdown), (5, false));
        assert_eq!(at21.push, (7, 0));
        assert!(area_effect(&b, (142, 100), 0).is_none());
        assert!(area_effect(&b, (100, 100), 1).is_none());
        let left = area_effect(&b, (90, 100), 0).unwrap();
        assert!(left.push.0 < 0 && left.knockdown);
    }

    #[test]
    fn explosion_sounds_its_frames_from_the_centre() {
        use crate::scene::{Layer, Sprite};
        assert_eq!(frame_sound(1), Some(3080));
        assert_eq!(frame_sound(29), Some(3084));
        assert_eq!(frame_sound(0), None);
        assert_eq!(frame_sound(148), None, "a faísca toca pelo impacto");
        let anim = std::rc::Rc::new(crate::anim::tests::anim());
        let player = crate::AnimPlayer::at_label(&anim, 1).unwrap();
        let mut e = crate::behaviour::test_support::logic(crate::entity::EXPLOSION, 0, 0, &[]);
        e.layer = 2;
        e.boxes.clear();
        e.sprite = Some(Sprite {
            layer: Layer::Object,
            anim,
            player,
            x: 0,
            y: 0,
            frozen: true,
            scale: (1, 1),
            mirror: false,
        });
        e.behaviour = Behaviour::Explosion(ExplosionState {
            applied: false,
            center: (40, 50),
        });
        assert_eq!(frame_cue(&e), None, "quadro 0 é mudo");
        let mut out = Out::default();
        tick(&mut e, &mut out);
        assert_eq!(
            out.events,
            vec![Event::Sound(Cue::at(3080, PRIORITY_FRAME, (40, 50), 2))]
        );
        tick(&mut e, &mut out);
        assert!(!e.alive);
        assert_eq!(out.events.len(), 1);
    }
}
