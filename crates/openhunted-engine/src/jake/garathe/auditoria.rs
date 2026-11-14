// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditoria de fidelidade (M17) dos movimentos do Garathe: um teste por correção, medindo o
//! porte contra o comportamento lido no `Hunter.exe` (`docs/auditoria/garathe-movimentos.md`).

use super::super::{Jake, Landing, State};
use super::tests::{floor_world, garathe, real_jumper, run};
use crate::input::{JUMP, PUNCH, RIGHT, SHOOT, UP};
use crate::physics::{SUB, World};

fn airborne(j: &mut Jake, w: &World, input: u32) {
    for _ in 0..30 {
        if !j.body.on_ground {
            return;
        }
        j.tick(input, w);
    }
    panic!("não saiu do chão: {:?}", j.state());
}

/// Ataque no ar (`0x43b43e`) nos pulos 19, 20, 21 e na queda 27: ATIRAR ou SOCO sem
/// recarga. O 20 e o 21 zeram antes a vertical (`+0x168 = 0`: `0x4295fd`, `0x42986f`); o 19
/// (`0x429719`) e o 27 (`0x4293ef`) não; nenhum mexe na horizontal (o `0x43b50b` não grava
/// `+0x164`).
#[test]
fn attacks_from_the_jumps_and_the_fall() {
    let w = floor_world();
    // 21 no ar: vy = 0 e um tique de gravidade
    let mut j = real_jumper(&w, 0, 44);
    j.tick(JUMP, &w);
    airborne(&mut j, &w, 0);
    assert_eq!(j.state(), State::StandJump);
    j.tick(SHOOT, &w);
    assert!(matches!(j.state(), State::Fire { .. }), "{:?}", j.state());
    // gravada antes da gravidade da física seguinte (`physics` vem antes da atualização)
    assert_eq!(j.body.vy, 0, "21: vertical zerada antes do golpe");
    // 21 ainda no chão (impulso): o golpe vale também (`0x429857` não olha o chão)
    let mut j = real_jumper(&w, 0, 44);
    j.tick(JUMP, &w);
    j.tick(PUNCH, &w);
    assert!(matches!(j.state(), State::Fire { .. }), "{:?}", j.state());
    // 20 no ar
    let mut j = real_jumper(&w, 0, 44);
    j.tick(JUMP | UP, &w);
    airborne(&mut j, &w, UP);
    j.tick(SHOOT, &w);
    assert!(matches!(j.state(), State::Fire { .. }), "{:?}", j.state());
    assert_eq!(j.body.vy, 0, "20: vertical zerada antes do golpe");
    // 19 subindo depois do quadro 9: a vertical segue
    let mut j = real_jumper(&w, 0, -400);
    j.set(State::JumpUp(true));
    run(&mut j, &w, UP, 11);
    assert_eq!(j.state(), State::JumpUp(true));
    let vy = j.body.vy;
    assert!(vy < 0);
    j.tick(SHOOT, &w);
    assert!(matches!(j.state(), State::Fire { .. }), "{:?}", j.state());
    assert_eq!(j.body.vy, vy + 2 * SUB, "19: a vertical não é zerada");
    // 27 com controle no ar: a horizontal fica
    let mut j = real_jumper(&w, 0, -400);
    j.set(State::Fall);
    j.tick(RIGHT, &w);
    assert_eq!(j.body.vx, 8 * SUB);
    let vy = j.body.vy;
    j.tick(PUNCH, &w);
    assert!(matches!(j.state(), State::Fire { .. }), "{:?}", j.state());
    assert_eq!((j.body.vx, j.body.vy), (8 * SUB, vy + 2 * SUB));
    // segurando o ataque até o chão: aterrissa (28)
    for _ in 0..60 {
        if matches!(j.state(), State::Land(_)) {
            break;
        }
        j.tick(PUNCH, &w);
    }
    assert_eq!(j.state(), State::Land(Landing::Fall));
    // no chão o garathe() de sempre ataca parado
    let mut j = garathe(&w, 0);
    j.tick(SHOOT, &w);
    assert!(matches!(j.state(), State::Fire { .. }));
}

