// SPDX-License-Identifier: GPL-3.0-or-later
//! Itens de vida e armadura (tipos 32 e 79, `0x0040b3f5`; `docs/re/itens.md`).

use super::{Ctx, Event, Out};
use crate::damage::Health;
use crate::entity::{ARMOR_ITEM, Entity, HEALTH_ITEM};

/// Som "Pickup generic object".
pub const SOUND_PICKUP: u16 = 5057;
/// O Jake pode pegar (`+0x178 & 0x100`).
const JAKE_CAN_TAKE: i32 = 0x100;

/// Efeito do item (tabela `0x00471d10`): (vida, armadura).
fn effect(kind: u32) -> (i32, i32) {
    match kind {
        HEALTH_ITEM => (25, 0),
        34 => (100, 0),
        76 => (0, 50),
        77 => (0, 100),
        78 => (0, 20),
        ARMOR_ITEM => (0, 25),
        _ => (0, 0),
    }
}

/// Aceita e aplica o item à vida `h` (`0x0040b6ed` + `0x0040b57b`); `false` = recusado.
pub fn take(kind: u32, h: &mut Health) -> bool {
    let (hp, armor) = effect(kind);
    if hp > 0 {
        let cap = if hp < 100 { 100 } else { 199 };
        if h.hp <= 0 || h.hp >= cap {
            return false;
        }
        h.hp = (h.hp + hp).min(cap);
        return true;
    }
    if armor > 0 {
        let cap = if armor < 100 { 100 } else { 199 };
        if h.armor >= cap {
            return false;
        }
        h.armor = (h.armor + armor).min(cap);
        return true;
    }
    false
}

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    if e.flags & JAKE_CAN_TAKE == 0 || e.layer != ctx.jake_layer {
        return;
    }
    let (Some(item), Some(j)) = (e.bounds(), ctx.jake_bounds()) else {
        return;
    };
    if !item.overlaps(&j) {
        return;
    }
    let Some(jake) = ctx.jake else { return };
    let mut h = out.heal.unwrap_or(jake.health);
    if take(e.kind, &mut h) {
        out.heal = Some(h);
        out.events.push(Event::Sound(crate::sound::Cue::ui(
            SOUND_PICKUP,
            crate::sound::PRIORITY_PICKUP,
        )));
        e.alive = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::behaviour::test_support::{ctx, ctx_with_jake, logic};

    #[test]
    fn health_and_armor_items_follow_the_table() {
        let mut h = Health { hp: 50, armor: 0 };
        assert!(take(HEALTH_ITEM, &mut h));
        assert_eq!(h, Health { hp: 75, armor: 0 });
        assert!(take(HEALTH_ITEM, &mut h));
        assert_eq!(h.hp, 100);
        assert!(!take(HEALTH_ITEM, &mut h), "recusado em 100");
        assert!(take(ARMOR_ITEM, &mut h));
        assert_eq!(h.armor, 25);
        h.armor = 100;
        assert!(!take(ARMOR_ITEM, &mut h));
        let mut dead = Health { hp: 0, armor: 0 };
        assert!(!take(HEALTH_ITEM, &mut dead), "morto não se cura");
    }

    #[test]
    fn item_is_taken_on_contact_and_disappears() {
        let mut item = logic(HEALTH_ITEM, 100, 0, &[]);
        item.flags = 0x803a9;
        let (j, w) = ctx_with_jake(100, 0);
        let on = |_: i32| None;
        let mut out = Out::default();
        tick(&mut item, &ctx(&j, &w, &on), &mut out);
        assert!(!item.alive);
        assert_eq!(out.heal, Some(Health { hp: 75, armor: 0 }));
        assert_eq!(
            out.events,
            vec![Event::Sound(crate::sound::Cue::ui(SOUND_PICKUP, 200))]
        );
        // dois itens no mesmo tique: os dois contam
        let mut armor = logic(ARMOR_ITEM, 100, 0, &[]);
        armor.flags = 0x803a9;
        tick(&mut armor, &ctx(&j, &w, &on), &mut out);
        assert_eq!(out.heal, Some(Health { hp: 75, armor: 25 }));
        // longe: nada
        let mut item = logic(HEALTH_ITEM, 100, 0, &[]);
        item.flags = 0x803a9;
        let (j, w) = ctx_with_jake(400, 0);
        let mut out = Out::default();
        tick(&mut item, &ctx(&j, &w, &on), &mut out);
        assert!(item.alive && out.heal.is_none());
    }
}
