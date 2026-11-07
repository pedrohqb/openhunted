// SPDX-License-Identifier: GPL-3.0-or-later
//! Testes dos dois jogadores (M15a).

use std::rc::Rc;

use super::*;
use crate::behaviour::test_support::logic;
use crate::input::{PUNCH, RIGHT, SHOOT, UP};
use crate::jake::{FISTS, GRENADES, PISTOL, ROCKETS, SHOTGUN};
use openhunted_formats::Level;

const J: Character = Character::Jake;
const G: Character = Character::Garathe;

fn obj(kind: u32, modes: i32) -> LevelObject {
    let mut o = super::super::tests::object(kind, 0, 0, "OAN.ANM");
    o.modes = modes;
    o
}

fn duel(chars: [Character; 2]) -> Mode {
    Mode::Deathmatch(Duel {
        chars,
        ..Duel::default()
    })
}

#[test]
fn mode_masks_follow_0x43c95c() {
    assert_eq!(Mode::Solo.mask(), 2);
    assert_eq!(duel([J, J]).mask(), 4);
    assert_eq!(duel([J, G]).mask(), 8);
    assert_eq!(duel([G, J]).mask(), 8);
    assert_eq!(duel([G, G]).mask(), 0x10);
    assert_eq!(Mode::Coop.mask(), 0xe0, "0x43cbb0 troca por 0xe0");
    assert_eq!(duel([J, G]).slot_mask(), 0x1c);
    assert_eq!(Mode::Coop.slot_mask(), 0xe0);
}

/// Vagas: na disputa, sorteio entre as do personagem escolhido (nunca a mesma duas vezes);
/// no coop, a última dos objetos para o jogador 1; sem vaga, o número do jogador.
#[test]
fn slots_follow_mode_and_pairing() {
    // as vagas de uma `M_*`: 2 Jakes 0xd e 2 Garathes 0x19, mais uma IA e um objeto
    let objs = [
        obj(1, 0xd),
        obj(2, 0x19),
        obj(1, 0xd),
        obj(2, 0x19),
        obj(1, 0xfe),
        obj(5, 0xfe),
    ];
    let refs: Vec<&LevelObject> = objs.iter().collect();
    for seed in 0..20 {
        let mut rng = Rng::new(seed);
        let v = assign_slots(&refs, &duel([J, J]), &mut rng).unwrap();
        assert_eq!(v.len(), 2);
        assert_ne!(v[0], v[1]);
        assert!(v.iter().all(|&k| k == 0 || k == 2), "{v:?}");
        let v = assign_slots(&refs, &duel([J, G]), &mut rng).unwrap();
        assert!([0, 2].contains(&v[0]) && [1, 3].contains(&v[1]), "{v:?}");
        let v = assign_slots(&refs, &duel([G, G]), &mut rng).unwrap();
        assert!(v.iter().all(|&k| k == 1 || k == 3) && v[0] != v[1]);
    }
    // sem duas vagas de Garathe
    let few = [obj(1, 0xd), obj(1, 0xd), obj(2, 0x19)];
    let refs: Vec<&LevelObject> = few.iter().collect();
    assert_eq!(assign_slots(&refs, &duel([G, G]), &mut Rng::new(1)), Err(2));
    // coop: as vagas 0x41/0x21/0x81; o 1 fica com a última dos objetos
    let coop = [obj(1, 0x41), obj(2, 0x41), obj(1, 0xd)];
    let refs: Vec<&LevelObject> = coop.iter().collect();
    assert_eq!(
        assign_slots(&refs, &Mode::Coop, &mut Rng::new(1)),
        Ok(vec![1, 0])
    );
    let one = [obj(1, 0x21)];
    let refs: Vec<&LevelObject> = one.iter().collect();
    assert_eq!(assign_slots(&refs, &Mode::Coop, &mut Rng::new(1)), Err(2));
}