/// Terremoto (`0x42bcf8`, contador `+0x274`): com gravidade e fora dos estados isentos, o
/// Garathe cambaleia no 121 do quadro 0 (`0x42bd73` → `0x4289da`) sem entrada; zerado o
/// contador, a rotina do parado decide. Não é o choque (121 do quadro 17, 10 tiques).
#[test]
fn quake_staggers_in_121_while_the_counter_lasts() {
    let w = floor_world();
    let mut j = garathe(&w, 0);
    j.start_quake(30);
    let x0 = j.body.x;
    let mut n = 0;
    j.tick(RIGHT, &w);
    assert_eq!((j.state(), j.label(), j.frame()), (State::Quake, 121, 0));
    while j.state() == State::Quake {
        n += 1;
        assert!(n < 60);
        j.tick(RIGHT, &w);
    }
    assert_eq!(n, 29, "o 30º tique zera o contador e decide");
    assert_eq!(j.state(), State::Run);
    assert!(
        j.body.x - x0 <= 15,
        "parado no 121 (a corrida só no último tique)"
    );
    // correndo: entra no 121 com a velocidade (o atrito do chão freia)
    let mut j = garathe(&w, 0);
    run(&mut j, &w, RIGHT, 3);
    j.start_quake(20);
    let x0 = j.body.x;
    j.tick(0, &w);
    assert_eq!(j.state(), State::Quake);
    assert!(j.body.x > x0, "desliza: {}", j.body.x - x0);
    // isentos: rastejando (36) e no ar (27); o contador corre assim mesmo
    let mut j = garathe(&w, 0);
    run(&mut j, &w, crate::input::DOWN, 6);
    assert_eq!(j.state(), State::Crawl);
    j.start_quake(5);
    run(&mut j, &w, crate::input::DOWN, 5);
    assert_eq!((j.state(), j.quake_ticks()), (State::Crawl, 0));
    let mut j = real_jumper(&w, 0, -400);
    j.set(State::Fall);
    j.start_quake(5);
    j.tick(0, &w);
    assert_eq!(j.state(), State::Fall);
}

/// Especial do Garathe nos personagens (`0x42c348`, laço até 140 px): a menos de 70 px,
/// derrubada com `4·(19600 − d)/19600` de dano e empurrão (∓15, 20) para longe do centro; de
/// 70 a 139 px, `+0x274 = 30` (terremoto). O do Jake (`0x427643`) fica como estava.
#[test]
fn garathe_special_knocks_down_near_and_quakes_farther() {
    use super::super::{PlayerHit, SpecialBlast};
    let g = SpecialBlast {
        center: (100, 100),
        layer: 0,
        garathe: true,
    };
    let Some(PlayerHit::Knock(p)) = g.player_hit((100, 100)) else {
        panic!("no centro, derrubada");
    };
    assert_eq!((p.knockdown, p.amount, p.push), (true, 4, (-15, 20)));
    let Some(PlayerHit::Knock(p)) = g.player_hit((31, 100)) else {
        panic!("a 69 px, derrubada");
    };
    assert_eq!((p.amount, p.push), (3, (-15, 20)));
    assert_eq!(g.player_hit((170, 100)), Some(PlayerHit::Quake(30)));
    assert_eq!(g.player_hit((100, 239)), Some(PlayerHit::Quake(30)));
    assert_eq!(g.player_hit((240, 100)), None);
    // o do Jake: só a menos de 70 px, sem dano, (±10, −10) (Ruling da M11, não mexido)
    let jk = SpecialBlast {
        garathe: false,
        ..g
    };
    let Some(PlayerHit::Knock(p)) = jk.player_hit((120, 100)) else {
        panic!("Jake: derrubada");
    };
    assert_eq!((p.amount, p.push), (0, (10, -10)));
    assert_eq!(jk.player_hit((170, 100)), None);
}

fn shove() -> crate::damage::Pending {
    crate::damage::Pending {
        amount: 1,
        shove: true,
        push: (3, 0),
        ..Default::default()
    }
}

/// Empurrão (`0x428531` → `0x42b93e`): a rotina do 58 decide em pé antes de tudo (com
/// entrada, o empurrão nem aparece) e não mexe na velocidade: o Garathe não é deslocado.
#[test]
fn shove_does_not_move_and_input_interrupts_it() {
    let w = floor_world();
    let mut j = garathe(&w, 0);
    let x0 = j.body.x;
    j.hurt(shove());
    j.tick(0, &w);
    assert_eq!((j.state(), j.label()), (State::Pushed, 58));
    let mut n = 0;
    while j.state() == State::Pushed {
        assert_eq!(j.body.x, x0, "sem deslocamento");
        j.tick(0, &w);
        n += 1;
        assert!(n < 30);
    }
    assert_eq!(j.state(), State::Idle);
    // no meio do 58, a direita corre
    let mut j = garathe(&w, 0);
    j.hurt(shove());
    j.tick(0, &w);
    j.tick(RIGHT, &w);
    assert_eq!(j.state(), State::Run);
    // segurando a direita no tique do empurrão: corre direto
    let mut j = garathe(&w, 0);
    j.hurt(shove());
    j.tick(RIGHT, &w);
    assert_eq!(j.state(), State::Run);
    // e o ataque também sai do 58 (a decisão `0x428ae8` ataca)
    let mut j = garathe(&w, 0);
    j.hurt(shove());
    j.tick(0, &w);
    j.tick(SHOOT, &w);
    assert!(matches!(j.state(), State::Fire { .. }), "{:?}", j.state());
}

