// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditoria M17 (`docs/auditoria/inimigos-2.md`): inimigos secundários e personagens da IA
//! sobre os dados reais (OPENHUNTED_DATA), contra medidas do oráculo.

use openhunted_engine::{Assets, Scene};
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

/// Oráculo `D_J_012`: o Jake da fase traz `values[4] = 1` (`+0x270`, veneno); o veneno age
/// no tique 0 (`0x4879c4 % 20 = 0`): vida 99 no fim dele e 98 no fim do tique 20.
#[test]
fn d_j_012_jake_starts_poisoned() {
    let Some(mut s) = load("D_J_012.LEV") else {
        return;
    };
    assert_eq!(s.jake.as_ref().unwrap().poison, 1);
    let mut hp = Vec::new();
    for _ in 0..21 {
        s.tick(0);
        hp.push(s.jake.as_ref().unwrap().health.hp);
    }
    assert_eq!((hp[0], hp[19], hp[20]), (99, 99, 98), "{hp:?}");
}