/// Fase de teste com as vagas, uma IA e objetos de máscaras parciais.
fn slot_level() -> Level {
    let mut l = super::super::tests::level();
    let slot = |kind, modes, x| {
        let mut o = super::super::tests::object(kind, x, 0, "OAN.ANM");
        o.modes = modes;
        o.fields = vec![0, 0, 0, 0x94801, 0];
        o.fields.resize(16, 0);
        o.fields[14] = 100;
        o
    };
    let item = |modes| {
        let mut o = super::super::tests::object(crate::entity::HEALTH_ITEM, 0, 0, "OAN.ANM");
        o.modes = modes;
        o
    };
    l.objects = vec![
        slot(1, 0xd, 10),
        slot(1, 0xd, 20),
        slot(2, 0x19, 30),
        slot(2, 0x19, 40),
        slot(1, 0x41, 50),
        slot(2, 0x41, 60),
        item(0x4),
        item(0x8),
        item(0xe0),
    ];
    l
}

/// Montagem: vagas atribuídas viram os jogadores (número, personagem, cor); só ficam os
/// objetos na máscara do modo; as vagas sem jogador somem.
#[test]
fn build_with_assigns_players_and_filters_by_mask() {
    let l = slot_level();
    let items = |s: &Scene| {
        s.entities
            .iter()
            .filter(|e| e.kind == crate::entity::HEALTH_ITEM)
            .count()
    };
    let mode = Mode::Deathmatch(Duel {
        chars: [J, G],
        colours: [37, 38],
        ..Duel::default()
    });
    let s = Scene::build_with(&l, &mut super::super::tests::assets(), mode).unwrap();
    assert_eq!(s.players.len(), 2);
    assert_eq!(s.player_jake(0).unwrap().character(), J);
    assert_eq!(s.player_jake(1).unwrap().character(), G);
    assert_eq!(s.player_jake(0).unwrap().colour(), 37);
    assert_eq!(s.player_jake(1).unwrap().tint().table, 38);
    assert_eq!((s.players[0].number, s.players[1].number), (1, 2));
    assert_eq!(items(&s), 1, "só o item 0x8 (Jake × Garathe)");
    assert!(
        s.entities.iter().all(|e| !matches!(e.kind, 1 | 2)),
        "vagas sem jogador não ficam"
    );
    let s = Scene::build_with(&l, &mut super::super::tests::assets(), Mode::Coop).unwrap();
    // coop: a última vaga coop (Garathe em x = 60) é a do 1
    assert_eq!(s.player_jake(0).unwrap().character(), G);
    assert_eq!(s.player_jake(0).unwrap().body.x, 60);
    assert_eq!(s.player_jake(1).unwrap().body.x, 50);
    assert_eq!(s.player_jake(0).unwrap().colour(), 0);
    assert_eq!(items(&s), 1, "só o item 0xe0");
    let s = Scene::build_with(&l, &mut super::super::tests::assets(), duel([G, G])).unwrap();
    assert_eq!(items(&s), 0);
    let mut short = slot_level();
    short.objects.retain(|o| o.modes != 0x41 || o.kind != 2);
    let e = Scene::build_with(&short, &mut super::super::tests::assets(), Mode::Coop);
    assert!(
        e.err()
            .is_some_and(|e| e.to_string().contains("assign to player 2")),
        "sem vaga"
    );
}

const FLOOR: i32 = 100;

fn body(x: i32, ch: Character, w: usize) -> Jake {
    let mut inv = crate::jake::Inventory::for_character(&[(0, 0), (1, 20)], ch);
    inv.slots[w] = (1, 10);
    inv.weapon = w;
    Jake::new(
        x,
        FLOOR - 47,
        Rect {
            x: 2,
            y: 3,
            w: 18,
            h: 44,
        },
        std::collections::HashMap::new(),
    )
    .with_character(ch)
    .with_inventory(inv)
}

