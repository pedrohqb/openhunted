// SPDX-License-Identifier: GPL-3.0-or-later
//! Objetos de fase sobre os dados reais (OPENHUNTED_DATA), auditoria M17
//! (`docs/auditoria/itens-objetos.md`).

use openhunted_engine::behaviour::opo::{self, OpoState, Part};
use openhunted_engine::{Assets, Behaviour, Scene};
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

fn steam_states(s: &Scene) -> Vec<i32> {
    s.entities
        .iter()
        .filter_map(|e| match &e.behaviour {
            Behaviour::Opo(OpoState {
                part: Part::Steam(st),
                ..
            }) => Some(st.state),
            _ => None,
        })
        .collect()
}

/// I_J_104: as 10 saliências gotejam (gotas 71 com o `OPO.BRD`, que caem e somem) e os 10
/// vapores (72) ligam e desligam pelos temporizadores.
#[test]
fn i_j_104_ledges_drip_and_steam_cycles() {
    let Some(mut s) = load("I_J_104.LEV") else {
        return;
    };
    let ledges = s.entities.iter().filter(|e| e.kind == 70).count();
    assert!(ledges > 0);
    assert!(
        steam_states(&s).iter().all(|&st| st == 0),
        "estado 0 até o 1.º tique"
    );
    let (mut max_drops, mut seen_states) = (0, std::collections::BTreeSet::new());
    let mut splashed = false;
    for _ in 0..300 {
        s.tick(0);
        let drops: Vec<_> = s
            .entities
            .iter()
            .filter(|e| e.alive && e.kind == opo::DROP)
            .collect();
        max_drops = max_drops.max(drops.len());
        for d in &drops {
            assert_eq!(d.boxes.len(), 1, "OPO.BRD");
            if let Behaviour::Opo(OpoState {
                part: Part::Drop(st),
                ..
            }) = &d.behaviour
            {
                splashed |= st.state == opo::DROP_SPLASH;
            }
        }
        seen_states.extend(steam_states(&s));
    }
    assert!(max_drops >= ledges / 2, "gotas vivas: {max_drops}");
    assert!(splashed, "alguma gota respingou");
    let dead = s
        .entities
        .iter()
        .filter(|e| !e.alive && e.kind == opo::DROP)
        .count();
    assert!(dead > 0, "as gotas somem ao fim do respingo");
    assert_eq!(
        seen_states.into_iter().collect::<Vec<_>>(),
        vec![1, 2, 3, 4],
        "o vapor passa pelos quatro estados"
    );
}
