// SPDX-License-Identifier: GPL-3.0-or-later
//! Paredes e portas `OH` (tipos 50–52, `0x0040e690`/`0x0040e952`/`0x0040ebdc`;
//! `docs/re/objetos-tutorial.md`): fechada (sólida), abrindo, aberta (passável), fechando.

use super::{Ctx, Event, Out, grow};
use crate::entity::{Behaviour, Entity};
use crate::sound::{Cue, PRIORITY_PICKUP};
use crate::triggers::satisfied;

pub const CLOSED: u8 = 1;
pub const CLOSING: u8 = 2;
pub const OPENING: u8 = 3;
pub const OPEN: u8 = 4;
/// "Secret discovered" (porta com o bit 1 de objetivo que abre pela primeira vez).
pub const SOUND_SECRET: u16 = 3093;
/// `values` padrão de quem começa no estado 0: empurrar 10, gatilho 0, fechar 0, repetir 1.
pub const DEFAULT_VALUES: [i32; 4] = [10, 0, 0, 1];
/// Só contato: aberta, não barra.
const CONTACT: i32 = 0x20;

#[derive(Debug, Clone)]
pub struct DoorState {
    pub state: u8,
    /// `values[0..4]`: tiques de empurrão, de gatilho, para fechar, e "reabre".
    pub values: [i32; 4],
    /// Contadores `+0x270/+0x274/+0x278`.
    pub push: i32,
    pub trigger: i32,
    pub close: i32,
    pub opened: bool,
}

impl DoorState {
    pub fn new(state: i32, values: &[i32]) -> Self {
        let v: [i32; 4] = if state == 0 {
            DEFAULT_VALUES
        } else {
            std::array::from_fn(|i| values.get(i).copied().unwrap_or(0))
        };
        Self {
            state: match state {
                2..=4 => state as u8,
                _ => CLOSED,
            },
            values: v,
            push: v[0],
            trigger: v[1],
            close: v[2],
            opened: false,
        }
    }

    /// Pode abrir: `values[3] ≠ 0` ou nunca aberta.
    fn may_open(&self) -> bool {
        self.values[3] != 0 || !self.opened
    }
}