/// Cena de 2000 × 1000 com chão em y = 100 e parede em x = 1200: o jogador 1 em `x1` com a
/// arma `w1`, o 2 em `x2` com `w2`, os dois olhando para a direita.
fn duo(mode: Mode, (x1, w1): (i32, usize), (x2, w2): (i32, usize)) -> Scene {
    let mut s = Scene::empty(2000, 1000);
    s.mode = mode;
    let anm = Rc::new(
        openhunted_formats::Animation::parse(&openhunted_formats::test_support::mkanm()).unwrap(),
    );
    for n in super::super::arsenal::ANIMS
        .iter()
        .chain(&["OEX.ANM", "OBU.ANM", "OBS00.ANM"])
    {
        s.projectile_anims.insert(n.to_string(), anm.clone());
    }
    s.explosion_anim = Some(anm.clone());
    let sprite = |x| Sprite {
        layer: Layer::Object,
        player: AnimPlayer::from_frame(&anm, 0),
        anim: anm.clone(),
        x,
        y: FLOOR - 47,
        frozen: false,
        scale: (1, 1),
        mirror: false,
    };
    let solids = vec![
        Rect {
            x: -100,
            y: FLOOR,
            w: 2200,
            h: 20,
        },
        Rect {
            x: 1200,
            y: -100,
            w: 20,
            h: 300,
        },
    ];
    let colours = mode.duel().map_or([0, 0], |d| d.colours.map(i32::from));
    let chars = mode.duel().map_or([J, J], |d| d.chars);
    for i in 0..2 {
        s.players.push(Player {
            number: i as u8 + 1,
            character: chars[i],
            colour: colours[i],
            stats: Stats::default(),
            credit: 0,
            start_layer: 0,
        });
    }
    s.jake = Some(body(x1, chars[0], w1).with_colour(colours[0]));
    s.jake_sprite = Some(sprite(x1));
    s.parked.push(Seat {
        player: 1,
        jake: Some(body(x2, chars[1], w2).with_colour(colours[1])),
        sprite: Some(sprite(x2)),
        ..Default::default()
    });
    for i in 0..2 {
        s.with_seat(i, |s| {
            s.cell_solids = solids.clone();
            s.cell_world = World::new(solids.clone());
            s.rebuild_world();
        });
    }
    s.layer_worlds = vec![World::new(solids)];
    s.tick_players([RIGHT, RIGHT]);
    s.tick_players([0, 0]);
    s
}

/// Mina saltadora (contada, tipo 11) de caixa alta pousada no chão em `x`.
fn hop_mine(x: i32) -> Entity {
    use crate::behaviour::hop_mine::{HEALTH, HopMineState};
    let mut m = logic(
        crate::entity::HOP_MINE,
        x,
        FLOOR - 40,
        &[0, 0, 0, 0, 0, 4, 0, 5],
    );
    m.flags = 0x80801;
    m.health = HEALTH;
    m.solid_shapes = true;
    m.boxes = vec![Rect {
        x: 0,
        y: 0,
        w: 17,
        h: 40,
    }];
    m.behaviour = Behaviour::HopMine(HopMineState::default());
    m
}

fn run(s: &mut Scene, inputs: [u32; 2], n: usize) {
    s.tick_players(inputs);
    for _ in 0..n {
        s.tick_players([0, 0]);
    }
}

