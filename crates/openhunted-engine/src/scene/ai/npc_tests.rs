// SPDX-License-Identifier: GPL-3.0-or-later
//! Personagens da IA alcançados pelo que fere todo objeto da camada (auditoria M17,
//! `docs/auditoria/inimigos-2.md`): dano e terremoto vindos dos objetos e o especial da IA.

use std::rc::Rc;

use super::*;
use crate::jake::{Inventory, SpecialBlast};
use crate::verdict;

const BOX: Rect = Rect {
    x: 2,
    y: 3,
    w: 18,
    h: 44,
};

fn sprite(x: i32) -> Sprite {
    let a = Rc::new(
        openhunted_formats::Animation::parse(&openhunted_formats::test_support::mkanm()).unwrap(),
    );
    Sprite {
        layer: Layer::Object,
        player: AnimPlayer::from_frame(&a, 0),
        anim: a,
        x,
        y: 53,
        frozen: false,
        scale: (1, 1),
        mirror: false,
    }
}

/// Chão em y = 100, o humano (Jake) em `hx` e um personagem da IA de `ch` em `ax`.
fn arena(hx: i32, ch: Character, ax: i32) -> Scene {
    let mut s = Scene::empty(2000, 1000);
    let solids = vec![Rect {
        x: -100,
        y: 100,
        w: 2200,
        h: 20,
    }];
    s.cell_solids = solids.clone();
    s.cell_world = World::new(solids);
    s.jake = Some(Jake::new(hx, 53, BOX, Default::default()).with_health(100, 0));
    s.jake_sprite = Some(sprite(hx));
    s.ai_grid = CellGrid::new((16, 8, 1));
    let body = Jake::new(ax, 53, BOX, Default::default())
        .with_character(ch)
        .with_health(100, 0)
        .with_inventory(Inventory::for_character(&[(0, 0), (1, 20)], ch));
    s.entities.push(Entity {
        kind: ch.kind(),
        x: ax,
        y: 53,
        flags: 0x94801,
        layer: 0,
        id: 0,
        values: vec![],
        triggers: vec![],
        objective: verdict::OBJ_DESTROY,
        health: 100,
        armor: 0,
        boxes: vec![BOX],
        solid_shapes: true,
        on: false,
        alive: true,
        sprite: Some(sprite(ax)),
        behaviour: Behaviour::AiPlayer(Box::new(AiPlayer {
            body,
            brain: Brain::new(),
            body_checked: false,
            rng: crate::rng::Rng::new(5),
            quake: 0,
        })),
    });
    s.rebuild_world();
    s
}

fn ai(s: &mut Scene) -> &mut AiPlayer {
    match &mut s.entities[0].behaviour {
        Behaviour::AiPlayer(a) => a,
        _ => panic!("não é IA"),
    }
}

#[test]
fn npcs_list_the_ai_characters() {
    let s = arena(100, Character::Jake, 600);
    let n = s.npcs();
    assert_eq!(n.len(), 1);
    assert_eq!((n[0].index, n[0].kind, n[0].origin), (0, 1, (600, 53)));
}

/// `Out::npc_hits` vai ao corpo do personagem (armadura, reações e morte do jogador).
#[test]
fn npc_hits_hurt_the_ai_body() {
    let mut s = arena(100, Character::Jake, 600);
    let mut out = behaviour::Out::default();
    out.npc_hits.push((
        0,
        Pending {
            amount: 4,
            ..Default::default()
        },
    ));
    s.apply_npc_out(&mut out);
    s.tick(0);
    assert_eq!(ai(&mut s).body.health.hp, 96);
}

/// Terremoto do Bruiser (`0x418229` percorre os tipos 1/2 vivos e em jogo, IA incluída): o
/// Jake da IA no chão cambaleia (choque, estado 15); o Garathe da IA conta o dele (121).
#[test]
fn bruiser_quake_shakes_ai_characters() {
    let mut s = arena(100, Character::Jake, 600);
    let mut out = behaviour::Out::default();
    out.npc_quakes.push((0, 20));
    s.apply_npc_out(&mut out);
    let mut shaken = false;
    for _ in 0..3 {
        s.tick(0);
        shaken |= matches!(ai(&mut s).body.state(), State::Shocked(_));
    }
    assert!(shaken, "o Jake da IA cambaleia");
    let mut g = arena(100, Character::Garathe, 600);
    let mut out = behaviour::Out::default();
    out.npc_quakes.push((0, 20));
    g.apply_npc_out(&mut out);
    assert!(
        ai(&mut g).body.quake_ticks() > 0,
        "o Garathe usa o contador dele"
    );
}

/// O especial do personagem da IA (`0x427643`) é aplicado: o humano a < 70 px é derrubado.
#[test]
fn ai_special_knocks_down_the_human() {
    let mut s = arena(100, Character::Jake, 140);
    let b = ai(&mut s).body.body.bounds();
    s.ai_special(
        0,
        SpecialBlast {
            center: (b.x + b.w / 2, b.y + b.h / 2),
            layer: 0,
            garathe: false,
        },
    );
    s.tick(0);
    assert!(
        matches!(s.jake.as_ref().unwrap().state(), State::KnockedDown(_)),
        "{:?}",
        s.jake.as_ref().unwrap().state()
    );
}
