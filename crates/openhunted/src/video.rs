// SPDX-License-Identifier: GPL-3.0-or-later
//! Filmes (`docs/re/videos-musica.md`): nove AVI com vídeo Indeo 3.2 (320×240, 15 qps) e
//! áudio PCM de 8 bits estéreo a 22 050 Hz. Decisão do port: não há decodificador Indeo 3
//! em Rust; o `ffmpeg` do PATH decodifica (vídeo `rgb24` e áudio `s16le` por pipes). Sem
//! ele, o filme é pulado. O tocador (`Movie`) não depende do `ffmpeg`: lê quadros de um
//! `Decoder`, no tempo, e desenha ampliado 2× no centro da tela 640×480.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::{Arc, OnceLock};

use openhunted_engine::{SCREEN_H, SCREEN_W};

use crate::audio::Pcm;

/// Tamanho do quadro e ritmo dos filmes.
pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;
pub const FPS: u64 = 15;
/// Taxa do áudio pedido ao `ffmpeg`.
pub const AUDIO_RATE: u32 = 22_050;

/// Fonte de quadros RGB (3 bytes por pixel, 320×240).
pub trait Decoder {
    fn next_frame(&mut self) -> Option<Vec<u8>>;
}

/// Um filme tocando.
pub struct Movie {
    decoder: Box<dyn Decoder>,
    /// Áudio inteiro, tocado pela saída de música ao começar.
    pub audio: Option<Arc<Pcm>>,
    elapsed: u64,
    shown: u64,
    frame: Vec<u8>,
    done: bool,
}

impl Movie {
    pub fn new(decoder: Box<dyn Decoder>, audio: Option<Pcm>) -> Self {
        let mut m = Self {
            decoder,
            audio: audio.map(Arc::new),
            elapsed: 0,
            shown: 0,
            frame: vec![0; WIDTH * HEIGHT * 3],
            done: false,
        };
        m.advance(0);
        m
    }

    /// Avança o relógio: lê os quadros devidos (o quadro `n` aparece em `n/15` s); sem
    /// mais quadros, o filme acabou.
    pub fn advance(&mut self, ms: u32) {
        if self.done {
            return;
        }
        self.elapsed += u64::from(ms);
        let due = self.elapsed * FPS / 1000 + 1;
        while self.shown < due {
            match self.decoder.next_frame() {
                Some(f) if f.len() == WIDTH * HEIGHT * 3 => {
                    self.frame = f;
                    self.shown += 1;
                }
                _ => {
                    self.done = true;
                    break;
                }
            }
        }
    }

    /// Qualquer tecla, botão do mouse ou do joystick pula (`0x43a123`).
    pub fn skip(&mut self) {
        self.done = true;
    }

    pub fn done(&self) -> bool {
        self.done
    }

    /// Quadros lidos até agora.
    pub fn frames_shown(&self) -> u64 {
        self.shown
    }

    /// Desenha o quadro corrente ampliado 2×, centrado, em RGBA 640×480 (fundo preto).
    pub fn render_rgba(&self, out: &mut [u8]) {
        out.fill(0);
        for o in out.chunks_exact_mut(4) {
            o[3] = 255;
        }
        let ox = (SCREEN_W - 2 * WIDTH) / 2;
        let oy = (SCREEN_H - 2 * HEIGHT) / 2;
        for y in 0..2 * HEIGHT {
            let src = &self.frame[(y / 2) * WIDTH * 3..][..WIDTH * 3];
            let row = &mut out[((oy + y) * SCREEN_W + ox) * 4..][..2 * WIDTH * 4];
            for (x, px) in row.chunks_exact_mut(4).enumerate() {
                px[..3].copy_from_slice(&src[(x / 2) * 3..][..3]);
            }
        }
    }
}

