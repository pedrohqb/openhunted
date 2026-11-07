// SPDX-License-Identifier: GPL-3.0-or-later
//! Tabela de estados do Garathe (`0x478980 + 0x70·s`, `docs/re/garathe-movimentos.md`):
//! rótulo do `MINOTAUR.ANM` e atraso (`+0x18`) de cada estado semântico da máquina comum.

use super::weapons::{self, weapon_of};
use super::{Climb, JumpKind, Landing, State};
use crate::character::Character;

/// Mortes (`0x428863`): veneno 61 `MNDIE1`, geral 63 `MNDIE2B`, derrubado para a frente
/// (`+0x250 & 8`) 67 `MNFALL3`, para trás (`& 0x10`) 69 `MNDIE3`.
pub const DIE_POISON: u16 = 61;
pub const DIE_DEFAULT: u16 = 63;
pub const DIE_FORWARD: u16 = 67;
pub const DIE_BACK: u16 = 69;
/// O choque (121 `MNQUAKE`) começa no quadro 17 (`0x428713`).
pub const SHOCK_FRAME: usize = 17;
/// Fim do pulo para cima da corda (19): parado e sem gravidade até o quadro 9, então
/// `vy = −30` px/tique (`0x4296df`).
pub const ROPE_JUMP_FRAME: usize = 9;
pub const ROPE_JUMP_VY: i32 = -30;
/// Sons: aterrissagem do pulo longo, empurrar e pulo contra a parede (médio/longo).
pub const SOUND_LONG_LANDING: u16 = 5053;
pub const SOUND_PUSH: u16 = 5043;
pub const SPEECH_WALL_MEDIUM: u16 = 3734;
pub const SPEECH_WALL_LONG: u16 = 3740;
/// Investida do pulo longo (`0x429bda`): 7 de dano com tranco e empurrão.
pub const BODY_CHECK_DAMAGE: i32 = 7;
pub const BODY_CHECK_PUSH: i32 = 8;

/// Rótulo e atraso do estado `s` no Garathe.
pub(super) fn anim(s: State) -> (u16, u32) {
    let g = Character::Garathe;
    match s {
        State::Idle | State::Teleporting(_) | State::Shake => (1, 1),
        State::Run => (10, 0),
        State::Push => (46, 1),
        State::StandJump => (21, 0),
        State::RunUp => (24, 1),
        State::Jump(JumpKind::Medium) => (22, 1),
        State::Jump(_) => (25, 1),
        State::Fall => (27, 0),
        State::WildFall => (29, 0),
        State::Land(Landing::Fall) => (28, 0),
        State::Land(Landing::Medium) => (23, 1),
        State::Land(Landing::Long) => (24, 2),
        State::Land(Landing::Hard) => (30, 0),
        State::GetUp => (31, 0),
        State::Rise => (35, 0),
        State::Jolted => (63, 0),
        State::Taunt => (76, 0),
        State::Pushed => (58, 0),
        State::KnockedDown(_) => (32, 1),
        State::Shocked(_) => (121, 1),
        State::Dying(label) => (label, 0),
        State::Dead => (0, 0),
        State::EnteringExit(_) => (12, 0),
        State::Fire { aim, w, alt, .. } => (
            weapon_of(g, w as usize).states[weapons::aim_slot(aim)][usize::from(alt)].0,
            0,
        ),
        State::Recover(w) => (weapon_of(g, w as usize).states[2][0].1.recover, 0),
        State::Ready(_, w) => (weapon_of(g, w as usize).ready, 0),
        State::Holster(w) => (weapon_of(g, w as usize).holster, 0),
        State::Stealth => (103, 0),
        State::JumpUp(false) => (20, 0),
        State::JumpUp(true) => (19, 0),
        State::Special(_) => (127, 0),
        State::Grab | State::LetGo => (4, 0),
        State::Climb(Climb::Vertical) => (52, 0),
        State::Climb(Climb::Side) => (53, 0),
        State::LayerDown(_) => (11, 0),
        State::LayerUp(_) => (12, 0),
        State::Crouch | State::StandUp => (37, 0),
        State::Crawl => (36, 0),
        State::Hide | State::Unhide => (54, 0),
        State::Hidden => (56, 0),
        State::BarHang => (47, 0),
        State::BarMove => (49, 1),
        State::BarPull | State::BarDown | State::BarUp => (50, 0),
    }
}

/// Rótulo do derrubado: 32 `BLASTFWD` se jogado para o lado em que olha, senão 33
/// `BLASTBK0` (`0x428531`, lado × sinal de `+0x244`).
pub(super) fn knocked_label(right: bool, facing_right: bool) -> u16 {
    if right == facing_right { 32 } else { 33 }
}

