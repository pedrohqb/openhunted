// SPDX-License-Identifier: GPL-3.0-or-later
//! Itens, armas e portas de célula da campanha do Jake sobre os dados reais
//! (OPENHUNTED_DATA; `docs/re/itens-campanha.md`, `armas-2.md`).

use openhunted_engine::entity::PICKUPS;
use openhunted_engine::input::{LEFT, RIGHT};
use openhunted_engine::jake::{ROCKETS, SHOTGUN};
use openhunted_engine::objects::{SOLO, present};
use openhunted_engine::scene::has_controllable_jake;
use openhunted_engine::{Assets, Behaviour, Scene};
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

fn load(level_name: &str) -> Option<Scene> {
    let gd = data()?;
    let l = level(&gd, level_name);
    let mut a = Assets::new(gd.resources().unwrap());
    Some(Scene::build(&l, &mut a).unwrap())
}

/// Leva o Jake até (x, y) na camada e deixa assentar.
fn place(s: &mut Scene, x: i32, y: i32, layer: i32, ticks: u32) {
    if let Some(j) = s.jake.as_mut() {
        j.set_layer(layer);
        j.body.x = x;
        j.body.y = y;
        j.body.vx = 0;
        j.body.vy = 0;
    }
    s.set_jake_layer(layer);
    for _ in 0..ticks {
        s.tick(0);
    }
}

/// Nomes das fases solo do Jake (`D_J`, `H_J`, `I_J`, `T_J`).
fn solo_levels(gd: &GameData) -> Vec<String> {
    let mut v: Vec<String> = gd
        .levels()
        .unwrap()
        .names()
        .into_iter()
        .filter(|n| {
            let u = n.to_uppercase();
            u.ends_with(".LEV")
                && ["D_J_", "H_J_", "I_J_", "T_J_"]
                    .iter()
                    .any(|p| u.starts_with(p))
        })
        .collect();
    v.sort();
    v
}

/// Os itens das células entram na cena: os coletáveis da cena são os da fase (lista global
/// e `cells[i].objects`) presentes no modo solo.
#[test]
fn campaign_items_inside_cells_are_loaded() {
    let Some(gd) = data() else { return };
    let mut total = 0;
    for name in solo_levels(&gd) {
        let l = level(&gd, &name);
        if !has_controllable_jake(&l) {
            continue;
        }
        let want = l
            .objects
            .iter()
            .chain(l.cells.iter().flatten().flat_map(|c| c.objects.iter()))
            .filter(|o| PICKUPS.contains(&o.kind) && present(o, SOLO))
            .filter(|o| o.kind != 26 || o.fields.get(5).copied().unwrap_or(0) == 1)
            .count();
        let mut a = Assets::new(gd.resources().unwrap());
        let s = Scene::build(&l, &mut a).unwrap();
        let got = s
            .entities
            .iter()
            .filter(|e| matches!(e.behaviour, Behaviour::Pickup(_)))
            .count();
        assert_eq!(got, want, "{name}");
        total += got;
    }
    assert!(total > 1000, "itens na campanha: {total}");
}

/// H_J_001: a espingarda (13) dá a arma, munição e passa a ser a corrente.
#[test]
fn h_j_001_shotgun_item_gives_ammo() {
    let Some(mut s) = load("H_J_001.LEV") else {
        return;
    };
    let (x, y, layer, v) = s
        .entities
        .iter()
        .find(|e| e.kind == 13 && e.alive)
        .map(|e| (e.x, e.y, e.layer, e.values.first().copied().unwrap_or(0)))
        .unwrap();
    let inv = &s.jake.as_ref().unwrap().inventory;
    let before = inv.slots[SHOTGUN];
    // só os foguetes ficam acima da espingarda na lista (1, 6, 5, 2, 3, 4)
    let want = if inv.available(ROCKETS) {
        ROCKETS
    } else {
        SHOTGUN
    };
    place(&mut s, x - 4, y - 30, layer, 4);
    let j = s.jake.as_ref().unwrap();
    assert_eq!(j.inventory.slots[SHOTGUN].0, 1);
    assert_eq!(j.inventory.slots[SHOTGUN].1, (before.1 + v.max(1)).min(99));
    assert_eq!(j.inventory.weapon, want);
}

