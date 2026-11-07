// SPDX-License-Identifier: GPL-3.0-or-later
//! Itens: vida, armadura, armas e munição (coleta genérica `0x0040b3f5`; `docs/re/itens.md`,
//! `docs/re/armas.md`).

use super::{Ctx, Event, Out};
use crate::character::Character;
use crate::damage::Health;
use crate::entity::{ARMOR_ITEM, Entity, HEALTH_ITEM};
use crate::jake::weapons::{CLUB, G_GRENADES, G_STARS, KNIVES, WHIP};
use crate::jake::{GRENADES, Inventory, PISTOL, ROCKETS, SHOTGUN, STARS};
use crate::physics::{Body, step};

/// Som "Pickup generic object".
pub const SOUND_PICKUP: u16 = 5057;

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

/// Tipos de arma e munição: pistola (12), espingarda (13), lança-foguetes (14), munição da
/// pistola (16), cartuchos (17), foguetes (18), granadas (26), estrelas (27).
pub const PISTOL_ITEM: u32 = 12;
pub const SHOTGUN_ITEM: u32 = 13;
pub const ROCKET_ITEM: u32 = 14;
pub const AMMO_ITEM: u32 = 16;
pub const SHELLS_ITEM: u32 = 17;
pub const ROCKETS_ITEM: u32 = 18;
pub const GRENADE_ITEM: u32 = 26;
pub const STAR_ITEM: u32 = 27;
/// Armas do Garathe: clava (15), faca (25), chicote (44) e mais chicote (45).
pub const CLUB_ITEM: u32 = 15;
pub const KNIFE_ITEM: u32 = 25;
pub const WHIP_ITEM: u32 = 44;
pub const WHIP_AMMO_ITEM: u32 = 45;
/// Chaves (62–67) e bandeiras (80–85): pares `tipo − 49` e `tipo − 73`.
pub const KEYS: std::ops::RangeInclusive<u32> = 62..=67;
pub const FLAGS: std::ops::RangeInclusive<u32> = 80..=85;
/// Som "Pickup flag or key".
pub const SOUND_KEY: u16 = 5060;
/// Teto da munição (`0x40a36d`).
pub const AMMO_CAP: i32 = 99;

/// Arma e munição (`0x409fe8` + aceitação `0x40b6ed`) com o valor `v` (`values[0]`) do item:
/// `None` se o item não é de arma, chave ou bandeira; `Some(false)` se recusado (munição ≥ 99
/// numa arma que já tem). Quem dá a arma aceita sempre quando o "tem" é 0 e pode torná-la a
/// corrente (`0x43af6c`, antes de a arma entrar). Chaves e bandeiras: "tem" = 1 e +1. Na
/// dificuldade média; `take_weapon_at` escala a munição.
pub fn take_weapon(kind: u32, v: i32, inv: &mut Inventory) -> Option<bool> {
    take_weapon_at(kind, v, inv, crate::Difficulty::Normal)
}

/// `take_weapon` com a munição escalada pela dificuldade (`0x40a36d`: ×1,5 / ×1 / ×0,75).
pub fn take_weapon_at(
    kind: u32,
    v: i32,
    inv: &mut Inventory,
    difficulty: crate::Difficulty,
) -> Option<bool> {
    if KEYS.contains(&kind) || FLAGS.contains(&kind) {
        let slot = (kind - if KEYS.contains(&kind) { 49 } else { 73 }) as usize;
        inv.slots[slot].0 = 1;
        inv.slots[slot].1 = (inv.slots[slot].1 + 1).min(AMMO_CAP);
        return Some(true);
    }
    let (slot, gives, amount) = match (inv.character, kind) {
        // Garathe (`0x409fe8`): 15, 25, 26, 27, 44 dão as armas 2, 4, 5, 6, 3 + v; 45 soma v
        (Character::Garathe, CLUB_ITEM) => (CLUB, true, v),
        (Character::Garathe, KNIFE_ITEM) => (KNIVES, true, v),
        (Character::Garathe, GRENADE_ITEM) => (G_GRENADES, true, v),
        (Character::Garathe, STAR_ITEM) => (G_STARS, true, v),
        (Character::Garathe, WHIP_ITEM) => (WHIP, true, v),
        (Character::Garathe, WHIP_AMMO_ITEM) => (WHIP, false, v),
        (Character::Garathe, _) => return None,
        (_, CLUB_ITEM | KNIFE_ITEM | WHIP_ITEM | WHIP_AMMO_ITEM) => return None,
        (_, kind) => match kind {
            PISTOL_ITEM => (PISTOL, true, 3 * v),
            AMMO_ITEM => (PISTOL, false, 3 * v),
            SHOTGUN_ITEM => (SHOTGUN, true, v),
            SHELLS_ITEM => (SHOTGUN, false, v),
            ROCKET_ITEM => (ROCKETS, true, v),
            ROCKETS_ITEM => (ROCKETS, false, v),
            GRENADE_ITEM => (GRENADES, true, v),
            STAR_ITEM => (STARS, true, v),
            _ => return None,
        },
    };
    let (has, qty) = inv.slots[slot];
    if !(gives && has == 0) && qty >= AMMO_CAP {
        return Some(false);
    }
    if gives {
        inv.gained(slot);
        inv.slots[slot].0 = 1;
    }
    if amount != 0 {
        inv.slots[slot].1 = (qty + difficulty.ammo(amount.max(1))).min(AMMO_CAP);
    }
    Some(true)
}

