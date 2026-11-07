// SPDX-License-Identifier: GPL-3.0-or-later
//! Música (`docs/re/videos-musica.md`): `SX_3000.WAV` em laço nas telas cheias e, na partida,
//! as faixas do CD em sequência com volta à primeira. A cópia do CD não tem as faixas de
//! áudio: decisão do port, elas vêm de uma pasta (`OPENHUNTED_MUSIC` ou `MusicDir` do
//! `hunter.ini`), em ordem de nome.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use openhunted_formats::Sound;

use crate::audio::{Pcm, SoundOut};

/// Música dos menus (classe 3, `0x449978`).
pub const MENU_ID: u16 = 3000;
/// Prioridade da música dos menus (`0x402d48`).
pub const MENU_PRIORITY: u16 = 5000;
/// Id da faixa na saída (2000 + faixa no original; aqui uma por vez).
pub const TRACK_ID: u16 = 2000;
/// Prioridade da faixa (`play tbCDaudio`, 30 000).
pub const TRACK_PRIORITY: u16 = 30_000;
/// Áudio dos filmes e vozes da sequência do meio.
pub const FILM_ID: u16 = 1;
/// Extensões aceitas na pasta de faixas (as que não são `.wav` passam pelo `ffmpeg`).
const EXTENSIONS: [&str; 7] = ["wav", "ogg", "mp3", "flac", "opus", "m4a", "aac"];

/// Faixas da pasta, em ordem de nome.
pub fn list_tracks(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    v.sort();
    v
}

