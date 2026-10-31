// SPDX-License-Identifier: GPL-3.0-or-later
//! Zona de proximidade (tipo 73, `0x0040e14e`; `docs/re/logica-fase.md`).

use super::{Ctx, Out};
use crate::entity::Entity;
use crate::triggers::satisfied;

/// Sentinelas do original.
const FOREVER: i32 = 77_777_777;
const STALE: i32 = 38_888_888;

#[derive(Debug, Clone, Default)]
pub struct ZoneState {
    pub timer: i32,
}

/// A zona está ocupada neste tique. `values[5]`: 0 só jogadores, 2 sempre ocupada; os
/// demais objetos vivos não são considerados nesta fatia (só o Jake conta).
fn occupied(e: &Entity, ctx: &Ctx) -> bool {
    let who = e.values.get(5).copied().unwrap_or(0);
    if who == 2 {
        return true;
    }
    let (Some(zone), Some(j)) = (e.bounds(), ctx.jake_bounds()) else {
        return false;
    };
    if ctx.jake_layer != e.layer {
        return false;
    }
    // desigualdades estritas
    j.x < zone.x + zone.w && j.x + j.w > zone.x && j.y < zone.y + zone.h && j.y + j.h > zone.y
}

pub fn tick(e: &mut Entity, ctx: &Ctx, _out: &mut Out) {
    let value = |i: usize| e.values.get(i).copied().unwrap_or(0);
    if !satisfied(&e.triggers, ctx.on) {
        return;
    }
    let busy = occupied(e, ctx);
    let (delay, duration, opts) = (value(2), value(3), value(4));
    let duration = if duration == 0 { FOREVER } else { duration };
    let Some(st) = (match &mut e.behaviour {
        crate::entity::Behaviour::Zone(z) => Some(z),
        _ => None,
    }) else {
        return;
    };
    if !e.on {
        if !busy && (opts & 1 == 0 || st.timer > STALE) {
            st.timer = 0;
        }
        if busy && st.timer <= 0 {
            st.timer = delay;
        }
        if busy {
            st.timer -= 1;
            if st.timer <= 0 {
                e.on = true;
                st.timer = duration;
            }
        }
    } else {
        if busy && opts & 2 != 0 {
            st.timer = duration;
        }
        st.timer -= 1;
        let expired = value(3) != 0 && st.timer <= 0;
        let emptied = value(3) == 0 && !busy;
        if expired || emptied {
            e.on = false;
            st.timer = FOREVER;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx_with_jake, zone};

    #[test]
    fn zone_turns_on_after_delay_with_jake_inside() {
        let mut z = zone(100, 100, 64, 16, &[0, 0, 3, 99_999_999, 0, 0]);
        let mut out = Out::default();
        // Jake longe: nada
        let (j, w) = ctx_with_jake(500, 100);
        for _ in 0..5 {
            let on = |_: i32| None;
            let ctx = ctx_with(&j, &w, &on);
            tick(&mut z, &ctx, &mut out);
        }
        assert!(!z.on);
        // Jake dentro: liga ao fim do atraso de 3 tiques
        let (j, w) = ctx_with_jake(110, 100);
        let on = |_: i32| None;
        for t in 1..=3 {
            let ctx = ctx_with(&j, &w, &on);
            tick(&mut z, &ctx, &mut out);
            assert_eq!(z.on, t == 3, "tique {t}");
        }
        // continua ligada mesmo vazia (duração 99999999)
        let (j, w) = ctx_with_jake(500, 100);
        for _ in 0..50 {
            let ctx = ctx_with(&j, &w, &on);
            tick(&mut z, &ctx, &mut out);
        }
        assert!(z.on);
    }

    #[test]
    fn zone_with_zero_duration_turns_off_when_empty_and_option_two_is_always_busy() {
        let mut z = zone(100, 100, 64, 16, &[0, 0, 1, 0, 0, 0]);
        let mut out = Out::default();
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(110, 100);
        tick(&mut z, &ctx_with(&j, &w, &on), &mut out);
        assert!(z.on);
        let (j, w) = ctx_with_jake(500, 100);
        tick(&mut z, &ctx_with(&j, &w, &on), &mut out);
        assert!(!z.on);
        let mut t = zone(100, 100, 64, 16, &[0, 0, 2, 5, 0, 2]);
        for _ in 0..2 {
            tick(&mut t, &ctx_with(&j, &w, &on), &mut out);
        }
        assert!(t.on, "values[5] = 2: ocupada sem ninguém");
        for _ in 0..5 {
            tick(&mut t, &ctx_with(&j, &w, &on), &mut out);
        }
        assert!(!t.on, "duração 5 vence");
    }

    #[test]
    fn zone_waits_for_its_own_triggers() {
        let mut z = zone(100, 100, 64, 16, &[0, 0, 1, 0, 0, 0]);
        z.triggers = vec![(7, Some(11))];
        let mut out = Out::default();
        let (j, w) = ctx_with_jake(110, 100);
        let off = |_: i32| Some(false);
        tick(&mut z, &ctx_with(&j, &w, &off), &mut out);
        assert!(!z.on);
        let on = |_: i32| Some(true);
        tick(&mut z, &ctx_with(&j, &w, &on), &mut out);
        assert!(z.on);
    }

    fn ctx_with<'a>(j: &'a Jake, w: &'a World, on: &'a dyn Fn(i32) -> Option<bool>) -> Ctx<'a> {
        crate::behaviour::test_support::ctx(j, w, on)
    }

    use crate::{Jake, World};
}
