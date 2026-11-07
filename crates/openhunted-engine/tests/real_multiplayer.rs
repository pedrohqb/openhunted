// SPDX-License-Identifier: GPL-3.0-or-later
//! Dois jogadores sobre os dados reais (OPENHUNTED_DATA, M15a): toda `M_*` monta nos três
//! pares da disputa e toda `C_*` no cooperativo com os dois jogadores controláveis; uma
//! disputa roteirizada até o limite de pontos; fuzz com entradas aleatórias nos dois.

use openhunted_engine::character::Character;
use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, UP};
use openhunted_engine::physics::CONTACT_SCENERY;
use openhunted_engine::scene::{Duel, Mode};
use openhunted_engine::{Assets, Jake, Outcome, Scene};
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

/// Fases cujo nome começa com `prefix` (`M_` ou `C_`).
fn names(gd: &GameData, prefix: &str) -> Vec<String> {
    let mut v: Vec<String> = gd
        .levels()
        .unwrap()
        .names()
        .into_iter()
        .filter(|n| {
            let u = n.to_ascii_uppercase();
            u.starts_with(prefix) && u.ends_with(".LEV")
        })
        .collect();
    v.sort();
    v
}

const PAIRS: [[Character; 2]; 3] = [
    [Character::Jake, Character::Jake],
    [Character::Jake, Character::Garathe],
    [Character::Garathe, Character::Garathe],
];

fn duel(chars: [Character; 2]) -> Mode {
    Mode::Deathmatch(Duel {
        chars,
        ..Duel::default()
    })
}

/// Jogador `i` em jogo: vivo, ativo, dentro da fase e fora do cenário da camada dele.
fn check_player(s: &Scene, name: &str, i: usize, t: u32) {
    let Some(j) = s.player_jake(i) else {
        panic!("{name}: jogador {} sumiu no tique {t}", i + 1)
    };
    let b = j.body.bounds();
    let c = (b.x + b.w / 2, b.y + b.h / 2);
    assert!(
        c.0 >= 0 && c.1 >= 0 && c.0 <= s.width && c.1 <= s.height,
        "{name}: jogador {} fora da fase no tique {t}: {b:?}",
        i + 1
    );
    if j.is_active() && j.box_matches_state() {
        assert!(
            s.layer_cells(j.layer()).contact(&b) & CONTACT_SCENERY == 0,
            "{name}: jogador {} dentro do cenário no tique {t}: {:?} {b:?}",
            i + 1,
            j.state()
        );
    }
}

fn controllable(j: &Jake) -> bool {
    j.is_active() && !j.is_dead()
}

/// Toda `M_*` monta nos três pares: dois jogadores dos personagens escolhidos, controláveis,
/// com as cores da `SEGOC`; pontos de renascimento entre 2 e 16.
#[test]
fn every_deathmatch_level_builds_in_all_pairings() {
    let Some(gd) = data() else { return };
    let levels = names(&gd, "M_");
    assert_eq!(levels.len(), 20);
    for name in &levels {
        let l = level(&gd, name);
        for chars in PAIRS {
            let mut a = Assets::new(gd.resources().unwrap());
            let mut s = Scene::build_with(&l, &mut a, duel(chars))
                .unwrap_or_else(|e| panic!("{name} {chars:?}: {e}"));
            assert_eq!(s.players.len(), 2, "{name}");
            for t in 0..5 {
                s.tick_players([0, 0]);
                for i in 0..2 {
                    check_player(&s, name, i, t);
                }
            }
            for (i, ch) in chars.iter().enumerate() {
                let j = s.player_jake(i).unwrap();
                assert_eq!(j.character(), *ch, "{name}");
                assert!(controllable(j), "{name}: jogador {}", i + 1);
                assert_eq!(j.colour(), [35, 36][i]);
                assert_eq!(s.players[i].number as usize, i + 1);
            }
            assert!(
                (2..=16).contains(&s.spawn_points.len()),
                "{name}: {} pontos",
                s.spawn_points.len()
            );
            assert_eq!(s.time_limit, 0, "{name}: campo 1004 ignorado");
        }
    }
}

/// Toda `C_*` monta no cooperativo com os dois jogadores das vagas: Jake e Garathe nas `C_2`,
/// dois Jakes nas `C_J`, dois Garathes nas `C_M`.
#[test]
fn every_coop_level_builds_with_its_two_players() {
    let Some(gd) = data() else { return };
    let levels = names(&gd, "C_");
    assert_eq!(levels.len(), 15);
    for name in &levels {
        let l = level(&gd, name);
        let mut a = Assets::new(gd.resources().unwrap());
        let mut s =
            Scene::build_with(&l, &mut a, Mode::Coop).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(s.players.len(), 2, "{name}");
        for t in 0..5 {
            s.tick_players([0, 0]);
            for i in 0..2 {
                check_player(&s, name, i, t);
            }
        }
        let mut chars: Vec<Character> = (0..2)
            .map(|i| s.player_jake(i).unwrap().character())
            .collect();
        chars.sort_by_key(|c| c.kind());
        let want = match &name.to_ascii_uppercase()[..3] {
            "C_2" => [Character::Jake, Character::Garathe],
            "C_J" => [Character::Jake, Character::Jake],
            _ => [Character::Garathe, Character::Garathe],
        };
        assert_eq!(chars, want, "{name}");
        for i in 0..2 {
            let j = s.player_jake(i).unwrap();
            assert!(controllable(j), "{name}: jogador {}", i + 1);
            assert_eq!(j.colour(), 0, "{name}: sem cor no coop");
        }
    }
}

