// SPDX-License-Identifier: GPL-3.0-or-later
//! Objetos das missões tutoriais T_J_003/005/007 sobre os dados reais (OPENHUNTED_DATA):
//! olho-laser, pistola, barris, fogo, teletransporte, peça do carro e portas `OH`
//! (`docs/re/missoes-tutorial.md`).

use openhunted_engine::input::{LEFT, RIGHT, SHOOT, UP};
use openhunted_engine::{Assets, Behaviour, Scene, State, verdict};
use openhunted_formats::{Level, open_data};

fn load(level: &str) -> Option<Scene> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Some(Scene::build(&l, &mut a).unwrap())
}

fn alive(s: &Scene, kind: u32) -> usize {
    s.entities
        .iter()
        .filter(|e| e.alive && e.kind == kind)
        .count()
}

/// Põe o Jake em (x, y) e deixa assentar.
fn place(s: &mut Scene, x: i32, y: i32, ticks: u32) {
    if let Some(j) = s.jake.as_mut() {
        j.body.x = x;
        j.body.y = y;
        j.body.vx = 0;
        j.body.vy = 0;
    }
    for _ in 0..ticks {
        s.tick(0);
    }
}

/// T_J_003: a pistola ao lado do início dá a arma (90 + 60, teto 99).
#[test]
fn t_j_003_pistol_next_to_the_start_gives_the_gun() {
    let Some(mut s) = load("T_J_003.LEV") else {
        return;
    };
    assert_eq!(s.jake.as_ref().unwrap().inventory.pistol_ammo(), 0);
    for _ in 0..10 {
        s.tick(0);
    }
    for _ in 0..8 {
        s.tick(LEFT);
    }
    let j = s.jake.as_ref().unwrap();
    assert_eq!(j.inventory.slots[2], (1, 99));
    assert_eq!(alive(&s, 12), 0);
}

/// T_J_003: o olho-laser do teto atira no Jake parado embaixo dele (5 por raio, com tranco)
/// e morre com um tiro de pistola para cima.
#[test]
fn t_j_003_laser_eye_fires_at_jake_and_a_bullet_kills_it() {
    let Some(mut s) = load("T_J_003.LEV") else {
        return;
    };
    // o olho em (104, 12): caixa x 124..134; o Jake com a boca para cima (x + 9) embaixo
    place(&mut s, 118, 150, 20);
    let hp0 = s.jake.as_ref().unwrap().health.hp;
    let mut bolts = 0;
    let mut shoved = false;
    let mut laser_sound = false;
    for _ in 0..200 {
        s.tick(0);
        bolts = bolts.max(alive(&s, 36));
        shoved |= s.jake.as_ref().unwrap().state() == State::Pushed;
        laser_sound |= s.events.drain(..).any(|e| e.sound_id() == Some(3037));
    }
    let hp = s.jake.as_ref().unwrap().health.hp;
    assert!(bolts >= 2, "raios no ar: {bolts}");
    assert!(hp < hp0, "o olho não feriu o Jake ({hp0} → {hp})");
    assert_eq!((hp0 - hp) % 5, 0, "5 por raio (Alpha), sem armadura");
    assert!(shoved && laser_sound);
    // a pistola, e tiros para cima
    if let Some(j) = s.jake.as_mut() {
        j.inventory.slots[2] = (1, 50);
    }
    let kills = s.kills;
    let mut t = 0;
    while alive(&s, 104) > 0 && t < 200 {
        s.tick(if t % 7 < 2 { SHOOT | UP } else { 0 });
        t += 1;
    }
    assert_eq!(alive(&s, 104), 0, "o olho sobreviveu a {t} tiques de tiro");
    assert_eq!(s.kills, kills + 1, "o olho morto pela bala conta");
    assert!(
        s.entities.iter().any(|e| e.alive && e.kind == 86),
        "explosão do olho"
    );
}