/// Som da coleta: 5060 para chaves, bandeiras e a peça do carro; senão 5057.
pub fn pickup_sound(kind: u32) -> u16 {
    if KEYS.contains(&kind) || FLAGS.contains(&kind) || kind == crate::entity::CAR_PIECE {
        SOUND_KEY
    } else {
        SOUND_PICKUP
    }
}

/// A estrela cravada (`0x40bcd7`): o Jake que a toca recolhe 1 estrela (coleta genérica).
pub fn collect_star(e: &mut Entity, ctx: &Ctx, out: &mut Out) {
    let (Some(item), Some(j), Some(jake)) = (e.bounds(), ctx.jake_bounds(), ctx.jake) else {
        return;
    };
    if e.layer != ctx.jake_layer || !item.overlaps(&j) {
        return;
    }
    let mut inv = out
        .inventory
        .clone()
        .unwrap_or_else(|| jake.inventory.clone());
    // Ruling: a faca cravada devolve 1 faca, como a estrela (o valor não foi lido)
    let item = match &e.behaviour {
        crate::entity::Behaviour::Star(s) if s.knife => KNIFE_ITEM,
        _ => STAR_ITEM,
    };
    if take_weapon(item, 1, &mut inv) == Some(true) {
        out.inventory = Some(inv);
        out.events.push(Event::Sound(crate::sound::Cue::ui(
            SOUND_PICKUP,
            crate::sound::PRIORITY_PICKUP,
        )));
        e.alive = false;
    }
}

