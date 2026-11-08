// SPDX-License-Identifier: GPL-3.0-or-later
//! Desenho das camadas com os dados reais (`docs/re/camadas-desenho.md`): na `T_J_007`, com
//! o Jake na camada 1, a camada 0 aparece só pelo buraco da porta (célula (3, 2, 1)),
//! escurecida pela tabela 27 e menor; objetos de outras camadas, só pelo buraco.

use openhunted_engine::scene::view::{View, depth_shadow};
use openhunted_engine::{Assets, Behaviour, Blend, Framebuffer, Rect, Scene, State};
use openhunted_formats::{Level, open_data};

fn load(level: &str) -> Option<(Scene, Assets, Level)> {
    let p = std::env::var_os("OPENHUNTED_DATA")?;
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    let l = Level::parse(&gd.levels().unwrap().get(level).unwrap()).unwrap();
    let mut a = Assets::new(gd.resources().unwrap());
    Some((Scene::build(&l, &mut a).unwrap(), a, l))
}

fn run(s: &mut Scene, n: usize, input: u32) {
    for _ in 0..n {
        s.tick(input);
    }
}

/// Anda até a zona `kind` da camada do Jake.
fn walk_to_zone(s: &mut Scene, kind: u16) {
    use openhunted_engine::input::{LEFT, RIGHT};
    for _ in 0..100 {
        let env = openhunted_engine::jake::Env {
            world: &s.world,
            zones: &s.prop_zones,
            layers: s.layers,
        };
        let j = s.jake.as_ref().unwrap();
        if j.zones_under(&env) & kind != 0 {
            run(s, 1, 0);
            return;
        }
        let b = j.body.bounds();
        let cx = b.x + b.w / 2;
        let target = s
            .prop_zones
            .iter()
            .filter(|z| z.layer == j.layer() && z.kind & kind != 0)
            .map(|z| z.rect.x + z.rect.w / 2)
            .min_by_key(|x| (x - cx).abs())
            .expect("zona");
        run(s, 1, if target < cx { LEFT } else { RIGHT });
    }
    panic!("zona {kind:#x} não alcançada");
}

/// A `T_J_007` com o Jake na porta da camada 0 (zona 2, x ≈ 451), parado; sem minas.
fn at_layer_door() -> Option<(Scene, Assets, Level)> {
    use openhunted_engine::input::{JUMP, LEFT, UP};
    let (mut s, a, l) = load("T_J_007.LEV")?;
    for e in s.entities.iter_mut() {
        if matches!(e.behaviour, Behaviour::Mine(_)) {
            e.alive = false;
        }
    }
    s.rebuild_world();
    let j = s.jake.as_mut().unwrap();
    j.body = openhunted_engine::Body::new(556, 440, j.body.boxr);
    run(&mut s, 28, UP);
    for _ in 0..200 {
        let b = s.jake.as_ref().unwrap().body.bounds();
        if b.y + b.h < 366 {
            break;
        }
        run(&mut s, 1, UP);
    }
    run(&mut s, 1, JUMP | UP | LEFT);
    run(&mut s, 30, 0);
    walk_to_zone(&mut s, openhunted_engine::jake::zone::LAYER_DOWN);
    Some((s, a, l))
}

/// A vista do jogador 1 na tela 640×480 centrada nele (área de jogo 640×405).
fn view(s: &Scene) -> View {
    let (fx, fy) = s.focus().unwrap();
    let mut v = s.view_of(
        0,
        (fx - 320, fy - 202),
        Rect {
            x: 0,
            y: 0,
            w: 640,
            h: 480,
        },
        (640, 405),
    );
    v.views = 1;
    v
}

fn render(s: &Scene, a: &mut Assets, v: &View) -> Framebuffer {
    let mut fb = Framebuffer::new(640, 480);
    s.draw_view(a, &mut fb, v, false);
    fb
}

