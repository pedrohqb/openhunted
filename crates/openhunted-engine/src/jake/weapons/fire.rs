// SPDX-License-Identifier: GPL-3.0-or-later
//! Fluxo do tiro medido contra `0x0043b50b`/`0x0043b1d0`/`0x0043b43e` (auditoria M17,
//! `docs/auditoria/armas.md`).

use super::*;
use crate::input::{DOWN, LEFT, PUNCH, RIGHT, SHOOT, UP};
use crate::jake::{Inventory, Jake, State};
use crate::physics::{Rect, World};
use std::collections::HashMap;

/// Quadros dos rótulos de arma do `JAKE.ANM` e do `MINOTAUR.ANM` real.
fn lengths() -> HashMap<u16, usize> {
    [
        (1, 34),
        (9, 8),
        (18, 1),
        (19, 9),
        (22, 4),
        (23, 8),
        (54, 7),
        (55, 7),
        (56, 7),
        (141, 12),
        (142, 12),
        (143, 12),
        (144, 12),
        (127, 1),
        (128, 9),
        (131, 5),
        (146, 8),
        (147, 8),
        (150, 6),
        (151, 9),
        (95, 12),
        (98, 12),
        (110, 1),
        (114, 12),
        (116, 12),
        (118, 12),
        (159, 13),
    ]
    .into_iter()
    .collect()
}

fn floor() -> World {
    World::new(vec![Rect {
        x: -10_000,
        y: 100,
        w: 20_000,
        h: 20,
    }])
}

fn armed(ch: Character, w: usize, ammo: i32) -> (Jake, World) {
    let world = floor();
    let mut inv = Inventory::for_character(&[(0, 0), (1, 20)], ch);
    inv.slots[w] = (1, ammo);
    inv.weapon = w;
    let mut j = Jake::new(
        0,
        53,
        Rect {
            x: 2,
            y: 3,
            w: 18,
            h: 44,
        },
        lengths(),
    )
    .with_character(ch)
    .with_inventory(inv);
    j.tick(0, &world);
    (j, world)
}

fn bullets(j: &mut Jake) -> usize {
    j.shots.drain(..).filter(|s| !s.is_casing()).count()
}

/// Soltar ATIRAR segurando uma direção sai do tiro no mesmo tique (`0x43b591`) e trava
/// novos tiros por 8 tiques (`+0x294 = 8`, cai 1 por tique antes dos estados).
#[test]
fn releasing_fire_while_moving_exits_and_locks_for_eight_ticks() {
    let (mut j, w) = armed(Character::Jake, PISTOL, 50);
    j.tick(SHOOT | RIGHT, &w);
    assert_eq!(bullets(&mut j), 1);
    j.tick(RIGHT, &w);
    assert_eq!(bullets(&mut j), 0, "a rajada para");
    assert!(!j.is_firing());
    let mut first = None;
    for k in 0..10 {
        j.tick(SHOOT, &w);
        if bullets(&mut j) > 0 && first.is_none() {
            first = Some(k);
        }
    }
    // trava 8 no tique da saída; 7 tiques sem tiro, o 8º já atira
    assert_eq!(first, Some(7));
    // soltar parado (sem direção) não interrompe: as três balas saem
    let (mut j, w) = armed(Character::Jake, PISTOL, 50);
    j.tick(SHOOT, &w);
    j.tick(0, &w);
    j.tick(0, &w);
    assert_eq!(bullets(&mut j), 3);
}

/// A mira é refeita a cada tique: CIMA no meio da rajada leva as balas seguintes para cima
/// sem recomeçar `t` (`0x43b66f`).
#[test]
fn aim_follows_the_input_in_the_middle_of_a_burst() {
    let (mut j, w) = armed(Character::Jake, PISTOL, 50);
    j.tick(SHOOT, &w);
    j.tick(SHOOT | UP, &w);
    j.tick(SHOOT | UP, &w);
    let v: Vec<(i32, i32)> = j
        .shots
        .iter()
        .filter(|s| !s.is_casing())
        .map(|s| (s.vx, s.vy))
        .collect();
    assert_eq!(v, vec![(-20, 0), (0, -20), (0, -20)]);
    assert_eq!(j.label(), 56);
    assert!(matches!(j.state(), State::Fire { t: 3, .. }));
}

/// Sem munição no meio da rajada: o aviso, a troca e a pose "pronto" saem no tique do tiro
/// vazio (`0x43b3b9`), não no próximo aperto.
#[test]
fn running_dry_mid_burst_switches_on_the_empty_shot() {
    let (mut j, w) = armed(Character::Jake, PISTOL, 2);
    j.tick(SHOOT, &w);
    j.tick(SHOOT, &w);
    assert_eq!(bullets(&mut j), 2);
    j.tick(SHOOT, &w);
    assert!(j.no_ammo);
    assert_eq!((j.inventory.weapon, j.label()), (FISTS, 146));
}