/// Crédito por arma: a mina morta pelo jogador 2 (bala, chumbo, foguete, granada, soco)
/// conta só para ele; no solo, todo morto conta para o 1.
#[test]
fn kill_credit_through_each_weapon_path() {
    for (w, x, n) in [
        (PISTOL, 220, 12),
        (SHOTGUN, 220, 12),
        (ROCKETS, 220, 12),
        (GRENADES, 180, 80),
        (FISTS, 0, 12),
    ] {
        let mut s = duo(Mode::Coop, (1800, PISTOL), (100, w));
        let mx = if w == FISTS {
            let b = s.player_jake(1).unwrap().body.bounds();
            b.x + b.w + 2
        } else {
            x
        };
        s.entities.push(hop_mine(mx));
        if w == FISTS {
            for _ in 0..4 {
                s.tick_players([0, PUNCH]);
                if let Some(e) = s.entities.first_mut() {
                    e.health = e.health.min(1);
                }
            }
            run(&mut s, [0, 0], n);
        } else {
            run(&mut s, [0, SHOOT], n);
        }
        let m = &s.entities[0];
        assert!(crate::behaviour::enemy::killed(m), "arma {w}: {}", m.health);
        assert_eq!(s.players[1].stats.kills, 1, "arma {w}");
        assert_eq!(s.players[0].stats.kills, 0, "arma {w}");
        assert_eq!(s.kills, 1);
    }
}

/// Mata o jogador `victim` (vida 1, sem armadura) com um tiro do `shooter` a 60 px.
fn frag(s: &mut Scene, shooter: usize, victim: usize) {
    let (x, y) = {
        let v = s.player_jake(victim).unwrap();
        (v.body.x, v.body.y)
    };
    let v = s.player_jake_mut(victim).unwrap();
    v.health = crate::damage::Health { hp: 1, armor: 0 };
    s.place_player(shooter, (x - 60, y), 0);
    // o atirador pode ter renascido sem armas
    let j = s.player_jake_mut(shooter).unwrap();
    j.inventory.slots[PISTOL] = (1, 10);
    j.inventory.weapon = PISTOL;
    let mut inputs = [0, 0];
    inputs[shooter] = SHOOT;
    for _ in 0..30 {
        s.tick_players(inputs);
        if s.player_jake(victim).unwrap().is_dead() {
            break;
        }
    }
    assert!(s.player_jake(victim).unwrap().is_dead(), "morreu");
    for _ in 0..30 {
        s.tick_players([0, 0]);
        if !s.player_jake(victim).unwrap().is_dead() {
            return;
        }
    }
    panic!("não renasceu");
}

/// F e S (`0x4268ac`): matar o outro dá F; morrer pela própria explosão dá S, limitado a F;
/// morrer de outra causa (crédito 0), nada.
#[test]
fn frags_and_suicides() {
    let mut s = duo(duel([J, J]), (100, PISTOL), (400, PISTOL));
    frag(&mut s, 0, 1);
    assert_eq!((s.players[0].stats.frags, s.players[1].stats.frags), (1, 0));
    frag(&mut s, 1, 0);
    assert_eq!(s.players[1].stats.frags, 1);
    // autoeliminação: o 1 com o foguete na parede a 20 px; F do 1 = 1 → S = 1
    let suicide = |s: &mut Scene| {
        s.place_player(0, (1170, FLOOR - 47), 0);
        let j = s.player_jake_mut(0).unwrap();
        j.health = crate::damage::Health { hp: 1, armor: 0 };
        j.inventory.slots[ROCKETS] = (1, 10);
        j.inventory.weapon = ROCKETS;
        s.place_player(1, (300, FLOOR - 47), 0);
        for _ in 0..60 {
            s.tick_players([SHOOT, 0]);
            if s.player_jake(0).unwrap().is_dead() {
                break;
            }
        }
        for _ in 0..30 {
            s.tick_players([0, 0]);
        }
    };
    suicide(&mut s);
    assert_eq!(s.players[0].stats.suicides, 1);
    suicide(&mut s);
    assert_eq!(s.players[0].stats.suicides, 1, "S limitado a F");
    assert_eq!(s.players[0].stats.score(), 0);
    // morte sem crédito (dano de um inimigo): nada
    let before = s.players.iter().map(|p| p.stats).collect::<Vec<_>>();
    if let Some(j) = s.player_jake_mut(1) {
        j.hurt(crate::damage::Pending {
            amount: 500,
            ..Default::default()
        });
    }
    run(&mut s, [0, 0], 30);
    let after = s.players.iter().map(|p| p.stats).collect::<Vec<_>>();
    assert_eq!(before, after);
}

