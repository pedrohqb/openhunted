// SPDX-License-Identifier: GPL-3.0-or-later
//! Cena montada a partir dos dados reais (OPENHUNTED_DATA).

use openhunted_engine::{Assets, Layer, Scene};
use openhunted_formats::{Level, open_data};

pub fn load(level: &str) -> Option<(Scene, Assets)> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Some((Scene::build(&l, &mut a).unwrap(), a))
}

#[test]
fn tutorial_scene_has_all_cells_and_objects() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    assert_eq!((s.width, s.height), (14 * 128, 128));
    assert_eq!(
        s.sprites
            .iter()
            .filter(|p| p.layer == Layer::Scenery)
            .count(),
        14
    );
    assert_eq!(
        s.sprites
            .iter()
            .filter(|p| p.layer == Layer::Object)
            .count(),
        11
    );
    assert_eq!(s.markers.len(), 4);
    assert!(s.warnings.is_empty(), "{:?}", s.warnings);
    // Decorações: o primeiro campo é o quadro da animação (DECALW1.ANM, quadros 41, 187, 185).
    let decos: Vec<usize> = s
        .sprites
        .iter()
        .filter(|p| p.layer == Layer::Decoration)
        .map(|p| p.player.frame())
        .collect();
    assert_eq!(decos, vec![41, 187, 185]);
}