/// Estados de tiro tocam o rótulo uma vez e param no último quadro (`0x423835`, modo 0xa),
/// `passo` quadros por tique.
#[test]
fn fire_animation_plays_once_at_step_frames_per_tick() {
    let (mut j, w) = armed(Character::Jake, PISTOL, 50);
    j.tick(SHOOT, &w);
    let mut frames = vec![j.frame()];
    for _ in 0..10 {
        j.tick(0, &w);
        frames.push(j.frame());
    }
    assert_eq!(frames, vec![0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7]);
    let (mut j, w) = armed(Character::Jake, STARS, 5);
    j.tick(SHOOT, &w);
    let mut frames = vec![j.frame()];
    for _ in 0..5 {
        j.tick(0, &w);
        frames.push(j.frame());
    }
    assert_eq!(frames, vec![0, 2, 4, 6, 8, 10]);
}

/// Sons de arma pelo quadro (`SOUND.TBL`): a estrela soa no 7º quadro (tique 3, junto com o
/// arremesso); a faca do Garathe para cima soa no quadro 7, que o passo 2 pula.
#[test]
fn weapon_sounds_follow_the_frame_table() {
    let (mut j, w) = armed(Character::Jake, STARS, 5);
    let mut heard = vec![];
    for k in 0..6 {
        j.tick(if k == 0 { SHOOT } else { 0 }, &w);
        if let Some(id) = j.fire_sound() {
            heard.push((k, id));
        }
    }
    assert_eq!(heard, vec![(3, 3110)]);
    let (mut j, w) = armed(Character::Jake, PISTOL, 50);
    j.tick(SHOOT, &w);
    assert_eq!(j.fire_sound(), Some(3040));
    j.tick(SHOOT, &w);
    assert_eq!(j.fire_sound(), None, "uma vez por animação");
    for (input, want) in [(SHOOT, Some(3034)), (SHOOT | UP, None)] {
        let (mut j, w) = armed(Character::Garathe, KNIVES, 5);
        let mut heard = None;
        for k in 0..6 {
            j.tick(if k == 0 { input } else { input & UP }, &w);
            heard = heard.or(j.fire_sound());
        }
        assert_eq!(heard, want, "entrada {input:#x}");
    }
}

/// O golpe vale em cada tique com `+0x04 < t ≤ +0x08` (`0x43b7a7`): soco 150 só em t = 3;
/// a clava (5..7) acerta em t = 6 e t = 7.
#[test]
fn blows_land_on_every_tick_of_the_hit_window() {
    let (mut j, w) = armed(Character::Jake, FISTS, 20);
    let mut hits = vec![];
    for k in 0..8 {
        j.tick(if k == 0 { PUNCH } else { 0 }, &w);
        if j.punch.is_some() {
            hits.push((k, j.label()));
        }
    }
    let label = hits.first().map(|h| h.1).unwrap();
    let want = if label == 150 {
        vec![(2, 150)]
    } else {
        vec![(4, 151)]
    };
    assert_eq!(hits, want);
    let (mut j, w) = armed(Character::Garathe, CLUB, 1);
    let mut hits = vec![];
    for k in 0..10 {
        j.tick(if k == 0 { PUNCH } else { 0 }, &w);
        if j.punch.is_some() {
            hits.push(k);
        }
    }
    assert_eq!(hits, vec![5, 6]);
}

/// BAIXO com SOCO também faz o especial (`0x43ae4c` não olha qual dos dois botões).
#[test]
fn punch_and_down_also_trigger_the_special() {
    let (mut j, w) = armed(Character::Jake, PISTOL, 50);
    j.tick(PUNCH | DOWN, &w);
    assert!(matches!(j.state(), State::Special(_)));
}

/// A pose "pronto" da troca de arma dura 6 tiques no repouso (`+0x26c = 7` decrementado no
/// mesmo tique, `0x43b15c`).
#[test]
fn switch_pose_lasts_six_ticks() {
    let (mut j, w) = armed(Character::Jake, PISTOL, 50);
    j.inventory.slots[STARS] = (1, 5);
    let mut ready = 0;
    j.tick(crate::input::PREV_WEAPON, &w);
    for _ in 0..10 {
        if j.label() == STARS_W.ready {
            ready += 1;
        }
        j.tick(0, &w);
    }
    assert_eq!(ready, 6);
    let _ = LEFT;
}