/// T_J_005: atirar nos barris empilhados de longe: explodem em cadeia (e as minas ao lado),
/// sem ferir o Jake a mais de 144 px.
#[test]
fn t_j_005_barrels_explode_when_shot_and_chain() {
    let Some(mut s) = load("T_J_005.LEV") else {
        return;
    };
    assert_eq!(alive(&s, 28), 2);
    place(&mut s, 740, 150, 30);
    assert!(s.jake.as_ref().unwrap().body.on_ground);
    s.tick(RIGHT);
    for _ in 0..10 {
        s.tick(0);
    }
    let hp0 = s.jake.as_ref().unwrap().health.hp;
    let mut t = 0;
    while alive(&s, 28) > 0 && t < 120 {
        s.tick(if t % 7 < 2 { SHOOT } else { 0 });
        t += 1;
    }
    for _ in 0..5 {
        s.tick(0);
    }
    assert_eq!(alive(&s, 28), 0, "os dois barris explodem");
    let near_mines = s
        .entities
        .iter()
        .filter(|e| e.kind == 102 && (900..1000).contains(&e.x) && e.y < 220)
        .filter(|e| e.alive)
        .count();
    assert_eq!(near_mines, 0, "as minas ao lado vão junto");
    assert_eq!(s.jake.as_ref().unwrap().health.hp, hp0);
}

/// T_J_005: o fogo do fosso queima 1 por tique.
#[test]
fn t_j_005_fire_hurts() {
    let Some(mut s) = load("T_J_005.LEV") else {
        return;
    };
    place(&mut s, 1180, 300, 10); // só no segundo fogo (as caixas dos dois se cruzam)
    let j = s.jake.as_ref().unwrap();
    let hp0 = j.health.hp;
    for _ in 0..10 {
        s.tick(0);
    }
    let hp = s.jake.as_ref().unwrap().health.hp;
    assert!((8..=10).contains(&(hp0 - hp)), "{hp0} → {hp}");
}

/// T_J_005: o teletransporte no fundo do fosso leva o Jake à zona id 1 (438, 98), no poço de
/// cima: some por 42 tiques e reaparece 256 px acima, centrado no destino.
#[test]
fn t_j_005_teleporter_moves_jake_to_zone_one() {
    let Some(mut s) = load("T_J_005.LEV") else {
        return;
    };
    place(&mut s, 375, 300, 0);
    let mut t = 0;
    while !matches!(s.jake.as_ref().unwrap().state(), State::Teleporting(_)) && t < 40 {
        s.tick(0);
        t += 1;
    }
    let j = s.jake.as_ref().unwrap();
    assert!(
        matches!(j.state(), State::Teleporting(_)),
        "não embarcou: {:?} em ({}, {})",
        j.state(),
        j.body.x,
        j.body.y
    );
    assert!(!j.is_visible());
    let (x0, y0) = (j.body.x, j.body.y);
    for _ in 0..42 {
        s.tick(0);
    }
    let j = s.jake.as_ref().unwrap();
    assert!(j.is_visible() && j.is_active());
    assert_eq!(j.body.x - x0, 454 - 12 - 374);
    assert!(j.body.y < y0 - 200, "subiu: {} → {}", y0, j.body.y);
    let tele = s.entities.iter().find(|e| e.kind == 46).unwrap();
    assert!(tele.on);
}

/// T_J_007: a peça do carro (camada 2, objetivo "acionar") só cumpre os objetivos depois de
/// apanhada.
#[test]
fn t_j_007_car_piece_pickup_satisfies_the_objective() {
    let Some(mut s) = load("T_J_007.LEV") else {
        return;
    };
    assert!(!verdict::objectives_met(&s.entities, s.jake.as_ref()));
    let piece = s.entities.iter().position(|e| e.kind == 9).unwrap();
    assert_eq!(s.entities[piece].objective, 2);
    let b = s.entities[piece].bounds().unwrap();
    // põe o Jake na camada 2, sobre a peça (a cena segue a camada do Jake)
    s.jake.as_mut().unwrap().set_layer(2);
    s.set_jake_layer(2);
    if let Some(j) = s.jake.as_mut() {
        j.body.x = b.x - 5;
        j.body.y = b.y - 20;
    }
    s.tick(0);
    assert!(!s.entities[piece].alive && s.entities[piece].on);
    assert!(
        s.events.iter().any(|e| e.sound_id() == Some(5060)),
        "som da peça"
    );
    assert!(verdict::objectives_met(&s.entities, s.jake.as_ref()));
}

