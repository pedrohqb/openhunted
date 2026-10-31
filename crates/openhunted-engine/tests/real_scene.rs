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

/// Nenhum objeto é ampliado mais de 2×: o quadro de referência da escala é o do rótulo do
/// tipo do objeto (ou, numa folha, o de proporção igual ao alvo), não o quadro 0.
#[test]
fn object_scale_reference_matches_object_kind() {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        return;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let la = gd.levels().unwrap();
    let missions =
        openhunted_formats::parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    let mut bad = Vec::new();
    for m in &missions {
        let l = Level::parse(&la.get(&m.file).unwrap()).unwrap();
        let s = Scene::build(&l, &mut a).unwrap();
        for sp in s.sprites.iter().filter(|p| p.layer == Layer::Object) {
            let (num, den) = sp.scale;
            if num > 2 * den {
                bad.push(format!("{} {}x{} {:?}", m.file, sp.x, sp.y, sp.scale));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} objetos ampliados demais: {:?}",
        bad.len(),
        &bad[..bad.len().min(10)]
    );
}

/// O Jake controlável é o início do jogador no modo solo (na D_J_009 o primeiro objeto de
/// tipo 1 é outro); numa fase de vários jogadores, sem início solo, é o primeiro tipo 1.
#[test]
fn player_start_is_jake() {
    let Some((s, _)) = load("D_J_009.LEV") else {
        return;
    };
    let j = s.jake.as_ref().unwrap();
    assert_eq!((j.body.x, j.body.y), (573, 581));
    let (s, _) = load("C_J_006.LEV").unwrap();
    let j = s.jake.as_ref().unwrap();
    assert_eq!((j.body.x, j.body.y), (110, 709));
}

/// Na T_J_007 o Jake nasce sobre peças OPL (objetos): parado, fica de pé.
#[test]
fn jake_stands_on_objects_in_t_j_007() {
    let Some((mut s, _)) = load("T_J_007.LEV") else {
        return;
    };
    let y0 = s.jake.as_ref().unwrap().body.y;
    for _ in 0..40 {
        s.tick(0);
    }
    let j = s.jake.as_ref().unwrap();
    assert!(j.body.y - y0 <= 16, "caiu {} px", j.body.y - y0);
    assert!(j.body.on_ground);
}

/// Todas as fases do `ENGLEVEL.TXT`.
fn mission_files() -> Vec<String> {
    let p = std::env::var_os("OPENHUNTED_DATA").unwrap();
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let la = gd.levels().unwrap();
    openhunted_formats::parse_mission_list(&la.get("ENGLEVEL.TXT").unwrap())
        .unwrap()
        .into_iter()
        .map(|m| m.file)
        .collect()
}

/// Índices dos sólidos que `r` intersecta.
fn overlapped(s: &Scene, r: &openhunted_engine::Rect) -> Vec<usize> {
    let solids = s.world.solids();
    (0..solids.len())
        .filter(|&i| solids[i].overlaps(r))
        .collect()
}

/// Em toda fase com Jake, entradas aleatórias (LCG fixa) nunca o fazem entrar num sólido que
/// ele ainda não intersectava (pode nascer sobreposto, como na I_J_107, e sair).
#[test]
fn random_input_never_enters_solids() {
    use openhunted_engine::input::{JUMP, LEFT, RIGHT};
    if std::env::var_os("OPENHUNTED_DATA").is_none() {
        return;
    }
    for level in mission_files() {
        let (mut s, _) = load(&level).unwrap();
        if s.jake.is_none() {
            continue;
        }
        let mut seed: u32 = 0x1234_5678;
        let mut input = 0;
        let mut inside = overlapped(&s, &s.jake.as_ref().unwrap().body.bounds());
        for t in 0..1000 {
            if t % 7 == 0 {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                input =
                    [0, LEFT, RIGHT, LEFT | JUMP, RIGHT | JUMP, JUMP][(seed >> 16) as usize % 6];
            }
            s.tick(input);
            let now = overlapped(&s, &s.jake.as_ref().unwrap().body.bounds());
            assert!(
                now.iter().all(|i| inside.contains(i)),
                "{level}: entrou num sólido no tique {t}"
            );
            inside = now;
        }
    }
}

/// Em toda fase com Jake, ele responde: depois de assentar, anda para algum dos lados.
#[test]
fn jake_can_move_in_every_level() {
    use openhunted_engine::input::{LEFT, RIGHT};
    if std::env::var_os("OPENHUNTED_DATA").is_none() {
        return;
    }
    for level in mission_files() {
        let (s0, _) = load(&level).unwrap();
        if s0.jake.is_none() {
            continue;
        }
        let moved = [LEFT, RIGHT].iter().any(|&dir| {
            let mut s = s0.clone();
            for _ in 0..40 {
                s.tick(0);
            }
            let x0 = s.jake.as_ref().unwrap().body.x;
            for _ in 0..10 {
                s.tick(dir);
            }
            s.jake.as_ref().unwrap().body.x != x0
        });
        assert!(moved, "{level}: o Jake não sai do lugar");
    }
}
