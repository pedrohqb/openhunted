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
/// Empurrar (`0x42a9a4`): 44 `PUSHHAND` toca uma vez (atraso 0); no fim, som 5043 e o laço
/// 46 (atraso 1), em cuja entrada a zona 0x20 pede a porta de célula (`0x42aa74`).
pub const PUSH_START: u16 = 44;
/// Investida do pulo longo (`0x429bda`): 7 de dano com tranco (`+0x250 = 1`).
pub const BODY_CHECK_DAMAGE: i32 = 7;

/// Dano da investida em quem o Garathe toca: 7 e tranco, sem a reação de empurrão; o
/// empurrão (`+0x244/+0x248`, `0x435951`) é o centro da vítima menos o do Garathe (caixas
/// de colisão, `0x4351b5`).
pub fn body_check(
    garathe: crate::physics::Rect,
    victim: crate::physics::Rect,
) -> crate::damage::Pending {
    let c = |r: crate::physics::Rect| (r.x + r.w / 2, r.y + r.h / 2);
    let (g, v) = (c(garathe), c(victim));
    crate::damage::Pending {
        amount: BODY_CHECK_DAMAGE,
        jolt: true,
        push: (v.0 - g.0, v.1 - g.1),
        ..Default::default()
    }
}

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
        State::Recover(w) => pose(weapon_of(g, w as usize).states[2][0].1.recover),
        State::Ready(_, w) => pose(weapon_of(g, w as usize).ready),
        State::Holster(w) => pose(weapon_of(g, w as usize).holster),
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