/// Veneno (33, 35: +1 em `+0x270`) e antídoto (37: zera; recusado sem veneno), da tabela
/// `0x00471d10`. `None` se o item não é desses.
pub fn take_poison(kind: u32, poison: &mut i32) -> Option<bool> {
    match kind {
        33 | 35 => {
            *poison += 1;
            Some(true)
        }
        37 if *poison == 0 => Some(false),
        37 => {
            *poison = 0;
            Some(true)
        }
        _ => None,
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
    // o Jake pega com 0x100, o Garathe com 0x200 (`itens.md`, passo 2)
    let can_take = ctx.jake.map_or(0x100, |j| j.profile().take_bit);
    if e.flags & can_take == 0 {
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
    if let Some(ok) = take_weapon_at(e.kind, v, &mut inv, ctx.difficulty) {
        if ok {
            out.inventory = Some(inv);
            out.events.push(Event::Sound(crate::sound::Cue::ui(
                pickup_sound(e.kind),
                crate::sound::PRIORITY_PICKUP,
            )));
            // apanhado fica ligado (`+0x178 = 0x40`, `itens.md` passo 11)
            e.on = true;
            e.alive = false;
        }
        return;
    }
    let mut poison = out.poison.unwrap_or(jake.poison);
    if let Some(ok) = take_poison(e.kind, &mut poison) {
        if ok {
            out.poison = Some(poison);
            out.events.push(Event::Sound(crate::sound::Cue::ui(
                SOUND_PICKUP,
                crate::sound::PRIORITY_PICKUP,
            )));
            e.on = true;
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
        e.on = true;
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

    /// `itens.md` passo 11: o item apanhado fica ligado (`+0x178 = 0x40`); é o que cumpre o
    /// objetivo "acionar" das granadas da I_J_107.
    #[test]
    fn collected_items_turn_on() {
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 0);
        for kind in [HEALTH_ITEM, GRENADE_ITEM] {
            let mut item = logic(kind, 100, 0, &[3]);
            item.flags = 0x803a9;
            tick(&mut item, &ctx(&j, &w, &on), &mut Out::default());
            assert!(!item.alive && item.on, "tipo {kind}");
        }
    }

    #[test]
    fn campaign_ammo_keys_and_flags() {
        use crate::jake::{FISTS, ROCKETS, SHOTGUN, STARS};
        let mut inv = Inventory::from_points(&[(0, 0), (1, 20)]);
        assert_eq!(inv.weapon, FISTS);
        assert_eq!(take_weapon(SHELLS_ITEM, 10, &mut inv), Some(true));
        assert_eq!(
            inv.slots[SHOTGUN],
            (0, 10),
            "cartuchos sem a arma não ligam o \"tem\""
        );
        assert_eq!(take_weapon(SHOTGUN_ITEM, 20, &mut inv), Some(true));
        assert_eq!(inv.slots[SHOTGUN], (1, 30));
        assert_eq!(inv.weapon, SHOTGUN, "a espingarda é melhor que os punhos");
        assert_eq!(take_weapon(ROCKETS_ITEM, 3, &mut inv), Some(true));
        assert_eq!(take_weapon(ROCKET_ITEM, 5, &mut inv), Some(true));
        assert_eq!((inv.slots[ROCKETS], inv.weapon), ((1, 8), ROCKETS));
        assert_eq!(take_weapon(STAR_ITEM, 10, &mut inv), Some(true));
        assert_eq!(inv.slots[STARS], (1, 10));
        assert_eq!(inv.weapon, ROCKETS, "as estrelas são piores");
        // chave 63 → par 14; bandeira 85 → par 12
        assert_eq!(take_weapon(63, 0, &mut inv), Some(true));
        assert_eq!(take_weapon(85, 0, &mut inv), Some(true));
        assert_eq!((inv.slots[14], inv.slots[12]), ((1, 1), (1, 1)));
        assert_eq!(pickup_sound(63), SOUND_KEY);
        assert_eq!(pickup_sound(SHELLS_ITEM), SOUND_PICKUP);
    }

    #[test]
    fn poison_and_antidote() {
        let mut p = 0;
        assert_eq!(take_poison(37, &mut p), Some(false), "sem veneno, recusa");
        assert_eq!(take_poison(33, &mut p), Some(true));
        assert_eq!(take_poison(35, &mut p), Some(true));
        assert_eq!(p, 2);
        assert_eq!(take_poison(37, &mut p), Some(true));
        assert_eq!(p, 0);
        assert_eq!(take_poison(HEALTH_ITEM, &mut p), None);
        // no Jake: perde `veneno` a cada 20 tiques, sem a armadura absorver
        let (mut j, w) = ctx_with_jake(100, 0);
        j.health.armor = 50;
        j.poison = 2;
        let hp = j.health.hp;
        for _ in 0..40 {
            j.tick(0, &w);
        }
        assert_eq!((j.health.hp, j.health.armor), (hp - 4, 50));
        // apanhar o veneno no contato
        let on = |_: i32| None;
        let (j, w) = ctx_with_jake(100, 0);
        let mut item = logic(33, 100, 0, &[]);
        item.flags = 0x803a9;
        let mut out = Out::default();
        tick(&mut item, &ctx(&j, &w, &on), &mut out);
        assert_eq!(out.poison, Some(1));
        assert!(!item.alive);
    }

    #[test]
    fn ammo_pickups_scale_with_difficulty() {
        use crate::Difficulty::{Easy, Hard, Normal};
        let got = |d| {
            let mut inv = Inventory::from_points(&[]);
            let before = inv.slots[PISTOL].1;
            take_weapon_at(AMMO_ITEM, 10, &mut inv, d);
            inv.slots[PISTOL].1 - before
        };
        assert_eq!([got(Easy), got(Normal), got(Hard)], [45, 30, 22]);
        let mut inv = Inventory::from_points(&[]);
        take_weapon_at(63, 0, &mut inv, Easy);
        assert_eq!(inv.slots[63 - 49], (1, 1), "chaves não escalam");
    }

    #[test]
    fn garathe_weapons_and_swapped_grenades_and_stars() {
        let mut inv = Inventory::for_character(&[], Character::Garathe);
        assert_eq!(take_weapon(PISTOL_ITEM, 20, &mut inv), None, "só o Jake");
        assert_eq!(take_weapon(CLUB_ITEM, 1, &mut inv), Some(true));
        assert_eq!(inv.slots[CLUB], (1, 1));
        assert_eq!(
            inv.weapon, CLUB,
            "melhor que as garras na lista 1,2,5,4,6,3"
        );
        assert_eq!(take_weapon(KNIFE_ITEM, 10, &mut inv), Some(true));
        assert_eq!((inv.slots[KNIVES], inv.weapon), ((1, 10), KNIVES));
        assert_eq!(take_weapon(GRENADE_ITEM, 3, &mut inv), Some(true));
        assert_eq!(inv.slots[G_GRENADES], (1, 3), "26 dá a arma 5 ao Garathe");
        assert_eq!(take_weapon(STAR_ITEM, 4, &mut inv), Some(true));
        assert_eq!(inv.slots[G_STARS], (1, 4), "27 dá a arma 6");
        assert_eq!(take_weapon(WHIP_AMMO_ITEM, 5, &mut inv), Some(true));
        assert_eq!(inv.slots[WHIP], (0, 5), "45 só soma");
        assert_eq!(take_weapon(WHIP_ITEM, 5, &mut inv), Some(true));
        assert_eq!((inv.slots[WHIP], inv.weapon), ((1, 10), WHIP));
        let mut jake = Inventory::from_points(&[]);
        assert_eq!(take_weapon(CLUB_ITEM, 1, &mut jake), None);
        assert_eq!(take_weapon(GRENADE_ITEM, 3, &mut jake), Some(true));
        assert_eq!(jake.slots[GRENADES], (1, 3));
    }
}
