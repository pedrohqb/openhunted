// SPDX-License-Identifier: GPL-3.0-or-later
//! Itens: vida, armadura, armas e munição (coleta genérica `0x0040b3f5`; `docs/re/itens.md`,
//! `docs/re/armas.md`).

use super::{Ctx, Event, Out};
use crate::damage::Health;
use crate::entity::{ARMOR_ITEM, Entity, HEALTH_ITEM};
use crate::jake::{Inventory, PISTOL};
use crate::physics::{Body, step};

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

/// Tipos de arma e munição: pistola (12), lança-foguetes (14), munição da pistola (16),
/// granadas (26).
pub const PISTOL_ITEM: u32 = 12;
pub const ROCKET_ITEM: u32 = 14;
pub const AMMO_ITEM: u32 = 16;
pub const GRENADE_ITEM: u32 = 26;
/// Teto da munição (`0x40a36d`).
pub const AMMO_CAP: i32 = 99;
const ROCKETS: usize = 4;
const GRENADES: usize = 6;

/// Arma e munição (`0x409fe8` + aceitação `0x40b6ed`) com o valor `v` (`values[0]`) do item:
/// `None` se o item não é de arma; `Some(false)` se recusado (munição ≥ 99 numa arma que já
/// tem). Quem dá a arma aceita sempre quando o "tem" é 0. A dificuldade fica em ×1.
pub fn take_weapon(kind: u32, v: i32, inv: &mut Inventory) -> Option<bool> {
    let (slot, gives, amount) = match kind {
        PISTOL_ITEM => (PISTOL, true, 3 * v),
        AMMO_ITEM => (PISTOL, false, 3 * v),
        ROCKET_ITEM => (ROCKETS, true, v),
        GRENADE_ITEM => (GRENADES, true, v),
        _ => return None,
    };
    let (has, qty) = inv.slots[slot];
    if !(gives && has == 0) && qty >= AMMO_CAP {
        return Some(false);
    }
    if gives {
        inv.slots[slot].0 = 1;
    }
    if amount != 0 {
        inv.slots[slot].1 = (qty + amount.max(1)).min(AMMO_CAP);
    }
    Some(true)
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

/// Velocidade de queda (1/8 px) do item que cai.
#[derive(Debug, Clone, Default)]
pub struct PickupState {
    pub vy: i32,
}

/// Sem `0x8` vindo da fase, o item cai (pistola e comida dos tutoriais).
const NO_GRAVITY: i32 = 0x8;

pub fn tick(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    if e.layer != ctx.jake_layer {
        return;
    }
    if e.flags & NO_GRAVITY == 0 {
        if let Some(b) = e.bounds() {
            let mut body = Body::new(
                e.x,
                e.y,
                crate::physics::Rect {
                    x: b.x - e.x,
                    y: b.y - e.y,
                    ..b
                },
            );
            if let crate::entity::Behaviour::Pickup(st) = &e.behaviour {
                body.vy = st.vy;
            }
            step(&mut body, ctx.world, true);
            e.x = body.x;
            e.y = body.y;
            if let crate::entity::Behaviour::Pickup(st) = &mut e.behaviour {
                st.vy = if body.on_ground { 0 } else { body.vy.max(0) };
            }
        }
    }
    if e.flags & JAKE_CAN_TAKE == 0 {
        return;
    }
    let (Some(item), Some(j)) = (e.bounds(), ctx.jake_bounds()) else {
        return;
    };
    if !item.overlaps(&j) {
        return;
    }
    let Some(jake) = ctx.jake else { return };
    let mut inv = out
        .inventory
        .clone()
        .unwrap_or_else(|| jake.inventory.clone());
    let v = e.values.first().copied().unwrap_or(0);
    if let Some(ok) = take_weapon(e.kind, v, &mut inv) {
        if ok {
            out.inventory = Some(inv);
            out.events.push(Event::Sound(crate::sound::Cue::ui(
                SOUND_PICKUP,
                crate::sound::PRIORITY_PICKUP,
            )));
            e.alive = false;
        }
        return;
    }
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
        // comida (+100 até 199) e armaduras 76/77/78
        let mut h = Health { hp: 150, armor: 0 };
        assert!(take(34, &mut h));
        assert_eq!(h.hp, 199);
        for (k, a) in [(76, 50), (77, 100), (78, 20)] {
            let mut h = Health { hp: 50, armor: 0 };
            assert!(take(k, &mut h));
            assert_eq!(h.armor, a);
        }
        // longe: nada
        let mut item = logic(HEALTH_ITEM, 100, 0, &[]);
        item.flags = 0x803a9;
        let (j, w) = ctx_with_jake(400, 0);
        let mut out = Out::default();
        tick(&mut item, &ctx(&j, &w, &on), &mut out);
        assert!(item.alive && out.heal.is_none());
    }

    #[test]
    fn weapons_and_ammo_follow_the_rules() {
        let mut inv = Inventory::from_points(&[(0, 0), (1, 20), (0, 90)]);
        assert_eq!(inv.pistol_ammo(), 0, "sem o \"tem\", a pistola não conta");
        assert_eq!(take_weapon(PISTOL_ITEM, 20, &mut inv), Some(true));
        assert_eq!(inv.slots[PISTOL], (1, 99), "90 + 60, teto 99");
        assert_eq!(take_weapon(AMMO_ITEM, 25, &mut inv), Some(false), "cheia");
        assert_eq!(take_weapon(PISTOL_ITEM, 20, &mut inv), Some(false));
        inv.slots[PISTOL].1 = 10;
        assert_eq!(take_weapon(AMMO_ITEM, 25, &mut inv), Some(true));
        assert_eq!(inv.slots[PISTOL].1, 85);
        assert_eq!(take_weapon(GRENADE_ITEM, 3, &mut inv), Some(true));
        assert_eq!(inv.slots[6], (1, 3));
        assert_eq!(take_weapon(ROCKET_ITEM, 5, &mut inv), Some(true));
        assert_eq!(inv.slots[4], (1, 5));
        assert_eq!(take_weapon(HEALTH_ITEM, 5, &mut inv), None);
    }

    #[test]
    fn pistol_falls_and_is_picked_up() {
        let mut gun = logic(PISTOL_ITEM, 100, -40, &[20]);
        gun.flags = 0x801a5;
        gun.behaviour = crate::entity::Behaviour::Pickup(PickupState::default());
        gun.boxes = vec![crate::physics::Rect {
            x: 0,
            y: 0,
            w: 10,
            h: 15,
        }];
        let (j, w) = ctx_with_jake(300, 0);
        let on = |_: i32| None;
        for _ in 0..20 {
            tick(&mut gun, &ctx(&j, &w, &on), &mut Out::default());
        }
        assert_eq!(gun.y + 15, 47, "pousou no chão");
        let (j, w) = ctx_with_jake(95, 0);
        let mut out = Out::default();
        tick(&mut gun, &ctx(&j, &w, &on), &mut out);
        assert!(!gun.alive);
        assert_eq!(out.inventory.unwrap().slots[PISTOL].0, 1);
    }
}