/// Queda (`0x4292d8`): o 58, o 59 e o 121 não são isentos; sem chão, a 5 px/tique, 27.
#[test]
fn jolt_and_quake_fall_off_a_ledge() {
    let floor = |x: i32, w: i32| {
        World::new(vec![crate::physics::Rect {
            x,
            y: 100,
            w,
            h: 20,
        }])
    };
    // tranco: desliza pelo esqueleto (rótulo 63: Δ 3 × k 2, para trás: olhando à esquerda,
    // para a direita) para fora da beirada
    let ledge = floor(-1000, 1020);
    let mut j = garathe(&ledge, 0);
    j.hurt(crate::damage::Pending {
        amount: 1,
        jolt: true,
        ..Default::default()
    });
    let mut seen = vec![];
    for _ in 0..30 {
        j.tick(0, &ledge);
        if seen.last() != Some(&j.state()) {
            seen.push(j.state());
        }
    }
    assert!(seen.contains(&State::Fall), "{seen:?}");
    // terremoto correndo para fora da beirada direita
    let ledge = floor(-1000, 1030);
    let mut j = garathe(&ledge, -40);
    run(&mut j, &ledge, RIGHT, 2);
    j.start_quake(30);
    let mut seen = vec![];
    for _ in 0..30 {
        j.tick(0, &ledge);
        if seen.last() != Some(&j.state()) {
            seen.push(j.state());
        }
    }
    assert_eq!(&seen[..2], &[State::Quake, State::Fall], "{seen:?}");
}

const COLPS: crate::physics::Rect = crate::physics::Rect {
    x: 10,
    y: 30,
    w: 20,
    h: 26,
};

/// Caixa `MN_COLPS` (coluna `+0x01` da tabela) nos 29–33, 35, 59 e nas mortes.
#[test]
fn collapse_box_while_down() {
    let w = floor_world();
    let mut j = garathe(&w, 0).with_collapse_box(COLPS);
    let feet = j.body.bounds().y + j.body.bounds().h;
    j.hurt(crate::damage::Pending {
        amount: 1,
        knockdown: true,
        push: (-4, -6),
        ..Default::default()
    });
    j.tick(0, &w);
    assert_eq!(j.body.boxr, COLPS, "32 com MN_COLPS");
    let mut seen = vec![];
    for _ in 0..80 {
        j.tick(0, &w);
        if matches!(
            j.state(),
            State::Land(Landing::Hard) | State::GetUp | State::Rise | State::Jolted
        ) {
            assert_eq!(j.body.boxr, COLPS, "{:?}", j.state());
        }
        seen.push(j.state());
        if j.state() == State::Idle {
            break;
        }
    }
    assert!(seen.contains(&State::GetUp));
    assert_eq!(j.body.boxr, super::tests::BOX, "de pé, a padrão");
    assert_eq!(j.body.bounds().y + j.body.bounds().h, feet, "pés no chão");
}

/// Morte adiada (`0x428863`): nos 4, 11, 12, 25, 29, 32 e 33 nada acontece; a vida a 0 é
/// vista de novo a cada tique. Derrubado com dano mortal, voa no 32, pousa no 30 e morre no
/// 67; o 28 morre no 63 mesmo com veneno.
#[test]
fn death_waits_for_the_knockdown_to_land() {
    let w = floor_world();
    let mut j = garathe(&w, 0).with_health(5, 0);
    j.hurt(crate::damage::Pending {
        amount: 50,
        knockdown: true,
        push: (-4, -6),
        ..Default::default()
    });
    j.tick(0, &w);
    assert_eq!(j.state(), State::KnockedDown(false), "voa no 32");
    let mut seen = vec![];
    for _ in 0..40 {
        j.tick(0, &w);
        if seen.last() != Some(&j.state()) {
            seen.push(j.state());
        }
    }
    assert!(
        seen.windows(2)
            .any(|p| p == [State::Land(Landing::Hard), State::Dying(super::DIE_FORWARD)]),
        "{seen:?}"
    );
    assert_eq!(seen[0], State::KnockedDown(false), "{seen:?}");
    // 28 com veneno mortal: 63
    let mut j = garathe(&w, 0).with_health(5, 0);
    j.set(State::Land(Landing::Fall));
    j.hurt(crate::damage::Pending {
        amount: 50,
        poison: true,
        ..Default::default()
    });
    j.tick(0, &w);
    assert_eq!(j.state(), State::Dying(super::DIE_DEFAULT));
}