/// Empurra a porta `i` dos dois lados da coluna (`strip`) por 40 tiques cada; devolve se o
/// Jake ouviu a fala da porta trancada sem a chave (3715 e 3714, em ordem).
fn push_door(s: &mut Scene, i: usize) -> bool {
    let (oy, layer) = (s.cell_doors[i].origin.1, s.cell_doors[i].layer);
    let col = s.cell_doors[i].strip();
    let mut heard = false;
    for (x, side) in [(col.x - 30, RIGHT), (col.x + col.w + 10, LEFT)] {
        place(s, x, oy + 40, layer, 15);
        for _ in 0..40 {
            s.tick(side);
            let ids: Vec<_> = s.events.drain(..).filter_map(|e| e.sound_id()).collect();
            heard |= ids.windows(2).any(|w| w == [3715, 3714]);
        }
        if s.cell_doors[i].state != 1 {
            break;
        }
    }
    heard
}

/// Porta com chave: sem ela o Jake empurra e ouve a fala; com ela (par 13 + bit), a porta
/// abre e o Jake passa pela coluna.
fn keyed_door_opens(s: &mut Scene, i: usize) {
    assert!(!s.cell_doors[i].closed.is_empty() && !s.cell_doors[i].open.is_empty());
    assert!(push_door(s, i), "fala da porta trancada");
    assert_eq!(s.cell_doors[i].state, 1);
    let bit = s.cell_doors[i].keys.trailing_zeros() as usize;
    s.jake.as_mut().unwrap().inventory.slots[13 + bit] = (1, 1);
    push_door(s, i);
    assert_ne!(s.cell_doors[i].state, 1, "a chave abriu");
    let col = s.cell_doors[i].strip();
    let x = s.jake.as_ref().unwrap().body.bounds().x;
    assert!(
        x > col.x + col.w || x + 18 < col.x,
        "passou pela porta: {x}"
    );
}

/// D_J_009: a porta de célula (9, 2, 3) pede a chave 63.
#[test]
fn d_j_009_cell_door_opens_with_its_key() {
    let Some(mut s) = load("D_J_009.LEV") else {
        return;
    };
    assert_eq!(s.cell_doors.len(), 13);
    let i = s
        .cell_doors
        .iter()
        .position(|d| d.origin == (9 * 128, 2 * 128) && d.layer == 3)
        .unwrap();
    assert_eq!(s.cell_doors[i].keys, 2);
    keyed_door_opens(&mut s, i);
}

/// H_J_018: uma porta com chave abre com ela.
#[test]
fn h_j_018_cell_door_opens_with_its_key() {
    let Some(mut s) = load("H_J_018.LEV") else {
        return;
    };
    let i = s
        .cell_doors
        .iter()
        .position(|d| d.keys != 0 && d.state == 1)
        .expect("porta com chave");
    keyed_door_opens(&mut s, i);
}

/// D_J_009: aberta, a porta fecha sozinha depois de `+0x4c` tiques e da animação, mas não
/// com o Jake parado na coluna.
#[test]
fn d_j_009_open_door_closes_by_itself_unless_jake_is_in_it() {
    let Some(mut s) = load("D_J_009.LEV") else {
        return;
    };
    let i = s
        .cell_doors
        .iter()
        .position(|d| d.origin == (9 * 128, 2 * 128) && d.layer == 3)
        .unwrap();
    s.jake.as_mut().unwrap().inventory.slots[14] = (1, 1);
    push_door(&mut s, i);
    let d = &s.cell_doors[i];
    let (col, layer, wait, oy) = (d.strip(), d.layer, d.close_after, d.origin.1);
    let anim = d.last - d.first;
    // o Jake parado no meio da coluna aberta segura a porta
    place(&mut s, col.x + col.w / 2 - 11, oy + 40, layer, 0);
    for _ in 0..wait as usize + anim + 20 {
        s.jake.as_mut().unwrap().health.hp = 100; // a fase fere o Jake parado ali
        s.tick(0);
    }
    assert_eq!(s.cell_doors[i].state, 4, "não fecha sobre o Jake");
    // longe dela, fecha
    place(&mut s, col.x - 80, oy + 40, layer, 0);
    for _ in 0..wait as usize + 2 * anim + 20 {
        s.tick(0);
    }
    assert_eq!(s.cell_doors[i].state, 1, "fechou sozinha");
}

