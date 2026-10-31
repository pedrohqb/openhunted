// SPDX-License-Identifier: GPL-3.0-or-later
//! Dica do tutorial (tipo 30, `0x004153d8`; `docs/re/dicas.md`).

use super::{Ctx, Event, Out};
use crate::entity::{Behaviour, Entity};
use crate::triggers::satisfied;

/// Som "MC Player Hint".
pub const SOUND_HINT: u16 = 3774;

#[derive(Debug, Clone, Default)]
pub struct HintState {
    pub shown: bool,
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let Behaviour::Hint(st) = &e.behaviour else {
        return;
    };
    if st.shown || e.on {
        return;
    }
    if !satisfied(&e.triggers, ctx.on) {
        return;
    }
    let line = e.values.first().copied().unwrap_or(0);
    if line <= 1 {
        return;
    }
    e.on = true;
    if let Behaviour::Hint(st) = &mut e.behaviour {
        st.shown = true;
    }
    let Some(text) = ctx.hints.get((line - 1) as usize).filter(|t| !t.is_empty()) else {
        return;
    };
    let left_of_jake = ctx.jake.is_some_and(|j| e.x < j.body.x);
    out.events.push(Event::Hint {
        text: text.clone(),
        left_of_jake,
    });
    out.events.push(Event::Sound(SOUND_HINT));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};

    #[test]
    fn hint_shows_once_with_the_right_line_and_side() {
        let hints = vec!["OK".to_string(), "Mines!".to_string(), "Up!".to_string()];
        let mut h = logic(crate::entity::HINT, 50, 0, &[2, 10, 0, 1]);
        h.triggers = vec![(1, Some(11))];
        let (j, w) = ctx_with_jake(300, 0);
        let mut out = Out::default();
        let off = |_: i32| Some(false);
        let mut c = ctx(&j, &w, &off);
        c.hints = &hints;
        tick(&mut h, &c, &mut out);
        assert!(out.events.is_empty());
        let on = |_: i32| Some(true);
        let mut c = ctx(&j, &w, &on);
        c.hints = &hints;
        tick(&mut h, &c, &mut out);
        assert_eq!(
            out.events[0],
            Event::Hint {
                text: "Mines!".into(),
                left_of_jake: true
            }
        );
        assert_eq!(out.events[1], Event::Sound(SOUND_HINT));
        out.events.clear();
        tick(&mut h, &c, &mut out);
        assert!(out.events.is_empty(), "uma vez só");
        // linha 1 nunca é dica
        let mut one = logic(crate::entity::HINT, 50, 0, &[1]);
        tick(&mut one, &c, &mut out);
        assert!(out.events.is_empty());
        assert!(!one.on, "só liga com linha > 1");
    }
}
