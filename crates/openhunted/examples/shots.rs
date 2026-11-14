// SPDX-License-Identifier: GPL-3.0-or-later
//! Capturas do jogo sem janela: roda um roteiro de entrada pelas telas (menu, briefing,
//! partida com a barra de estado, pausa, resultado) e grava PNGs RGBA
//! (uso interno: `cargo run -p openhunted --example shots -- DADOS SAIDA`).

#[allow(dead_code)]
#[path = "../src/audio.rs"]
mod audio;
#[allow(dead_code)]
#[path = "../src/music.rs"]
mod music;
#[allow(dead_code)]
#[path = "../src/storage.rs"]
mod storage;
#[allow(dead_code)]
#[path = "../src/video.rs"]
mod video;
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
    m9(Path::new(&args[1]), out);
    m15(Path::new(&args[1]), out);
}

/// Telas da M15b: `SMULT`, lista da disputa, `SEGOC`, `SHVLD`, a partida dividida (disputa e
/// coop) e o `SH2HS`. Jogador 1 anda à direita (setas), o 2 à esquerda (J) no mesmo teclado.
fn m15(data: &Path, out: &Path) {
    use openhunted_engine::shell::{choose, levels, limits, menu};
    use shell::PageKind;
    let root = std::env::temp_dir().join("openhunted-shots-m15");
    let _ = std::fs::remove_dir_all(&root);
    let press = |app: &mut App, id: u16, name: Option<&str>| {
        let r = app.screen().unwrap().rect(id).unwrap();
        app.pointer_move(r.x + r.w / 2, r.y + r.h / 2);
        app.pointer_button(true);
        run(app, 50);
        app.pointer_button(false);
        run(app, 600);
        if let Some(name) = name {
            save(app, &out.join(format!("{name}.png")));
        }
    };
    let play = |app: &mut App, ms: u32| {
        for _ in 0..ms / 50 {
            app.read_input(&|vk| vk == 39 || vk == 74, 0);
            app.advance(50);
        }
    };
    let mut app = App::load(data, None).unwrap();
    app.set_storage(storage::Storage::at(&root));
    press(&mut app, menu::MULTI_PLAYER, Some("m15_smult"));
    press(&mut app, 13001, Some("m15_levels_dm"));
    press(&mut app, levels::LOAD, None);
    press(&mut app, choose::ids::RED, None);
    run(&mut app, 1500);
    save(&mut app, &out.join("m15_segoc.png"));
    press(&mut app, choose::ids::OK, None);
    press(&mut app, choose::ids::OK, None);
    assert_eq!(app.page_kind(), Some(PageKind::Limits));
    press(&mut app, limits::ids::TIME_UP, None);
    press(&mut app, limits::ids::KILLS_UP, None);
    press(&mut app, limits::ids::KILLS_UP, Some("m15_shvld"));
    press(&mut app, limits::ids::GO, None);
    assert_eq!(app.mode(), Mode::Play);
    play(&mut app, 3000);
    save(&mut app, &out.join("m15_duel.png"));
    // o limite de um minuto encerra a partida
    let mut n = 0;
    while app.mode() == Mode::Play && n < 2000 {
        play(&mut app, 50);
        n += 1;
    }
    run(&mut app, 3000);
    save(&mut app, &out.join("m15_sh2hs.png"));
    let mut b = App::load(data, None).unwrap();
    b.set_storage(storage::Storage::at(&root));
    press(&mut b, menu::MULTI_PLAYER, None);
    press(&mut b, 13002, None);
    press(&mut b, levels::LOAD, None);
    assert_eq!(b.mode(), Mode::Briefing);
    b.key_down(Key::Enter);
    run(&mut b, 200);
    play(&mut b, 3000);
    save(&mut b, &out.join("m15_coop.png"));
    let _ = std::fs::remove_dir_all(&root);
}

