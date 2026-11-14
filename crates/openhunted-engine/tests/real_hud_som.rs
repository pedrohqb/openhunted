// SPDX-License-Identifier: GPL-3.0-or-later
//! Auditoria M17 (HUD, som, colisão do cenário e câmera) com os dados reais (OPENHUNTED_DATA).

use openhunted_engine::{Assets, Layer, Scene};
use openhunted_formats::{Level, open_data};

fn load(level: &str) -> Option<(Scene, Assets)> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Some((Scene::build(&l, &mut a).unwrap(), a))
}

/// Decalques espelham com a célula (`0x4305ac`: retângulo espelhado por `0x405775` e desenho
/// com `+0x40` da célula XOR `+0x30` do decalque). Na `T_J_001` os três estão em células
/// espelhadas: a seta sai "<<<" no original (oráculo, t 15).
#[test]
fn decals_mirror_with_their_cell() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let decals: Vec<_> = s
        .sprites
        .iter()
        .filter(|p| p.layer == Layer::Decoration)
        .collect();
    assert_eq!(decals.len(), 3);
    assert!(decals.iter().all(|d| d.mirror), "espelhados com a célula");
    // a seta (quadro 41, 72×72 na célula 5, em (88, 30)): x' = 128 − 88 − largura de referência
    let arrow = decals[0];
    let rw = i32::from(arrow.anim.frames[0].width);
    assert_eq!(arrow.x, 5 * 128 + 128 - 88 - rw);
    assert_eq!(arrow.y, 30);
}

/// Alvo da câmera (`0x436386`, oráculo `T_J_001`): o centro da vista fica em `(x + 11, y)`
/// do Jake (retângulo de referência 22×47 de `JAKE.ANM`, topo), no mesmo tique, e não é preso
/// à fase (a vista mostra o vazio além da última célula).
#[test]
fn camera_target_is_the_reference_rect_top_centre() {
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let mut center = openhunted_engine::camera::target(&s, 0).unwrap();
    let mut seen = Vec::new();
    for _ in 0..5 {
        s.tick(0);
        center = openhunted_engine::camera::chase(
            center,
            openhunted_engine::camera::target(&s, 0).unwrap(),
        );
        let j = s.jake.as_ref().unwrap();
        assert_eq!(center, (j.body.x + 11, j.body.y));
        seen.push(center);
    }
    // oráculo: (1707, 48), (1707, 52), (1707, 58), (1707, 66), (1707, 69)
    assert_eq!(seen.last(), Some(&(1707, 69)));
    assert!(center.0 + 320 > s.width, "sem limite na borda da fase");
}

/// Passos pelo `SOUND.TBL` (lista 1, `JAKE.ANM`): correndo, 5101 e 5100 alternados a cada 6
/// tiques (quadros 40 e 37 do ciclo), prioridade 3, no centro do Jake; a pistola continua só
/// pela tabela das armas.
#[test]
fn running_jake_plays_footsteps_from_the_sound_table() {
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let mut steps = Vec::new();
    for t in 0..40 {
        s.tick(if t < 20 {
            0
        } else {
            openhunted_engine::input::LEFT
        });
        for e in s.events.drain(..) {
            if let openhunted_engine::Event::Sound(c) = e
                && (5100..=5101).contains(&c.id)
            {
                assert_eq!(c.priority, 3);
                assert!(c.at.is_some());
                steps.push((t, c.id));
            }
        }
    }
    assert!(steps.len() >= 3, "{steps:?}");
    for w in steps.windows(2) {
        assert_eq!(w[1].0 - w[0].0, 6, "{steps:?}");
        assert_ne!(w[0].1, w[1].1, "alternados: {steps:?}");
    }
}
