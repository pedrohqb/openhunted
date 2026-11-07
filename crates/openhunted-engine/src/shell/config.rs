// SPDX-License-Identifier: GPL-3.0-or-later
//! Configuração no formato do `HUNTER.INI` (`docs/re/opcoes-salvamento.md`): seção
//! `[Hunter]`, chaves `Difficulty`, `MusicVolume`, `SfxVolume`, `Brightness`,
//! `NoMusicStart` e a do port `MusicDir`. A leitura e a escrita são puras; o arquivo (na
//! pasta de configuração do usuário) fica com a aplicação.

/// Seção (`0x44a51c`: o nome do executável sem extensão).
pub const SECTION: &str = "Hunter";
/// Padrões (`0x43d33b`).
pub const DIFFICULTY: i32 = 1;
pub const MUSIC_VOLUME: i32 = 25;
pub const SFX_VOLUME: i32 = 75;
pub const BRIGHTNESS: i32 = 1402;
/// Volume abaixo do qual o som está desligado (`0x49730c`/`0x497310`).
pub const VOLUME_OFF: i32 = 10;
/// Faixa do brilho (comandos 1402–1406 → `WORLD_G1…G5.PAL`).
pub const BRIGHTNESS_MIN: i32 = 1402;
pub const BRIGHTNESS_MAX: i32 = 1406;

/// Filmes (`docs/re/videos-musica.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Film {
    /// Abertura (`hh*.avi`).
    Intro,
    /// 7ª peça do carro (`mid*.avi`).
    Middle,
    /// 15ª peça (`out*.avi`).
    End,
}

/// Opções do jogo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// 0 fácil, 1 médio, 2 difícil.
    pub difficulty: i32,
    /// 0–100.
    pub music_volume: i32,
    pub sfx_volume: i32,
    /// 1402–1406.
    pub brightness: i32,
    pub no_music_start: i32,
    /// Pasta das faixas de música (extensão do port: o CD de áudio não existe na cópia).
    pub music_dir: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            difficulty: DIFFICULTY,
            music_volume: MUSIC_VOLUME,
            sfx_volume: SFX_VOLUME,
            brightness: BRIGHTNESS,
            no_music_start: 0,
            music_dir: None,
        }
    }
}

/// Divide `chave=valor` (espaços em volta ignorados).
fn key_value(line: &str) -> Option<(&str, &str)> {
    let (k, v) = line.split_once('=')?;
    Some((k.trim(), v.trim()))
}

/// Nome de seção de uma linha `[Nome]`.
fn section(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[')?.strip_suffix(']').map(str::trim)
}

impl Config {
    /// Lê a seção `[Hunter]`; chaves ausentes ou fora da faixa ficam no padrão (a
    /// dificuldade fora de 0–2 vira 1, `0x43dfef`).
    pub fn parse(text: &str) -> Self {
        let mut c = Self::default();
        let mut inside = false;
        for line in text.lines() {
            if let Some(s) = section(line) {
                inside = s.eq_ignore_ascii_case(SECTION);
                continue;
            }
            if !inside {
                continue;
            }
            let Some((k, v)) = key_value(line) else {
                continue;
            };
            let n = v.parse::<i32>().ok();
            match k.to_ascii_lowercase().as_str() {
                "difficulty" => c.difficulty = n.filter(|d| (0..=2).contains(d)).unwrap_or(1),
                "musicvolume" => {
                    c.music_volume = n.filter(|v| (0..=100).contains(v)).unwrap_or(MUSIC_VOLUME)
                }
                "sfxvolume" => {
                    c.sfx_volume = n.filter(|v| (0..=100).contains(v)).unwrap_or(SFX_VOLUME)
                }
                "brightness" => {
                    c.brightness = n
                        .filter(|b| (BRIGHTNESS_MIN..=BRIGHTNESS_MAX).contains(b))
                        .unwrap_or(BRIGHTNESS)
                }
                "nomusicstart" => c.no_music_start = n.unwrap_or(0),
                "musicdir" => c.music_dir = Some(v.to_string()).filter(|s| !s.is_empty()),
                _ => {}
            }
        }
        c
    }

    fn pairs(&self) -> Vec<(&'static str, String)> {
        let mut v = vec![
            ("Difficulty", self.difficulty.to_string()),
            ("MusicVolume", self.music_volume.to_string()),
            ("SfxVolume", self.sfx_volume.to_string()),
            ("Brightness", self.brightness.to_string()),
            ("NoMusicStart", self.no_music_start.to_string()),
        ];
        if let Some(d) = &self.music_dir {
            v.push(("MusicDir", d.clone()));
        }
        v
    }