/// Renascer na disputa: cadáver clonado na cor do jogador, ponto de renascimento, sem armas,
/// vida 100, armadura de antes.
#[test]
fn deathmatch_respawn_wipes_inventory_and_keeps_armor() {
    let mode = Mode::Deathmatch(Duel {
        chars: [J, J],
        colours: [35, 38],
        ..Duel::default()
    });
    let mut s = duo(mode, (100, PISTOL), (400, ROCKETS));
    s.spawn_points = vec![(700, 40, 0)];
    let v = s.player_jake_mut(1).unwrap();
    v.health = crate::damage::Health { hp: 1, armor: 0 };
    s.place_player(0, (340, FLOOR - 47), 0);
    for _ in 0..30 {
        s.tick_players([SHOOT, 0]);
        if s.player_jake(1).unwrap().is_dead() {
            break;
        }
    }
    s.player_jake_mut(1).unwrap().health.armor = 33;
    for _ in 0..30 {
        s.tick_players([0, 0]);
        if !s.player_jake(1).unwrap().is_dead() {
            break;
        }
    }
    let j = s.player_jake(1).unwrap();
    assert!(!j.is_dead());
    assert_eq!(j.health, crate::damage::Health { hp: 100, armor: 33 });
    assert_eq!(j.inventory.weapon, FISTS);
    assert_eq!(j.inventory.ammo(ROCKETS), 0);
    let b = j.body.bounds();
    assert_eq!((b.x + b.w / 2, b.y), (700, 40), "centro x e topo no ponto");
    let corpses: Vec<&Entity> = s
        .entities
        .iter()
        .filter(|e| matches!(e.behaviour, Behaviour::Corpse(_)))
        .collect();
    assert_eq!(corpses.len(), 1);
    assert!(matches!(corpses[0].behaviour, Behaviour::Corpse(38)));
    assert!(!corpses[0].solid_shapes && corpses[0].flags & 0x8000 != 0);
    assert_eq!(s.players[0].stats.frags, 1);
}

/// No coop, morto vira cadáver e não renasce; derrota só com os dois mortos.
#[test]
fn coop_defeat_needs_both_dead() {
    let mut s = duo(Mode::Coop, (100, PISTOL), (400, PISTOL));
    s.player_jake_mut(1).unwrap().hurt(crate::damage::Pending {
        amount: 500,
        ..Default::default()
    });
    run(&mut s, [0, 0], 100);
    assert!(s.player_jake(1).unwrap().is_dead(), "fica morto");
    assert_eq!(s.outcome, None);
    assert!(
        !s.entities
            .iter()
            .any(|e| matches!(e.behaviour, Behaviour::Corpse(_)))
    );
    s.player_jake_mut(0).unwrap().hurt(crate::damage::Pending {
        amount: 500,
        ..Default::default()
    });
    run(&mut s, [0, 0], 60);
    assert_eq!(s.outcome, Some(Outcome::Defeat));
}

/// Saída com zona 0x40 e caixa em `x` (1 ligada = 1 tique de CIMA parado diante dela).
fn exit_at(s: &mut Scene, x: i32) {
    let mut e = logic(crate::entity::EXIT, x, FLOOR - 50, &[]);
    e.objective = crate::verdict::OBJ_ACTIVATE;
    e.behaviour = Behaviour::Exit(Default::default());
    e.boxes = vec![Rect {
        x: 0,
        y: 0,
        w: 40,
        h: 50,
    }];
    s.entities.push(e);
    s.prop_zones.push(crate::jake::Zone {
        layer: 0,
        kind: crate::jake::zone::EXIT,
        rect: Rect {
            x,
            y: FLOOR - 50,
            w: 40,
            h: 50,
        },
    });
}

