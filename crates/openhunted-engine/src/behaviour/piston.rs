// SPDX-License-Identifier: GPL-3.0-or-later
//! Pistão (tipo 39, `0x004106ec`/`0x004107c8`/`0x004108c0`; `docs/re/itens-campanha.md`):
//! parado, sem espera e com gatilhos verdadeiros (e algum código), anda pelos quadros do
//! rótulo 1; nos extremos espera `values[0]` tiques. Andando, esmaga quem a caixa do quadro
//! (2 px mais baixa) toca: 35, derrubada, empurrão (±48, −3).

use super::{Ctx, Event, Out, show};
use crate::damage::Pending;
use crate::entity::{Behaviour, Entity};
use crate::triggers::{armed, satisfied};

/// Linha `(39, 0)` do `DAMAGE.TBL`.
pub const DAMAGE: i32 = 35;
pub const PUSH: (i32, i32) = (48, -3);
pub const LABEL: u16 = 1;
/// Sons do `SOUND.TBL` (tipo 39): quadros 1/9 → 3058, 2/8 → 3057.
pub const SOUND_A: u16 = 3058;
pub const SOUND_B: u16 = 3057;

#[derive(Debug, Clone, Default)]
pub struct PistonState {
    /// Quadro dentro do rótulo (`+0x184`), sentido (`+0x27c`: −1, 0, +1) e espera (`+0x278`).
    pub frame: usize,
    pub dir: i32,
    pub wait: i32,
}