/// Estados que tocam o rótulo ao contrário: 5 (4), 38 (37), 55 (54), 51 (50), 26 (24).
pub(super) fn reversed(s: State) -> bool {
    matches!(
        s,
        State::LetGo | State::StandUp | State::Unhide | State::BarDown | State::Land(Landing::Long)
    )
}

/// Estados de arte espelhada (fim do tique nega `+0x16c`/`+0x164`): o andar 3 e os de arma.
pub(super) fn mirrored(s: State) -> bool {
    matches!(
        s,
        State::Stealth
            | State::Fire { .. }
            | State::Ready(..)
            | State::Holster(_)
            | State::Recover(_)
    )
}

/// Todos os rótulos usados (para conferir contra o `MINOTAUR.ANM` real).
pub const ALL_LABELS: [u16; 48] = [
    1,
    76,
    10,
    46,
    21,
    24,
    22,
    25,
    27,
    29,
    28,
    23,
    30,
    31,
    35,
    63,
    58,
    32,
    33,
    121,
    12,
    103,
    20,
    19,
    127,
    4,
    52,
    53,
    11,
    37,
    36,
    54,
    56,
    47,
    49,
    50,
    DIE_POISON,
    DIE_FORWARD,
    DIE_BACK,
    100,
    110,
    152,
    81,
    140,
    128,
    85,
    130,
    154,
];

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::rc::Rc;

    use openhunted_formats::Skeleton;
    use openhunted_formats::test_support::{SynthSkeletonEntry, mkske};

    use super::super::{Aim, Inventory, Jake, ShotKind, Skel, weapons};
    use super::*;
    use crate::damage::Pending;
    use crate::input::{DOWN, JUMP, LEFT, PUNCH, RIGHT, SHOOT, STEALTH, UP};
    use crate::physics::{Rect, SUB, World};

    /// Caixa envolvente do `MN_DEFLT` (pés em y + 56).
    const BOX: Rect = Rect {
        x: 10,
        y: 5,
        w: 20,
        h: 51,
    };

    fn skeleton() -> Skel {
        let xs = |v: &[i16]| v.iter().map(|&x| (x, 0)).collect::<Vec<_>>();
        let ys = |v: &[i16]| v.iter().map(|&y| (0, y)).collect::<Vec<_>>();
        let mut e: Vec<SynthSkeletonEntry> = vec![(0, (0, 0), vec![], 0); 170];
        e[103] = (0, (0, 0), xs(&[0, -5, -9, -13]), 0);
        e[21] = (0, (0, 0), ys(&[0, 0, 4, 9, 9, 4, 0]), 0);
        e[20] = (0, (0, 0), ys(&[0, 0, 6, 9, 9, 6, 0]), 0);
        e[36] = (0, (0, 0), (0..10).map(|i| (-2 * i, 0)).collect(), 0);
        e[49] = (0, (0, 0), (0..10).map(|i| (-i, 0)).collect(), 0);
        e[63] = (0, (0, 0), xs(&[0, 3, 6, 9]), 0);
        let ske = Skeleton::parse(&mkske(2, (39, 58), &e)).unwrap();
        Skel::new(Rc::new(ske), (39, 58))
    }

    fn lengths() -> HashMap<u16, usize> {
        let mut m: HashMap<u16, usize> = ALL_LABELS.iter().map(|&l| (l, 6)).collect();
        for (l, n) in [
            (1, 47),
            (10, 9),
            (103, 4),
            (21, 7),
            (20, 7),
            (19, 15),
            (36, 10),
            (49, 10),
            (63, 4),
            (121, 22),
            (127, 28),
        ] {
            m.insert(l, n);
        }
        for l in weapons::labels_of(Character::Garathe) {
            m.entry(l).or_insert(12);
        }
        m
    }

    fn floor_world() -> World {
        World::new(vec![Rect {
            x: -10_000,
            y: 100,
            w: 20_000,
            h: 20,
        }])
    }

    fn garathe(world: &World, x: i32) -> Jake {
        let mut j = Jake::new(x, 44, BOX, lengths())
            .with_skeleton(skeleton())
            .with_character(Character::Garathe);
        j.tick(0, world);
        assert!(j.body.on_ground);
        j
    }

    fn run(j: &mut Jake, w: &World, input: u32, n: usize) {
        for _ in 0..n {
            j.tick(input, w);
        }
    }

    #[test]
    fn runs_at_fifteen_and_walks_in_fighting_stance() {
        let w = floor_world();
        let mut j = garathe(&w, 0);
        assert_eq!(j.label(), 1);
        j.tick(RIGHT, &w);
        let x0 = j.body.x;
        j.tick(RIGHT, &w);
        assert_eq!((j.state(), j.label()), (State::Run, 10));
        assert_eq!(j.body.x - x0, 15, "15 px/tique (`0x47d144`)");
        // FURTIVO: andar 3 (rótulo 103), arte espelhada, esqueleto com k = 3
        let mut j = garathe(&w, 0);
        j.tick(LEFT | STEALTH, &w);
        assert_eq!((j.state(), j.label()), (State::Stealth, 103));
        assert!(
            j.sprite_mirrored(),
            "olhando à esquerda, a arte espelhada vira"
        );
        let x0 = j.body.x;
        j.tick(LEFT | STEALTH, &w);
        // à esquerda, "para a frente" é x negativo: Δ −5 da raiz (espelhado) × 3
        assert_eq!((j.body.x - x0).abs(), 15);
    }

    #[test]
    fn jumps_follow_the_table_and_long_has_a_prep() {
        // chão contínuo: médio (13, −12)
        let w = floor_world();
        let mut j = garathe(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | JUMP, &w);
        assert_eq!(j.state(), State::Jump(JumpKind::Medium));
        assert_eq!(j.label(), 22);
        assert_eq!(j.body.vx, 13 * SUB);
        // fosso entre 160 e 330: sem chão a 156, com chão a 350 → preparo 24 → longo 25
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 100,
                w: 1060,
                h: 20,
            },
            Rect {
                x: 330,
                y: 100,
                w: 2000,
                h: 20,
            },
        ]);
        let mut j = garathe(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | JUMP, &w);
        assert_eq!((j.state(), j.label()), (State::RunUp, 24));
        let mut n = 0;
        while j.state() == State::RunUp {
            j.tick(RIGHT, &w);
            n += 1;
            assert!(n < 20);
        }
        assert_eq!((j.state(), j.label()), (State::Jump(JumpKind::Long), 25));
        assert_eq!(j.body.vx, 25 * SUB);
        run(&mut j, &w, RIGHT, 40);
        assert!(j.body.x > 330, "o longo cruza o fosso: {}", j.body.x);
        // correndo, PULAR+CIMA+lado → longo direto
        let w = floor_world();
        let mut j = garathe(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | UP | JUMP, &w);
        assert_eq!(j.state(), State::Jump(JumpKind::Long));
        assert_eq!(
            j.body.vy,
            -14 * SUB + 2 * SUB,
            "lançado a −14 e um tique de gravidade"
        );
    }

    #[test]
    fn standing_and_up_jumps_use_their_skeleton_factors() {
        let w = floor_world();
        let mut j = garathe(&w, 0);
        j.tick(JUMP, &w);
        assert_eq!((j.state(), j.label()), (State::StandJump, 21));
        j.tick(0, &w);
        j.tick(0, &w);
        // quadro 2: Δy = 4 para cima × 3
        assert_eq!(j.body.vy, -12 * SUB + 2 * SUB);
        let mut j = garathe(&w, 0);
        j.tick(JUMP | UP, &w);
        assert_eq!((j.state(), j.label()), (State::JumpUp(false), 20));
        j.tick(UP, &w);
        j.tick(UP, &w);
        // quadro 2: Δy = 6 × 2
        assert_eq!(j.body.vy, -12 * SUB + 2 * SUB);
    }

    #[test]
    fn crawls_with_k5_and_needs_20_px_to_stand() {
        let w = floor_world();
        let mut j = garathe(&w, 100).with_boxes(
            Rect {
                x: 10,
                y: 20,
                w: 20,
                h: 36,
            },
            Rect {
                x: 10,
                y: 5,
                w: 20,
                h: 21,
            },
        );
        run(&mut j, &w, DOWN, 6);
        assert_eq!((j.state(), j.label()), (State::Crawl, 36));
        let x0 = j.body.x;
        j.tick(DOWN | LEFT, &w);
        j.tick(DOWN | LEFT, &w);
        assert_eq!(x0 - j.body.x, 2 * 5 + 2 * 5, "Δ 2 × k 5 por tique");
        // teto a 21 px da caixa de rastejar: cabe a folga de 20, levanta
        assert_eq!(Character::Garathe.profile().headroom, 20);
        j.tick(0, &w);
        assert_eq!(j.state(), State::StandUp);
    }

    #[test]
    fn bar_grab_hand_over_hand_and_release() {
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 300,
                w: 3000,
                h: 20,
            },
            // barra: teto logo acima da caixa
            Rect {
                x: -1000,
                y: 30,
                w: 3000,
                h: 19,
            },
        ]);
        let mut j = Jake::new(100, 44, BOX, lengths())
            .with_skeleton(skeleton())
            .with_character(Character::Garathe);
        j.set(State::BarHang);
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::BarHang, 47));
        j.tick(LEFT, &w);
        assert_eq!((j.state(), j.label()), (State::BarPull, 50));
        run(&mut j, &w, LEFT, 7);
        assert_eq!((j.state(), j.label()), (State::BarMove, 49));
        let x0 = j.body.x;
        run(&mut j, &w, LEFT, 4);
        assert!(x0 - j.body.x >= 6, "mão ante mão, k = 6: {}", x0 - j.body.x);
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::BarDown, 50));
        assert!(j.reversed());
        run(&mut j, &w, 0, 8);
        assert_eq!(j.state(), State::BarHang);
        j.tick(DOWN, &w);
        assert_eq!(j.state(), State::Fall);
    }

    #[test]
    fn hides_in_54_to_56() {
        let w = floor_world();
        let mut j = garathe(&w, 0);
        j.tick(UP, &w);
        assert_eq!((j.state(), j.label()), (State::Hide, 54));
        assert!(j.is_hidden());
        run(&mut j, &w, UP, 6);
        assert_eq!((j.state(), j.label()), (State::Hidden, 56));
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::Unhide, 54));
        assert!(j.reversed() && j.is_hidden());
    }

    #[test]
    fn reactions_and_deaths() {
        let w = floor_world();
        // empurrão em pé → 58
        let mut j = garathe(&w, 0);
        j.hurt(Pending {
            amount: 1,
            shove: true,
            push: (3, 0),
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::Pushed, 58));
        // correndo, o empurrão não pega (só 1, 54–56, 58)
        let mut j = garathe(&w, 0);
        j.tick(RIGHT, &w);
        j.hurt(Pending {
            amount: 1,
            shove: true,
            push: (3, 0),
            ..Default::default()
        });
        j.tick(RIGHT, &w);
        assert_eq!(j.state(), State::Run);
        // derrubada para o lado em que olha (esquerda) → 32 → no chão 30
        let mut j = garathe(&w, 0);
        j.hurt(Pending {
            amount: 1,
            knockdown: true,
            push: (-4, -6),
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!(j.label(), 32);
        run(&mut j, &w, 0, 30);
        assert!(matches!(
            j.state(),
            State::Land(Landing::Hard) | State::GetUp | State::Idle
        ));
        // para trás → 33 → 35
        let mut j = garathe(&w, 0);
        j.hurt(Pending {
            amount: 1,
            knockdown: true,
            push: (4, -6),
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!(j.label(), 33);
        let mut seen_rise = false;
        for _ in 0..30 {
            j.tick(0, &w);
            seen_rise |= j.state() == State::Rise;
        }
        assert!(seen_rise);
        // tranco → 59 (rótulo 63) → 35
        let mut j = garathe(&w, 0);
        j.hurt(Pending {
            amount: 1,
            jolt: true,
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!((j.state(), j.label()), (State::Jolted, 63));
        run(&mut j, &w, 0, 6);
        assert_eq!((j.state(), j.label()), (State::Rise, 35));
        // choque → 121 a partir do quadro 17
        let mut j = garathe(&w, 0);
        j.hurt(Pending {
            amount: 1,
            electric: true,
            ..Default::default()
        });
        j.tick(0, &w);
        assert_eq!((j.label(), j.frame()), (121, SHOCK_FRAME));
        // mortes
        for (p, label) in [
            (
                Pending {
                    amount: 500,
                    poison: true,
                    ..Default::default()
                },
                DIE_POISON,
            ),
            (
                Pending {
                    amount: 500,
                    ..Default::default()
                },
                DIE_DEFAULT,
            ),
            (
                Pending {
                    amount: 500,
                    knockdown: true,
                    push: (-3, 0),
                    ..Default::default()
                },
                DIE_FORWARD,
            ),
            (
                Pending {
                    amount: 500,
                    knockdown: true,
                    push: (3, 0),
                    ..Default::default()
                },
                DIE_BACK,
            ),
        ] {
            let mut j = garathe(&w, 0).with_health(10, 0);
            j.hurt(p);
            j.tick(0, &w);
            assert_eq!(j.state(), State::Dying(label));
        }
    }

    #[test]
    fn special_costs_one_per_tick_for_sixteen_ticks() {
        let w = floor_world();
        let mut j = garathe(&w, 0).with_health(100, 0);
        let mut blasts = 0;
        let mut ticks = 0;
        j.tick(DOWN | SHOOT, &w);
        assert_eq!(j.label(), 127);
        while matches!(j.state(), State::Special(_)) {
            blasts += usize::from(j.special.is_some());
            j.tick(DOWN, &w);
            ticks += 1;
            assert!(ticks < 40);
        }
        assert_eq!(blasts, 16, "efeito em 5 ≤ t ≤ 20");
        assert_eq!(j.health.hp, 100 - 16);
        assert_eq!(ticks, 24, "o estado acaba com t > 24");
    }

    fn armed(w: &World, slots: &[(usize, (i32, i32))]) -> Jake {
        let mut points = [(0, 0); 8];
        for &(s, v) in slots {
            points[s] = v;
        }
        let inv = Inventory::for_character(&points, Character::Garathe);
        garathe(w, 0).with_inventory(inv)
    }

    #[test]
    fn claws_club_and_whip_blows() {
        let w = floor_world();
        // sem clava: SOCO usa as garras (104/105), golpe 15 com empurrão em t = 3
        let mut j = armed(&w, &[]);
        assert_eq!(j.inventory.weapon, weapons::CLAWS);
        j.tick(PUNCH, &w);
        assert!(matches!(j.label(), 104 | 105));
        let mut hit = None;
        for t in 1..8 {
            j.tick(0, &w);
            if j.punch.is_some() {
                hit = Some((t, j.blow.unwrap()));
            }
        }
        let (t, b) = hit.expect("um golpe");
        assert_eq!(t, 3);
        assert_eq!((b.amount, b.shove, b.push), (15, true, (-2, 0)));
        // com a clava (par 2): SOCO usa a clava, 114 na frente, derrubada (20, 10)
        let mut j = armed(&w, &[(2, (1, 1))]);
        j.tick(PUNCH, &w);
        assert_eq!(j.label(), 114);
        let mut blow = None;
        for _ in 0..8 {
            j.tick(0, &w);
            blow = blow.or(j.blow);
        }
        let b = blow.unwrap();
        assert_eq!((b.amount, b.knockdown, b.push), (15, true, (-20, 10)));
        // chicote (par 3): golpe 18 e o projétil 108 em t = 10 (passo 2)
        let mut j = armed(&w, &[(3, (1, 5))]);
        assert_eq!(
            j.inventory.weapon,
            weapons::WHIP,
            "a melhor da lista 1,2,5,4,6,3"
        );
        j.tick(SHOOT, &w);
        assert_eq!(j.label(), 159);
        let mut shots = vec![];
        let mut blows = 0;
        for _ in 0..12 {
            j.tick(0, &w);
            shots.append(&mut j.shots);
            blows += usize::from(j.blow.is_some_and(|b| b.amount == 18));
        }
        assert_eq!(blows, 1);
        assert_eq!(shots.len(), 1);
        assert_eq!(shots[0].kind, ShotKind::Whip);
        assert_eq!((shots[0].vx, shots[0].vy), (-20, 0));
        assert_eq!(j.inventory.ammo(weapons::WHIP), 4);
    }

    #[test]
    fn thrown_weapons_use_the_garathe_slots() {
        let w = floor_world();
        for (slot, kind, label) in [
            (weapons::KNIVES, ShotKind::Knife, 95),
            (weapons::G_GRENADES, ShotKind::Grenade, 147),
            (weapons::G_STARS, ShotKind::Star, 135),
        ] {
            let mut j = armed(&w, &[(slot, (1, 3))]);
            j.inventory.weapon = slot;
            j.tick(SHOOT, &w);
            assert_eq!(j.label(), label);
            let mut shots = vec![];
            for _ in 0..8 {
                j.tick(0, &w);
                shots.append(&mut j.shots);
            }
            assert_eq!(shots.len(), 1, "{kind:?}");
            assert_eq!(shots[0].kind, kind);
            assert_eq!(j.inventory.ammo(slot), 2);
        }
        // estrelas para cima: 138, `+0x14/+0x18` 20/24
        let r = weapons::G_STARS_W.states[weapons::aim_slot(Aim::Up)][0].1;
        assert_eq!((r.again, r.reeval), (20, 24));
    }

    #[test]
    fn four_directions_taunt_with_label_76() {
        let w = floor_world();
        let mut j = garathe(&w, 0);
        j.tick(crate::jake::TAUNT_INPUT, &w);
        assert_eq!((j.state(), j.label()), (State::Taunt, 76));
        run(&mut j, &w, 0, 6);
        assert_eq!(j.state(), State::Idle, "76 com 6 quadros, depois parado");
    }
}
