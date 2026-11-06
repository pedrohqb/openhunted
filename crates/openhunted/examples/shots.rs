// SPDX-License-Identifier: GPL-3.0-or-later
//! Capturas do jogo sem janela: roda um roteiro de entrada pelas telas (menu, briefing,
//! partida com a barra de estado, pausa, resultado) e grava PNGs RGBA
//! (uso interno: `cargo run -p openhunted --example shots -- DADOS SAIDA`).

#[allow(dead_code)]
#[path = "../src/audio.rs"]
mod audio;
#[allow(dead_code)]
#[path = "../src/viewer.rs"]
mod viewer;

#[allow(dead_code)]
#[path = "../src/shell.rs"]
mod shell;

use std::path::Path;

use shell::{App, Held, Key, Mode};

fn save(app: &mut App, path: &Path) {
    let mut rgba = vec![0u8; 640 * 480 * 4];
    app.render_rgba(&mut rgba);
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut enc = png::Encoder::new(file, 640, 480);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
    println!("{}", path.display());
}

fn run(app: &mut App, ms: u32) {
    for _ in 0..ms / 50 {
        app.advance(50);
    }
}

fn hold(left: bool) -> Held {
    Held {
        left,
        ..Default::default()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(&args[2]);
    let mut app = App::load(Path::new(&args[1]), None).unwrap();
    run(&mut app, 300);
    save(&mut app, &out.join("menu.png"));
    // setas levam o ponteiro aos botões
    app.key_down(Key::Down);
    app.key_down(Key::Down);
    run(&mut app, 100);
    save(&mut app, &out.join("menu_nav.png"));
    app.key_down(Key::Up);
    app.key_down(Key::Up);
    app.key_down(Key::Enter);
    run(&mut app, 200);
    assert_eq!(app.mode(), Mode::Briefing);
    run(&mut app, 300);
    save(&mut app, &out.join("briefing.png"));
    app.key_down(Key::Enter);
    run(&mut app, 200);
    assert_eq!(app.mode(), Mode::Play);
    // corre até a primeira dica abrir
    app.set_held(hold(true));
    let mut ms = 0;
    let modal = |a: &App| a.viewer().is_some_and(|v| v.modal().is_some());
    while !modal(&app) && ms < 60_000 {
        app.advance(50);
        ms += 50;
    }
    save(&mut app, &out.join("hint.png"));
    run(&mut app, 600);
    app.key_up();
    app.key_down(Key::Pause);
    save(&mut app, &out.join("pause.png"));
    app.key_down(Key::Enter);
    app.set_held(hold(true));
    while !modal(&app) && ms < 120_000 {
        app.advance(50);
        app.set_held(hold(true));
        ms += 50;
    }
    save(&mut app, &out.join("hud.png"));
    run(&mut app, 600);
    app.key_up();
    while app
        .viewer()
        .and_then(|v| v.focus())
        .is_some_and(|(x, _)| x > 108)
    {
        app.set_held(hold(true));
        app.advance(50);
    }
    app.set_held(hold(false));
    run(&mut app, 300);
    app.key_down(Key::Up);
    let mut n = 0;
    while app.mode() != Mode::Results && n < 400 {
        app.advance(50);
        n += 1;
    }
    run(&mut app, 300);
    save(&mut app, &out.join("results_count.png"));
    run(&mut app, 2000);
    save(&mut app, &out.join("results.png"));
    // NEXT: briefing da próxima missão solo
    app.key_down(Key::Enter);
    run(&mut app, 300);
    save(&mut app, &out.join("briefing_next.png"));
}