/// Há um `ffmpeg` executável no PATH (verificado uma vez).
pub fn ffmpeg_available() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        Command::new("ffmpeg")
            .arg("-version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Começa um `ffmpeg` que lê `bytes` pela entrada padrão (numa thread) com os argumentos.
fn spawn(bytes: Option<Arc<Vec<u8>>>, args: &[&str]) -> Option<Child> {
    let mut child = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-nostdin"])
        .args(args)
        .stdin(if bytes.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    if let (Some(b), Some(mut stdin)) = (bytes, child.stdin.take()) {
        // o processo pode morrer antes de ler tudo (filme pulado): o erro é esperado
        std::thread::spawn(move || {
            let _ = stdin.write_all(&b);
        });
    }
    Some(child)
}

fn pcm_from_s16le(raw: &[u8]) -> Option<Pcm> {
    let samples: Vec<i16> = raw
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    (samples.len() >= 2).then_some(Pcm {
        rate: AUDIO_RATE,
        channels: 2,
        samples,
    })
}

/// Lê todo o áudio de um arquivo pelo `ffmpeg` (estéreo, 16 bits, 22 050 Hz).
pub fn ffmpeg_audio_file(path: &Path) -> Option<Pcm> {
    if !ffmpeg_available() {
        return None;
    }
    let p = path.to_str()?;
    let out = spawn(
        None,
        &[
            "-i", p, "-vn", "-f", "s16le", "-ac", "2", "-ar", "22050", "pipe:1",
        ],
    )?
    .wait_with_output()
    .ok()?;
    pcm_from_s16le(&out.stdout)
}

/// Decodificador de vídeo pelo `ffmpeg` (AVI pela entrada padrão).
pub struct Ffmpeg {
    child: Child,
    stdout: ChildStdout,
}

impl Decoder for Ffmpeg {
    fn next_frame(&mut self) -> Option<Vec<u8>> {
        let mut f = vec![0; WIDTH * HEIGHT * 3];
        self.stdout.read_exact(&mut f).ok()?;
        Some(f)
    }
}

impl Drop for Ffmpeg {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Abre um filme AVI (bytes do CD): o áudio inteiro primeiro, depois o vídeo em fluxo.
/// `None` sem `ffmpeg` ou se ele não decodifica.
pub fn open_ffmpeg(avi: Vec<u8>) -> Option<Movie> {
    if !ffmpeg_available() {
        return None;
    }
    let avi = Arc::new(avi);
    let audio = spawn(
        Some(avi.clone()),
        &[
            "-f", "avi", "-i", "pipe:0", "-vn", "-f", "s16le", "-ac", "2", "-ar", "22050", "pipe:1",
        ],
    )
    .and_then(|c| c.wait_with_output().ok())
    .and_then(|o| pcm_from_s16le(&o.stdout));
    let mut child = spawn(
        Some(avi),
        &[
            "-f", "avi", "-i", "pipe:0", "-an", "-r", "15", "-f", "rawvideo", "-pix_fmt", "rgb24",
            "pipe:1",
        ],
    )?;
    let stdout = child.stdout.take()?;
    let mut dec = Ffmpeg { child, stdout };
    // sem um primeiro quadro, não há filme
    let first = dec.next_frame()?;
    struct First(Option<Vec<u8>>, Ffmpeg);
    impl Decoder for First {
        fn next_frame(&mut self) -> Option<Vec<u8>> {
            self.0.take().or_else(|| self.1.next_frame())
        }
    }
    Some(Movie::new(Box::new(First(Some(first), dec)), audio))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Decodificador falso: `n` quadros, o quadro `i` pintado com o valor `i`.
    pub(crate) struct Fake {
        pub n: u8,
        pub i: u8,
    }

    impl Decoder for Fake {
        fn next_frame(&mut self) -> Option<Vec<u8>> {
            (self.i < self.n).then(|| {
                self.i += 1;
                vec![self.i - 1; WIDTH * HEIGHT * 3]
            })
        }
    }

    fn movie(n: u8) -> Movie {
        Movie::new(Box::new(Fake { n, i: 0 }), None)
    }

    #[test]
    fn frames_follow_fifteen_per_second() {
        let mut m = movie(30);
        assert_eq!(m.frames_shown(), 1, "o primeiro quadro já na abertura");
        m.advance(60);
        assert_eq!(m.frames_shown(), 1);
        m.advance(10);
        assert_eq!(m.frames_shown(), 2, "o 2º em 66,7 ms");
        m.advance(930);
        assert_eq!(m.frames_shown(), 16);
        assert!(!m.done());
        m.advance(5000);
        assert!(m.done(), "acabaram os quadros");
        assert_eq!(m.frames_shown(), 30);
    }

    #[test]
    fn renders_doubled_and_skips() {
        let mut m = movie(3);
        m.advance(70);
        let mut out = vec![9u8; SCREEN_W * SCREEN_H * 4];
        m.render_rgba(&mut out);
        assert_eq!(&out[..8], &[1, 1, 1, 255, 1, 1, 1, 255]);
        let last = out.len() - 4;
        assert_eq!(&out[last..], &[1, 1, 1, 255]);
        m.skip();
        assert!(m.done());
        m.advance(1000);
        assert_eq!(m.frames_shown(), 2, "pulado não lê mais");
    }

    #[test]
    fn short_or_empty_decoders_end_at_once() {
        struct Bad;
        impl Decoder for Bad {
            fn next_frame(&mut self) -> Option<Vec<u8>> {
                Some(vec![0; 10])
            }
        }
        assert!(Movie::new(Box::new(Bad), None).done());
        assert!(movie(0).done());
        assert!(pcm_from_s16le(&[1]).is_none());
        assert_eq!(pcm_from_s16le(&[1, 0, 2, 0]).unwrap().samples, [1, 2]);
    }
}