/// Número de quadros do rótulo 1 do sprite.
fn frames(e: &Entity) -> usize {
    let Some(sp) = e.sprite.as_ref() else {
        return 1;
    };
    let first = sp
        .anim
        .labels
        .iter()
        .find(|l| l.id == LABEL)
        .map_or(0, |l| l.frame as usize);
    let next = sp
        .anim
        .labels
        .iter()
        .map(|l| l.frame as usize)
        .filter(|&f| f > first)
        .min()
        .unwrap_or(sp.anim.frames.len());
    next.saturating_sub(first).max(1)
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let n = frames(e);
    let go = armed(&e.triggers) && satisfied(&e.triggers, ctx.on);
    let wait_after = e.values.first().copied().unwrap_or(0);
    let Behaviour::Piston(st) = &mut e.behaviour else {
        return;
    };
    if st.dir == 0 {
        if st.wait > 0 {
            st.wait -= 1;
        } else if go {
            st.dir = if st.frame == 0 { 1 } else { -1 };
        }
    }
    if st.dir == 0 {
        return;
    }
    let f = (st.frame as i32 + st.dir).clamp(0, n as i32 - 1) as usize;
    st.frame = f;
    let dir = st.dir;
    if f == 0 || f + 1 >= n {
        st.dir = 0;
        st.wait = wait_after;
    }
    show(e, LABEL, f);
    // Ruling: os sons na primeira metade de cada curso
    let sound = match (dir, f) {
        (1, 1) | (-1, 9) => Some(SOUND_A),
        (1, 2) | (-1, 8) => Some(SOUND_B),
        _ => None,
    };
    if let Some(id) = sound {
        out.events.push(Event::Sound(crate::sound::Cue::at(
            id,
            crate::sound::PRIORITY_FRAME,
            e.center(),
            e.layer,
        )));
    }
    // esmaga (`0x004108c0`): caixa do quadro 2 px mais baixa contra o Jake na camada
    let (Some(r), Some(j)) = (e.sprite_rect(), ctx.jake_bounds()) else {
        return;
    };
    let r = crate::physics::Rect { y: r.y + 2, ..r };
    if e.layer == ctx.jake_layer && r.overlaps(&j) {
        out.jake_damage.add(&Pending {
            amount: DAMAGE,
            knockdown: true,
            push: (PUSH.0 * dir, PUSH.1),
            ..Default::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};

    /// Pistão em (0, 0) com 10 quadros no rótulo 1, cada um 10 px mais alto (desce).
    fn piston_with_frames(wait: i32) -> Entity {
        use std::rc::Rc;
        let frame = |h| openhunted_formats::AnimFrame {
            x: 0,
            y: 0,
            width: 30,
            height: h,
            script_offset: 0,
            extra: 0,
        };
        let anim = Rc::new(openhunted_formats::Animation {
            version: 1004,
            width: 30,
            height: 100,
            extra: [0; 3],
            table_a: vec![],
            frames: (1..=10).map(|k| frame(10 * k)).collect(),
            labels: vec![openhunted_formats::AnimLabel {
                frame: 0,
                id: LABEL,
                name: None,
            }],
            script: vec![],
            bitmaps: vec![],
            palette: None,
        });
        let mut p = logic(crate::entity::PISTON, 0, 0, &[wait]);
        p.sprite = Some(crate::Sprite {
            layer: crate::Layer::Object,
            player: crate::AnimPlayer::from_frame(&anim, 0),
            anim,
            x: 0,
            y: 0,
            frozen: true,
            scale: (1, 1),
            mirror: false,
        });
        p.triggers = vec![(7, Some(11))];
        p.behaviour = Behaviour::Piston(PistonState::default());
        p
    }

    /// Com o gatilho, anda os 10 quadros (sons 3058/3057 no começo do curso), espera
    /// `values[0]` no fim e volta; andando, esmaga o Jake sob a caixa do quadro: 35,
    /// derrubada, empurrão (48·sentido, −3). Longe, nada.
    #[test]
    fn piston_moves_waits_returns_and_crushes() {
        let on = |_: i32| Some(true);
        // Jake longe: o curso inteiro
        let (j, w) = ctx_with_jake(5000, 0);
        let mut p = piston_with_frames(3);
        let mut frames = vec![];
        let mut sounds = vec![];
        for _ in 0..16 {
            let mut out = Out::default();
            tick(&mut p, &ctx(&j, &w, &on), &mut out);
            assert!(out.jake_damage.is_empty());
            sounds.extend(out.events.iter().filter_map(|e| e.sound_id()));
            let Behaviour::Piston(st) = &p.behaviour else {
                panic!()
            };
            frames.push(st.frame);
        }
        assert_eq!(
            frames,
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 9, 8, 7, 6, 5],
            "anda, espera 3 e volta"
        );
        // na volta o curso começa no quadro 8 (o 9 é o extremo): só o 3057
        assert_eq!(sounds, vec![SOUND_A, SOUND_B, SOUND_B]);
        // Jake sob o pistão (topo em y = 60): o quadro 6 (70 px + 2) o alcança
        let (j, w) = ctx_with_jake(5, 57);
        let mut p = piston_with_frames(3);
        let mut hit = None;
        for _ in 0..9 {
            let mut out = Out::default();
            tick(&mut p, &ctx(&j, &w, &on), &mut out);
            if !out.jake_damage.is_empty() && hit.is_none() {
                hit = Some(out.jake_damage);
            }
        }
        let d = hit.expect("esmagou");
        assert_eq!((d.amount, d.knockdown, d.push), (DAMAGE, true, (48, -3)));
    }

    #[test]
    fn piston_needs_an_armed_trigger_and_waits_at_the_ends() {
        let (j, w) = ctx_with_jake(5000, 0);
        let mut p = logic(crate::entity::PISTON, 0, 0, &[3]);
        p.behaviour = Behaviour::Piston(PistonState::default());
        // sem sprite: um quadro só → não sai do lugar; dá-lhe 10 quadros falsos via estado
        let on_true = |_: i32| Some(true);
        let mut out = Out::default();
        tick(&mut p, &ctx(&j, &w, &on_true), &mut out);
        let Behaviour::Piston(st) = &p.behaviour else {
            panic!()
        };
        assert_eq!(st.dir, 0, "sem gatilhos nunca anda");
        p.triggers = vec![(7, Some(11))];
        let on_off = |_: i32| Some(false);
        tick(&mut p, &ctx(&j, &w, &on_off), &mut out);
        let Behaviour::Piston(st) = &p.behaviour else {
            panic!()
        };
        assert_eq!(st.dir, 0, "gatilho falso");
    }
}