/// Cenário das células `cells`, só ele, num quadro zerado, com a projeção de `d` e o modo
/// `blend`, na ordem das células.
fn cells_alone(
    s: &Scene,
    a: &mut Assets,
    l: &Level,
    cells: &[(i32, i32, i32)],
    v: &View,
    d: i32,
    blend: Blend,
) -> Framebuffer {
    let (gx, gy) = (i32::from(l.grid_x), i32::from(l.grid_y));
    let mut fb = Framebuffer::new(640, 480);
    // os sprites de cenário estão na ordem das células com animação
    let with_anim = l
        .cells
        .iter()
        .enumerate()
        .filter(|(_, c)| c.as_ref().is_some_and(|c| !c.animation.is_empty()));
    for (k, (i, _)) in with_anim.enumerate() {
        let i = i as i32;
        let cell = (i % gx, (i / gx) % gy, i / (gx * gy));
        if !cells.contains(&cell) {
            continue;
        }
        let sp = &s.sprites[k];
        assert_eq!((sp.x, sp.y), (cell.0 * 128, cell.1 * 128));
        for op in openhunted_engine::draw_ops(&sp.anim, sp.player.frame()) {
            let bmp = a.bitmap(&op.bitmap).unwrap();
            let f = &bmp.frames[op.frame as usize];
            let mut x = op.x;
            if sp.mirror {
                let rw = sp.anim.frames.first().map_or(0, |f| i32::from(f.width));
                x = rw - x - i32::from(f.width);
            }
            let world = Rect {
                x: sp.x + x,
                y: sp.y + op.y,
                w: i32::from(f.width),
                h: i32::from(f.height),
            };
            // amostrado pela escala da camada (`reference / d`)
            let r = (i64::from(v.reference()), i64::from(d));
            fb.blit_ratio(
                f,
                v.project_rect(world, d),
                (r, r),
                sp.mirror,
                None,
                blend,
                v.screen,
            );
        }
    }
    fb
}

#[test]
fn back_layer_shows_through_the_hole_through_shadow_27() {
    use openhunted_engine::input::DOWN;
    let Some((mut s, mut a, l)) = at_layer_door() else {
        return;
    };
    run(&mut s, 8, DOWN);
    run(&mut s, 10, 0);
    assert_eq!((s.jake.as_ref().unwrap().layer(), s.views[0].layer), (1, 1));
    assert_eq!(s.views[0].counter, 0);
    // só o cenário: sem objetos, decalques nem o Jake
    s.entities.clear();
    s.jake_sprite = None;
    for sp in s.sprites.iter_mut() {
        if sp.layer == openhunted_engine::Layer::Decoration {
            sp.x = -100_000;
        }
    }
    let v = view(&s);
    let (d1, d0) = (v.layer_distance(1), v.layer_distance(0));
    assert_eq!((d1, d0), (23, 34));
    let fb = render(&s, &mut a, &v);
    let front = cells_alone(&s, &mut a, &l, &[(3, 2, 1)], &v, d1, Blend::Opaque);
    let around: Vec<_> = (0..9)
        .flat_map(|x| (0..4).map(move |y| (x, y, 0)))
        .collect();
    let back = cells_alone(&s, &mut a, &l, &around, &v, d0, Blend::Behind);
    let t27 = *a.dls().unwrap().row(depth_shadow(1)).unwrap();
    // buraco (56, 60, 29×40) da célula (3, 2, 1), aumentado 1 px
    let hole = v.project_rect(
        Rect {
            x: 3 * 128 + 55,
            y: 2 * 128 + 59,
            w: 31,
            h: 42,
        },
        d1,
    );
    let cell = v.project_rect(
        Rect {
            x: 3 * 128,
            y: 2 * 128,
            w: 128,
            h: 128,
        },
        d1,
    );
    let within = |r: &Rect, x: i32, y: i32| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h;
    let (mut seen, mut checked) = (0, 0);
    for y in cell.y.max(0)..(cell.y + cell.h).min(405) {
        for x in cell.x.max(0)..(cell.x + cell.w).min(640) {
            let i = (y * 640 + x) as usize;
            if front.pixels[i] != 0 {
                continue;
            }
            checked += 1;
            if within(&hole, x, y) {
                // o índice 0 da frente, dentro do buraco: a camada 0 pela tabela 27
                let want = t27[usize::from(back.pixels[i])];
                assert_eq!(fb.pixels[i], want, "pixel ({x}, {y})");
                seen += usize::from(want != 0);
            } else {
                // fora do buraco o 0 da frente fica 0 (preto)
                assert_eq!(fb.pixels[i], 0, "pixel ({x}, {y}) fora do buraco");
            }
        }
    }
    assert!(
        checked > 0 && seen > 300,
        "{checked} conferidos, {seen} da camada de trás"
    );
}

