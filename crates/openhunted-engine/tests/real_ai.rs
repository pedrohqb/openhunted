// SPDX-License-Identifier: GPL-3.0-or-later
//! Personagens da IA sobre os dados reais (OPENHUNTED_DATA): os Minotauros da `H_J_006`
//! vão atrás do Jake; fuzz em todas as fases com personagens da IA.

use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, UP};
use openhunted_engine::physics::CONTACT_SCENERY;
use openhunted_engine::scene::has_controllable_player;
use openhunted_engine::{Assets, Behaviour, Jake, Scene};
use openhunted_formats::{GameData, Level, open_data};

fn data() -> Option<GameData> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    Some(open_data(std::path::Path::new(&p)).unwrap())
}

fn level(gd: &GameData, name: &str) -> Level {
    Level::parse(&gd.levels().unwrap().get(name).unwrap()).unwrap()
}

/// Índices e corpos dos personagens da IA.
fn ais(s: &Scene) -> Vec<(usize, &Jake)> {
    s.entities
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match &e.behaviour {
            Behaviour::AiPlayer(a) => Some((i, &a.body)),
            _ => None,
        })
        .collect()
}

fn center(j: &Jake) -> (i32, i32) {
    let b = j.body.bounds();
    (b.x + b.w / 2, b.y + b.h / 2)
}

fn dist(a: (i32, i32), b: (i32, i32)) -> i64 {
    let (dx, dy) = (i64::from(a.0 - b.0), i64::from(a.1 - b.1));
    ((dx * dx + dy * dy) as f64).sqrt() as i64
}

/// Nenhum personagem da IA em jogo dentro das células da camada dele nem fora da fase.
fn check_ais(s: &Scene, name: &str, t: u32) {
    for (i, j) in ais(s) {
        let b = j.body.bounds();
        let c = center(j);
        assert!(
            c.0 >= 0 && c.1 >= 0 && c.0 <= s.width && c.1 <= s.height,
            "{name}: IA {i} fora da fase no tique {t}: {b:?}"
        );
        if j.is_active() && j.box_matches_state() {
            assert!(
                s.layer_cells(j.layer()).contact(&b) & CONTACT_SCENERY == 0,
                "{name}: IA {i} dentro do cenário no tique {t}: {:?} {b:?}",
                j.state()
            );
        }
    }
}

/// No início o Jake está fora do alcance da busca (outra camada, distância Manhattan > 10
/// células): os Minotauros esperam. Posto no corredor da camada 1, a ~250 px à esquerda do
/// segundo (à direita há uma porta `OH` fechada, que a busca não conhece), este vai atrás dele.
#[test]
fn h_j_006_minotaurs_go_after_the_jake() {
    let Some(gd) = data() else { return };
    let l = level(&gd, "H_J_006.LEV");
    let mut a = Assets::new(gd.resources().unwrap());
    let mut s = Scene::build(&l, &mut a).unwrap();
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    let minotaurs = ais(&s);
    assert_eq!(minotaurs.len(), 3, "3 Minotauros da IA");
    assert!(
        minotaurs
            .iter()
            .all(|(_, j)| j.character() == openhunted_engine::character::Character::Garathe)
    );
    let (k, m) = minotaurs[1];
    assert_eq!(m.layer(), 1);
    let mb = m.body.bounds();
    // um lugar livre no mesmo andar, à esquerda
    let cells = s.layer_cells(1).clone();
    let jb = s.jake.as_ref().unwrap().body.boxr;
    let x = (150..=300)
        .rev()
        .map(|d| mb.x - d)
        .find(|&x| {
            let r = openhunted_engine::Rect {
                x: x + jb.x,
                y: mb.y + mb.h - jb.h,
                ..jb
            };
            !cells.blocked(&r) && cells.standing_on(&r, 4)
        })
        .expect("chão livre ao lado");
    let j = s.jake.as_mut().unwrap();
    j.body.x = x;
    j.body.y = mb.y + mb.h - jb.h - jb.y;
    j.set_layer(1);
    s.set_jake_layer(1);
    let d0 = dist(center(ais(&s)[1].1), center(s.jake.as_ref().unwrap()));
    let mut best = d0;
    for t in 0..200 {
        s.tick(0);
        check_ais(&s, "H_J_006", t);
        let h = center(s.jake.as_ref().unwrap());
        best = best.min(dist(center(ais(&s)[1].1), h));
    }
    assert_eq!(ais(&s)[1].0, k);
    assert!(best + 100 < d0, "o Minotauro se aproximou: {d0} → {best}");
}

/// Entrada aleatória em todas as fases jogáveis com personagens da IA: sem pânico, a IA
/// nunca dentro do cenário da camada dela nem fora da fase, entidades limitadas.
#[test]
fn fuzz_random_input_on_all_levels_with_ai_characters() {
    let Some(gd) = data() else { return };
    let mut seed: u32 = 0x4149_4149;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let names: Vec<String> = gd
        .levels()
        .unwrap()
        .names()
        .into_iter()
        .filter(|n| n.to_ascii_uppercase().ends_with(".LEV"))
        .collect();
    let mut played = 0;
    for name in names {
        let l = level(&gd, &name);
        if !has_controllable_player(&l) {
            continue;
        }
        let mut a = Assets::new(gd.resources().unwrap());
        let mut s = Scene::build(&l, &mut a).unwrap();
        if ais(&s).is_empty() {
            continue;
        }
        played += 1;
        let n0 = s.entities.len();
        let mut input = 0;
        for t in 0..2000 {
            if t % 7 == 0 {
                input = next() & (UP | DOWN | LEFT | RIGHT | SHOOT | JUMP | 0x400 | 0x40000);
            }
            s.tick(input);
            check_ais(&s, &name, t);
            let alive = s.entities.iter().filter(|e| e.alive).count();
            assert!(
                alive < n0 + 400,
                "{name}: {alive} entidades vivas no tique {t}"
            );
        }
        assert!(s.entities.len() < n0 + 8000, "{name}: {}", s.entities.len());
        eprintln!(
            "DBG {name}: {:?}",
            ais(&s)
                .iter()
                .map(|(_, j)| (j.state(), j.health.hp))
                .collect::<Vec<_>>()
        );
    }
    assert!(played >= 25, "fases com IA: {played}");
}