/// 61 e 63 deslizam pelo esqueleto (k = 2, só x: `0x42bc84`, `0x42bb62`); 67 e 69 não. O
/// cadáver (`0x42b604`) só com `vx = 0`: o resto da velocidade se gasta no atrito.
#[test]
fn deaths_61_and_63_slide_by_the_skeleton() {
    let w = floor_world();
    let mut j = garathe(&w, 0).with_health(5, 0);
    let x0 = j.body.x;
    j.hurt(crate::damage::Pending {
        amount: 50,
        ..Default::default()
    });
    run(&mut j, &w, 0, 4);
    assert_eq!((j.state(), j.frame()), (State::Dying(63), 3));
    // rótulo 63 sintético: 0, 3, 6, 9 → Δ 3 por quadro × 2, para trás (olhando à esquerda)
    // pedido 6 px por quadro, andado no tique seguinte e pela metade no chão
    // (`physics::step`: (48/2 − 1) >> 3 = 2 px; o oráculo mede ~metade da vx guardada)
    assert_eq!(j.body.x - x0, 4, "recuo pelo esqueleto");
    run(&mut j, &w, 0, 12);
    assert_eq!((j.state(), j.body.vx, j.frame()), (State::Dead, 0, 3));
    assert!(
        j.body.x - x0 > 4,
        "o atrito gasta o resto: {}",
        j.body.x - x0
    );
    // 67: parado
    let mut j = garathe(&w, 0).with_health(5, 0);
    j.set(State::Land(Landing::Hard));
    j.hurt(crate::damage::Pending {
        amount: 50,
        ..Default::default()
    });
    run(&mut j, &w, 0, 12);
    assert_eq!((j.state(), j.body.x), (State::Dead, x0));
}

/// O lado (`+0x288`) segue a entrada em todo tique (`0x427c9c`), também no ar: caindo com o
/// controle no ar para a esquerda, o Garathe olha para a esquerda.
#[test]
fn facing_follows_the_input_in_the_air() {
    let w = floor_world();
    let mut j = real_jumper(&w, 0, -400);
    j.set(State::Fall);
    j.tick(RIGHT, &w);
    assert!(j.facing_right());
    j.tick(crate::input::LEFT, &w);
    assert_eq!(j.body.vx, -8 * SUB);
    assert!(!j.facing_right(), "olha para onde vai");
}

/// 49 (`0x42a573`): só sem CIMA nem lado (`0x487a2c & 0xd = 0`) solta a mão (51); com CIMA
/// sozinho fica no 49, parado no quadro (oráculo, `T_M_002` "10*,UJ,60*U": 47 → 50 → 49
/// e fica).
#[test]
fn up_alone_holds_hand_over_hand() {
    let w = World::new(vec![
        crate::physics::Rect {
            x: -1000,
            y: 300,
            w: 3000,
            h: 20,
        },
        crate::physics::Rect {
            x: -1000,
            y: 30,
            w: 3000,
            h: 19,
        },
    ]);
    let mut j = Jake::new(100, 44, super::tests::BOX, super::tests::lengths())
        .with_skeleton(super::tests::skeleton())
        .with_character(crate::character::Character::Garathe);
    j.set(State::BarHang);
    j.tick(0, &w);
    let mut seen = vec![];
    for _ in 0..30 {
        j.tick(UP, &w);
        if seen.last() != Some(&j.state()) {
            seen.push(j.state());
        }
    }
    assert_eq!(seen, vec![State::BarPull, State::BarMove], "{seen:?}");
    let (x, f) = (j.body.x, j.frame());
    j.tick(UP, &w);
    assert_eq!((j.body.x, j.frame()), (x, f), "parado no quadro");
    j.tick(0, &w);
    assert_eq!(j.state(), State::BarDown);
}