/// Nome do BRD da porta pelo tipo e rótulo (`0x0040e690`).
pub fn border_name(kind: u32, label: i32) -> String {
    if label < 100 {
        format!("OH_W{:03}.BRD", (kind as i32 - 49) * 100 + label - 49)
    } else {
        match label % 100 {
            0 => "OH_F.BRD".into(),
            1 => "OH_C.BRD".into(),
            _ => "OH_B.BRD".into(),
        }
    }
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let trig = satisfied(&e.triggers, ctx.on);
    let (touching, inside) = match (e.bounds(), ctx.jake_bounds()) {
        (Some(d), Some(j)) if ctx.jake_layer == e.layer => {
            (grow(d, 1).overlaps(&j), d.overlaps(&j))
        }
        _ => (false, false),
    };
    let secret = e.objective & 1 != 0;
    let Behaviour::Door(st) = &mut e.behaviour else {
        return;
    };
    let v = st.values;
    let mut next = st.state;
    match st.state {
        CLOSED => {
            if v[0] != 0 && st.may_open() && touching {
                st.push -= 1;
                if st.push <= 0 {
                    next = OPENING;
                }
            } else if trig && v[1] != 0 && !touching && st.may_open() {
                st.trigger -= 1;
                if st.trigger <= 0 {
                    next = OPENING;
                }
            }
        }
        OPEN if trig && !touching && !inside && v[2] != 0 => {
            st.close -= 1;
            if st.close <= 0 {
                next = CLOSING;
            }
        }
        _ => {}
    }
    let state = st.state;
    // abrindo/fechando: um quadro por tique até o fim/começo do rótulo
    let done = match (state, e.sprite.as_mut()) {
        (OPENING, Some(sp)) => sp.player.tick_once(),
        (CLOSING, Some(sp)) => sp.player.tick_back(),
        (OPENING | CLOSING, None) => true,
        _ => false,
    };
    let Behaviour::Door(st) = &mut e.behaviour else {
        return;
    };
    if done {
        next = if state == OPENING { OPEN } else { CLOSED };
    }
    if next != state {
        st.state = next;
        match next {
            OPEN => {
                if secret && !st.opened {
                    out.events
                        .push(Event::Sound(Cue::ui(SOUND_SECRET, PRIORITY_PICKUP)));
                }
                st.opened = true;
                st.close = v[2];
                e.flags |= CONTACT;
            }
            CLOSED => {
                st.push = v[0];
                st.trigger = v[1];
                e.flags &= !CONTACT;
            }
            _ => e.flags &= !CONTACT,
        }
    }
    if let Behaviour::Door(st) = &e.behaviour {
        e.on = st.state > CLOSING;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};
    use crate::physics::Rect;

    fn door(values: &[i32]) -> Entity {
        let mut d = logic(50, 200, 0, values);
        d.flags = 0x80089;
        d.boxes = vec![Rect {
            x: 0,
            y: 12,
            w: 29,
            h: 104,
        }];
        d.solid_shapes = true;
        d.triggers = vec![(1, Some(11))];
        d.behaviour = Behaviour::Door(DoorState::new(1, values));
        d
    }

    #[test]
    fn border_names_follow_the_label() {
        assert_eq!(border_name(50, 54), "OH_W105.BRD");
        assert_eq!(border_name(50, 55), "OH_W106.BRD");
        assert_eq!(border_name(51, 54), "OH_W205.BRD");
        assert_eq!(border_name(50, 100), "OH_F.BRD");
        assert_eq!(border_name(52, 201), "OH_C.BRD");
        assert_eq!(border_name(50, 302), "OH_B.BRD");
    }

    #[test]
    fn tutorial_door_opens_only_when_its_trigger_turns_on() {
        let (j, w) = ctx_with_jake(170, 0); // encostado à esquerda
        let off = |_: i32| Some(false);
        let mut d = door(&[0, 1, 0, 1]);
        for _ in 0..20 {
            tick(&mut d, &ctx(&j, &w, &off), &mut Out::default());
        }
        assert_eq!((d.flags & CONTACT, d.on), (0, false), "não abre por toque");
        let (j, w) = ctx_with_jake(500, 0);
        let on = |_: i32| Some(true);
        tick(&mut d, &ctx(&j, &w, &on), &mut Out::default());
        let Behaviour::Door(st) = &d.behaviour else {
            panic!()
        };
        assert_eq!(st.state, OPENING);
        assert!(d.on, "nos gatilhos, 50–52 ligadas = estado > 2");
        tick(&mut d, &ctx(&j, &w, &on), &mut Out::default());
        assert_eq!(d.flags & CONTACT, CONTACT, "aberta: passável");
    }

    #[test]
    fn pushed_door_opens_and_closes_on_timer() {
        let (j, w) = ctx_with_jake(180, 0); // encostado (caixa até x = 200)
        let none = |_: i32| None;
        let mut d = door(&[3, 0, 2, 1]);
        d.triggers.clear();
        d.objective = 1;
        let mut out = Out::default();
        for _ in 0..4 {
            tick(&mut d, &ctx(&j, &w, &none), &mut out);
        }
        assert!(d.on);
        assert!(
            out.events
                .iter()
                .any(|e| e.sound_id() == Some(SOUND_SECRET))
        );
        // longe: fecha depois de 2 tiques e volta a ser sólida
        let (j, w) = ctx_with_jake(600, 0);
        for _ in 0..3 {
            tick(&mut d, &ctx(&j, &w, &none), &mut out);
        }
        let Behaviour::Door(st) = &d.behaviour else {
            panic!()
        };
        assert_eq!(st.state, CLOSED);
        assert_eq!(d.flags & CONTACT, 0);
    }
}