/// Coop: um morto, o outro na saída: os dois vencem.
#[test]
fn coop_one_at_the_exit_wins_for_both() {
    let mut s = duo(Mode::Coop, (100, PISTOL), (400, PISTOL));
    exit_at(&mut s, 95);
    s.player_jake_mut(1).unwrap().hurt(crate::damage::Pending {
        amount: 500,
        ..Default::default()
    });
    run(&mut s, [0, 0], 20);
    run(&mut s, [UP, 0], 60);
    assert_eq!(s.outcome, Some(Outcome::Victory));
}

/// Fins da disputa: limite de pontos (a saída não conta), tempo real, saída sem limite.
#[test]
fn deathmatch_ends_by_points_time_or_exit() {
    let with = |points_limit, time_limit_ms| {
        Mode::Deathmatch(Duel {
            chars: [J, J],
            points_limit,
            time_limit_ms,
            ..Duel::default()
        })
    };
    // pontos
    let mut s = duo(with(2, 0), (100, PISTOL), (400, PISTOL));
    exit_at(&mut s, 95);
    run(&mut s, [UP, 0], 60);
    assert_eq!(s.outcome, None, "com limite de pontos a saída não acaba");
    s.players[1].stats.frags = 2;
    run(&mut s, [0, 0], 39);
    assert_eq!(s.outcome, Some(Outcome::Defeat));
    assert_eq!(s.winner(), Some(1));
    // tempo (relógio de quem chama)
    let mut s = duo(with(0, 60_000), (100, PISTOL), (400, PISTOL));
    s.elapsed_ms = 60_000;
    run(&mut s, [0, 0], 50);
    assert_eq!(s.outcome, None);
    s.elapsed_ms = 60_001;
    run(&mut s, [0, 0], 39);
    assert_eq!(s.outcome, Some(Outcome::Defeat));
    assert_eq!(s.winner(), None, "empate");
    // saída, sem objetivos
    let mut s = duo(with(0, 0), (100, PISTOL), (400, PISTOL));
    exit_at(&mut s, 95);
    let mut piece = logic(crate::entity::CAR_PIECE, 900, 0, &[]);
    piece.objective = crate::verdict::OBJ_ACTIVATE;
    piece.behaviour = Behaviour::CarPiece;
    s.entities.push(piece);
    run(&mut s, [UP, 0], 60);
    assert_eq!(s.outcome, Some(Outcome::Victory));
}

/// Item de vida em `x` (só o Jake o pega: 0x100).
fn health_item(x: i32) -> Entity {
    let mut it = logic(crate::entity::HEALTH_ITEM, x, FLOOR - 20, &[]);
    it.flags = 0x801a9;
    it.boxes = vec![Rect {
        x: 0,
        y: 0,
        w: 20,
        h: 20,
    }];
    it.behaviour = Behaviour::Pickup(Default::default());
    it
}

