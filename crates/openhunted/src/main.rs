// SPDX-License-Identifier: GPL-3.0-or-later
//! OpenHunted: o jogo Hunter Hunted numa janela SDL3: menu principal, briefing, a partida
//! com a barra de estado e a tela de resultado.
//!
//! Uso: `openhunted --data <CD, pasta Hunter/ ou .iso> [--level T_J_001.LEV] [--frames N]`
//! Nas telas: mouse, setas (levam o ponteiro aos botões), Enter/espaço clicam, Esc cancela
//! (no menu, sai). No jogo: setas movem o Jake, espaço pula, Ctrl atira (segurando,
//! rajadas; com as setas, mira para cima e nas diagonais), CIMA usa a saída, P/Pause pausa;
//! qualquer tecla solta fecha a dica do tutorial. Os efeitos sonoros tocam pelo dispositivo
//! de áudio padrão (sem ele, o jogo segue mudo). Sem Jake na fase, as setas rolam a tela,
//! PageUp/PageDown pulam uma tela e Home/End vão às pontas. Tab mostra a colisão e os
//! objetos de lógica.

mod audio;
// partes da API usadas só pelos testes e pelo exemplo `shots`
#[allow(dead_code)]
mod shell;
#[allow(dead_code)]
mod viewer;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use openhunted_engine::{SCREEN_H, SCREEN_W};
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{Keycode, Scancode};
use sdl3::messagebox::{MessageBoxFlag, show_simple_message_box};
use sdl3::mouse::MouseButton;
use sdl3::pixels::PixelFormat;

use shell::{App, Held, Key};

struct Args {
    data: PathBuf,
    level: Option<String>,
    frames: Option<u64>,
}

fn parse_args() -> Result<Args, String> {
    let mut data = std::env::var_os("OPENHUNTED_DATA").map(PathBuf::from);
    let mut level = None;
    let mut frames = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--data" => data = it.next().map(PathBuf::from),
            "--level" => level = Some(it.next().ok_or("--level sem valor")?),
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

/// Tecla da SDL → tecla da máquina de telas.
fn key(k: Keycode) -> Key {
    match k {
        Keycode::Left => Key::Left,
        Keycode::Right => Key::Right,
        Keycode::Up => Key::Up,
        Keycode::Down => Key::Down,
        Keycode::Return | Keycode::KpEnter => Key::Enter,
        Keycode::Space => Key::Space,
        Keycode::Escape => Key::Escape,
        Keycode::P | Keycode::Pause => Key::Pause,
        Keycode::Tab => Key::Tab,
        Keycode::PageUp => Key::PageUp,
        Keycode::PageDown => Key::PageDown,
        Keycode::Home => Key::Home,
        Keycode::End => Key::End,
        _ => Key::Other,
    }
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let mut app = App::load(&args.data, args.level.as_deref()).map_err(|e| e.to_string())?;

    let sdl = sdl3::init().map_err(|e| e.to_string())?;
    // Som: sem dispositivo de áudio, avisa uma vez e segue mudo.
    match audio::SdlOut::open(&sdl) {
        Ok(out) => app.set_sound(audio::Sfx::new(Box::new(out))),
        Err(e) => eprintln!("aviso: sem áudio ({e}); o jogo segue sem som"),
    }
    let video = sdl.video().map_err(|e| e.to_string())?;
    let window = video
        .window("OpenHunted", SCREEN_W as u32 * 2, SCREEN_H as u32 * 2)
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
    let mut shown = String::new();

    'main: loop {
        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. } => break 'main,
                // sem foco: solta o mouse e pausa o jogo
                Event::Window {
                    win_event: WindowEvent::FocusLost,
                    ..
                } => app.focus_lost(),
                Event::KeyUp { .. } => app.key_up(),
                Event::KeyDown {
                    keycode: Some(k),
                    repeat: false,
                    ..
                } => app.key_down(key(k)),
                Event::MouseMotion { x, y, .. } => {
                    let (lx, ly) = to_logical(&canvas, x, y);
                    app.pointer_move(lx, ly);
                }
                // só o botão esquerdo clica
                Event::MouseButtonDown {
                    x,
                    y,
                    mouse_btn: MouseButton::Left,
                    ..
                }
                | Event::MouseButtonUp {
                    x,
                    y,
                    mouse_btn: MouseButton::Left,
                    ..
                } => {
                    let (lx, ly) = to_logical(&canvas, x, y);
                    app.pointer_move(lx, ly);
                    app.pointer_button(matches!(ev, Event::MouseButtonDown { .. }));
                }
                _ => {}
            }
        }
        if app.quit_requested() {
            break;
        }
        let keys = events.keyboard_state();
        let held = |sc: &[Scancode]| sc.iter().any(|&s| keys.is_scancode_pressed(s));
        app.set_held(Held {
            left: held(&[Scancode::Left]),
            right: held(&[Scancode::Right]),
            up: held(&[Scancode::Up]),
            down: held(&[Scancode::Down]),
            jump: held(&[Scancode::Space]),
            shoot: held(&[Scancode::LCtrl, Scancode::RCtrl]),
        });
        let status = app.status();
        if status != shown {
            let _ = canvas
                .window_mut()
                .set_title(&format!("OpenHunted — {status}"));
            shown = status;
        }
        let now = Instant::now();
        let ms = now.duration_since(last).as_millis().min(1000) as u32;
        last = now;
        app.advance(ms);
        app.render_rgba(&mut rgba);
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

/// Coordenadas da janela → tela lógica 640×480 (com a escala e as bordas).
fn to_logical(canvas: &sdl3::render::WindowCanvas, x: f32, y: f32) -> (i32, i32) {
    let (mut lx, mut ly) = (x, y);
    // SAFETY: o renderizador é válido enquanto o canvas existe; os ponteiros são locais.
    unsafe {
        sdl3::sys::render::SDL_RenderCoordinatesFromWindow(canvas.raw(), x, y, &mut lx, &mut ly);
    }
    (lx as i32, ly as i32)
}