#[test]
fn objects_on_other_layers_show_only_through_holes_and_darkened() {
    use openhunted_engine::input::DOWN;
    let Some((mut s, mut a, _)) = at_layer_door() else {
        return;
    };
    run(&mut s, 8, DOWN);
    run(&mut s, 10, 0);
    s.jake_sprite = None;
    let v = view(&s);
    // um objeto com sprite, posto no buraco da porta
    let k = s
        .entities
        .iter()
        .position(|e| e.sprite.is_some() && matches!(e.behaviour, Behaviour::Mine(_)))
        .unwrap();
    for (i, e) in s.entities.iter_mut().enumerate() {
        e.alive = i == k;
    }
    let base = {
        let mut t = s.clone();
        t.entities[k].alive = false;
        render(&t, &mut a, &v)
    };
    let place = |s: &mut Scene, layer: i32, at: (i32, i32)| {
        let e = &mut s.entities[k];
        (e.x, e.y, e.layer) = (at.0, at.1, layer);
        let sp = e.sprite.as_mut().unwrap();
        (sp.x, sp.y) = at;
    };
    let diff = |fb: &Framebuffer| -> Vec<(usize, u8)> {
        fb.pixels
            .iter()
            .zip(&base.pixels)
            .enumerate()
            .filter(|(_, (p, q))| p != q)
            .map(|(i, (p, _))| (i, *p))
            .collect()
    };
    let door = (3 * 128 + 60, 2 * 128 + 70);
    // na camada da vista: aparece, sem escurecer
    place(&mut s, 1, door);
    assert!(!diff(&render(&s, &mut a, &v)).is_empty());
    // na camada da frente (2): nunca
    place(&mut s, 2, door);
    assert!(
        diff(&render(&s, &mut a, &v)).is_empty(),
        "camada à frente desenhada"
    );
    // na camada de trás, longe do buraco: escondido pela frente
    place(&mut s, 0, (3 * 128 - 200, 2 * 128 + 70));
    assert!(
        diff(&render(&s, &mut a, &v)).is_empty(),
        "atrás e fora do buraco"
    );
    // na camada de trás, no buraco: só dentro dele e pela tabela 27
    place(&mut s, 0, door);
    let d = diff(&render(&s, &mut a, &v));
    assert!(!d.is_empty(), "atrás, no buraco, não apareceu");
    let hole = v.project_rect(
        Rect {
            x: 3 * 128 + 55,
            y: 2 * 128 + 59,
            w: 31,
            h: 42,
        },
        v.layer_distance(1),
    );
    let t27 = *a.dls().unwrap().row(depth_shadow(1)).unwrap();
    for (i, p) in d {
        let (x, y) = ((i % 640) as i32, (i / 640) as i32);
        assert!(
            x >= hole.x && x < hole.x + hole.w && y >= hole.y && y < hole.y + hole.h,
            "({x}, {y}) fora do buraco"
        );
        assert!(t27.contains(&p), "({x}, {y}) = {p} não sai da tabela 27");
    }
}

/// Descendo pela porta, a vista passa à camada 1 no 1º tique e a transição corre por
/// 7 tiques (`0x436b20`); o Jake fica no tamanho do começo.
#[test]
fn layer_door_runs_the_view_transition() {
    use openhunted_engine::input::DOWN;
    let Some((mut s, _, _)) = at_layer_door() else {
        return;
    };
    assert_eq!(s.views[0].layer, 0);
    let mut got = Vec::new();
    for _ in 0..8 {
        run(&mut s, 1, DOWN);
        let v = s.views[0];
        got.push((v.layer, v.counter));
    }
    assert!(matches!(
        s.jake.as_ref().unwrap().state(),
        State::LayerDown(_) | State::Idle
    ));
    assert_eq!(
        got,
        vec![
            (1, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (1, 6),
            (1, 0),
            (1, 0),
            (1, 0)
        ]
    );
    // `+0x4c` sem normalizar (20) e normalizado pela vista de 640×405 que desenha (23)
    assert_eq!(s.views[0].focus_distance, 20);
    let screen = Rect {
        x: 0,
        y: 0,
        w: 640,
        h: 480,
    };
    assert_eq!(
        s.view_of(0, (0, 0), screen, (640, 405)).focus_distance(),
        23
    );
}
