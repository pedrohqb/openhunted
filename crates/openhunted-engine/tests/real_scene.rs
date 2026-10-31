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
        10 // 8 globais (o Jake fica à parte) + 2 de células
    );
    assert!(s.jake.is_some() && s.jake_sprite.is_some());
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

fn fnv1a(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

/// Render de referência da T_J_001 na câmera (0, 0), conferido visualmente em 2026-10-31.
#[test]
fn tutorial_render_matches_reference() {
    use openhunted_engine::{Framebuffer, SCREEN_H, SCREEN_W, background_index};
    let Some((s, mut a)) = load("T_J_001.LEV") else {
        return;
    };
    let pal = openhunted_formats::Palette::parse(a.archive().get("WORLD.PAL").unwrap()).unwrap();
    let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
    fb.clear(background_index(&pal));
    s.draw(&mut a, &mut fb, (0, 0), false);
    let h = fnv1a(&fb.pixels);
    eprintln!("hash do render: {h:#018x}");
    assert_eq!(h, REFERENCE);
}

const REFERENCE: u64 = 0x88f1_d9d8_c9b1_e7f4;

/// Camadas z ≥ 1 usam o índice 0 como transparência: nada de índice 0 pode chegar à tela.
#[test]
fn multilayer_level_has_no_transparent_holes() {
    use openhunted_engine::{Framebuffer, SCREEN_H, SCREEN_W};
    for lev in ["I_J_101.LEV", "H_J_001.LEV", "C_J_027.LEV"] {
        let Some((s, mut a)) = load(lev) else { return };
        let mut fb = Framebuffer::new(SCREEN_W, SCREEN_H);
        fb.clear(37);
        s.draw(&mut a, &mut fb, (0, 0), false);
        let holes = fb.pixels.iter().filter(|&&p| p == 0).count();
        assert_eq!(holes, 0, "{lev}: {holes} pixels de índice 0");
    }
}

/// Objetos do tipo 6 (OGF.ANM, uma folha de itens num rótulo só) ficam parados.
#[test]
fn generic_item_objects_do_not_cycle() {
    let Some((mut s, _)) = load("H_J_027.LEV") else {
        return;
    };
    let frames = |s: &openhunted_engine::Scene| -> Vec<usize> {
        s.sprites
            .iter()
            .filter(|p| p.anim.frames.len() == 256)
            .map(|p| p.player.frame())
            .collect()
    };
    let before = frames(&s);
    assert!(!before.is_empty());
    for _ in 0..40 {
        s.tick(0);
    }
    assert_eq!(frames(&s), before);
}

/// O Jake da T_J_001 é desenhado a 47/88 (tamanho-alvo 22×47 nos campos do objeto).
#[test]
fn jake_is_drawn_scaled() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let jake = s.jake_sprite.as_ref().unwrap();
    assert_eq!(jake.scale, (47, 88));
}

/// Na T_J_001 o Jake pousa no chão e corre para a esquerda sem atravessar o cenário.
#[test]
fn jake_lands_and_runs_left_in_tutorial() {
    use openhunted_engine::input::LEFT;
    let Some((mut s, _)) = load("T_J_001.LEV") else {
        return;
    };
    let x0 = s.jake.as_ref().unwrap().body.x;
    for _ in 0..20 {
        s.tick(0);
    }
    assert!(s.jake.as_ref().unwrap().body.on_ground, "não pousou");
    for _ in 0..150 {
        s.tick(LEFT);
        let b = s.jake.as_ref().unwrap().body.bounds();
        assert!(b.y + b.h <= 128 && b.x >= 0, "saiu do cenário: {b:?}");
    }
    assert!(s.jake.as_ref().unwrap().body.x < x0 - 400);
}