/// Disputa roteirizada na `M_J_024` (Jake × Jake), limite de 2 pontos: o jogador 1 é posto ao
/// lado do 2, vira para ele e atira; o 2 fica parado. Cada morte do 2 dá F ao 1; o 2 renasce
/// num ponto de renascimento com vida 100, e a partida acaba com a derrota "2" e o 1 vencedor.
#[test]
fn scripted_deathmatch_reaches_the_points_limit() {
    let Some(gd) = data() else { return };
    let l = level(&gd, "M_J_024.lev");
    let mut a = Assets::new(gd.resources().unwrap());
    let mode = Mode::Deathmatch(Duel {
        chars: [Character::Jake, Character::Jake],
        colours: [37, 38],
        points_limit: 2,
        time_limit_ms: 0,
    });
    let mut s = Scene::build_with(&l, &mut a, mode).unwrap();
    // a pistola para o 1 (as vagas da `M_J_024` só têm os punhos)
    let j1 = s.player_jake_mut(0).unwrap();
    j1.inventory.slots[2] = (1, 99);
    j1.inventory.weapon = 2;
    for _ in 0..30 {
        s.tick_players([0, 0]);
    }
    s.tick_players([LEFT, 0]);
    s.tick_players([0, 0]);
    let mut deaths = 0;
    let mut was_dead = false;
    for t in 0..3000u32 {
        if s.outcome.is_some() {
            break;
        }
        // a cada tique, o 1 de pé à direita do 2 (mesma camada e chão), virado para ele
        let (x2, y2, l2, dead2) = {
            let j2 = s.player_jake(1).unwrap();
            (j2.body.x, j2.body.y, j2.layer(), j2.is_dead())
        };
        if dead2 && !was_dead {
            deaths += 1;
        }
        was_dead = dead2;
        if !dead2 {
            s.place_player(0, (x2 + 60, y2), l2);
        }
        // munição à vontade
        if let Some(j1) = s.player_jake_mut(0) {
            j1.inventory.slots[2] = (1, 99);
        }
        s.tick_players([SHOOT, 0]);
        if !dead2 {
            check_player(&s, "M_J_024", 1, t);
        }
    }
    assert_eq!(s.outcome, Some(Outcome::Defeat), "fim pelo limite: \"2\"");
    assert!(deaths >= 2, "{deaths} mortes do 2");
    assert_eq!(s.players[0].stats.frags, 2);
    assert_eq!(s.players[1].stats.frags, 0);
    assert_eq!(s.winner(), Some(0));
    let j2 = s.player_jake(1).unwrap();
    assert!(j2.health.hp > 0);
    assert_eq!(j2.colour(), 38);
    let corpses = s
        .entities
        .iter()
        .filter(|e| matches!(e.behaviour, openhunted_engine::Behaviour::Corpse(38)))
        .count();
    assert!(corpses >= 2, "{corpses} cadáveres do 2");
}

/// Entradas aleatórias nos dois jogadores em todas as `M_*` (pares sorteados) e `C_*`, 3000
/// tiques: sem pânico, jogadores fora do cenário e dentro da fase, entidades limitadas.
#[test]
#[ignore = "fuzz longo: rode com check.sh --full"]
fn fuzz_two_players_on_all_multiplayer_levels() {
    let Some(gd) = data() else { return };
    let mut seed: u32 = 0x4d31_3561;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let mask = UP | DOWN | LEFT | RIGHT | SHOOT | JUMP | 0x400 | 0x40000;
    let mut levels: Vec<(String, Mode)> = Vec::new();
    for (k, n) in names(&gd, "M_").into_iter().enumerate() {
        levels.push((n, duel(PAIRS[k % 3])));
    }
    for n in names(&gd, "C_") {
        levels.push((n, Mode::Coop));
    }
    for (name, mode) in levels {
        let l = level(&gd, &name);
        let mut a = Assets::new(gd.resources().unwrap());
        let mut s = Scene::build_with(&l, &mut a, mode).unwrap();
        let n0 = s.entities.len();
        let mut inputs = [0, 0];
        for t in 0..3000u32 {
            if t % 7 == 0 {
                inputs = [next() & mask, next() & mask];
            }
            s.elapsed_ms = u64::from(t) * 50;
            s.tick_players(inputs);
            s.events.clear();
            for i in 0..2 {
                check_player(&s, &name, i, t);
            }
            let alive = s.entities.iter().filter(|e| e.alive).count();
            assert!(
                alive < n0 + 400,
                "{name}: {alive} entidades vivas no tique {t}"
            );
        }
        assert!(s.entities.len() < n0 + 8000, "{name}: {}", s.entities.len());
    }
}
