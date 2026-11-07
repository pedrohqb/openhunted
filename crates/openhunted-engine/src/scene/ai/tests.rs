// SPDX-License-Identifier: GPL-3.0-or-later
//! Personagens da IA numa cena sintética: aproximação e golpe, pistola do humano, tiros
//! com dono.

use std::rc::Rc;

use super::*;
use crate::input::{LEFT, SHOOT};
use crate::jake::{FISTS, Inventory, PISTOL};
use crate::verdict;

const BOX: Rect = Rect {
    x: 2,
    y: 3,
    w: 18,
    h: 44,
};

fn anim() -> Rc<openhunted_formats::Animation> {
    Rc::new(
        openhunted_formats::Animation::parse(&openhunted_formats::test_support::mkanm()).unwrap(),
    )
}

fn sprite(x: i32) -> Sprite {
    let a = anim();
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

/// Cena de 2000 px com chão em y = 100 e o jogador humano (Jake, punhos e pistola) em `hx`.
fn arena(hx: i32) -> Scene {
    let mut s = Scene::empty(2000, 1000);
    let a = anim();
    for n in super::super::arsenal::ANIMS
        .iter()
        .chain(&["OBU.ANM", "OEX.ANM"])
    {
        s.projectile_anims.insert(n.to_string(), a.clone());
    }
    s.explosion_anim = Some(a);
    let solids = vec![Rect {
        x: -100,
        y: 100,
        w: 2200,
        h: 20,
    }];
    s.cell_solids = solids.clone();
    s.cell_world = World::new(solids);
    let mut inv = Inventory::for_character(&[(0, 0), (1, 20), (1, 50)], Character::Jake);
    inv.weapon = PISTOL;
    s.jake = Some(
        Jake::new(hx, 53, BOX, std::collections::HashMap::new())
            .with_health(100, 0)
            .with_inventory(inv),
    );
    s.jake_sprite = Some(sprite(hx));
    s.rebuild_world();
    s
}

/// Personagem da IA de `ch` em `x`, com vida `hp` e a arma `w` (pares `points`).
fn ai_at(ch: Character, x: i32, hp: i32, points: &[(i32, i32)], w: usize) -> Entity {
    let mut inv = Inventory::for_character(points, ch);
    inv.weapon = w;
    let body = Jake::new(x, 53, BOX, std::collections::HashMap::new())
        .with_character(ch)
        .with_health(hp, 0)
        .with_inventory(inv);
    Entity {
        kind: if ch == Character::Jake { 1 } else { 2 },
        x,
        y: 53,
        flags: 0x94801,
        layer: 0,
        id: 0,
        values: vec![],
        triggers: vec![],
        objective: verdict::OBJ_DESTROY,
        health: hp,
        armor: 0,
        boxes: vec![BOX],
        solid_shapes: true,
        on: false,
        alive: true,
        sprite: Some(sprite(x)),
        behaviour: Behaviour::AiPlayer(Box::new(AiPlayer {
            body,
            brain: Brain::new(),
            body_checked: false,
            rng: crate::rng::Rng::new(5),
        })),
    }
}

fn body(s: &Scene, i: usize) -> &Jake {
    match &s.entities[i].behaviour {
        Behaviour::AiPlayer(a) => &a.body,
        _ => panic!("não é IA"),
    }
}

fn human_hp(s: &Scene) -> i32 {
    s.jake.as_ref().unwrap().health.hp
}

/// O golpe só cobre a metade da frente da caixa (`0x43accb`): as garras pegam quem chega
/// perto. O humano corre para o Minotauro, que corre para ele e o fere.
#[test]
fn ai_minotaur_approaches_and_hits_the_human() {
    let mut s = arena(100);
    s.entities
        .push(ai_at(Character::Garathe, 400, 100, &[(0, 0), (1, 20)], 1));
    let x0 = body(&s, 0).body.x;
    let mut hit = None;
    for t in 0..60 {
        s.tick(crate::input::RIGHT);
        if human_hp(&s) < 100 {
            hit = Some(t);
            break;
        }
    }
    assert!(body(&s, 0).body.x < x0, "correu para o humano");
    assert!(hit.is_some(), "as garras feriram o humano");
    assert!(matches!(body(&s, 0).state(), State::Fire { w: 1, .. }));
    assert_eq!(body(&s, 0).health.hp, 100);
    // o personagem da IA não barra o humano nem entra no mundo dele
    assert!(s.object_solids.is_empty());
}

#[test]
fn human_pistol_kills_an_ai_jake_into_a_corpse() {
    let mut s = arena(100);
    // o Jake da IA, só com os punhos, a 200 px (o humano atira para a direita)
    s.entities
        .push(ai_at(Character::Jake, 300, 4, &[(0, 0)], FISTS));
    s.tick(crate::input::RIGHT);
    let mut dead_at = None;
    for t in 0..100 {
        s.tick(if t % 2 == 0 { SHOOT } else { 0 });
        if dead_at.is_none() && body(&s, 0).is_dead() {
            dead_at = Some(t);
        }
    }
    assert!(
        dead_at.is_some(),
        "a pistola matou (e ele se abaixou das balas antes)"
    );
    let e = &s.entities[0];
    assert!(e.alive, "o cadáver fica");
    assert_eq!(body(&s, 0).state(), State::Dead);
    assert!(e.health <= 0 && !e.solid_shapes && e.flags & 0x8000 != 0);
    assert!(
        verdict::objectives_met(&s.entities, s.jake.as_ref()),
        "destruir"
    );
    assert_eq!(behaviour::enemy::kills(&s.entities), 0, "tipo 1 não conta");
    assert_eq!(s.kills_total, 0, "nem no total");
    // o cadáver não para mais as balas
    let n = s.entities.len();
    for t in 0..40 {
        s.tick(if t % 2 == 0 { SHOOT } else { 0 });
    }
    assert!(s.entities.len() > n);
}

#[test]
fn ai_pistol_hurts_the_human_and_never_its_owner() {
    let mut s = arena(100);
    s.entities.push(ai_at(
        Character::Jake,
        250,
        100,
        &[(0, 0), (1, 20), (1, 50)],
        PISTOL,
    ));
    for _ in 0..60 {
        s.tick(0);
    }
    assert!(
        s.entities
            .iter()
            .any(|e| matches!(e.behaviour, Behaviour::Bullet(_))),
        "a IA atirou"
    );
    assert!(human_hp(&s) < 100, "as balas da IA ferem o humano");
    assert_eq!(body(&s, 0).health.hp, 100, "nunca o dono");
    assert!(
        s.entities.iter().all(|e| !matches!(
            &e.behaviour,
            Behaviour::Bullet(b) if b.player_shot
        )),
        "sem crédito"
    );
}

#[test]
fn ai_without_health_is_a_corpse_that_ignores_everything() {
    let mut s = arena(100);
    s.entities
        .push(ai_at(Character::Jake, 900, 0, &[(0, 0)], FISTS));
    for _ in 0..30 {
        s.tick(LEFT);
    }
    let b = body(&s, 0);
    assert!(b.is_dead(), "vida 0 morre no primeiro tique");
    assert_eq!(s.entities.len(), 1, "nada criado");
}

#[test]
fn ai_characters_draw_with_the_player_colour_tables() {
    let j = ai_at(Character::Jake, 0, 100, &[(0, 0)], FISTS);
    let g = ai_at(Character::Garathe, 0, 100, &[(0, 0)], 1);
    assert_eq!(
        crate::tint::entity(&j, 0, 0).map(|t| t.table),
        Some(crate::tint::JAKE)
    );
    assert_eq!(
        crate::tint::entity(&g, 0, 0).map(|t| t.table),
        Some(crate::tint::GARATHE)
    );
}
