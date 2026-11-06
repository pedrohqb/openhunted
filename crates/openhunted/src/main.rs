// SPDX-License-Identifier: GPL-3.0-or-later
//! OpenHunted: abre uma fase do Hunter Hunted numa janela SDL3, com animações e o Jake
//! controlável (M3a).
//!
//! Uso: `openhunted --data <CD, pasta Hunter/ ou .iso> [--level T_J_001.LEV] [--frames N]`
//! Teclas: setas movem o Jake, espaço pula, Ctrl atira (segurando, rajadas; com as setas,
//! mira para cima e nas diagonais) e CIMA usa a saída (a câmera o acompanha);
//! qualquer tecla solta fecha a dica do tutorial. Os efeitos sonoros tocam pelo dispositivo
//! de áudio padrão (sem ele, o jogo segue mudo). Sem Jake na fase, as setas
//! rolam a tela, PageUp/PageDown pulam uma tela e Home/End vão às pontas. Tab mostra a colisão e
//! os objetos de lógica, Esc sai.

mod audio;
mod viewer;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use openhunted_engine::{SCREEN_H, SCREEN_W};
use sdl3::event::Event;
use sdl3::keyboard::{Keycode, Scancode};
use sdl3::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl3::pixels::PixelFormat;

use viewer::{Action, Viewer};

struct Args {
    data: PathBuf,
    level: String,
    frames: Option<u64>,
}

fn parse_args() -> Result<Args, String> {
    let mut data = std::env::var_os("OPENHUNTED_DATA").map(PathBuf::from);
    let mut level = "T_J_001.LEV".to_string();
    let mut frames = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => data = it.next().map(PathBuf::from),
            "--level" => level = it.next().ok_or("--level sem valor")?,
            "--frames" => {
                frames = Some(
                    it.next()
                        .and_then(|n| n.parse().ok())
                        .ok_or("--frames precisa de um número")?,
                )
            }
            "-h" | "--help" => {
                return Err(
                    "uso: openhunted --data <CD|Hunter/|.iso> [--level ARQ.LEV] [--frames N]"
                        .into(),
                );
            }
            other => return Err(format!("argumento desconhecido: {other}")),
        }
    }
    let data = data.ok_or("informe --data (raiz do CD, pasta Hunter/ ou imagem .iso)")?;
    Ok(Args {
        data,
        level,
        frames,
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            let _ = show_simple_message_box(MessageBoxFlag::ERROR, "OpenHunted", &e, None);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let mut viewer = Viewer::load(&args.data, &args.level).map_err(|e| e.to_string())?;

    let sdl = sdl3::init().map_err(|e| e.to_string())?;
    // Som: sem dispositivo de áudio, avisa uma vez e segue mudo.
    match audio::SdlOut::open(&sdl) {
        Ok(out) => viewer.set_sound(audio::Sfx::new(Box::new(out))),
        Err(e) => eprintln!("aviso: sem áudio ({e}); o jogo segue sem som"),
    }
    let video = sdl.video().map_err(|e| e.to_string())?;
    let window = video
        .window(
            &format!("OpenHunted — {}", args.level),
            SCREEN_W as u32 * 2,
            SCREEN_H as u32 * 2,
        )
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas();
    canvas
        .set_logical_size(
            SCREEN_W as u32,
            SCREEN_H as u32,
            sdl3::sys::render::SDL_LOGICAL_PRESENTATION_INTEGER_SCALE,
        )
        .map_err(|e| e.to_string())?;
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_streaming(PixelFormat::RGBA32, SCREEN_W as u32, SCREEN_H as u32)
        .map_err(|e| e.to_string())?;
    texture.set_scale_mode(sdl3::render::ScaleMode::Nearest);
    let mut events = sdl.event_pump().map_err(|e| e.to_string())?;
    let mut rgba = vec![0u8; SCREEN_W * SCREEN_H * 4];
    let mut last = Instant::now();
    let mut frame: u64 = 0;
    let mut shown = (viewer.camera(), viewer.overlay(), String::new());

    'main: loop {
        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                Event::KeyUp { .. } => viewer.key_released(),
                Event::KeyDown {
                    keycode: Some(k),
                    repeat: false,
                    ..
                } => match k {
                    Keycode::Tab => viewer.apply(Action::ToggleOverlay),
                    // com a dica aberta, a tecla só a fecha (não vira pulo nem CIMA)
                    Keycode::Space if viewer.modal().is_none() => {
                        viewer.tap(openhunted_engine::input::JUMP)
                    }
                    Keycode::Up if viewer.modal().is_none() => {
                        viewer.tap(openhunted_engine::input::UP)
                    }
                    Keycode::PageUp => viewer.apply(Action::PageLeft),
                    Keycode::PageDown => viewer.apply(Action::PageRight),
                    Keycode::Home => viewer.apply(Action::Home),
                    Keycode::End => viewer.apply(Action::End),
                    _ => {}
                },
                _ => {}
            }
        }
        let keys = events.keyboard_state();
        if viewer.has_jake() {
            // Setas e espaço controlam o Jake (bits como no original).
            use openhunted_engine::input::{DOWN, JUMP, LEFT, RIGHT, SHOOT, UP};
            let mut bits = 0;
            for (sc, bit) in [
                (Scancode::Left, LEFT),
                (Scancode::Right, RIGHT),
                (Scancode::Up, UP),
                (Scancode::Down, DOWN),
                (Scancode::Space, JUMP),
                (Scancode::LCtrl, SHOOT),
                (Scancode::RCtrl, SHOOT),
            ] {
                if keys.is_scancode_pressed(sc) {
                    bits |= bit;
                }
            }
            viewer.set_input(bits);
        } else {
            for (sc, act) in [
                (Scancode::Left, Action::ScrollLeft),
                (Scancode::Right, Action::ScrollRight),
                (Scancode::Up, Action::ScrollUp),
                (Scancode::Down, Action::ScrollDown),
            ] {
                if keys.is_scancode_pressed(sc) {
                    viewer.apply(act);
                }
            }
        }
        let status = match (viewer.outcome(), viewer.modal()) {
            (Some(openhunted_engine::Outcome::Victory), _) => " — missão cumprida".to_string(),
            (Some(openhunted_engine::Outcome::Defeat), _) => " — Jake morreu".to_string(),
            (None, Some(_)) => " — dica (solte uma tecla)".to_string(),
            (None, None) => String::new(),
        };
        if (viewer.camera(), viewer.overlay(), status.clone()) != shown {
            shown = (viewer.camera(), viewer.overlay(), status);
            let mode = if shown.1 { " — colisão" } else { "" };
            let jake = viewer
                .focus()
                .map_or(String::new(), |(x, y)| format!(" — Jake {x},{y}"));
            let title = format!(
                "OpenHunted — {} — x={} y={}{mode}{jake}{}",
                args.level, shown.0.0, shown.0.1, shown.2
            );
            let _ = canvas.window_mut().set_title(&title);
        }
        let now = Instant::now();
        let ms = now.duration_since(last).as_millis().min(1000) as u32;
        last = now;
        viewer.advance(ms);
        viewer.render_rgba(&mut rgba);
        texture
            .update(None, &rgba, SCREEN_W * 4)
            .map_err(|e| e.to_string())?;
        canvas.clear();
        canvas
            .copy(&texture, None, None)
            .map_err(|e| e.to_string())?;
        canvas.present();
        frame += 1;
        if args.frames.is_some_and(|n| frame >= n) {
            break;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    Ok(())
}