/// D_J_007: as três bandeiras ligam o objetivo "acionar" quando apanhadas.
#[test]
fn d_j_007_flags_turn_on_when_collected() {
    let Some(mut s) = load("D_J_007.LEV") else {
        return;
    };
    let flags: Vec<(i32, i32, i32)> = s
        .entities
        .iter()
        .filter(|e| (80..=85).contains(&e.kind))
        .map(|e| (e.x, e.y, e.layer))
        .collect();
    assert_eq!(flags.len(), 3);
    assert!(
        s.entities
            .iter()
            .filter(|e| (80..=85).contains(&e.kind))
            .all(|e| e.objective & 2 != 0 && !e.on)
    );
    for (x, y, layer) in flags {
        place(&mut s, x - 4, y - 30, layer, 3);
    }
    assert!(
        s.entities
            .iter()
            .filter(|e| (80..=85).contains(&e.kind))
            .all(|e| e.on && !e.alive)
    );
    let inv = &s.jake.as_ref().unwrap().inventory;
    assert_eq!((inv.slots[7].0, inv.slots[8].0, inv.slots[9].0), (1, 1, 1));
}

/// Entrada aleatória (todas as teclas, troca de arma e escolha direta, inclusive o SELECT = 7
/// que não é arma) por 1000 tiques em todas as fases solo, com três sementes; a vida é
/// reposta a cada tique para o Jake seguir jogando. Invariantes: munição ≥ 0, arma em 1–6,
/// nenhuma porta abrindo/fechando além da animação, o Jake não fica dentro de um sólido.
#[test]
fn fuzz_random_input_on_all_solo_levels() {
    // estados `+0x54` da porta: 2 fechando, 3 abrindo
    const CLOSING: u8 = 2;
    const OPENING: u8 = 3;
    let Some(gd) = data() else { return };
    for start in [0x1234_5678u32, 0x9e37_79b9, 0x0bad_f00d] {
        let mut seed = start;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed
        };
        for name in solo_levels(&gd) {
            let l = level(&gd, &name);
            if !has_controllable_jake(&l) {
                continue;
            }
            let mut a = Assets::new(gd.resources().unwrap());
            let mut s = Scene::build(&l, &mut a).unwrap();
            if let Some(j) = s.jake.as_mut() {
                // o arsenal inteiro, para exercitar todos os projéteis
                for w in 2..=6 {
                    j.inventory.slots[w] = (1, 99);
                }
            }
            let mut moving = vec![0usize; s.cell_doors.len()];
            let mut stuck = 0;
            let mut input = 0;
            for t in 0..1000 {
                if t % 4 == 0 {
                    let r = next();
                    input = r & 0x4_1f0f;
                    if r & 0x0700_0000 != 0 {
                        input |= ((r >> 24) % 8) << 24; // 0–7: o 7 não é arma
                    }
                }
                if let Some(j) = s.jake.as_mut() {
                    j.health.hp = j.health.hp.max(100);
                }
                s.tick(input);
                s.events.clear();
                let ctx = format!("{name}, semente {start:#x}, tique {t}");
                if let Some(j) = s.jake.as_ref() {
                    let inv = &j.inventory;
                    assert!((1..=6).contains(&inv.weapon), "arma {}: {ctx}", inv.weapon);
                    assert!(
                        inv.slots[1..=6].iter().all(|w| w.1 >= 0),
                        "munição negativa {:?}: {ctx}",
                        &inv.slots[1..=6]
                    );
                    // só as células: nascer sobreposto a um objeto sólido é dado real
                    // (I_J_107) e a física deixa sair (`blocked_from`)
                    if j.is_active() && s.cell_world().blocked(&j.body.bounds()) {
                        stuck += 1;
                        assert!(stuck <= 60, "Jake dentro de um sólido: {ctx}");
                    } else {
                        stuck = 0;
                    }
                }
                for (d, n) in s.cell_doors.iter().zip(moving.iter_mut()) {
                    if matches!(d.state, OPENING | CLOSING) {
                        *n += 1;
                        assert!(
                            *n <= d.last - d.first + 2,
                            "porta em {:?} presa no estado {}: {ctx}",
                            d.origin,
                            d.state
                        );
                    } else {
                        *n = 0;
                    }
                }
            }
        }
    }
}