/// Itens renascem só na disputa: 300 tiques de espera, de volta no quadro 8 do efeito; na
/// disputa o Garathe pega o item só do Jake.
#[test]
fn items_regenerate_only_in_deathmatch() {
    for mode in [duel([G, J]), Mode::Coop] {
        let mut s = duo(mode, (100, FISTS), (700, FISTS));
        // o 1 é o Garathe nos dois modos
        let j = s.player_jake_mut(0).unwrap();
        *j = j.clone().with_character(G);
        let x = s.player_jake(0).unwrap().body.bounds().x;
        s.player_jake_mut(0).unwrap().health.hp = 50;
        s.entities.push(health_item(x));
        let k = s.entities.len() - 1;
        s.tick_players([0, 0]);
        let dm = mode.duel().is_some();
        // coop: o Garathe não pega o item do Jake
        assert_eq!(!s.entities[k].alive, dm, "{mode:?}");
        if !dm {
            continue;
        }
        assert_eq!(s.player_jake(0).unwrap().health.hp, 75);
        s.place_player(0, (300, FLOOR - 47), 0);
        // 300 tiques escondido e o quadro 8 do efeito: volta no 308º tique desde a coleta
        let mut back = None;
        for t in 1..400 {
            s.tick_players([0, 0]);
            if s.entities[k].alive {
                back = Some(t + 1);
                break;
            }
        }
        assert_eq!(back, Some(REGEN_WAIT as usize + REGEN_FRAME));
        assert!(!s.entities[k].on, "voltou");
        run(&mut s, [0, 0], 20);
        assert!(s.regens.is_empty());
        assert!(
            s.entities
                .iter()
                .filter(|e| e.kind == REGENERATE)
                .all(|e| !e.alive)
        );
    }
    // coop com o Jake: pega e não volta
    let mut s = duo(Mode::Coop, (100, FISTS), (700, FISTS));
    let x = s.player_jake(0).unwrap().body.bounds().x;
    s.player_jake_mut(0).unwrap().health.hp = 50;
    s.entities.push(health_item(x));
    let k = s.entities.len() - 1;
    run(&mut s, [0, 0], 400);
    assert!(!s.entities[k].alive && s.regens.is_empty());
}

/// Uma mina flutuante entre os dois, mais perto do 2, vai até ele.
#[test]
fn enemies_target_the_nearer_player() {
    use crate::behaviour::mine::MineState;
    let mut s = duo(Mode::Coop, (100, FISTS), (700, FISTS));
    let mut m = logic(crate::entity::MINE, 600, FLOOR - 40, &[0]);
    m.flags = 0x80801;
    m.boxes = vec![Rect {
        x: 0,
        y: 0,
        w: 16,
        h: 16,
    }];
    m.health = 100;
    m.behaviour = Behaviour::Mine(MineState::new(0));
    s.entities.push(m);
    let k = s.entities.len() - 1;
    assert_eq!(s.entity_targets()[k], 1);
    let x0 = s.entities[k].x;
    run(&mut s, [0, 0], 20);
    assert!(
        s.entities[k].x > x0,
        "foi para a direita: {x0} → {}",
        s.entities[k].x
    );
    // o 2 sai de jogo (morto): a mina passa ao 1
    s.player_jake_mut(1).unwrap().hurt(crate::damage::Pending {
        amount: 500,
        ..Default::default()
    });
    run(&mut s, [0, 0], 2);
    assert_eq!(s.entity_targets()[k], 0);
}

/// Pontos de renascimento: `fields[14] & 4`, na ordem das células, até 16.
#[test]
fn spawn_points_from_cells() {
    let mut l = super::super::tests::level();
    l.grid_x = 5;
    l.grid_y = 4;
    l.grid_z = 2;
    let cell = |f14: i32| {
        let mut c = l.cells[1].clone().unwrap();
        c.fields = vec![0; 15];
        c.fields[14] = f14;
        Some(c)
    };
    l.cells = (0..40)
        .map(|i| cell(if i % 2 == 1 { 4 } else { 1 }))
        .collect();
    let p = spawn_points(&l);
    assert_eq!(p.len(), MAX_SPAWNS);
    assert_eq!(p[0], (128 + 64, 32, 0));
    assert_eq!(p[3], (128 * 2 + 64, 128 + 32, 0), "célula 7 = (2, 1)");
    assert_eq!(p[10], (128 + 64, 32, 1), "célula 21 = (1, 0, 1)");
}

/// O ouvinte é a vista mais perto; o foco de cada jogador segue o corpo dele.
#[test]
fn listener_and_focus_per_player() {
    let s = duo(Mode::Coop, (100, FISTS), (700, FISTS));
    assert_eq!(s.nearest_player((0, 0)), Some(0));
    assert_eq!(s.nearest_player((900, 50)), Some(1));
    let f = s.focus_of(1).unwrap();
    assert!((690..760).contains(&f.0), "{f:?}");
    assert_eq!(s.layer_of(1), Some(0));
}