    /// Texto do arquivo com estes valores: troca as chaves conhecidas da seção `[Hunter]`
    /// de `original` e acrescenta as que faltam; o resto (outras chaves e seções) fica.
    pub fn write(&self, original: &str) -> String {
        let pairs = self.pairs();
        let mut done = vec![false; pairs.len()];
        let mut out: Vec<String> = Vec::new();
        let mut inside = false;
        let mut found = false;
        let flush = |out: &mut Vec<String>, done: &mut [bool]| {
            for (i, (k, v)) in pairs.iter().enumerate() {
                if !done[i] {
                    out.push(format!("{k}={v}"));
                    done[i] = true;
                }
            }
        };
        for line in original.lines() {
            if let Some(s) = section(line) {
                if inside {
                    flush(&mut out, &mut done);
                }
                inside = s.eq_ignore_ascii_case(SECTION);
                found |= inside;
                out.push(line.to_string());
                continue;
            }
            if inside {
                if let Some((k, _)) = key_value(line) {
                    let alias = |k: &str| {
                        if k.eq_ignore_ascii_case("SFXVolume") {
                            "SfxVolume".to_string()
                        } else {
                            k.to_string()
                        }
                    };
                    let k = alias(k);
                    if let Some(i) = pairs.iter().position(|(p, _)| p.eq_ignore_ascii_case(&k)) {
                        if !done[i] {
                            out.push(format!("{}={}", pairs[i].0, pairs[i].1));
                            done[i] = true;
                        }
                        continue;
                    }
                }
            }
            out.push(line.to_string());
        }
        if inside {
            flush(&mut out, &mut done);
        }
        if !found {
            out.push(format!("[{SECTION}]"));
            flush(&mut out, &mut done);
        }
        let mut s = out.join("\n");
        s.push('\n');
        s
    }

    /// Volume efetivo (0–100) da música: abaixo de 10, desligada.
    pub fn music_gain(&self) -> u8 {
        effective(self.music_volume)
    }

    pub fn sfx_gain(&self) -> u8 {
        effective(self.sfx_volume)
    }

    /// Paleta do jogo pelo brilho (`WORLD_G%d.PAL`, 1 a mais escura).
    pub fn palette(&self) -> String {
        let b = self.brightness.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX);
        format!("WORLD_G{}.PAL", b - (BRIGHTNESS_MIN - 1))
    }

    /// Arquivo do filme pelo brilho. Decisão do port: a mesma regra da abertura
    /// (`0x4399e0`: 1402 escuro, 1403–1404 normal, 1405–1406 claro) vale para os três
    /// filmes; a tabela do meio/final do original (`0x480488`) está invertida.
    pub fn movie(&self, film: Film) -> &'static str {
        let v = match self.brightness {
            ..=1402 => 0,
            1403 | 1404 => 1,
            _ => 2,
        };
        match film {
            Film::Intro => ["HHDARK.AVI", "HHNORMAL.AVI", "HHLIGHT.AVI"][v],
            Film::Middle => ["MIDDARK.AVI", "MIDMED.AVI", "MIDLITE.AVI"][v],
            Film::End => ["OUTDARK.AVI", "OUTMED.AVI", "OUTLITE.AVI"][v],
        }
    }
}

fn effective(v: i32) -> u8 {
    if v < VOLUME_OFF {
        0
    } else {
        v.clamp(0, 100) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_ranges() {
        let c = Config::parse("");
        assert_eq!(c, Config::default());
        let c = Config::parse(
            "[Other]\nDifficulty=2\n[hunter]\n difficulty = 7 \nMusicVolume=40\nSFXVolume=5\nBrightness=1405\nMusicDir=/m\n",
        );
        assert_eq!(
            c.difficulty, 1,
            "fora de 0–2 vira 1; a outra seção não vale"
        );
        assert_eq!((c.music_volume, c.sfx_volume, c.brightness), (40, 5, 1405));
        assert_eq!(c.music_dir.as_deref(), Some("/m"));
        assert_eq!((c.music_gain(), c.sfx_gain()), (40, 0), "< 10 desliga");
        assert_eq!(Config::parse("[Hunter]\nBrightness=99").brightness, 1402);
    }

    #[test]
    fn write_replaces_known_keys_and_keeps_the_rest() {
        let orig = "; comentário\n[Hunter]\nHunterX=10\nSFXVolume=20\n[Sierra]\nCDPath=.\n";
        let c = Config {
            difficulty: 2,
            sfx_volume: 60,
            ..Default::default()
        };
        let w = c.write(orig);
        assert!(
            w.starts_with("; comentário\n[Hunter]\nHunterX=10\nSfxVolume=60\n"),
            "{w}"
        );
        assert!(w.contains("Difficulty=2\n"));
        assert!(w.contains("[Sierra]\nCDPath=.\n"));
        assert_eq!(w.matches("SfxVolume").count(), 1);
        assert_eq!(Config::parse(&w), c);
        // sem arquivo: só a seção
        let w = c.write("");
        assert!(w.starts_with("[Hunter]\nDifficulty=2\n"), "{w}");
        assert_eq!(Config::parse(&w), c);
        // regravar não duplica
        assert_eq!(c.write(&w), w);
    }

    #[test]
    fn palette_and_movie_follow_brightness() {
        let mut c = Config::default();
        assert_eq!(c.palette(), "WORLD_G1.PAL");
        assert_eq!(c.movie(Film::Intro), "HHDARK.AVI");
        assert_eq!(
            c.movie(Film::Middle),
            "MIDDARK.AVI",
            "não a tabela invertida"
        );
        c.brightness = 1404;
        assert_eq!(c.movie(Film::End), "OUTMED.AVI");
        c.brightness = 1406;
        assert_eq!(c.palette(), "WORLD_G5.PAL");
        assert_eq!(c.movie(Film::Intro), "HHLIGHT.AVI");
        assert_eq!(c.movie(Film::Middle), "MIDLITE.AVI");
    }
}