/// T_J_007: a parede `OH1` de x = 635 (camada 1) é sólida até a saída ligar.
#[test]
fn t_j_007_oh_door_is_solid_until_triggered() {
    let Some(mut s) = load("T_J_007.LEV") else {
        return;
    };
    let door = s
        .entities
        .iter()
        .position(|e| e.kind == 50 && e.x == 635)
        .unwrap();
    let wall = s.entities[door].bounds().unwrap();
    s.jake.as_mut().unwrap().set_layer(1);
    s.set_jake_layer(1);
    let blocks = |s: &mut Scene| {
        s.rebuild_world();
        s.object_solids.iter().any(|r| r.overlaps(&wall))
    };
    for _ in 0..20 {
        s.tick(0);
    }
    assert!(blocks(&mut s), "fechada barra");
    assert!(!s.entities[door].on);
    let exit = s.entities.iter().position(|e| e.kind == 5).unwrap();
    s.entities[exit].on = true;
    for _ in 0..30 {
        s.tick(0);
    }
    assert!(s.entities[door].on, "abriu com a saída ligada");
    assert!(!blocks(&mut s), "aberta não barra");
    assert!(matches!(&s.entities[door].behaviour, Behaviour::Door(d) if d.state == 4));
}

/// Todas as fases solo, entrada aleatória (inclusive tiro, CIMA e BAIXO), 1500 tiques com
/// duas sementes: nada entra em pânico.
#[test]
fn random_play_never_panics() {
    use openhunted_engine::input::{DOWN, JUMP};
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        return;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let la = gd.levels().unwrap();
    let missions =
        openhunted_formats::parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap()).unwrap();
    let keys = [
        0,
        LEFT,
        RIGHT,
        LEFT | JUMP,
        RIGHT | JUMP,
        JUMP,
        SHOOT,
        SHOOT | UP,
        SHOOT | RIGHT | UP,
        UP,
        DOWN,
        LEFT | SHOOT,
    ];
    let mut n = 0;
    for m in missions {
        let Some(s0) = load(&m.file) else { return };
        if s0.jake.is_none() {
            continue;
        }
        n += 1;
        for seed0 in [0x1234_5678_u32, 0xdead_beef] {
            let mut s = s0.clone();
            let mut seed = seed0;
            let mut input = 0;
            for t in 0..1500 {
                if t % 5 == 0 {
                    seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    input = keys[(seed >> 16) as usize % keys.len()];
                }
                s.tick(input);
                s.events.clear();
            }
        }
    }
    assert!(n > 20, "poucas fases com Jake: {n}");
}

/// I_J_104: os teletransportes id 1 (810, 366) e id 2 (790, 110), camada 0, apontam um para o
/// outro. Levado de um ao outro, o Jake reaparece sobre o destino e recupera o controle: o
/// destino espera 40 tiques a partir da mudança de lugar e lembra o passageiro enquanto ele
/// viaja, sem mandá-lo de volta.
#[test]
fn i_j_104_teleporter_pair_does_not_ping_pong() {
    let Some(mut s) = load("I_J_104.LEV") else {
        return;
    };
    s.jake.as_mut().unwrap().set_layer(0);
    s.set_jake_layer(0);
    place(&mut s, 812, 330, 0);
    let mut trips = 0;
    let mut was = false;
    let mut active_run = 0;
    for _ in 0..300 {
        s.tick(0);
        let j = s.jake.as_ref().unwrap();
        let now = matches!(j.state(), State::Teleporting(_));
        if now && !was {
            trips += 1;
        }
        was = now;
        active_run = if j.is_active() { active_run + 1 } else { 0 };
    }
    let j = s.jake.as_ref().unwrap();
    assert_eq!(trips, 1, "viagens: {trips}");
    assert!(active_run > 200, "sem controle: {active_run}");
    // uma célula acima (o id 2 está na célula de cima)
    assert!(j.body.y < 336 - 100, "não chegou ao id 2: y {}", j.body.y);
}