/// Telas da M9: opções, missões, heróis, nome, dispositivos, teclas, sobre, carro.
fn m9(data: &Path, out: &Path) {
    use openhunted_engine::shell::menu;
    let root = std::env::temp_dir().join("openhunted-shots");
    let _ = std::fs::remove_dir_all(&root);
    let mut app = App::load(data, None).unwrap();
    app.set_storage(storage::Storage::at(&root));
    let press = |app: &mut App, id: u16, name: &str| {
        let r = app.screen().unwrap().rect(id).unwrap();
        app.pointer_move(r.x + r.w / 2, r.y + r.h / 2);
        app.pointer_button(true);
        run(app, 50);
        app.pointer_button(false);
        run(app, 600);
        save(app, &out.join(format!("{name}.png")));
    };
    press(&mut app, menu::OPTIONS, "m9_options");
    app.key_down(Key::Escape);
    run(&mut app, 100);
    press(&mut app, menu::CHOOSE_LEVEL, "m9_levels");
    app.key_down(Key::Escape);
    run(&mut app, 100);
    press(&mut app, menu::HERO_LIST, "m9_heroes");
    app.key_down(Key::Escape);
    run(&mut app, 100);
    press(&mut app, menu::INPUT_DEVICES, "m9_input");
    press(&mut app, 29013, "m9_keys");
    app.key_down(Key::Escape);
    app.key_down(Key::Escape);
    run(&mut app, 100);
    press(&mut app, menu::HELP, "m9_about");
    app.key_down(Key::Escape);
    run(&mut app, 100);
    press(&mut app, menu::MULTI_PLAYER, "m9_multi");
    app.key_down(Key::Escape);
    run(&mut app, 100);
    let mut b = App::load(data, None).unwrap();
    b.set_storage(storage::Storage::at(&root));
    press(&mut b, menu::RESTORE, "m9_load");
    b.text_input("JAKE");
    run(&mut b, 100);
    save(&mut b, &out.join("m9_load_typed.png"));
    let _ = std::fs::remove_dir_all(&root);
    // carro com 7 peças e um quadro do filme (se houver `ffmpeg`)
    {
        use openhunted_engine::shell::heroes::Car;
        use openhunted_engine::shell::{Input, Page, Placement, Screen};
        let gd = openhunted_formats::open_data(data).unwrap();
        let mut assets = openhunted_engine::Assets::new(gd.resources().unwrap());
        let itf = openhunted_formats::Interface::parse(assets.archive().get("S_CAR.ITF").unwrap())
            .unwrap();
        let mut car = Car::new(Screen::new(&itf, &mut assets, Placement::Full).unwrap(), 7);
        for _ in 0..40 {
            car.handle(Input::Tick, &mut Vec::new());
        }
        let mut fb = openhunted_engine::Framebuffer::new(640, 480);
        car.draw(&mut fb);
        let pal = openhunted_engine::display_palette(
            &openhunted_formats::Palette::parse(assets.archive().get("WORLD.PAL").unwrap())
                .unwrap(),
        );
        let rgba: Vec<u8> = fb
            .pixels
            .iter()
            .flat_map(|&p| {
                let c = pal.0[p as usize];
                [c[0], c[1], c[2], 255]
            })
            .collect();
        write_png(&rgba, &out.join("m9_car.png"));
        if let Some(mut m) = video::open_ffmpeg(gd.read_movie("HHNORMAL.AVI").unwrap()) {
            m.advance(6000);
            let mut rgba = vec![0u8; 640 * 480 * 4];
            m.render_rgba(&mut rgba);
            write_png(&rgba, &out.join("m9_movie.png"));
            println!(
                "áudio: {:?} amostras",
                m.audio.as_ref().map(|a| a.samples.len())
            );
        }
    }
}

fn write_png(rgba: &[u8], path: &Path) {
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut enc = png::Encoder::new(file, 640, 480);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
    println!("{}", path.display());
}