/// Número da faixa pelo nome do arquivo (primeiro grupo de dígitos: `track02.wav` → 2).
pub fn track_number(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_str()?;
    let digits: String = stem
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Nome da faixa: faixa *t* (2–21) → texto `10010 + t` do `SYSTEM.RES` ("Shock Wave"…);
/// senão o nome do arquivo.
pub fn track_name(path: &Path, system: &[String]) -> String {
    track_number(path)
        .filter(|t| (2..=21).contains(t))
        .and_then(|t| system.get(9 + t as usize).cloned())
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
}

/// Lê um arquivo de áudio: `.wav` direto, os demais pelo `ffmpeg`.
pub fn decode_file(path: &Path) -> Option<Pcm> {
    let wav = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
    if wav {
        if let Some(p) = std::fs::read(path)
            .ok()
            .and_then(|d| Sound::parse(&d).ok())
            .and_then(|s| Pcm::decode(&s))
        {
            return Some(p);
        }
    }
    crate::video::ffmpeg_audio_file(path)
}

/// A música do jogo numa saída própria.
pub struct Music {
    out: Box<dyn SoundOut>,
    menu: Option<Arc<Pcm>>,
    tracks: Vec<PathBuf>,
    current: Option<usize>,
    /// Faixas que não decodificaram (não tenta de novo).
    bad: Vec<bool>,
    gain: u8,
    decode: fn(&Path) -> Option<Pcm>,
}

impl Music {
    pub fn new(out: Box<dyn SoundOut>, menu: Option<Arc<Pcm>>, tracks: Vec<PathBuf>) -> Self {
        let bad = vec![false; tracks.len()];
        Self {
            out,
            menu,
            tracks,
            current: None,
            bad,
            gain: 100,
            decode: decode_file,
        }
    }

    /// Troca o decodificador das faixas (testes).
    pub fn with_decoder(mut self, decode: fn(&Path) -> Option<Pcm>) -> Self {
        self.decode = decode;
        self
    }

    /// Volume da música (0–100; 0 = desligada).
    pub fn set_gain(&mut self, gain: u8) {
        self.gain = gain;
        self.out.set_master(gain);
    }

    /// Faixa corrente.
    pub fn current(&self) -> Option<&Path> {
        self.current.map(|i| self.tracks[i].as_path())
    }

    /// Telas cheias (modo de janela 1): para a faixa e toca a música dos menus em laço, sem
    /// recomeçar se já toca.
    pub fn menu(&mut self) {
        self.out.stop(TRACK_ID);
        self.out.stop(FILM_ID);
        if self.gain == 0 {
            return;
        }
        if let Some(p) = &self.menu {
            if !self.out.is_playing(MENU_ID) {
                self.out.play_looped(MENU_ID, p.clone(), MENU_PRIORITY, 100);
            }
        }
    }

    /// Partida: para a música dos menus; quando nenhuma faixa toca, começa a próxima (da
    /// última volta à primeira). Devolve o caminho da faixa que começou.
    pub fn game(&mut self) -> Option<PathBuf> {
        self.out.stop(MENU_ID);
        if self.gain == 0 || self.tracks.is_empty() || self.out.is_playing(TRACK_ID) {
            return None;
        }
        let n = self.tracks.len();
        for _ in 0..n {
            let i = self.current.map_or(0, |c| (c + 1) % n);
            self.current = Some(i);
            if self.bad[i] {
                continue;
            }
            match (self.decode)(&self.tracks[i]) {
                Some(p) => {
                    self.out.play(TRACK_ID, Arc::new(p), TRACK_PRIORITY, 100, 0);
                    return Some(self.tracks[i].clone());
                }
                None => {
                    eprintln!("aviso: faixa ilegível: {}", self.tracks[i].display());
                    self.bad[i] = true;
                }
            }
        }
        None
    }

    /// Para tudo (filmes e a tela preta da sequência).
    pub fn silence(&mut self) {
        for id in [MENU_ID, TRACK_ID, FILM_ID] {
            self.out.stop(id);
        }
    }

    /// Toca o áudio de um filme ou uma voz, a volume cheio (decisão do port).
    pub fn play_film(&mut self, pcm: Arc<Pcm>) {
        self.silence();
        self.out.set_master(100);
        self.out.play(FILM_ID, pcm, TRACK_PRIORITY, 100, 0);
    }

    /// Fim do filme ou da voz: para e volta ao volume da música.
    pub fn end_film(&mut self) {
        self.out.stop(FILM_ID);
        self.out.set_master(self.gain);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::Mixer;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Registro: o que tocou, o que está tocando e o volume mestre.
    type Log = (Vec<String>, Vec<u16>, u8);

    /// Saída que registra e deixa o teste dizer o que acabou.
    #[derive(Clone, Default)]
    struct Fake(Rc<RefCell<Log>>);

    impl SoundOut for Fake {
        fn play(&mut self, id: u16, _: Arc<Pcm>, _: u16, _: u8, _: i16) {
            let mut s = self.0.borrow_mut();
            s.0.push(format!("play {id}"));
            s.1.push(id);
        }
        fn play_looped(&mut self, id: u16, _: Arc<Pcm>, _: u16, _: u8) {
            let mut s = self.0.borrow_mut();
            s.0.push(format!("loop {id}"));
            s.1.push(id);
        }
        fn stop(&mut self, id: u16) {
            self.0.borrow_mut().1.retain(|&i| i != id);
        }
        fn is_playing(&mut self, id: u16) -> bool {
            self.0.borrow().1.contains(&id)
        }
        fn set_master(&mut self, v: u8) {
            self.0.borrow_mut().2 = v;
        }
    }

    fn pcm() -> Arc<Pcm> {
        Arc::new(Pcm {
            rate: 11025,
            channels: 1,
            samples: vec![1, 2],
        })
    }

    fn fake_decode(p: &Path) -> Option<Pcm> {
        (!p.to_string_lossy().contains("bad")).then(|| Pcm {
            rate: 11025,
            channels: 1,
            samples: vec![1],
        })
    }

    #[test]
    fn menu_music_loops_without_restarting() {
        let f = Fake::default();
        let mut m = Music::new(Box::new(f.clone()), Some(pcm()), vec![]);
        m.menu();
        m.menu();
        assert_eq!(f.0.borrow().0, ["loop 3000"]);
        assert_eq!(m.game(), None, "sem faixas");
        assert!(f.0.borrow().1.is_empty(), "a partida para a dos menus");
        m.set_gain(0);
        m.menu();
        assert!(f.0.borrow().1.is_empty(), "desligada");
        assert_eq!(f.0.borrow().2, 0);
    }

    #[test]
    fn tracks_play_in_sequence_and_wrap_around() {
        let f = Fake::default();
        let tracks: Vec<PathBuf> = ["a01.wav", "bad02.wav", "c03.wav"]
            .iter()
            .map(PathBuf::from)
            .collect();
        let mut m = Music::new(Box::new(f.clone()), Some(pcm()), tracks).with_decoder(fake_decode);
        assert_eq!(m.game(), Some(PathBuf::from("a01.wav")));
        assert_eq!(m.game(), None, "ainda toca");
        f.0.borrow_mut().1.clear(); // a faixa acabou
        assert_eq!(
            m.game(),
            Some(PathBuf::from("c03.wav")),
            "a ilegível é pulada"
        );
        f.0.borrow_mut().1.clear();
        assert_eq!(m.game(), Some(PathBuf::from("a01.wav")), "volta à primeira");
        m.menu();
        assert_eq!(f.0.borrow().1, [MENU_ID], "o menu para a faixa");
        m.play_film(pcm());
        assert_eq!(f.0.borrow().1, [FILM_ID]);
        assert_eq!(f.0.borrow().2, 100);
        m.set_gain(40);
        m.end_film();
        assert!(f.0.borrow().1.is_empty());
        assert_eq!(f.0.borrow().2, 40);
    }

    #[test]
    fn track_names_come_from_system_res() {
        let mut sys = vec![String::new(); 45];
        sys[11] = "Shock Wave".into();
        sys[20] = "Song #10".into();
        assert_eq!(track_name(Path::new("/m/track02.ogg"), &sys), "Shock Wave");
        assert_eq!(track_name(Path::new("Track11.wav"), &sys), "Song #10");
        assert_eq!(track_name(Path::new("intro.wav"), &sys), "intro");
        assert_eq!(track_number(Path::new("x/07 Time.flac")), Some(7));
    }

    #[test]
    fn lists_only_audio_files_sorted() {
        let d = std::env::temp_dir().join(format!("oh-music-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for n in ["b.WAV", "a.ogg", "notes.txt"] {
            std::fs::write(d.join(n), b"x").unwrap();
        }
        let l = list_tracks(&d);
        let names: Vec<_> = l.iter().map(|p| p.file_name().unwrap()).collect();
        assert_eq!(names, ["a.ogg", "b.WAV"]);
        std::fs::remove_dir_all(&d).unwrap();
        assert!(list_tracks(&d).is_empty());
        // com o mixer de verdade, o laço dos menus não acaba
        let mut mx = Mixer::new(11025);
        let mut m = Music::new(Box::new(Mixer::new(11025)), Some(pcm()), vec![]);
        m.menu();
        mx.play_looped(MENU_ID, pcm(), MENU_PRIORITY, 100);
        let mut out = [0i16; 64];
        mx.mix(&mut out);
        assert!(mx.is_playing(MENU_ID));
    }
}
