// SPDX-License-Identifier: GPL-3.0-or-later
//! Cores dos sprites sobre os dados reais (OPENHUNTED_DATA; `docs/re/cores-sprites.md`):
//! os 7 tipos com rotina de desenho saem pela tabela do `DLS.TBL`, nunca com os índices
//! crus da arte.

use openhunted_engine::behaviour::chameleon::FADE_TICKS;
use openhunted_engine::render::Framebuffer;
use openhunted_engine::{Assets, Behaviour, Scene, tint};
use openhunted_formats::{Level, open_data};

fn load(level: &str) -> Option<(Scene, Assets)> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    let s = Scene::build(&l, &mut a).unwrap();
    Some((s, a))
}

/// Desenha só a entidade `e` (sem cenário nem jogador) com a câmera sobre ela.
fn draw_alone(s: &Scene, a: &mut Assets, e: &openhunted_engine::Entity) -> Framebuffer {
    let mut only = s.clone();
    // sem a luz por célula: a cor da rotina, isolada (o objeto pode estar numa sala apagada)
    only.lights = Default::default();
    only.sprites.clear();
    only.jake_sprite = None;
    only.entities = vec![e.clone()];
    let mut fb = Framebuffer::new(320, 240);
    only.draw(a, &mut fb, (e.x - 120, e.y - 100), false);
    fb
}

/// Confere, pixel a pixel, que o desenho com cor é o desenho cru passado pela tabela (0
/// remapeado transparente) e que a arte usa a metade alta; devolve os pixels opacos.
fn assert_tinted(s: &Scene, a: &mut Assets, i: usize, what: &str) -> usize {
    let e = &s.entities[i];
    let t = tint::entity(e, 0, s.tick_count).unwrap_or_else(|| panic!("{what}: sem cor"));
    let map = a.dls().expect("DLS.TBL").table(t, 0);
    let tinted = draw_alone(s, a, e);
    let mut raw_e = e.clone();
    raw_e.behaviour = Behaviour::Static;
    let raw = draw_alone(s, a, &raw_e);
    let mut high = 0;
    for (k, (&p, &q)) in raw.pixels.iter().zip(&tinted.pixels).enumerate() {
        let want = if p == 0 { 0 } else { map[usize::from(p)] };
        assert_eq!(q, want, "{what}: pixel {k} (cru {p})");
        high += usize::from(p >= 128);
    }
    assert!(high > 0, "{what}: a arte deveria usar a metade alta");
    tinted.pixels.iter().filter(|&&p| p != 0).count()
}

fn find(s: &Scene, kind: u32) -> usize {
    s.entities
        .iter()
        .position(|e| e.kind == kind && e.visible() && e.sprite.is_some())
        .unwrap_or_else(|| panic!("tipo {kind} ausente"))
}

#[test]
fn enemies_with_draw_routine_use_their_table() {
    let Some((mut s, mut a)) = load("H_J_027.LEV") else {
        return;
    };
    for _ in 0..7 {
        s.tick(0);
    }
    for (kind, what) in [
        (4, "Camaleão"),
        (105, "Anticorpo"),
        (8, "robô"),
        (103, "gerador"),
        (102, "mina"),
    ] {
        let i = find(&s, kind);
        assert!(
            assert_tinted(&s, &mut a, i, what) > 0,
            "{what}: nada desenhado"
        );
    }
}

#[test]
fn invisible_chameleon_keeps_only_low_source_indices() {
    let Some((s, mut a)) = load("H_J_027.LEV") else {
        return;
    };
    let i = find(&s, 4);
    let visible = assert_tinted(&s, &mut a, i, "Camaleão");
    let mut s2 = s.clone();
    let Behaviour::Chameleon(c) = &mut s2.entities[i].behaviour else {
        panic!()
    };
    c.invisible = true;
    c.fade = FADE_TICKS;
    let hidden = assert_tinted(&s2, &mut a, i, "Camaleão invisível");
    assert!(hidden < visible / 2, "{hidden} de {visible} pixels");
    let mut raw_e = s2.entities[i].clone();
    raw_e.behaviour = Behaviour::Static;
    let raw = draw_alone(&s2, &mut a, &raw_e);
    let tinted = draw_alone(&s2, &mut a, &s2.entities[i]);
    for (&p, &q) in raw.pixels.iter().zip(&tinted.pixels) {
        if q != 0 {
            assert!(p < 0x91, "sobrou o índice {p}");
        }
    }
}

/// Jake e Garathe: o desenho da cena passa o sprite do jogador pela tabela 35/36.
fn player_tinted(level: &str, table: i32) {
    let Some((mut s, mut a)) = load(level) else {
        return;
    };
    s.tick(0);
    let j = s.jake.as_ref().unwrap();
    assert_eq!(j.tint().table, table);
    let sp = s.jake_sprite.clone().unwrap();
    let camera = (sp.x - 120, sp.y - 100);
    let mut only = s.clone();
    // sem a luz por célula: a cor da rotina, isolada (o objeto pode estar numa sala apagada)
    only.lights = Default::default();
    only.sprites.clear();
    only.entities.clear();
    let mut tinted = Framebuffer::new(320, 240);
    only.draw(&mut a, &mut tinted, camera, false);
    // cru: o mesmo sprite como decoração (sem rotina de desenho)
    let mut raw_scene = only.clone();
    raw_scene.jake_sprite = None;
    raw_scene.sprites = vec![sp];
    let mut raw = Framebuffer::new(320, 240);
    raw_scene.draw(&mut a, &mut raw, camera, false);
    let map = a.dls().unwrap().table(tint::Tint::new(table, 0), 0);
    let mut high = 0;
    for (&p, &q) in raw.pixels.iter().zip(&tinted.pixels) {
        assert_eq!(q, if p == 0 { 0 } else { map[usize::from(p)] });
        high += usize::from(p >= 128);
    }
    assert!(high > 0);
}

#[test]
fn jake_uses_table_35() {
    player_tinted("T_J_001.LEV", 35);
}

#[test]
fn garathe_uses_table_36() {
    player_tinted("D_M_030.LEV", 36);
}

#[test]
fn ramps_of_player_tables_differ_from_raw_indices() {
    let Some((_, mut a)) = load("T_J_001.LEV") else {
        return;
    };
    let d = a.dls().unwrap();
    for n in [35, 36] {
        let t = d.table(tint::Tint::new(n, 0), 0);
        let same = (128..256).filter(|&i| usize::from(t[i]) == i).count();
        assert!(same < 8, "tabela {n}: {same} índices crus na rampa");
    }
}