/// 22 e 25 animam a cada tique (`0x429ee7`, `0x429d21`: `0x406c71(obj, 0xa)` direto, sem o
/// atraso `+0x18 = 1` da tabela); oráculo `T_M_002` "10*,4*R,RJ,30*R": 22 q0…q6 em 7 tiques.
#[test]
fn medium_and_long_jumps_animate_every_tick() {
    let w = floor_world();
    let mut j = garathe(&w, 0);
    j.tick(RIGHT, &w);
    j.tick(RIGHT | JUMP, &w);
    assert_eq!(j.state(), State::Jump(super::super::JumpKind::Medium));
    let mut frames = vec![j.frame()];
    for _ in 0..3 {
        j.tick(RIGHT, &w);
        frames.push(j.frame());
    }
    assert_eq!(frames, vec![0, 1, 2, 3]);
    assert_eq!(super::anim(State::Jump(super::super::JumpKind::Long)).1, 0);
}

/// 26 (`0x429fec`) decide em pé antes de tudo (o oráculo sai do 26 para a corrida no tique
/// seguinte ao pouso, com a direita apertada); 23 (`0x42a0b6`) no fim chama a rotina do
/// parado no mesmo tique (decisão: com a direita, corre já).
#[test]
fn landings_23_and_26_hand_over_to_the_decision() {
    let w = floor_world();
    let mut j = garathe(&w, 0);
    j.set(State::Land(Landing::Long));
    j.tick(RIGHT, &w);
    assert_eq!(j.state(), State::Run, "26 interrompido pela decisão");
    let mut j = garathe(&w, 0);
    j.set(State::Land(Landing::Medium));
    j.tick(RIGHT, &w);
    assert_eq!(
        j.state(),
        State::Land(Landing::Medium),
        "23 não decide no meio"
    );
    let mut n = 0;
    while j.state() == State::Land(Landing::Medium) {
        j.tick(RIGHT, &w);
        n += 1;
        assert!(n < 30);
    }
    assert_eq!(j.state(), State::Run, "fim do 23: decisão no mesmo tique");
}

/// Guardar a arma (`0x43ba0b` → `0x423894`) toca o rótulo de trás para a frente (oráculo,
/// `T_M_002` "10*,P,25*": o 111 entra no quadro 9 e desce).
#[test]
fn holster_plays_backwards() {
    let w = floor_world();
    let mut j = garathe(&w, 0);
    j.tick(PUNCH, &w);
    let mut n = 0;
    while !matches!(j.state(), State::Holster(_)) {
        j.tick(0, &w);
        n += 1;
        assert!(n < 80, "{:?}", j.state());
    }
    let first = j.frame();
    assert!(first > 0, "começa do fim: {first}");
    run(&mut j, &w, 0, 2);
    assert!(j.frame() < first || j.state() != State::Holster(1));
}

/// 49 (`0x42a4af`): sem CIMA nem lado (PULAR+BAIXO, PULAR sozinho) solta a mão (51), não
/// cai; só PULAR na borda com CIMA ou lado cai. Andando, a cada 2 tiques (`atraso + 1`)
/// avança dois quadros (`0x428a9f` + `0x406c71(9)`) e pede o esqueleto (k = 6); no tique do
/// meio a velocidade fica (oráculo: 49 avança um quadro por tique em média).
#[test]
fn hand_over_hand_two_frames_every_other_tick() {
    let w = World::new(vec![
        crate::physics::Rect {
            x: -1000,
            y: 300,
            w: 3000,
            h: 20,
        },
        crate::physics::Rect {
            x: -1000,
            y: 30,
            w: 3000,
            h: 19,
        },
    ]);
    let hang = || {
        let mut j = Jake::new(100, 44, super::tests::BOX, super::tests::lengths())
            .with_skeleton(super::tests::skeleton())
            .with_character(crate::character::Character::Garathe);
        j.set(State::BarHang);
        j.tick(0, &w);
        for _ in 0..12 {
            j.tick(UP, &w);
        }
        assert_eq!(j.state(), State::BarMove);
        j
    };
    let mut j = hang();
    j.tick(JUMP | crate::input::DOWN, &w);
    assert_eq!(j.state(), State::BarDown, "PULAR+BAIXO no 49: 51");
    let mut j = hang();
    j.tick(JUMP | RIGHT, &w);
    assert_eq!(j.state(), State::Fall, "PULAR (borda) com lado: cai");
    let mut j = hang();
    let mut frames = vec![];
    let mut moved = vec![];
    for _ in 0..6 {
        let x = j.body.x;
        j.tick(RIGHT, &w);
        frames.push(j.frame());
        moved.push(j.body.x - x);
    }
    let adv: Vec<usize> = frames.windows(2).map(|p| (p[1] + 10 - p[0]) % 10).collect();
    assert!(adv.contains(&2) && adv.contains(&0), "{frames:?}");
    assert!(moved[2..].iter().all(|&d| d != 0), "{moved:?}");
}
