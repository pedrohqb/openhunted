// SPDX-License-Identifier: GPL-3.0-or-later
//! Capturas do visualizador sem janela: roda um roteiro de entrada e grava PNGs RGBA
//! (uso interno: `cargo run -p openhunted --example shots -- DADOS SAIDA`).

#[allow(dead_code)]
#[path = "../src/viewer.rs"]
mod viewer;

use std::path::Path;

fn save(v: &mut viewer::Viewer, path: &Path) {
    let mut rgba = vec![0u8; 640 * 480 * 4];
    v.render_rgba(&mut rgba);
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut enc = png::Encoder::new(file, 640, 480);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(&args[2]);
    let mut v = viewer::Viewer::load(Path::new(&args[1]), "T_J_001.LEV").unwrap();
    use openhunted_engine::input::{LEFT, UP};
    v.set_input(LEFT);
    // corre até a primeira dica abrir
    let mut ms = 0;
    while v.modal().is_none() && ms < 60_000 {
        v.advance(50);
        ms += 50;
    }
    save(&mut v, &out.join("hint.png"));
    v.advance(600);
    v.key_released();
    // corre até perto da saída
    while v.modal().is_none() && ms < 120_000 {
        v.advance(50);
        ms += 50;
    }
    save(&mut v, &out.join("hint2.png"));
    v.advance(600);
    v.key_released();
    while v.focus().is_some_and(|(x, _)| x > 108) {
        v.advance(50);
    }
    v.set_input(0);
    for _ in 0..6 {
        v.advance(50);
    }
    v.tap(UP);
    for _ in 0..70 {
        v.advance(50);
    }
    save(&mut v, &out.join("end.png"));
}