/// D_J_012: o pistão anda com o gatilho (o alvo 3 não existe na fase: vale verdadeiro,
/// `0x435bd8`), mas não com o alvo desligado; esmaga o Jake na frente dele.
#[test]
fn d_j_012_piston_moves_on_its_trigger_and_crushes() {
    let Some(mut s) = load("D_J_012.LEV") else {
        return;
    };
    let pi = s.entities.iter().position(|e| e.kind == 39).unwrap();
    let target = s.entities[pi].triggers[0].0;
    let frame = |s: &Scene| match &s.entities[pi].behaviour {
        Behaviour::Piston(p) => p.frame,
        _ => unreachable!(),
    };
    assert!(s.entities.iter().all(|e| e.id != target));
    // um alvo desligado com esse id segura o pistão
    let mut off = s.entities[pi].clone();
    off.kind = 0;
    off.id = target;
    off.on = false;
    off.behaviour = Behaviour::Static;
    s.entities.push(off);
    let parked = s.entities.len() - 1;
    if let Behaviour::Piston(p) = &mut s.entities[pi].behaviour {
        *p = Default::default();
    }
    for _ in 0..10 {
        s.tick(0);
    }
    assert_eq!(frame(&s), 0, "parado com o alvo desligado");
    s.entities[parked].on = true;
    let r = s.entities[pi].sprite_rect().unwrap();
    let layer = s.entities[pi].layer;
    let hp = s.jake.as_ref().unwrap().health.hp;
    place(&mut s, r.x + r.w / 2, r.y, layer, 0);
    let mut moved = 0;
    for _ in 0..12 {
        s.tick(0);
        moved = moved.max(frame(&s));
    }
    assert!(moved >= 5, "o pistão andou: {moved}");
    assert!(s.jake.as_ref().unwrap().health.hp < hp, "esmagou o Jake");
}

/// D_J_001: CIMA diante de um interruptor (zona PR 0x40) o inverte.
#[test]
fn d_j_001_up_flips_a_switch() {
    let Some(mut s) = load("D_J_001.LEV") else {
        return;
    };
    let zones: Vec<_> = s
        .prop_zones
        .iter()
        .filter(|z| z.kind & 0x40 != 0)
        .copied()
        .collect();
    let si = s
        .entities
        .iter()
        .position(|e| {
            e.kind == 74
                && e.bounds().is_some_and(|b| {
                    zones
                        .iter()
                        .any(|z| z.layer == e.layer && z.rect.overlaps(&b))
                })
        })
        .expect("interruptor sobre zona 0x40");
    let b = s.entities[si].bounds().unwrap();
    let z = zones
        .iter()
        .find(|z| z.layer == s.entities[si].layer && z.rect.overlaps(&b))
        .copied()
        .unwrap();
    let was = s.entities[si].on;
    place(
        &mut s,
        z.rect.x + z.rect.w / 2 - 11,
        z.rect.y - 10,
        z.layer,
        15,
    );
    s.tick(openhunted_engine::input::UP);
    assert_ne!(s.entities[si].on, was);
}
