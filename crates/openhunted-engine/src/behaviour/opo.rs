// SPDX-License-Identifier: GPL-3.0-or-later
//! Obstáculos `OPO` (tipos 68–72, inicialização `0x0040f324`; `docs/re/objetos.md`): não são
//! plataformas. O 68 e o 69 (poça de veneno) só animam em laço (`0x0040f459`); o 70 (saliência
//! que goteja) anima e solta gotas (71, ainda não no porte). Os três têm a rotina `+0x1c8`
//! `0x0040fa16`: a cada `+0x260` tiques (`values[0]`, padrão 10) de `0x4879c4`, todo objeto
//! presente na mesma camada que cruza o retângulo do quadro de referência (`0x0043585c`)
//! leva o dano da linha do tipo (`0x0043d629(obj, outro, 0)`: veneno, 1); um jogador (tipo
//! 1/2) fica envenenado (`+0x270 = 1`) e, se ainda não estava e é humano (`+0x19c ≠ 0`),
//! fala "Oh no it's poison" (`0x0040b9eb(…, 0xe85, 0xe99, 200, 200)`). O portão 72 usa a mesma
//! rotina fora do estado 2; no porte ele fica parado no rótulo 272 (o estado 2) e não fere.

use super::{Ctx, Event, Hazard, Out};
use crate::damage::{Geometry, Pending, Strike};
use crate::entity::{Behaviour, Entity};
use crate::physics::Rect;
use crate::sound::Cue;

/// Período padrão do teste (`+0x260 = 10` se a fase deu 0, `0x0040f3a7`).
pub const PERIOD: i32 = 10;
/// Fala do Jake ao se envenenar (`0xe85`; a do Garathe é 3737, `0xe99`).
pub const SPEECH: u16 = 3717;
/// Prioridade da fala (`0x0040b9eb(…, 200, 200)`).
pub const SPEECH_PRIORITY: u16 = 200;

#[derive(Debug, Clone)]
pub struct OpoState {
    /// Retângulo do quadro de referência, relativo à posição (`0x0043585c`).
    pub rect: Rect,
    /// `+0x260`: período em tiques.
    pub period: i32,
}

impl OpoState {
    pub fn new(rect: Rect, value0: i32) -> Self {
        Self {
            rect,
            period: if value0 != 0 { value0 } else { PERIOD },
        }
    }
}

/// O dano da linha do tipo `kind` contra `victim` (`0x0043d629(opo, obj, 0)`), com o
/// envenenamento do jogador (`+0x270 = 1`, `0x0040fb2e`).
pub fn damage(kind: u32, victim: u32) -> Pending {
    let mut p = Strike {
        attacker: kind,
        variant: 0,
    }
    .row(victim)
    .map(|r| {
        r.pending(Geometry {
            vel: (0, 0),
            toward: (0, 0),
        })
    })
    .unwrap_or_default();
    p.poisons = matches!(victim, 1 | 2);
    p
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let Behaviour::Opo(st) = &e.behaviour else {
        return;
    };
    if st.period > 0 && ctx.tick % st.period as u32 != 0 {
        return;
    }
    let r = Rect {
        x: e.x + st.rect.x,
        y: e.y + st.rect.y,
        ..st.rect
    };
    for (k, j, layer) in ctx.players() {
        if layer != e.layer || !j.is_active() || !j.body.bounds().overlaps(&r) {
            continue;
        }
        let ch = j.character();
        if j.poison == 0 {
            out.events
                .push(Event::Sound(Cue::ui(ch.speech(SPEECH), SPEECH_PRIORITY)));
        }
        out.hurt_player(k, &damage(e.kind, ch.kind()));
    }
    // os demais objetos (barris, personagens da IA): a linha de vítima qualquer, e o
    // personagem da IA também se envenena (sem fala: `+0x19c = 0`)
    out.hazards.push(Hazard {
        rect: r,
        layer: e.layer,
        damage: Pending {
            poisons: true,
            ..damage(e.kind, 0)
        },
        source: None,
        from: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};

    fn puddle(x: i32, y: i32) -> Entity {
        let mut e = logic(68, x, y, &[]);
        e.boxes.clear();
        e.behaviour = Behaviour::Opo(OpoState::new(
            Rect {
                x: 0,
                y: 0,
                w: 66,
                h: 8,
            },
            0,
        ));
        e
    }

    #[test]
    fn rows_68_to_72_poison_with_one_and_the_gate_with_five() {
        for k in 68..=71 {
            let p = damage(k, 1);
            assert_eq!((p.amount, p.poison, p.poisons), (1, true, true), "{k}");
        }
        assert_eq!(damage(72, 2).amount, 5);
        assert!(!damage(68, 28).poisons, "só o jogador se envenena");
    }

    #[test]
    fn poisons_whoever_crosses_it_every_ten_ticks() {
        let on = |_: i32| None;
        // Jake de pé com os pés em y + 47: a poça (66 × 8) no chão, sob ele
        let (j, w) = ctx_with_jake(100, 0);
        let b = j.body.bounds();
        let mut e = puddle(b.x - 20, b.y + b.h - 6);
        let mut c = ctx(&j, &w, &on);
        let mut hits = vec![];
        for t in 0..30 {
            c.tick = t;
            let mut out = Out::default();
            tick(&mut e, &c, &mut out);
            if !out.jake_damage.is_empty() {
                hits.push(t);
                assert_eq!(out.jake_damage.amount, 1);
                assert!(out.jake_damage.poison && out.jake_damage.poisons);
                assert_eq!(
                    out.events
                        .iter()
                        .filter_map(Event::sound_id)
                        .collect::<Vec<_>>(),
                    vec![SPEECH],
                    "fala com o jogador ainda sem veneno"
                );
                assert_eq!(out.hazards.len(), 1);
            }
        }
        assert_eq!(hits, vec![0, 10, 20]);
        // fora da caixa, ou em outra camada: nada
        let far = puddle(b.x + 200, b.y);
        let mut e2 = far.clone();
        c.tick = 0;
        let mut out = Out::default();
        tick(&mut e2, &c, &mut out);
        assert!(out.jake_damage.is_empty());
        let mut e3 = puddle(b.x - 20, b.y + b.h - 6);
        e3.layer = 1;
        let mut out = Out::default();
        tick(&mut e3, &c, &mut out);
        assert!(out.jake_damage.is_empty());
    }

    #[test]
    fn already_poisoned_player_is_hurt_without_speech() {
        let on = |_: i32| None;
        let (mut j, w) = ctx_with_jake(100, 0);
        j.poison = 2;
        let b = j.body.bounds();
        let mut e = puddle(b.x, b.y);
        let c = ctx(&j, &w, &on);
        let mut out = Out::default();
        tick(&mut e, &c, &mut out);
        assert_eq!(out.jake_damage.amount, 1);
        assert!(out.events.is_empty());
    }

    #[test]
    fn hurt_with_poisons_sets_the_poison_to_one() {
        let (mut j, w) = ctx_with_jake(100, 0);
        j.poison = 3;
        j.hurt(damage(68, 1));
        j.tick(0, &w);
        assert_eq!(j.poison, 1, "`+0x270 = 1` (atribui, não soma)");
        let (mut j, w) = ctx_with_jake(100, 0);
        j.hurt(damage(68, 1));
        j.tick(0, &w);
        assert_eq!(j.poison, 1);
    }
}
