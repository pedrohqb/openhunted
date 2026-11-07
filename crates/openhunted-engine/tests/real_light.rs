// SPDX-License-Identifier: GPL-3.0-or-later
//! Luz por célula sobre os dados reais (OPENHUNTED_DATA; `docs/re/casca-acabamento.md`): as
//! salas da `H_J_006` que dependem do alvo 17 começam apagadas, somem com os objetos e
//! voltam pelas tabelas de sombra 26…19 do `DLS.TBL`.

use openhunted_engine::render::Framebuffer;
use openhunted_engine::scene::light::{Look, MAX_SHADE};
use openhunted_engine::{Assets, Scene};
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

fn draw(s: &Scene, a: &mut Assets, cam: (i32, i32)) -> Framebuffer {
    let mut fb = Framebuffer::new(256, 256);
    s.draw(a, &mut fb, cam, false);
    fb
}

#[test]
fn h_j_006_dark_rooms_hide_and_fade_in_through_the_shadow_tables() {
    let Some((mut s, mut a)) = load("H_J_006.LEV") else {
        return;
    };
    // só o cenário nas comparações de pixel
    let mut scenery = s.clone();
    scenery.entities.clear();
    scenery.jake_sprite = None;
    assert_eq!(s.lights.cells.len(), 12);
    let c = s.lights.cells[0].clone();
    assert_eq!(c.triggers[0], (17, Some(11)), "o alvo 17 ligado acende");
    let target = s.entities.iter().find(|e| e.id == 17).expect("alvo 17");
    assert!(!target.on, "a sala começa apagada");
    assert_eq!((c.visible, c.shade), (false, MAX_SHADE));
    let cam = c.origin;
    // apagada: o cenário da célula não aparece
    let dark = draw(&s, &mut a, cam);
    let mut lit = s.clone();
    lit.lights.init(&|_| Some(true));
    let full = draw(&lit, &mut a, cam);
    let cell = |fb: &Framebuffer| -> Vec<u8> {
        (0..128)
            .flat_map(|y| fb.pixels[y * 256..y * 256 + 128].to_vec())
            .collect()
    };
    assert_ne!(cell(&dark), cell(&full), "a célula apagada some");
    // um objeto posto na sala apagada não é desenhado
    let mut probe = lit.clone();
    let i = probe
        .entities
        .iter()
        .position(|e| e.visible() && e.sprite.is_some())
        .unwrap();
    probe.entities.truncate(i + 1);
    probe.entities.drain(..i);
    probe.sprites.clear();
    probe.jake_sprite = None;
    let e = &mut probe.entities[0];
    let (dx, dy) = (e.center().0 - e.x, e.center().1 - e.y);
    e.x = c.origin.0 + 64 - dx;
    e.y = c.origin.1 + 64 - dy;
    e.layer = c.layer;
    if let Some(sp) = e.sprite.as_mut() {
        sp.x = e.x;
        sp.y = e.y;
    }
    assert!(draw(&probe, &mut a, cam).pixels.iter().any(|&p| p != 0));
    probe.lights.init(&|_| Some(false));
    assert!(
        draw(&probe, &mut a, cam).pixels.iter().all(|&p| p == 0),
        "objeto na célula invisível"
    );
    // acende: sombra 7 (tabela 25) no primeiro tique, 0 depois de mais 7
    let mut full_scenery = scenery.clone();
    full_scenery.lights.init(&|_| Some(true));
    let full = draw(&full_scenery, &mut a, cam);
    scenery.lights.tick(&|_| Some(true));
    s.lights.tick(&|_| Some(true));
    assert_eq!(s.lights.cells[0].look(), Some(Look::Shaded(25)));
    let shaded = draw(&scenery, &mut a, cam);
    let t = a
        .dls()
        .unwrap()
        .table(openhunted_engine::tint::Tint::new(1, 0), 25);
    for (k, (&p, &q)) in cell(&full).iter().zip(&cell(&shaded)).enumerate() {
        let want = if p == 0 { 0 } else { t[usize::from(p)] };
        assert_eq!(q, want, "pixel {k}");
    }
    for _ in 0..7 {
        s.lights.tick(&|_| Some(true));
        scenery.lights.tick(&|_| Some(true));
    }
    assert_eq!(s.lights.cells[0].look(), None);
    assert_eq!(cell(&draw(&scenery, &mut a, cam)), cell(&full));
}

/// Determinismo: a mesma entrada dá o mesmo rastro (posição do Jake, tiques, mortos).
#[test]
fn same_input_gives_the_same_trace() {
    let Some((s, _)) = load("T_J_001.LEV") else {
        return;
    };
    use openhunted_engine::input::{JUMP, LEFT, RIGHT, SHOOT};
    let script: Vec<u32> = (0..600)
        .map(|t| match t % 90 {
            0..30 => RIGHT,
            30..35 => RIGHT | JUMP,
            35..60 => SHOOT,
            _ => LEFT,
        })
        .collect();
    let run = |mut s: Scene| -> Vec<(i32, i32, i32)> {
        script
            .iter()
            .map(|&i| {
                s.tick(i);
                let j = s.jake.as_ref().unwrap();
                (j.body.x, j.body.y, s.kills)
            })
            .collect()
    };
    let a = run(s.clone());
    let b = run(s);
    assert_eq!(a, b);
    assert!(a.windows(2).any(|w| w[0].0 != w[1].0), "o Jake andou");
}