/// Pronto/guardar/recuperação: o estado é o próprio rótulo; atraso 1 nos da tabela
/// (81, 82, 85, 111, 128–130, 140–142, 152–154), 0 nos demais (100, 101, 110).
fn pose(label: u16) -> (u16, u32) {
    let slow = matches!(label, 81 | 82 | 85 | 111 | 128..=130 | 140..=142 | 152..=154);
    (label, u32::from(slow))
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
pub const ALL_LABELS: [u16; 49] = [
    1,
    76,
    10,
    46,
    PUSH_START,
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
            (44, 3),
            (46, 24),
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

    /// Empurrar: 44 (3 quadros, atraso 0) → som 5043 → laço 46 (atraso 1); a porta de célula
    /// é pedida ao entrar no 46 (`0x42a9a4`, `0x42aa74`), não no fim do primeiro laço do 46.
    #[test]
    fn push_plays_44_then_loops_46_and_asks_the_door_on_entry() {
        let mut w = floor_world();
        w = World::new(
            w.solids()
                .iter()
                .copied()
                .chain([Rect {
                    x: 32,
                    y: 0,
                    w: 20,
                    h: 100,
                }])
                .collect(),
        );
        let mut j = garathe(&w, 0);
        let mut seen = vec![];
        for _ in 0..12 {
            j.tick(RIGHT, &w);
            seen.push((j.state(), j.label(), j.door_push, j.sound));
        }
        let first = seen
            .iter()
            .position(|s| s.0 == State::Push)
            .expect("empurra");
        let labels: Vec<u16> = seen[first..].iter().map(|s| s.1).collect();
        assert_eq!(&labels[..5], &[44, 44, 44, 46, 46]);
        let doors: Vec<usize> = (first..seen.len()).filter(|&i| seen[i].2).collect();
        assert_eq!(
            doors,
            vec![first + 3],
            "pede a porta uma vez, ao entrar no 46"
        );
        assert_eq!(seen[first + 3].3, Some(SOUND_PUSH));
        assert!(seen[first + 4..].iter().all(|s| s.3.is_none()));
    }

    /// Atraso (`+0x18`) dos pronto/guardar/recuperação: 1 em 81, 82, 85, 111, 128–130,
    /// 140–142 e 152–154; 0 nas garras (100, 101) e no pronto da clava (110).
    #[test]
    fn weapon_pose_delays_follow_the_state_table() {
        use weapons::{CLAWS, CLUB, G_GRENADES, G_STARS, KNIVES, WHIP};
        let d = |s: State| anim(s);
        assert_eq!(d(State::Ready(0, KNIVES as u8)), (81, 1));
        assert_eq!(d(State::Holster(KNIVES as u8)), (82, 1));
        assert_eq!(d(State::Recover(KNIVES as u8)), (85, 1));
        assert_eq!(d(State::Ready(0, CLUB as u8)), (110, 0));
        assert_eq!(d(State::Holster(CLUB as u8)), (111, 1));
        assert_eq!(d(State::Ready(0, CLAWS as u8)), (100, 0));
        assert_eq!(d(State::Holster(CLAWS as u8)), (101, 0));
        for w in [G_STARS, G_GRENADES, WHIP] {
            let (r, h, c) = (
                d(State::Ready(0, w as u8)),
                d(State::Holster(w as u8)),
                d(State::Recover(w as u8)),
            );
            assert_eq!((r.1, h.1, c.1), (1, 1, 1), "arma {w}: {r:?} {h:?} {c:?}");
        }
    }

    /// Tranco (`0x42859e`): o 38 (levantar) também o sente; na barra (47, 49, 50, 51) nada
    /// acontece; só a escalada 52/53 vira queda.
    #[test]
    fn jolt_reaches_the_stand_up_but_not_the_bar() {
        use super::super::moves::joltable;
        assert!(joltable(State::StandUp));
        for s in [
            State::BarHang,
            State::BarMove,
            State::BarPull,
            State::BarDown,
        ] {
            assert!(!joltable(s), "{s:?}");
        }
        assert!(joltable(State::Climb(Climb::Vertical)));
    }

    /// Investida (`0x429bda`): tranco e 7, empurrão pelos centros (`0x435951`), sem empurrão
    /// como reação.
    #[test]
    fn body_check_jolts_and_pushes_by_the_centres() {
        let r = |x, y, w, h| Rect { x, y, w, h };
        let p = body_check(r(0, 0, 20, 50), r(30, 10, 10, 10));
        assert_eq!(
            (p.amount, p.jolt, p.shove, p.knockdown),
            (7, true, false, false)
        );
        assert_eq!(p.push, (25, -10));
        assert_eq!(body_check(r(40, 0, 20, 50), r(0, 0, 20, 50)).push, (-40, 0));
    }

    /// Pouso duro 30 (`0x42951f`): com 20 px livres, decisão em pé; sem eles, só anima (não
    /// rasteja) e no fim vai a 31. 31 e 35 (`0x429555`, `0x42a248`): teto a
    /// menos de 20 px ou BAIXO → rastejar 36; senão decisão; fim → parado.
    #[test]
    fn hard_landing_does_not_crawl_but_get_up_does() {
        let w = floor_world();
        let mut j = garathe(&w, 0);
        j.set(State::Land(Landing::Hard));
        j.tick(DOWN, &w);
        assert_eq!(
            j.state(),
            State::Crouch,
            "no 30, BAIXO passa pela decisão (37)"
        );
        let mut j = garathe(&w, 0);
        j.set(State::Land(Landing::Hard));
        let mut n = 0;
        while j.state() == State::Land(Landing::Hard) && n < 20 {
            j.tick(0, &w);
            n += 1;
        }
        assert_eq!(j.state(), State::GetUp, "fim do 30 → 31");
        j.tick(DOWN, &w);
        assert_eq!(j.state(), State::Crawl, "31 com BAIXO → 36");
        // teto baixo (10 px acima da cabeça): o 30 segue, o 31 rasteja
        let head = 44 + BOX.y;
        let low = World::new(
            w.solids()
                .iter()
                .copied()
                .chain([Rect {
                    x: -100,
                    y: head - 30,
                    w: 300,
                    h: 20,
                }])
                .collect(),
        );
        let mut j = garathe(&w, 0);
        j.set(State::Land(Landing::Hard));
        j.tick(0, &low);
        assert_eq!(j.state(), State::Land(Landing::Hard));
        j.set(State::GetUp);
        j.tick(0, &low);
        assert_eq!(j.state(), State::Crawl);
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

    /// 22/25 (`0x429e46`): chão, parede e contato só são vistos quando a animação acaba;
    /// encostar na parede no meio do pulo não o encerra.
    #[test]
    fn mid_air_wall_brush_does_not_end_the_jump() {
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 100,
                w: 3000,
                h: 20,
            },
            Rect {
                x: 60,
                y: -200,
                w: 20,
                h: 240,
            },
        ]);
        let mut j = garathe(&w, 0);
        j.tick(RIGHT, &w);
        j.tick(RIGHT | JUMP, &w);
        assert_eq!(j.state(), State::Jump(JumpKind::Medium));
        let mut brushed = false;
        let mut n = 0;
        while j.state() == State::Jump(JumpKind::Medium) && n < 60 {
            j.tick(0, &w);
            brushed |= j.body.hit_wall;
            if j.state() == State::Jump(JumpKind::Medium) {
                assert_eq!(j.sound, None);
            }
            n += 1;
        }
        assert!(brushed, "encostou na parede no ar");
        // 22 com 6 quadros e atraso 1: ao menos 10 tiques no ar antes de decidir
        assert!(n >= 10, "o pulo durou {n} tiques");
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

    /// 47/49/50/51 (`0x42a29b`, `0x42a4af`, `0x42a7c2`, `0x42a8bb`): com ATIRAR/SOCO (e sem
    /// recarga) o Garathe solta o teto (desliga 0x8, `+0x1b4 = 0x79`) e ataca no ar
    /// (`0x43b43e`); cai atacando enquanto segurar o ataque e, sem ele, a queda (`0x4292d8`)
    /// o leva ao 27.
    #[test]
    fn bar_attack_lets_go_and_strikes_in_the_air() {
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 300,
                w: 3000,
                h: 20,
            },
            Rect {
                x: -1000,
                y: 30,
                w: 3000,
                h: 19,
            },
        ]);
        let hang = |s: State| {
            let mut j = Jake::new(100, 44, BOX, lengths())
                .with_skeleton(skeleton())
                .with_character(Character::Garathe);
            j.set(State::BarHang);
            j.tick(0, &w);
            if s != State::BarHang {
                j.tick(LEFT, &w);
                assert_eq!(j.state(), State::BarPull);
            }
            if s == State::BarMove {
                run(&mut j, &w, LEFT, 7);
            }
            if s == State::BarDown {
                run(&mut j, &w, LEFT, 7);
                j.tick(0, &w);
            }
            assert_eq!(j.state(), s);
            j
        };
        for (s, input) in [
            (State::BarHang, SHOOT),
            (State::BarHang, PUNCH),
            (State::BarHang, SHOOT | DOWN | LEFT),
            (State::BarPull, PUNCH | LEFT),
            (State::BarMove, SHOOT | LEFT),
            (State::BarDown, PUNCH),
        ] {
            let mut j = hang(s);
            let y0 = j.body.y;
            j.tick(input, &w);
            assert!(
                matches!(j.state(), State::Fire { .. }),
                "{s:?} + {input:#x}: {:?}",
                j.state()
            );
            assert!(j.body.y > y0 || j.body.vy > 0, "{s:?}: soltou (gravidade)");
            let mut hit = false;
            for _ in 0..3 {
                j.tick(input & (SHOOT | PUNCH), &w);
                hit |= j.punch.is_some();
                assert!(
                    matches!(j.state(), State::Fire { .. }),
                    "{s:?}: ataca caindo"
                );
            }
            assert!(hit, "{s:?}: golpe das garras no ar");
            j.tick(0, &w);
            assert_eq!(j.state(), State::Fall, "{s:?}: sem ataque, a queda (27)");
        }
        // segurando o ataque até o chão: aterrissa (`0x429395` → 28)
        let mut j = hang(State::BarHang);
        j.tick(SHOOT, &w);
        for _ in 0..40 {
            if j.body.on_ground {
                break;
            }
            j.tick(SHOOT, &w);
        }
        assert!(j.body.on_ground);
        assert_eq!(j.state(), State::Land(Landing::Fall));
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
        assert_eq!((b.attacker, b.variant), (2, 0));
        // contra um objeto qualquer à esquerda (olha para a esquerda)
        let on = |b: crate::damage::Strike| {
            b.row(7).unwrap().pending(crate::damage::Geometry {
                vel: (0, 0),
                toward: (-10, 0),
            })
        };
        let p = on(b);
        assert_eq!((p.amount, p.shove, p.push), (15, true, (-2, 0)));
        // com a clava (par 2): SOCO usa a clava, 114 na frente, derrubada (20, 10)
        let mut j = armed(&w, &[(2, (1, 1))]);
        j.tick(PUNCH, &w);
        assert_eq!(j.label(), 114);
        let mut blow = None;
        for _ in 0..8 {
            j.tick(0, &w);
            blow = blow.or(j.blow);
        }
        let p = on(blow.unwrap());
        assert_eq!((p.amount, p.knockdown, p.push), (15, true, (-20, 10)));
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
            blows += usize::from(j.blow.is_some_and(|b| on(b).amount == 18));
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

    /// Pulos 20/21 com as raízes do `MINOTAUR.SKE` (a raiz só sobe: o rótulo acaba no alto
    /// do impulso) e escala 1 (S = K): 20 = 0 ×13, 27, 39; 21 = 0 ×6, 9, 28, 32.
    fn real_jumper(world: &World, x: i32, y: i32) -> Jake {
        let ys = |v: &[i16]| v.iter().map(|&y| (0, y)).collect::<Vec<_>>();
        let mut e: Vec<SynthSkeletonEntry> = vec![(0, (0, 0), vec![], 0); 170];
        let mut up = vec![0i16; 13];
        up.extend([27, 39]);
        e[20] = (0, (0, 0), ys(&up), 0);
        e[21] = (0, (0, 0), ys(&[0, 0, 0, 0, 0, 0, 9, 28, 32]), 0);
        let ske = Skeleton::parse(&mkske(2, (39, 58), &e)).unwrap();
        let mut len = lengths();
        len.insert(20, 15);
        len.insert(21, 9);
        let mut j = Jake::new(x, y, BOX, len)
            .with_skeleton(Skel::new(Rc::new(ske), (39, 58)))
            .with_character(Character::Garathe);
        j.tick(0, world);
        j
    }

    /// Sobe até o pico e devolve (subida em px, estados vistos até pousar).
    fn flight(j: &mut Jake, w: &World, first: u32, rest: u32) -> (i32, Vec<State>) {
        let y0 = j.body.y;
        let mut top = y0;
        let mut seen = vec![];
        j.tick(first, w);
        for _ in 0..80 {
            j.tick(rest, w);
            top = top.min(j.body.y);
            if seen.last() != Some(&j.state()) {
                seen.push(j.state());
            }
            if matches!(j.state(), State::Land(_)) {
                break;
            }
        }
        (y0 - top, seen)
    }

    /// 20 (`0x4295a8`): a animação toca uma vez (`0x4289da`) e, enquanto não acaba, pede a
    /// velocidade do esqueleto (k = 2). Acabada no ar, o estado **segue no 20** sem pedido
    /// algum: a última velocidade (−24 px/tique depois da gravidade) vira voo livre até o
    /// chão, e só então 28 (`0x407031` → `0x407009` → `0x4294fa`).
    #[test]
    fn up_jump_flies_on_after_its_animation_and_lands_in_28() {
        let w = floor_world();
        let mut j = real_jumper(&w, 0, 44);
        let (rise, seen) = flight(&mut j, &w, JUMP | UP, UP);
        // 54 − 2, 24 − 2, depois 20 + 18 + … + 2 = 110
        assert_eq!(rise, 52 + 22 + 110);
        assert_eq!(seen, vec![State::JumpUp(false), State::Land(Landing::Fall)]);
        assert_eq!((j.label(), j.body.vx, j.body.vy), (28, 0, 0));
    }

    /// 21 (`0x429817`, k = 3): no fim, estado 1 e a rotina da queda (`0x4293c2`) no mesmo
    /// tique, sem zerar a velocidade: o impulso que restou continua subindo na queda 27.
    #[test]
    fn standing_jump_ends_in_a_fall_that_keeps_its_momentum() {
        let w = floor_world();
        let mut j = real_jumper(&w, 0, 44);
        let (rise, seen) = flight(&mut j, &w, JUMP, 0);
        // 27 − 2, 57 − 2, 12 − 2, depois 8 + 6 + 4 + 2
        assert_eq!(rise, 25 + 55 + 10 + 20);
        assert_eq!(
            seen,
            vec![State::StandJump, State::Fall, State::Land(Landing::Fall)]
        );
    }

    /// O teto alcançado no voo livre do 20 também é agarrado (47): o teste de teto
    /// (`0x40712f`, `+0x220 = 2`) vem antes da animação em todo tique do 20.
    #[test]
    fn up_jump_grabs_a_ceiling_reached_in_free_flight() {
        let w = World::new(vec![
            Rect {
                x: -1000,
                y: 100,
                w: 3000,
                h: 20,
            },
            // fundo do teto 150 px acima da cabeça (caixa do topo em y + 5 = 49)
            Rect {
                x: -1000,
                y: -121,
                w: 3000,
                h: 20,
            },
        ]);
        let mut j = real_jumper(&w, 0, 44);
        j.tick(JUMP | UP, &w);
        let mut n = 0;
        while j.state() == State::JumpUp(false) && n < 40 {
            j.tick(UP, &w);
            n += 1;
        }
        assert_eq!((j.state(), j.label()), (State::BarHang, 47));
        assert_eq!(j.body.bounds().y, -101, "pendurado rente ao teto");
    }

    /// Controle no ar (`0x408205`) no 20: nada no chão (o impulso de 13 quadros não anda); no
    /// ar, o lado põe ±8 px/tique e, sem lado, o voo livre mantém a horizontal.
    #[test]
    fn up_jump_steers_only_in_the_air() {
        let w = floor_world();
        let mut j = real_jumper(&w, 0, 44);
        let x0 = j.body.x;
        j.tick(JUMP | UP, &w);
        for _ in 0..12 {
            j.tick(UP | RIGHT, &w);
            assert!(j.body.on_ground);
        }
        assert_eq!(j.body.x, x0, "impulso no chão: parado");
        run(&mut j, &w, UP | RIGHT, 4);
        assert_eq!(j.state(), State::JumpUp(false));
        assert_eq!(j.body.vx, 8 * SUB);
        let x1 = j.body.x;
        j.tick(UP, &w);
        assert_eq!(j.body.vx, 8 * SUB, "voo livre: a horizontal segue");
        assert_eq!(j.body.x - x1, 8);
    }

    /// 19 (`0x4296df`, PULAR+CIMA na corda): parado e sem gravidade até o quadro 9, então
    /// `vy = −30`; **não há saída no fim da animação**: só o chão (`0x429395` → 28).
    #[test]
    fn rope_jump_flies_until_the_ground() {
        let w = floor_world();
        let mut j = real_jumper(&w, 0, -400);
        j.set(State::JumpUp(true));
        let y0 = j.body.y;
        let mut top = y0;
        let mut seen = vec![];
        for _ in 0..80 {
            j.tick(UP, &w);
            top = top.min(j.body.y);
            if seen.last() != Some(&j.state()) {
                seen.push(j.state());
            }
            if matches!(j.state(), State::Land(_)) {
                break;
            }
        }
        assert_eq!(y0 - top, (2..=28).step_by(2).sum::<i32>());
        assert_eq!(seen, vec![State::JumpUp(true), State::Land(Landing::Fall)]);
    }

    /// Queda 27 (`0x4293c2`): controle no ar (`0x408205`), ±8 px/tique pelo lado pedido.
    #[test]
    fn fall_steers_with_the_air_control() {
        let w = floor_world();
        let mut j = real_jumper(&w, 0, -400);
        j.set(State::Fall);
        j.tick(0, &w);
        assert_eq!(j.body.vx, 0);
        j.tick(RIGHT, &w);
        assert_eq!((j.state(), j.body.vx), (State::Fall, 8 * SUB));
        j.tick(0, &w);
        assert_eq!(j.body.vx, 8 * SUB, "sem lado, mantém");
        j.tick(LEFT, &w);
        assert_eq!(j.body.vx, -8 * SUB);
    }
}
